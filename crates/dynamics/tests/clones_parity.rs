//! Dynamics on clones against O39 `clones-fall` (docs/specs/DYNAMICS.md §6, §7).

mod common;

use common::*;
use forge_core::{Document, EvalCtx, NodeId, Value};
use forge_dynamics::{BodyKey, DynamicsCache, RIGID_BODY};
use forge_generators::{cloner_instances, evaluate};
use forge_parity_harness::{Thresholds, compare_trajectory};

struct Scene {
    doc: Document,
    cloner: NodeId,
}

fn scene() -> Scene {
    let mut doc = Document::new(registry());
    let cloner = doc
        .transact("clones", |tx| {
            let plane = add_object(tx, &Obj::plane());
            let _ = plane;
            let c = tx.create("forge.cloner", None, None)?;
            tx.set_param(c, "mode", Value::Enum("grid".into()))?;
            tx.set_param(c, "grid_count", Value::Vec3([3.0, 1.0, 3.0]))?;
            tx.set_param(c, "grid_size", Value::Vec3([200.0; 3]))?;
            tx.set_param(c, "transform.pos", Value::Vec3([0.0, 300.0, 0.0]))?;
            let cube = tx.create("forge.cube", Some(c), None)?;
            for a in ["size_x", "size_y", "size_z"] {
                tx.set_param(cube, a, Value::Float(30.0))?;
            }
            let t = tx.attach_tag(RIGID_BODY, c, None)?;
            tx.set_param(t, "bounce", Value::Float(0.05))?;
            Ok(c)
        })
        .unwrap();
    Scene { doc, cloner }
}

fn cache(s: &Scene) -> DynamicsCache {
    DynamicsCache::build(s.doc.scene(), &mesh_fn(&s.doc)).unwrap()
}

fn clone_track(s: &Scene, index: u32, frames: i64) -> Vec<glam::DMat4> {
    let mut c = cache(s);
    (0..=frames)
        .map(|f| {
            c.at(f)[&BodyKey::Clone {
                cloner: s.cloner,
                index,
            }]
        })
        .collect()
}

#[test]
fn every_visible_clone_is_a_body_and_the_cloner_itself_is_not() {
    let s = scene();
    let c = cache(&s);
    let clones = c
        .keys()
        .iter()
        .filter(|k| matches!(k, BodyKey::Clone { .. }))
        .count();
    assert_eq!(clones, 9);
    assert!(!c.keys().contains(&BodyKey::Node(s.cloner)));
    // Clone 4 starts at the cloner's position, clone 0 at the grid corner.
    let m4 = clone_track(&s, 4, 0)[0];
    assert!((m4.w_axis.truncate() - glam::DVec3::new(0.0, 300.0, 0.0)).length() < 1e-9);
    let m0 = clone_track(&s, 0, 0)[0];
    assert!((m0.w_axis.truncate() - glam::DVec3::new(-200.0, 300.0, -200.0)).length() < 1e-9);
}

#[test]
fn clone4_lands_on_the_plane_like_the_reference() {
    let s = scene();
    let ms = clone_track(&s, 4, 90);
    let ey = track("dynamics", "clones-fall", "clone4", "pos_y");
    let oy = column(&ms, 1);
    // The fall before the contact (frames 0..22) within 1 cm.
    let free = (0..=22).map(|f| (ey[f] - oy[f]).abs()).fold(0.0, f64::max);
    assert!(free < 1.0, "free fall {free}");
    let th = Thresholds {
        rest_position: Some(0.05),
        first_contact_frames: Some(1.0),
        ..Default::default()
    };
    let rep = compare_trajectory(&ey, &oy, &th);
    assert!(rep.passed(), "{:?}", rep.failures());
    // It stays upright (the reference tilts by less than 1 degree).
    let hpb = hpb_deg(&ms[90]);
    assert!(hpb[1].abs() < 1.0 && hpb[2].abs() < 1.0, "{hpb:?}");
    // Pinned: the reference drifts 1.87 cm sideways after landing, ours does not.
    let ex = track("dynamics", "clones-fall", "clone4", "pos_x")[90];
    assert!(
        (ms[90].w_axis.x - ex).abs() > 1.0,
        "lateral drift matches now"
    );
}

#[test]
fn clone0_on_the_plane_corner_touches_down_at_the_same_frame() {
    let s = scene();
    let ms = clone_track(&s, 0, 90);
    let ey = track("dynamics", "clones-fall", "clone0", "pos_y");
    let oy = column(&ms, 1);
    // The free fall before the contact, to 1 cm, and the contact frame.
    let free = (0..=22).map(|f| (ey[f] - oy[f]).abs()).fold(0.0, f64::max);
    assert!(free < 1.0, "free fall {free}");
    let th = Thresholds {
        first_contact_frames: Some(1.0),
        ..Default::default()
    };
    let rep = compare_trajectory(&ey, &oy, &th);
    assert!(rep.passed(), "{:?}", rep.failures());
    // Pinned (KNOWN_MISMATCH): the cube's centre is exactly over the plane's corner. The
    // reference tips off the plane and falls to y -2227.8; ours stays on it (y 15.08). Which
    // way a body balanced on an edge goes is decided by the engine's contact noise.
    assert!(ey[90] < -1000.0, "reference");
    assert!(
        oy[90] > 0.0,
        "ours falls off now: remove the pin ({})",
        oy[90]
    );
}

#[test]
fn results_write_back_into_the_clone_data() {
    let mut s = scene();
    let mut c = cache(&s);
    let poses = c.moving_at(60);
    let (working, skipped) = s
        .doc
        .scene()
        .with_params(forge_dynamics::apply::overrides(s.doc.scene(), &poses));
    assert!(skipped.is_empty(), "{skipped:?}");
    let inst = cloner_instances(&working, s.cloner, &EvalCtx::default())
        .unwrap()
        .unwrap();
    let world = working.world_matrix(s.cloner).unwrap();
    for i in 0..9u32 {
        let want = poses[&BodyKey::Clone {
            cloner: s.cloner,
            index: i,
        }];
        let got = world * inst.clones.matrices[i as usize];
        assert!(
            (got.w_axis - want.w_axis).length() < 1e-6,
            "clone {i}: {:?} vs {:?}",
            got.w_axis,
            want.w_axis
        );
    }
    // The expanded geometry follows: 9 cubes of 8 points.
    let mesh = evaluate(&working, s.cloner, &EvalCtx::default())
        .unwrap()
        .unwrap();
    assert_eq!(mesh.points.len(), 72);
    let _ = &mut s;
}

#[test]
fn clone_simulation_is_deterministic() {
    let s = scene();
    let a = clone_track(&s, 4, 40);
    let b = clone_track(&s, 4, 40);
    assert_eq!(a, b);
}
