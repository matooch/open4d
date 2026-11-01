//! The cloner as a scene node (clone-toolset spec C2, C3): prototypes, consumption,
//! reset coordinates, instance modes, Make Editable.

use std::sync::Arc;

use forge_clone_engine::CloneToolset;
use forge_core::{Document, EvalCtx, NodeId, Registry, Value};
use forge_generators::{
    BuildError, EditableError, Primitives, clone_data, cloner_prototypes, evaluate, is_consumed,
    make_editable,
};
use forge_geometry::PolyMesh;

fn doc() -> Document {
    let mut reg = Registry::with_builtins();
    reg.install(&Primitives).unwrap();
    reg.install(&CloneToolset).unwrap();
    reg.install(&forge_modifiers::Deformers).unwrap();
    Document::new(Arc::new(reg))
}

fn mesh(d: &Document, id: NodeId) -> Option<PolyMesh> {
    evaluate(d.scene(), id, &EvalCtx::default()).unwrap()
}

/// A Linear cloner (count 3) with a 20 cm cube child; returns (cloner, cube).
fn linear_cube(d: &mut Document) -> (NodeId, NodeId) {
    d.transact("setup", |tx| {
        let c = tx.create("forge.cloner", None, None)?;
        tx.set_param(c, "mode", Value::Enum("linear".into()))?;
        let cube = tx.create("forge.cube", Some(c), None)?;
        for a in ["size_x", "size_y", "size_z"] {
            tx.set_param(cube, a, Value::Float(20.0))?;
        }
        Ok((c, cube))
    })
    .unwrap()
}

#[test]
fn a_cloner_without_children_has_no_geometry() {
    let mut d = doc();
    let c = d
        .transact("c", |tx| tx.create("forge.cloner", None, None))
        .unwrap();
    assert_eq!(mesh(&d, c), None);
    // The clone data still exists: the default grid of 9.
    let data = clone_data(d.scene(), c, &EvalCtx::default())
        .unwrap()
        .unwrap();
    assert_eq!(data.len(), 9);
}

#[test]
fn prototypes_are_consumed_and_deformers_are_not_prototypes() {
    let mut d = doc();
    let (c, cube) = linear_cube(&mut d);
    let bend = d
        .transact("bend", |tx| tx.create("forge.bend", Some(c), None))
        .unwrap();
    assert_eq!(cloner_prototypes(d.scene(), c), vec![cube]);
    assert!(is_consumed(d.scene(), cube));
    assert!(!is_consumed(d.scene(), c));
    assert!(!is_consumed(d.scene(), bend));
}

#[test]
fn clones_are_expanded_in_clone_order() {
    let mut d = doc();
    let (c, _) = linear_cube(&mut d);
    let m = mesh(&d, c).unwrap();
    assert_eq!((m.points.len(), m.polys.len()), (24, 18));
    // Clone k occupies points 8k..8k+8, centred at y = 50 k.
    for k in 0..3 {
        let y: f64 = m.points[8 * k..8 * k + 8].iter().map(|p| p.y).sum::<f64>() / 8.0;
        assert!((y - 50.0 * k as f64).abs() < 1e-9, "clone {k}: {y}");
    }
    // Faces of clone 1 index its own block.
    assert!(m.polys[6..12].iter().flatten().all(|v| (8..16).contains(v)));
}

#[test]
fn reset_coordinates_ignores_the_prototype_placement() {
    let mut d = doc();
    let (c, cube) = linear_cube(&mut d);
    d.transact("move", |tx| {
        tx.set_param(cube, "transform.pos", Value::Vec3([1000.0, 0.0, 0.0]))
    })
    .unwrap();
    let on = mesh(&d, c).unwrap();
    assert!(on.points.iter().all(|p| p.x.abs() <= 10.0), "reset on");
    d.transact("off", |tx| {
        tx.set_param(c, "reset_coordinates", Value::Bool(false))
    })
    .unwrap();
    let off = mesh(&d, c).unwrap();
    assert!(off.points.iter().all(|p| p.x >= 990.0), "reset off");
}

#[test]
fn instance_modes_do_not_change_the_geometry() {
    let mut d = doc();
    let (c, _) = linear_cube(&mut d);
    let base = mesh(&d, c).unwrap();
    for m in ["render_instance", "multi_instance"] {
        d.transact("mode", |tx| {
            tx.set_param(c, "instance_mode", Value::Enum(m.into()))
        })
        .unwrap();
        assert_eq!(mesh(&d, c).unwrap(), base, "{m}");
    }
}

#[test]
fn two_prototypes_alternate_in_child_order() {
    let mut d = doc();
    let (c, _) = linear_cube(&mut d);
    let plane = d
        .transact("plane", |tx| tx.create("forge.plane", Some(c), None))
        .unwrap();
    let plane_mesh = mesh(&d, plane).unwrap();
    d.transact("n", |tx| tx.set_param(c, "linear_count", Value::Int(4)))
        .unwrap();
    let m = mesh(&d, c).unwrap();
    // The cube (8 points, 6 faces) and the plane alternate: cube, plane, cube, plane.
    assert_eq!(
        (m.points.len(), m.polys.len()),
        (
            16 + 2 * plane_mesh.points.len(),
            12 + 2 * plane_mesh.polys.len()
        )
    );
    d.transact("order", |tx| {
        tx.set_param(c, "clones_mode", Value::Enum("sort".into()))
    })
    .unwrap();
    assert!(matches!(
        evaluate(d.scene(), c, &EvalCtx::default()),
        Err(BuildError::Unsupported(_))
    ));
}

#[test]
fn object_mode_is_unsupported_until_brief_066() {
    let mut d = doc();
    let (c, _) = linear_cube(&mut d);
    d.transact("m", |tx| {
        tx.set_param(c, "mode", Value::Enum("object".into()))
    })
    .unwrap();
    assert!(matches!(
        evaluate(d.scene(), c, &EvalCtx::default()),
        Err(BuildError::Unsupported(_))
    ));
}

#[test]
fn deformers_deform_the_expanded_mesh() {
    let mut d = doc();
    let (c, _) = linear_cube(&mut d);
    let plain = mesh(&d, c).unwrap();
    d.transact("twist", |tx| {
        let t = tx.create("forge.twist", Some(c), None)?;
        tx.set_param(t, "angle", Value::Float(1.0))
    })
    .unwrap();
    let twisted = mesh(&d, c).unwrap();
    assert_eq!(twisted.points.len(), plain.points.len());
    assert_ne!(twisted.points, plain.points);
}

#[test]
fn make_editable_bakes_the_clones_and_removes_the_prototypes() {
    let mut d = doc();
    let (c, _) = linear_cube(&mut d);
    let expected = mesh(&d, c).unwrap();
    let new = d
        .transact("me", |tx| {
            make_editable(tx, c).map_err(|e| match e {
                EditableError::Edit(e) => e,
                other => panic!("{other}"),
            })
        })
        .unwrap();
    assert!(d.scene().node(c).is_none());
    assert_eq!(d.scene().len(), 1, "the prototype cube is gone");
    assert_eq!(mesh(&d, new).unwrap(), expected);
}
