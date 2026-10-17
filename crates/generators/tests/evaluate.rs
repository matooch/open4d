//! docs/specs/SUBDIVISION.md §1, §4: the generator node and scene evaluation.

use std::sync::Arc;

use forge_core::{Document, Registry, Value};
use forge_generators::{Primitives, evaluate};

fn doc() -> Document {
    let mut r = Registry::with_builtins();
    r.install(&Primitives).unwrap();
    Document::new(Arc::new(r))
}

#[test]
fn subdivision_consumes_its_first_mesh_child() {
    let mut d = doc();
    let (sub, cube) = d
        .transact("b", |tx| {
            let sub = tx.create("forge.subdivision", None, None)?;
            let null = tx.create("forge.null", Some(sub), None)?; // no mesh: skipped
            let cube = tx.create("forge.cube", Some(sub), None)?;
            let _ = null;
            Ok((sub, cube))
        })
        .unwrap();
    let m = evaluate(d.scene(), sub).unwrap().expect("subdivided cube");
    assert_eq!(
        (m.points.len(), m.polys.len()),
        (98, 96),
        "default viewport level 2"
    );

    // the child's transform places the input inside the generator
    d.transact("move", |tx| {
        tx.set_param(cube, "transform.pos", Value::Vec3([1000.0, 0.0, 0.0]))
    })
    .unwrap();
    let m = evaluate(d.scene(), sub).unwrap().unwrap();
    assert!(m.points.iter().all(|p| p.x > 900.0));

    d.transact("lvl", |tx| tx.set_param(sub, "level_view", Value::Int(0)))
        .unwrap();
    assert_eq!(evaluate(d.scene(), sub).unwrap().unwrap().points.len(), 8);
}

#[test]
fn nodes_without_geometry_evaluate_to_none() {
    let mut d = doc();
    let (sub, null) = d
        .transact("b", |tx| {
            Ok((
                tx.create("forge.subdivision", None, None)?,
                tx.create("forge.null", None, None)?,
            ))
        })
        .unwrap();
    assert!(evaluate(d.scene(), sub).unwrap().is_none());
    assert!(evaluate(d.scene(), null).unwrap().is_none());
    let cube = d
        .transact("c", |tx| tx.create("forge.cube", None, None))
        .unwrap();
    assert_eq!(evaluate(d.scene(), cube).unwrap().unwrap().points.len(), 8);
}
