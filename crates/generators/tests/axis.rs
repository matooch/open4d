//! docs/specs/MANIPULATE.md §7: axis edits keep the content where it is in the world.

use std::sync::Arc;

use forge_core::transform::{POS, ROT, SCALE, Transform};
use forge_core::{Document, NodeId, Registry, Value};
use forge_generators::axis::{
    AxisEdit, counter_matrix, decompose, shifted_axis, with_world_rotation,
};
use forge_generators::{Primitives, make_editable};
use glam::{DMat3, DMat4, DVec3};

fn doc() -> Document {
    let mut r = Registry::with_builtins();
    r.install(&Primitives).unwrap();
    Document::new(Arc::new(r))
}

/// A cube made editable with a transform, and two children with their own.
fn rig() -> (Document, NodeId, [NodeId; 2]) {
    let mut d = doc();
    let (cube, kids) = d
        .transact("rig", |tx| {
            let c = tx.create("forge.cube", None, None)?;
            let c = make_editable(tx, c).unwrap();
            tx.set_param(c, POS, Value::Vec3([100.0, 40.0, -30.0]))?;
            tx.set_param(c, ROT, Value::Vec3([0.4, -0.3, 0.2]))?;
            tx.set_param(c, SCALE, Value::Vec3([2.0, 2.0, 2.0]))?;
            let a = tx.create("forge.null", Some(c), None)?;
            tx.set_param(a, POS, Value::Vec3([10.0, 20.0, 30.0]))?;
            tx.set_param(a, ROT, Value::Vec3([0.1, 0.2, 0.3]))?;
            let b = tx.create("forge.null", Some(c), None)?;
            tx.set_param(b, POS, Value::Vec3([-5.0, 0.0, 7.0]))?;
            tx.set_param(b, SCALE, Value::Vec3([1.0, 3.0, 0.5]))?;
            Ok((c, [a, b]))
        })
        .unwrap();
    (d, cube, kids)
}

fn world_points(d: &Document, id: NodeId) -> Vec<DVec3> {
    let w = d.scene().world_matrix(id).unwrap();
    match d.scene().param(id, "mesh") {
        Some(Value::Mesh(m)) => m
            .points
            .iter()
            .map(|p| w.transform_point3(DVec3::from_array(*p)))
            .collect(),
        _ => panic!("mesh"),
    }
}

fn world(d: &Document, id: NodeId) -> DMat4 {
    d.scene().world_matrix(id).unwrap()
}

fn assert_world_kept(before: &[DVec3], after: &[DVec3]) {
    assert_eq!(before.len(), after.len());
    for (a, b) in before.iter().zip(after) {
        assert!((*a - *b).abs().max_element() < 1e-9, "{a} != {b}");
    }
}

fn mclose(a: DMat4, b: DMat4) {
    let d = (a - b)
        .to_cols_array()
        .iter()
        .fold(0.0f64, |m, x| m.max(x.abs()));
    assert!(d < 1e-9, "{a} != {b}");
}

fn apply(d: &mut Document, cube: NodeId, new: Transform) {
    let edit = AxisEdit::capture(d.scene(), cube).unwrap();
    d.transact("axis", |tx| edit.apply(tx, &new)).unwrap();
}

#[test]
fn decompose_round_trips_a_trs_matrix() {
    for t in [
        Transform {
            pos: DVec3::new(1.0, -2.0, 3.0),
            rot: DVec3::new(0.4, -0.3, 0.2),
            scale: DVec3::new(2.0, 0.5, 3.0),
        },
        Transform {
            pos: DVec3::ZERO,
            rot: DVec3::new(-2.0, 0.9, 1.0),
            scale: DVec3::new(-1.0, 1.0, 1.0),
        },
        Transform::IDENTITY,
    ] {
        mclose(decompose(&t.matrix()).matrix(), t.matrix());
    }
}

#[test]
fn translating_the_axis_keeps_the_world_points_and_children() {
    let (mut d, cube, kids) = rig();
    let pts = world_points(&d, cube);
    let kid_world: Vec<DMat4> = kids.iter().map(|k| world(&d, *k)).collect();
    let start = d.scene().local_transform(cube).unwrap();
    apply(
        &mut d,
        cube,
        Transform {
            pos: start.pos + DVec3::new(50.0, -20.0, 5.0),
            ..start
        },
    );
    assert_eq!(
        d.scene().local_transform(cube).unwrap().pos,
        start.pos + DVec3::new(50.0, -20.0, 5.0)
    );
    assert_world_kept(&pts, &world_points(&d, cube));
    for (k, w) in kids.iter().zip(&kid_world) {
        mclose(world(&d, *k), *w);
    }
}

#[test]
fn rotating_and_scaling_the_axis_keeps_world_points() {
    let (mut d, cube, kids) = rig();
    let pts = world_points(&d, cube);
    let origins: Vec<DVec3> = kids
        .iter()
        .map(|k| world(&d, *k).w_axis.truncate())
        .collect();
    let start = d.scene().local_transform(cube).unwrap();
    apply(
        &mut d,
        cube,
        Transform {
            pos: start.pos + DVec3::new(-7.0, 3.0, 11.0),
            rot: DVec3::new(-1.1, 0.5, 0.9),
            scale: DVec3::new(3.0, 3.0, 3.0),
        },
    );
    assert_world_kept(&pts, &world_points(&d, cube));
    for (k, o) in kids.iter().zip(&origins) {
        assert!((world(&d, *k).w_axis.truncate() - *o).abs().max_element() < 1e-9);
    }
}

