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
    scene_at([0.0, 300.0, 0.0])
}

fn scene_at(pos: [f64; 3]) -> Scene {
    let mut doc = Document::new(registry());
    let cloner = doc
        .transact("clones", |tx| {
            let plane = add_object(tx, &Obj::plane());
            let _ = plane;
            let c = tx.create("forge.cloner", None, None)?;
            tx.set_param(c, "mode", Value::Enum("grid".into()))?;
            tx.set_param(c, "grid_count", Value::Vec3([3.0, 1.0, 3.0]))?;
            tx.set_param(c, "grid_size", Value::Vec3([200.0; 3]))?;
            tx.set_param(c, "transform.pos", Value::Vec3(pos))?;
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

/// Reads `samples-<object>-<param>.json` of a dynamics case.
fn case_track(case: &str, object: &str, param: &str) -> Vec<f64> {
    track("dynamics", case, object, param)
}

/// `clones-fall-2` (cloner at (13, 300, 7), so no clone sits exactly on the plane's edge):
/// clone 4 lands like in `clones-fall`; clone 0 (centre 13 and 7 cm inside the corner) still
/// leaves the plane in the reference (spec §7).
#[test]
fn clones_fall_2_moved_cloner() {
    let s = scene_at([13.0, 300.0, 7.0]);
    // Clone 4.
    let ms = clone_track(&s, 4, 90);
    let (ey, oy) = (
        case_track("clones-fall-2", "clone4", "pos_y"),
        column(&ms, 1),
    );
    let free = (0..=22).map(|f| (ey[f] - oy[f]).abs()).fold(0.0, f64::max);
    assert!(free < 1.0, "free fall {free}");
    let th = Thresholds {
        rest_position: Some(0.05),
        first_contact_frames: Some(1.0),
        ..Default::default()
    };
    let rep = compare_trajectory(&ey, &oy, &th);
    assert!(rep.passed(), "{:?}", rep.failures());
    // Pinned: the reference ends 7.9 cm in -x and 3.05 cm in +z from its start (13, 7); ours
    // drifts 0.7 and 1.6 cm the other way (written tolerance: both stay within 8 cm of the start,
    // the observed drift, and ours is within 9 cm of the reference).
    let (ex, ez) = (
        case_track("clones-fall-2", "clone4", "pos_x")[90],
        case_track("clones-fall-2", "clone4", "pos_z")[90],
    );
    assert!((ex - 13.0).abs() > 1.0, "reference drift");
    assert!(
        ((ex - 13.0).powi(2) + (ez - 7.0).powi(2)).sqrt() < 8.5,
        "reference"
    );
    let (dx, dz) = (ms[90].w_axis.x - 13.0, ms[90].w_axis.z - 7.0);
    assert!((dx * dx + dz * dz).sqrt() < 8.0, "ours drifts {dx} {dz}");
    assert!((ms[90].w_axis.x - ex).abs() < 9.0 && (ms[90].w_axis.z - ez).abs() < 9.0);
    assert!(
        (ms[90].w_axis.x - ex).abs() > 1.0,
        "lateral drift matches now"
    );
    // Clone 0.
    let m0 = clone_track(&s, 0, 90);
    let (ey0, oy0) = (
        case_track("clones-fall-2", "clone0", "pos_y"),
        column(&m0, 1),
    );
    let free = (0..=22)
        .map(|f| (ey0[f] - oy0[f]).abs())
        .fold(0.0, f64::max);
    assert!(free < 1.0, "clone 0 free fall {free}");
    let th = Thresholds {
        first_contact_frames: Some(1.0),
        ..Default::default()
    };
    let rep = compare_trajectory(&ey0, &oy0, &th);
    assert!(rep.passed(), "{:?}", rep.failures());
    // Pinned (KNOWN_MISMATCH): the reference cube, whose centre is 13 and 7 cm inside the
    // plane's corner, is kicked outwards at the landing (about 4.7 cm per frame in x and in z)
    // and falls to y -1986.9; ours stays on the plane. "Exactly over the corner" did not
    // explain `clones-fall`: the cause is the reference's contact, not a balance.
    assert!(ey0[90] < -1000.0, "reference");
    assert!(
        oy0[90] > 0.0,
        "ours falls off now: remove the pin ({})",
        oy0[90]
    );
}

/// `collision-noise-0` (the body tag's Collision Noise 0 instead of 0.5 %, centre clone only):
/// the landing is the same; the sideways drift shrinks but does not vanish (spec §6, §7).
#[test]
fn collision_noise_zero_clone_drift() {
    let s = scene();
    let ms = clone_track(&s, 4, 90);
    let oy = column(&ms, 1);
    let ey = case_track("collision-noise-0", "clone4", "pos_y");
    let th = Thresholds {
        rest_position: Some(0.05),
        first_contact_frames: Some(1.0),
        ..Default::default()
    };
    let rep = compare_trajectory(&ey, &oy, &th);
    assert!(rep.passed(), "{:?}", rep.failures());
    let ex = case_track("collision-noise-0", "clone4", "pos_x")[90];
    let ez = case_track("collision-noise-0", "clone4", "pos_z")[90];
    let (nx, nz) = (
        case_track("clones-fall", "clone4", "pos_x")[90],
        case_track("clones-fall", "clone4", "pos_z")[90],
    );
    // The reference drifts without noise too (0.94, 0.50 cm against 1.87, 1.87 with the default
    // 0.5 %), so noise does not explain the drift. Ours has no noise and still drifts (-2.3, 1.6
    // cm: the cubes touch their neighbours' contacts differently).
    assert!(ex.abs() > 0.4 && ez.abs() > 0.4);
    assert!(ex.abs() < nx.abs() && ez.abs() < nz.abs());
    // Written tolerance: ours drifts at most 2.5 cm laterally and is within 4.5 cm of both
    // references (the drift directions differ: not derived, pinned as inexact).
    assert!(ms[90].w_axis.x.abs() < 2.5 && ms[90].w_axis.z.abs() < 2.5);
    for (gx, gz) in [(ex, ez), (nx, nz)] {
        assert!((ms[90].w_axis.x - gx).abs() < 4.5 && (ms[90].w_axis.z - gz).abs() < 4.5);
    }
    assert!(
        (ms[90].w_axis.x - ex).abs() > 1.0,
        "lateral drift matches now: remove the pin"
    );
}
