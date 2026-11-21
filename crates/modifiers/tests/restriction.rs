//! Restriction tags (docs/specs/EDITABLE.md §7.4, brief 027): a deformer acts on the points
//! of a named point selection (0/1) or by the weights of a named vertex map.

use std::sync::Arc;

use forge_core::tags::{POINT_SELECTION, RESTRICTION, VERTEX_MAP};
use forge_core::{Document, EvalCtx, NodeId, Registry, Value};
use forge_geometry::PolyMesh;
use forge_modifiers::{Deformers, apply_deformer, apply_deformer_mesh};
use glam::DVec3;

fn scene() -> (Document, NodeId, NodeId, NodeId) {
    let mut reg = Registry::with_builtins();
    reg.install(&Deformers).unwrap();
    let mut d = Document::new(Arc::new(reg));
    let (target, deformer, restriction) = d
        .transact("scene", |tx| {
            let target = tx.create("forge.null", None, None)?;
            let deformer = tx.create("forge.spherify", Some(target), None)?;
            tx.set_param(deformer, "radius", Value::Float(100.0))?;
            tx.set_param(deformer, "strength", Value::Float(100.0))?;
            let sel = tx.attach_tag(POINT_SELECTION, target, None)?;
            tx.rename(sel, "Left")?;
            tx.set_param(sel, "indices", Value::IndexList(vec![0, 2]))?;
            let vm = tx.attach_tag(VERTEX_MAP, target, None)?;
            tx.rename(vm, "Half")?;
            tx.set_param(vm, "weights", Value::FloatList(vec![0.0, 0.5, 1.0]))?;
            let r = tx.attach_tag(RESTRICTION, deformer, None)?;
            Ok((target, deformer, r))
        })
        .unwrap();
    (d, target, deformer, restriction)
}

fn points() -> [DVec3; 3] {
    [DVec3::new(50.0, 0.0, 0.0); 3]
}

fn set(d: &mut Document, tag: NodeId, key: &str, v: Value) {
    d.transact("set", |tx| tx.set_param(tag, key, v)).unwrap();
}

#[test]
fn without_a_name_the_deformer_acts_on_every_point() {
    let (d, target, deformer, _) = scene();
    let mut p = points();
    apply_deformer(d.scene(), deformer, target, &mut p, &EvalCtx::default());
    assert!(p.iter().all(|q| (q.x - 100.0).abs() < 1e-9), "{p:?}");
}

#[test]
fn a_point_selection_restricts_with_zero_or_one() {
    let (mut d, target, deformer, r) = scene();
    set(&mut d, r, "selection", Value::String("Left".into()));
    let mut p = points();
    apply_deformer(d.scene(), deformer, target, &mut p, &EvalCtx::default());
    assert!((p[0].x - 100.0).abs() < 1e-9 && (p[2].x - 100.0).abs() < 1e-9);
    assert_eq!(
        p[1],
        DVec3::new(50.0, 0.0, 0.0),
        "point 1 is outside the selection"
    );
    set(&mut d, r, "invert", Value::Bool(true));
    let mut p = points();
    apply_deformer(d.scene(), deformer, target, &mut p, &EvalCtx::default());
    assert_eq!(p[0], DVec3::new(50.0, 0.0, 0.0));
    assert!((p[1].x - 100.0).abs() < 1e-9);
}

#[test]
fn a_vertex_map_multiplies_the_effect() {
    let (mut d, target, deformer, r) = scene();
    set(&mut d, r, "selection", Value::String("Half".into()));
    let mut p = points();
    apply_deformer(d.scene(), deformer, target, &mut p, &EvalCtx::default());
    assert_eq!(p[0].x, 50.0);
    assert!((p[1].x - 75.0).abs() < 1e-9, "{:?}", p[1]);
    assert!((p[2].x - 100.0).abs() < 1e-9);
}

#[test]
fn the_mesh_entry_point_is_restricted_too_and_a_disabled_tag_is_ignored() {
    let (mut d, target, deformer, r) = scene();
    set(&mut d, r, "selection", Value::String("Half".into()));
    let mesh = PolyMesh {
        points: points().to_vec(),
        polys: vec![vec![0, 1, 2]],
    };
    let mut m = mesh.clone();
    apply_deformer_mesh(d.scene(), deformer, target, &mut m, &EvalCtx::default());
    assert!((m.points[1].x - 75.0).abs() < 1e-9);
    set(&mut d, r, "enabled", Value::Bool(false));
    let mut m = mesh;
    apply_deformer_mesh(d.scene(), deformer, target, &mut m, &EvalCtx::default());
    assert!((m.points[0].x - 100.0).abs() < 1e-9);
}
