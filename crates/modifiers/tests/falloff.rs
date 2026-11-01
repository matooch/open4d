//! Falloff behaviour beyond the O24 cases (docs/specs/MODIFIERS.md §10): every deformer
//! type takes the list, and links that are not live fields are skipped.

use std::sync::Arc;

use forge_core::{Document, EvalCtx, Registry, Value};
use forge_fields::FieldToolset;
use forge_modifiers::{Deformers, apply_deformer};
use glam::DVec3;

fn registry() -> Arc<Registry> {
    let mut reg = Registry::with_builtins();
    reg.install(&Deformers).unwrap();
    reg.install(&FieldToolset).unwrap();
    Arc::new(reg)
}

fn doc() -> Document {
    Document::new(registry())
}

#[test]
fn every_deformer_has_a_fields_list() {
    let reg = registry();
    for id in [
        "forge.bend",
        "forge.twist",
        "forge.taper",
        "forge.bulge",
        "forge.shear",
        "forge.spherify",
    ] {
        let p = reg.get(id).unwrap().find_param("fields").unwrap();
        assert_eq!(p.default, Value::LinkList(vec![]), "{id}");
    }
}

#[test]
fn weight_blends_towards_the_deformed_point() {
    let mut d = doc();
    // Spherify at 100 % to radius 100; a Linear field (X+, length 100, inner 0) at the
    // origin gives 0.5 + x/200 at a point on the x axis.
    let (target, deformer, field) = d
        .transact("scene", |tx| {
            let target = tx.create("forge.null", None, None)?;
            let deformer = tx.create("forge.spherify", Some(target), None)?;
            tx.set_param(deformer, "radius", Value::Float(100.0))?;
            tx.set_param(deformer, "strength", Value::Float(100.0))?;
            let field = tx.create("forge.field.linear", None, None)?;
            tx.set_param(field, "inner_offset", Value::Float(0.0))?;
            tx.set_param(deformer, "fields", Value::LinkList(vec![field]))?;
            Ok((target, deformer, field))
        })
        .unwrap();
    let ctx = EvalCtx::default();
    let mut pts = [DVec3::new(50.0, 0.0, 0.0)];
    apply_deformer(d.scene(), deformer, target, &mut pts, &ctx);
    // w = 0.75, full deform -> x = 100: 50 + 0.75 * 50.
    assert!((pts[0].x - 87.5).abs() < 1e-9, "{:?}", pts[0]);

    // A deleted field is skipped: the list is then empty, i.e. full effect.
    d.transact("delete", |tx| tx.delete(field)).unwrap();
    let mut pts = [DVec3::new(50.0, 0.0, 0.0)];
    apply_deformer(d.scene(), deformer, target, &mut pts, &ctx);
    assert!((pts[0].x - 100.0).abs() < 1e-9, "{:?}", pts[0]);
}
