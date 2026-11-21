//! Soft body solver behaviour the golden cases cannot see (docs/specs/DYNAMICS.md §8).

mod common;

use common::*;
use forge_core::{EvalCtx, NodeId, Value};
use forge_dynamics::{BodyKey, DynamicsCache, RIGID_BODY};
use forge_generators::evaluate;
use glam::DVec3;

fn soft_scene(dynamic_tag_edit: impl Fn(&mut forge_core::Tx, NodeId)) -> (Built, NodeId) {
    let mut b = build(&[Obj::plane()]);
    let s = b
        .doc
        .transact("soft", |tx| {
            let s = tx.create("forge.sphere", None, None)?;
            tx.set_param(s, "radius", Value::Float(50.0))?;
            tx.set_param(s, "transform.pos", Value::Vec3([0.0, 200.0, 0.0]))?;
            let t = tx.attach_tag(RIGID_BODY, s, None)?;
            tx.set_param(t, "soft_body", Value::Enum("polygons".into()))?;
            dynamic_tag_edit(tx, t);
            Ok(s)
        })
        .unwrap();
    (b, s)
}

fn points(b: &Built, s: NodeId, frame: i64) -> Vec<DVec3> {
    let mesh = |id: NodeId| evaluate(b.doc.scene(), id, &EvalCtx::default()).unwrap();
    let mut c = DynamicsCache::build(b.doc.scene(), &mesh).unwrap();
    c.soft_at(frame)[&BodyKey::Node(s)].clone()
}

#[test]
fn a_soft_object_is_not_a_rigid_body() {
    let (b, s) = soft_scene(|_, _| {});
    let c = DynamicsCache::build(b.doc.scene(), &mesh_fn(&b.doc)).unwrap();
    assert!(!c.keys().contains(&BodyKey::Node(s)));
    // The shape is the evaluated mesh: 114 points for 16 segments.
    assert_eq!(points(&b, s, 0).len(), 114);
}

#[test]
fn the_start_shape_is_the_mesh_in_world_space() {
    let (b, s) = soft_scene(|_, _| {});
    let p = points(&b, s, 0);
    let mesh = evaluate(b.doc.scene(), s, &EvalCtx::default())
        .unwrap()
        .unwrap();
    for (a, m) in p.iter().zip(&mesh.points) {
        assert!((*a - (*m + DVec3::new(0.0, 200.0, 0.0))).length() < 1e-9);
    }
}

#[test]
fn two_runs_are_identical_and_scrubbing_does_not_change_frames() {
    let (b, s) = soft_scene(|_, _| {});
    assert_eq!(points(&b, s, 40), points(&b, s, 40));
    let mesh = |id: NodeId| evaluate(b.doc.scene(), id, &EvalCtx::default()).unwrap();
    let mut c = DynamicsCache::build(b.doc.scene(), &mesh).unwrap();
    let a = c.soft_at(40)[&BodyKey::Node(s)].clone();
    let _ = c.soft_at(5);
    assert_eq!(c.soft_at(40)[&BodyKey::Node(s)], a);
    // Jumping straight to a frame gives the same shape as stepping there (the reference only
    // simulates when stepped; ours simulates up to the frame).
    assert_eq!(points(&b, s, 40), a);
}

#[test]
fn the_body_lands_and_rests_above_the_plane_without_tunnelling() {
    let (b, s) = soft_scene(|_, _| {});
    let mesh = |id: NodeId| evaluate(b.doc.scene(), id, &EvalCtx::default()).unwrap();
    let mut c = DynamicsCache::build(b.doc.scene(), &mesh).unwrap();
    for f in 0..=90 {
        let lo = c.soft_at(f)[&BodyKey::Node(s)]
            .iter()
            .map(|p| p.y)
            .fold(f64::MAX, f64::min);
        assert!(
            lo > -0.01,
            "frame {f}: a particle is below the plane ({lo})"
        );
    }
    let end = &c.soft_at(90)[&BodyKey::Node(s)];
    let lo = end.iter().map(|p| p.y).fold(f64::MAX, f64::min);
    assert!((lo - 0.5).abs() < 0.2, "rests on the margin: {lo}");
    let speed =
        (c.soft_at(90)[&BodyKey::Node(s)][5] - c.soft_at(89)[&BodyKey::Node(s)][5]).length();
    assert!(speed < 0.5, "at rest: {speed} cm per frame");
}

#[test]
fn keep_shape_off_lets_the_body_sag_more() {
    let sag = |keep: bool| {
        let (b, s) = soft_scene(move |tx, t| {
            tx.set_param(t, "keep_shape", Value::Bool(keep)).unwrap();
        });
        let p = points(&b, s, 90);
        let (lo, hi) = (
            p.iter().map(|q| q.y).fold(f64::MAX, f64::min),
            p.iter().map(|q| q.y).fold(f64::MIN, f64::max),
        );
        hi - lo
    };
    assert!(
        sag(false) < sag(true) - 3.0,
        "{} vs {}",
        sag(false),
        sag(true)
    );
}

#[test]
fn zero_stiffness_everywhere_lets_the_body_collapse() {
    let (b, s) = soft_scene(|tx, t| {
        for k in ["structural", "shear", "flexion"] {
            tx.set_param(t, k, Value::Float(0.0)).unwrap();
        }
        tx.set_param(t, "keep_shape", Value::Bool(false)).unwrap();
    });
    let p = points(&b, s, 90);
    let hi = p.iter().map(|q| q.y).fold(f64::MIN, f64::max);
    assert!(hi < 5.0, "all particles are on the plane: {hi}");
}

#[test]
fn a_soft_body_lands_on_a_rigid_box_and_does_not_move_it() {
    let mut b = build(&[Obj::plane()]);
    let (s, bx) = b
        .doc
        .transact("scene", |tx| {
            let bx = add_object(tx, &{
                let mut o = Obj::cube("box", [0.0, 25.0, 0.0]);
                o.dynamic = "off";
                o
            });
            let s = tx.create("forge.sphere", None, None)?;
            tx.set_param(s, "radius", Value::Float(20.0))?;
            tx.set_param(s, "transform.pos", Value::Vec3([0.0, 120.0, 0.0]))?;
            let t = tx.attach_tag(RIGID_BODY, s, None)?;
            tx.set_param(t, "soft_body", Value::Enum("polygons".into()))?;
            Ok((s, bx))
        })
        .unwrap();
    let mesh = |id: NodeId| evaluate(b.doc.scene(), id, &EvalCtx::default()).unwrap();
    let mut c = DynamicsCache::build(b.doc.scene(), &mesh).unwrap();
    let end = c.soft_at(90)[&BodyKey::Node(s)].clone();
    let lo = end.iter().map(|p| p.y).fold(f64::MAX, f64::min);
    assert!((lo - 50.5).abs() < 1.0, "rests on the box top (y 50): {lo}");
    assert_eq!(c.at(90)[&BodyKey::Node(bx)].w_axis.y, 25.0);
}
