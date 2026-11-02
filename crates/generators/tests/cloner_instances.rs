//! `cloner_instances`: the cloner as prototypes plus clone data, for GPU instancing
//! (docs/specs/VIEWPORT.md §10.1). It must agree with the expanded mesh.

use std::sync::Arc;

use forge_clone_engine::CloneToolset;
use forge_core::{Document, EvalCtx, NodeId, Registry, Value};
use forge_generators::{Primitives, cloner_instances, evaluate};

fn doc() -> Document {
    let mut reg = Registry::with_builtins();
    reg.install(&Primitives).unwrap();
    reg.install(&CloneToolset).unwrap();
    reg.install(&forge_modifiers::Deformers).unwrap();
    Document::new(Arc::new(reg))
}

/// A default (grid) cloner with a cube and a sphere child; returns the cloner.
fn two_prototypes(d: &mut Document) -> NodeId {
    d.transact("setup", |tx| {
        let c = tx.create("forge.cloner", None, None)?;
        tx.create("forge.cube", Some(c), None)?;
        tx.create("forge.sphere", Some(c), None)?;
        Ok(c)
    })
    .unwrap()
}

#[test]
fn instances_agree_with_the_expanded_mesh() {
    let mut d = doc();
    let c = two_prototypes(&mut d);
    let ctx = EvalCtx::default();
    let inst = cloner_instances(d.scene(), c, &ctx).unwrap().unwrap();
    assert_eq!(inst.prototypes.len(), 2);
    assert!(inst.clones.is_consistent() && inst.clones.len() > 1);
    let expanded = evaluate(d.scene(), c, &ctx).unwrap().unwrap();
    // rebuild the expansion from the instances
    let (mut points, mut polys) = (0usize, 0usize);
    let mut k = 0;
    for i in 0..inst.clones.len() {
        if !inst.clones.visible[i] {
            continue;
        }
        let proto = &inst.prototypes[inst.clones.child[i] as usize];
        for (a, p) in proto.points.iter().enumerate() {
            let q = inst.clones.matrices[i].transform_point3(*p);
            assert!((q - expanded.points[points + a]).length() < 1e-9);
        }
        points += proto.points.len();
        polys += proto.polys.len();
        k += 1;
    }
    assert!(k > 1);
    assert_eq!(
        (points, polys),
        (expanded.points.len(), expanded.polys.len())
    );
    // both prototypes are used
    assert!(inst.clones.child.contains(&0) && inst.clones.child.contains(&1));
}

#[test]
fn none_means_expand_instead() {
    let mut d = doc();
    let ctx = EvalCtx::default();
    // not a cloner
    let cube = d
        .transact("cube", |tx| tx.create("forge.cube", None, None))
        .unwrap();
    assert!(cloner_instances(d.scene(), cube, &ctx).unwrap().is_none());
    // a cloner without a prototype
    let c = d
        .transact("c", |tx| tx.create("forge.cloner", None, None))
        .unwrap();
    assert!(cloner_instances(d.scene(), c, &ctx).unwrap().is_none());
    // a cloner with a prototype is instanced...
    d.transact("p", |tx| tx.create("forge.cube", Some(c), None))
        .unwrap();
    assert!(cloner_instances(d.scene(), c, &ctx).unwrap().is_some());
    // ...unless an enabled deformer child deforms the combined mesh
    d.transact("bend", |tx| tx.create("forge.bend", Some(c), None))
        .unwrap();
    assert!(cloner_instances(d.scene(), c, &ctx).unwrap().is_none());
    assert!(
        evaluate(d.scene(), c, &ctx).unwrap().is_some(),
        "the expansion still works"
    );
}

#[test]
fn reset_coordinates_and_child_placement_are_baked_like_the_expansion() {
    let mut d = doc();
    let ctx = EvalCtx::default();
    let (c, cube) = d
        .transact("setup", |tx| {
            let c = tx.create("forge.cloner", None, None)?;
            tx.set_param(c, "mode", Value::Enum("linear".into()))?;
            let cube = tx.create("forge.cube", Some(c), None)?;
            Ok((c, cube))
        })
        .unwrap();
    d.transact("move", |tx| {
        tx.set_param(cube, "transform.pos", Value::Vec3([100.0, 0.0, 0.0]))
    })
    .unwrap();
    let mean_x = |d: &Document| {
        let inst = cloner_instances(d.scene(), c, &ctx).unwrap().unwrap();
        let p = &inst.prototypes[0];
        p.points.iter().map(|q| q.x).sum::<f64>() / p.points.len() as f64
    };
    // Reset Coordinates (the default) ignores the child's placement
    assert!(mean_x(&d).abs() < 1e-9);
    d.transact("off", |tx| {
        tx.set_param(c, "reset_coordinates", Value::Bool(false))
    })
    .unwrap();
    assert!((mean_x(&d) - 100.0).abs() < 1e-9);
    let inst = cloner_instances(d.scene(), c, &ctx).unwrap().unwrap();
    let expanded = evaluate(d.scene(), c, &ctx).unwrap().unwrap();
    // the expansion agrees with the instance placement
    let first = inst.clones.matrices[0].transform_point3(inst.prototypes[0].points[0]);
    assert!((first - expanded.points[0]).length() < 1e-9);
}