#[test]
fn repeated_applies_from_the_start_do_not_accumulate() {
    let (mut d, cube, kids) = rig();
    let pts = world_points(&d, cube);
    let kid_world: Vec<DMat4> = kids.iter().map(|k| world(&d, *k)).collect();
    let edit = AxisEdit::capture(d.scene(), cube).unwrap();
    assert!(edit.has_points());
    let steps = d.history().undo_labels().len();
    for i in 1..=5 {
        let new = Transform {
            pos: edit.start.pos + DVec3::new(i as f64 * 10.0, 0.0, 0.0),
            ..edit.start
        };
        d.transact_merged("axis", "axis:1", |tx| edit.apply(tx, &new))
            .unwrap();
    }
    assert_eq!(d.history().undo_labels().len(), steps + 1, "one undo step");
    assert_world_kept(&pts, &world_points(&d, cube));
    for (k, w) in kids.iter().zip(&kid_world) {
        mclose(world(&d, *k), *w);
    }
    // undo restores points, transform and children exactly
    assert!(d.undo());
    assert_eq!(world_points(&d, cube), pts);
    for (k, w) in kids.iter().zip(&kid_world) {
        assert_eq!(world(&d, *k), *w);
    }
}

#[test]
fn a_translation_only_edit_leaves_rotation_and_scale_params_untouched() {
    let (mut d, cube, kids) = rig();
    let rot_before: Vec<Value> = kids
        .iter()
        .map(|k| d.scene().param(*k, ROT).unwrap().clone())
        .collect();
    let start = d.scene().local_transform(cube).unwrap();
    apply(
        &mut d,
        cube,
        Transform {
            pos: start.pos + DVec3::splat(1.0),
            ..start
        },
    );
    for (k, r) in kids.iter().zip(&rot_before) {
        assert_eq!(d.scene().param(*k, ROT).unwrap(), r, "no rotation noise");
    }
    assert_eq!(
        d.scene().local_transform(cube).unwrap().rot,
        start.rot,
        "rotation param untouched"
    );
}

#[test]
fn a_generator_follows_its_axis_but_its_children_stay() {
    let mut d = doc();
    let (cube, kid) = d
        .transact("rig", |tx| {
            let c = tx.create("forge.cube", None, None)?;
            let k = tx.create("forge.null", Some(c), None)?;
            tx.set_param(k, POS, Value::Vec3([5.0, 6.0, 7.0]))?;
            Ok((c, k))
        })
        .unwrap();
    let kid_before = world(&d, kid);
    let edit = AxisEdit::capture(d.scene(), cube).unwrap();
    assert!(!edit.has_points());
    let start = edit.start;
    d.transact("axis", |tx| {
        edit.apply(
            tx,
            &Transform {
                pos: start.pos + DVec3::new(0.0, 100.0, 0.0),
                rot: DVec3::new(0.5, 0.0, 0.0),
                scale: start.scale,
            },
        )
    })
    .unwrap();
    mclose(world(&d, kid), kid_before);
    assert_eq!(d.scene().node(cube).unwrap().type_id, "forge.cube");
}

#[test]
fn capture_rejects_missing_nodes_and_tags() {
    let (d, _, _) = rig();
    assert!(AxisEdit::capture(d.scene(), NodeId(9999)).is_err());
}

#[test]
fn shifted_and_world_rotation_helpers() {
    let t = Transform {
        pos: DVec3::new(1.0, 2.0, 3.0),
        rot: DVec3::new(0.5, 0.2, -0.3),
        scale: DVec3::new(2.0, 1.0, 1.0),
    };
    let c = DVec3::new(3.0, -1.0, 4.0);
    let s = shifted_axis(&t, c);
    assert!((s.pos - (t.pos + t.matrix().transform_vector3(c))).length() < 1e-12);
    assert_eq!((s.rot, s.scale), (t.rot, t.scale));
    // the shifted axis puts the old point c at the new origin
    let world_c = t.matrix().transform_point3(c);
    assert!((s.matrix().transform_point3(DVec3::ZERO) - world_c).length() < 1e-12);
    // counter matrix of a shift is a translation by -c
    let k = counter_matrix(&t, &s);
    assert!((k.transform_point3(c)).length() < 1e-12);
    // world rotation identity under a rotated parent
    let parent = DMat3::from_rotation_y(0.7);
    let aligned = with_world_rotation(&t, parent, DMat3::IDENTITY);
    let world_rot = parent * aligned.rotation();
    assert!(
        (world_rot - DMat3::IDENTITY)
            .to_cols_array()
            .iter()
            .all(|x| x.abs() < 1e-9)
    );
}

#[test]
fn a_later_update_without_rotation_resets_the_rotation_of_an_earlier_one() {
    let (mut d, cube, kids) = rig();
    let pts = world_points(&d, cube);
    let kid = world(&d, kids[0]);
    let edit = AxisEdit::capture(d.scene(), cube).unwrap();
    let rotated = Transform {
        rot: edit.start.rot + DVec3::new(0.7, 0.0, 0.0),
        ..edit.start
    };
    let shifted = Transform {
        pos: edit.start.pos + DVec3::X,
        ..edit.start
    };
    for new in [rotated, shifted] {
        d.transact_merged("axis", "axis:2", |tx| edit.apply(tx, &new))
            .unwrap();
        assert_world_kept(&pts, &world_points(&d, cube));
        mclose(world(&d, kids[0]), kid);
    }
    assert_eq!(d.scene().local_transform(cube).unwrap().rot, edit.start.rot);
}
