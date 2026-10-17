//! docs/specs/MANIPULATE.md §1.

mod common;

use forge_core::transform::{Transform, hpb_from_rotation, rotation_part};
use forge_core::{NodeId, Value};
use glam::{DMat3, DMat4, DVec3};

fn rot(h: f64, p: f64, b: f64) -> Transform {
    Transform {
        rot: DVec3::new(h, p, b).map(f64::to_radians),
        ..Transform::IDENTITY
    }
}

fn mat_close(a: DMat3, b: DMat3) {
    let d = (a - b)
        .to_cols_array()
        .iter()
        .fold(0.0f64, |m, x| m.max(x.abs()));
    assert!(d < 1e-9, "{a} != {b}");
}

#[test]
fn hpb_round_trips_away_from_the_singularity() {
    for (h, p, b) in [
        (0.0, 0.0, 0.0),
        (30.0, 45.0, 60.0),
        (-120.0, 10.0, 170.0),
        (90.0, -80.0, -45.0),
    ] {
        let t = rot(h, p, b);
        let got = hpb_from_rotation(t.rotation());
        let want = DVec3::new(h, p, b).map(f64::to_radians);
        assert!(
            (got - want).abs().max_element() < 1e-9,
            "({h}, {p}, {b}) -> {got}"
        );
    }
}

#[test]
fn rotation_part_matches_matrix_and_any_split_at_the_pole() {
    let t = Transform {
        rot: DVec3::new(0.3, 0.4, 0.5),
        pos: DVec3::splat(7.0),
        ..Transform::IDENTITY
    };
    mat_close(t.rotation(), DMat3::from_mat4(t.matrix()));
    let pole = rot(20.0, 90.0, 35.0);
    let back = hpb_from_rotation(pole.rotation());
    mat_close(
        Transform {
            rot: back,
            ..Transform::IDENTITY
        }
        .rotation(),
        pole.rotation(),
    );
}

#[test]
fn rotation_part_normalizes_scaled_columns() {
    let m = DMat4::from_scale(DVec3::new(2.0, 3.0, 4.0));
    mat_close(
        rotation_part(&(rot(30.0, 0.0, 0.0).matrix() * m)),
        rot(30.0, 0.0, 0.0).rotation(),
    );
}

#[test]
fn parent_world_matrix() {
    let mut doc = common::doc();
    let (p, c, t) = doc
        .transact("b", |tx| {
            let p = tx.create("forge.null", None, None)?;
            tx.set_param(p, "transform.pos", Value::Vec3([1.0, 2.0, 3.0]))?;
            let c = tx.create("forge.null", Some(p), None)?;
            let t = tx.attach_tag("test.sel", c, None)?;
            Ok((p, c, t))
        })
        .unwrap();
    let s = doc.scene();
    assert_eq!(s.parent_world_matrix(p), Some(DMat4::IDENTITY));
    assert_eq!(s.parent_world_matrix(c), s.world_matrix(p));
    assert_eq!(s.parent_world_matrix(t), None);
    assert_eq!(s.parent_world_matrix(NodeId(77)), None);
}
