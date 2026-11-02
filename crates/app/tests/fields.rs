//! The field nodes through the headless application core (docs/specs/FIELDS.md F1, F2):
//! created by `object.create`, parameters set through `param.set`, undo, save/load, and
//! sampled through a field list.

use forge_app::App;
use forge_core::{EvalCtx, NodeId};
use forge_fields::{FieldError, FieldList};
use glam::DVec3;
use serde_json::{Value, json};

fn create(app: &mut App, ty: &str) -> NodeId {
    let out = app.run("object.create", json!({"type": ty})).unwrap();
    NodeId(out["node"].as_u64().unwrap())
}

fn set(app: &mut App, node: NodeId, param: &str, value: Value) {
    app.run(
        "param.set",
        json!({"node": node.0, "param": param, "value": value}),
    )
    .unwrap();
}

fn value_at(app: &App, node: NodeId, p: DVec3) -> f64 {
    FieldList::from_nodes(app.doc.scene(), &[node])
        .unwrap()
        .sample(p, &EvalCtx::default())
        .value
}

#[test]
fn object_create_makes_each_field_with_descriptor_defaults() {
    let mut app = App::new_default();
    for (ty, inner) in [
        ("forge.field.linear", 0.0),
        ("forge.field.spherical", 0.5),
        ("forge.field.box", 0.5),
    ] {
        let node = create(&mut app, ty);
        let t = app.doc.scene().registry().get(ty).unwrap().clone();
        assert_eq!(t.category, forge_core::Category::Field);
        assert_eq!(
            t.find_param("inner_offset").unwrap().default,
            forge_core::Value::Float(inner)
        );
        assert!(t.params.iter().all(|p| !p.label.is_empty()));
        assert!(value_at(&app, node, DVec3::ZERO).is_finite());
    }
}

#[test]
fn params_set_in_display_units_drive_the_samples_and_undo_restores() {
    let mut app = App::new_default();
    let f = create(&mut app, "forge.field.spherical");
    let at = DVec3::new(75.0, 0.0, 0.0);
    assert!((value_at(&app, f, at) - 0.5).abs() < 1e-12);
    set(&mut app, f, "inner_offset", json!(0.0));
    assert!((value_at(&app, f, at) - 0.25).abs() < 1e-12);
    set(&mut app, f, "transform.pos", json!([75.0, 0.0, 0.0]));
    assert!((value_at(&app, f, at) - 1.0).abs() < 1e-12);
    app.run("edit.undo", json!({})).unwrap();
    assert!((value_at(&app, f, at) - 0.25).abs() < 1e-12);
}

#[test]
fn field_lists_use_strength_and_an_empty_list_is_full_effect() {
    let mut app = App::new_default();
    let f = create(&mut app, "forge.field.linear");
    let ctx = EvalCtx::default();
    let p = DVec3::new(75.0, 10.0, -5.0);
    assert_eq!(FieldList::new().sample(p, &ctx).value, 1.0);
    let list = FieldList::from_nodes(app.doc.scene(), &[f])
        .unwrap()
        .with_strength(0, 0.5);
    assert!((list.sample(p, &ctx).value - 0.4375).abs() < 1e-12);
    let many = list.sample_many(&[p, p], &ctx);
    assert_eq!(many.len(), 2);
}

#[test]
fn curve_contour_is_unsupported_and_other_nodes_are_not_fields() {
    let mut app = App::new_default();
    let f = create(&mut app, "forge.field.box");
    set(&mut app, f, "contour_mode", json!("curve"));
    let err = FieldList::from_nodes(app.doc.scene(), &[f]).err().unwrap();
    assert_eq!(err, FieldError::Unsupported("remapping curve"));
    let cube = create(&mut app, "forge.cube");
    let err = FieldList::from_nodes(app.doc.scene(), &[cube])
        .err()
        .unwrap();
    assert_eq!(err, FieldError::NotAField(cube.0));
}

#[test]
fn remapping_disabled_returns_the_raw_ramp_and_scene_round_trips() {
    let mut app = App::new_default();
    let f = create(&mut app, "forge.field.spherical");
    set(&mut app, f, "enable_remapping", json!(false));
    set(&mut app, f, "invert", json!(true));
    assert!((value_at(&app, f, DVec3::new(75.0, 0.0, 0.0)) - 0.25).abs() < 1e-12);
    let dir = std::env::temp_dir().join(format!("forge-fields-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("field.forge").to_string_lossy().into_owned();
    app.save(&path).unwrap();
    let mut other = App::new_default();
    other.open(&path).unwrap();
    assert!((value_at(&other, f, DVec3::new(75.0, 0.0, 0.0)) - 0.25).abs() < 1e-12);
}
