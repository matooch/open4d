//! Connectors and springs against O41 `hinge`, `ball-socket` and `spring`
//! (docs/specs/DYNAMICS.md §3, §7).

mod common;

use common::*;
use forge_core::Value;
use glam::DVec3;

/// The worst frame differs by 6.6 cm of a 600 cm swing at about 150 cm per frame: a timing
/// error of 0.04 frame (integration scheme), see the spec.
const POS_TOL: f64 = 7.0;
const ANGLE_TOL: f64 = 3.0;

fn angle_diff(a: f64, b: f64) -> f64 {
    let d = (a - b).rem_euclid(360.0);
    d.min(360.0 - d)
}

/// The hinge scene of the cases: left cube static at (-26, 300, 0), right cube dynamic at
/// (26, 300, 0) (their `mesh.obj` and frame-0 samples), a connector at the origin.
fn hinge_scene(kind: &str) -> (Built, forge_core::NodeId) {
    let mut left = Obj::cube("left", [-26.0, 300.0, 0.0]);
    left.dynamic = "off";
    let right = Obj::cube("right", [26.0, 300.0, 0.0]);
    let mut b = build(&[Obj::plane(), left, right]);
    let (l, r) = (b.ids["left"], b.ids["right"]);
    b.doc
        .transact("connector", |tx| {
            Ok(add_connector(tx, kind, Some(l), Some(r)))
        })
        .unwrap();
    (b, r)
}

fn check_swing(case: &str, kind: &str) {
    let (b, r) = hinge_scene(kind);
    let sim = simulate(&b, 90);
    let ms = &sim[&r];
    let ex = track("soft-bodies", case, "right", "pos_x");
    let ey = track("soft-bodies", case, "right", "pos_y");
    let ez = track("soft-bodies", case, "right", "pos_z");
    let eb = track("soft-bodies", case, "right", "rot_b");
    for f in 0..=90 {
        let m = &ms[f];
        let d = (m.w_axis.x - ex[f]).abs().max((m.w_axis.y - ey[f]).abs());
        assert!(d < POS_TOL, "{case} frame {f}: off by {d:.2} cm");
        assert!(
            m.w_axis.z.abs() < 0.5 && ez[f].abs() < 1e-9,
            "{case} frame {f}: left the plane"
        );
        let bank = hpb_deg(m)[2];
        assert!(
            angle_diff(bank, eb[f]) < ANGLE_TOL,
            "{case} frame {f}: B {bank} vs {}",
            eb[f]
        );
        // The pivot is the connector's position: the cube keeps its distance from it (301 cm).
        let r = DVec3::new(m.w_axis.x, m.w_axis.y, 0.0).length();
        assert!((r - 301.0).abs() < 1.5, "{case} frame {f}: radius {r}");
    }
}

#[test]
fn hinge_swings_about_the_connector_like_the_reference() {
    check_swing("hinge", "hinge");
}

#[test]
fn ball_socket_gives_the_same_track_in_this_planar_scene() {
    // The reference's ball-socket track is bit-identical to its hinge track (observation note:
    // the type change does not reach its solver). Ours is a spherical joint: it matches the same
    // track because nothing pushes the cube out of the plane.
    check_swing("ball-socket", "ball_socket");
    let (b, r) = hinge_scene("hinge");
    let (c, rc) = hinge_scene("ball_socket");
    let (h, s) = (simulate(&b, 90), simulate(&c, 90));
    for f in 0..=90 {
        let d = (h[&r][f].w_axis - s[&rc][f].w_axis).length();
        assert!(d < 1.0, "frame {f}: {d}");
    }
}

fn spring_scene(stiffness: f64, damping: f64) -> (Built, forge_core::NodeId) {
    let mut top = Obj::cube("top", [0.0, 300.0, 0.0]);
    top.dynamic = "off";
    let bottom = Obj::cube("bottom", [0.0, 150.0, 0.0]);
    let mut b = build(&[Obj::plane(), top, bottom]);
    let (t, r) = (b.ids["top"], b.ids["bottom"]);
    b.doc
        .transact("spring", |tx| {
            let s = tx.create(forge_dynamics::SPRING, None, None)?;
            tx.set_param(s, "object_a", Value::Link(Some(t)))?;
            tx.set_param(s, "object_b", Value::Link(Some(r)))?;
            tx.set_param(s, "stiffness", Value::Float(stiffness))?;
            tx.set_param(s, "damping", Value::Float(damping))
        })
        .unwrap();
    (b, r)
}

#[test]
fn spring_oscillates_and_settles_like_the_reference() {
    let (b, r) = spring_scene(1.0, 0.2);
    let sim = simulate(&b, 90);
    let ms = &sim[&r];
    let ey = track("soft-bodies", "spring", "cube", "pos_y");
    for f in 0..=90 {
        let d = (ms[f].w_axis.y - ey[f]).abs();
        assert!(d < 3.0, "frame {f}: off by {d:.2} cm");
        // x, z and every rotation stay 0 (observed).
        assert!(
            ms[f].w_axis.x.abs() < 0.1 && ms[f].w_axis.z.abs() < 0.1,
            "frame {f}"
        );
        for a in hpb_deg(&ms[f]) {
            assert!(angle_diff(a, 0.0) < 0.5, "frame {f} rotated");
        }
    }
    // Settles 10 cm below the rest length (190.037 observed), within 0.5 cm.
    assert!(
        (ms[90].w_axis.y - ey[90]).abs() < 0.5,
        "settled at {}",
        ms[90].w_axis.y
    );
    // The first peak (frame 10) and the first trough (frame 19), within 3 cm of the observed
    // 210.47 / 179.19: the period is right.
    let peak = (5..15)
        .max_by(|a, b| ms[*a].w_axis.y.total_cmp(&ms[*b].w_axis.y))
        .unwrap();
    assert!((9..=11).contains(&peak), "peak at frame {peak}");
}
