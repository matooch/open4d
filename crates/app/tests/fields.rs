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
        ("forge.field.cylinder", 0.5),
        ("forge.field.cone", 0.5),
        ("forge.field.torus", 0.5),
        ("forge.field.capsule", 0.5),
        ("forge.field.radial", 0.0),
        ("forge.field.random", 0.0),
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

#[test]
fn new_field_types_set_params_save_and_report_unsupported_noise() {
    let mut app = App::new_default();
    let cyl = create(&mut app, "forge.field.cylinder");
    set(&mut app, cyl, "radius", json!(200.0));
    set(&mut app, cyl, "height", json!(300.0));
    let at = DVec3::new(0.0, 0.0, 100.0);
    // Radius 200: ramp 1 - 100/200 = 0.5, inner offset 50 % makes it 1.
    assert!((value_at(&app, cyl, at) - 1.0).abs() < 1e-12);
    set(&mut app, cyl, "inner_offset", json!(0.0));
    assert!((value_at(&app, cyl, at) - 0.5).abs() < 1e-12);
    let rnd = create(&mut app, "forge.field.random");
    set(&mut app, rnd, "random_mode", json!("random"));
    let before = value_at(&app, rnd, at);
    set(&mut app, rnd, "noise_type", json!("zada"));
    assert_eq!(
        value_at(&app, rnd, at),
        before,
        "Random mode ignores the noise type"
    );
    set(&mut app, rnd, "random_mode", json!("noise"));
    let err = FieldList::from_nodes(app.doc.scene(), &[rnd])
        .err()
        .unwrap();
    assert_eq!(err, FieldError::Unsupported("noise type"));
    set(&mut app, rnd, "noise_type", json!("fbm"));
    let noise = value_at(&app, rnd, at);
    let dir = std::env::temp_dir().join(format!("forge-fields-new-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("fields.forge").to_string_lossy().into_owned();
    app.save(&path).unwrap();
    let mut other = App::new_default();
    other.open(&path).unwrap();
    assert_eq!(value_at(&other, rnd, at), noise);
    assert!((value_at(&other, cyl, at) - 0.5).abs() < 1e-12);
}

fn rows(app: &App, owner: NodeId) -> Vec<forge_core::LinkRow> {
    match app.doc.scene().param(owner, "fields") {
        Some(forge_core::Value::LinkRows(r)) => r.clone(),
        other => panic!("not a row list: {other:?}"),
    }
}

#[test]
fn group_formula_and_shader_fields_are_created_with_their_defaults() {
    let mut app = App::new_default();
    let g = create(&mut app, "forge.field.group");
    let f = create(&mut app, "forge.field.formula");
    let s = create(&mut app, "forge.field.shader");
    assert_eq!(
        app.doc.scene().param(g, "enable_remapping"),
        Some(&forge_core::Value::Bool(false))
    );
    assert_eq!(
        app.doc.scene().param(f, "clamp_min"),
        Some(&forge_core::Value::Bool(false))
    );
    assert!(rows(&app, g).is_empty());
    // an empty group has no value, the default formula is in 0..1, the shader field is Unsupported
    assert_eq!(value_at(&app, g, DVec3::ZERO), 0.0);
    let v = value_at(&app, f, DVec3::ZERO);
    assert!((0.0..=1.0).contains(&v));
    assert!(matches!(
        FieldList::from_nodes(app.doc.scene(), &[s]).err(),
        Some(FieldError::Unsupported(_))
    ));
    // a text that does not parse is reported, not silently zero
    set(&mut app, f, "formula", json!("1 +"));
    assert!(matches!(
        FieldList::from_nodes(app.doc.scene(), &[f]).err(),
        Some(FieldError::Formula(_))
    ));
}

#[test]
fn list_rows_are_edited_by_param_set_as_one_undo_step_and_read_back_by_the_query() {
    let mut app = App::new_default();
    let a = create(&mut app, "forge.field.spherical");
    let b = create(&mut app, "forge.field.linear");
    let bend = create(&mut app, "forge.bend");
    // bare ids are default rows
    set(&mut app, bend, "fields", json!([a.0, b.0]));
    let r = rows(&app, bend);
    assert_eq!(
        r.iter()
            .map(|x| (x.blend.as_str(), x.strength, x.enabled))
            .collect::<Vec<_>>(),
        vec![("normal", 1.0, true), ("normal", 1.0, true)]
    );
    // row objects carry blend, strength (percent) and enabled
    let steps = app.doc.history().undo_labels().len();
    set(
        &mut app,
        bend,
        "fields",
        json!([
            {"target": a.0, "blend": "add", "strength": 50.0},
            {"target": b.0, "enabled": false},
        ]),
    );
    assert_eq!(app.doc.history().undo_labels().len(), steps + 1);
    let r = rows(&app, bend);
    assert_eq!(
        (r[0].blend.as_str(), r[0].strength, r[0].enabled),
        ("add", 0.5, true)
    );
    assert!(!r[1].enabled);
    // the same field in another list has its own row
    let taper = create(&mut app, "forge.taper");
    set(&mut app, taper, "fields", json!([a.0]));
    assert_eq!(rows(&app, taper)[0].strength, 1.0);
    // the query shows rows with percent strengths
    let node = forge_app::query::node_json(app.doc.scene(), bend);
    let shown = &node["params"]["fields"];
    assert_eq!(shown[0]["blend"], json!("add"));
    assert_eq!(shown[0]["strength"], json!(50.0));
    assert_eq!(shown[1]["enabled"], json!(false));
    // bad rows are refused and change nothing
    for bad in [
        json!([{"target": a.0, "blend": "dissolve"}]),
        json!([{"strength": 50.0}]),
        json!([a.0, a.0]),
    ] {
        assert!(
            app.run(
                "param.set",
                json!({"node": bend.0, "param": "fields", "value": bad})
            )
            .is_err()
        );
    }
    assert_eq!(rows(&app, bend).len(), 2);
    // undo steps back through the taper's list, the taper's creation, then the row edit of the bend
    app.run("edit.undo", json!({})).unwrap();
    assert!(rows(&app, taper).is_empty());
    app.run("edit.undo", json!({})).unwrap();
    app.run("edit.undo", json!({})).unwrap();
    assert_eq!(rows(&app, bend)[0].blend, "normal");
}

#[test]
fn rows_survive_save_and_load() {
    let mut app = App::new_default();
    let a = create(&mut app, "forge.field.spherical");
    let bend = create(&mut app, "forge.bend");
    set(
        &mut app,
        bend,
        "fields",
        json!([{"target": a.0, "blend": "multiply", "strength": 25.0, "enabled": false}]),
    );
    let text = forge_core::file::save(app.doc.scene());
    assert!(text.contains("version: 11"));
    let (scene, report) =
        forge_core::file::load(app.doc.scene().registry().clone(), &text).unwrap();
    assert!(report.warnings.is_empty(), "{:?}", report.warnings);
    assert_eq!(
        scene.param(bend, "fields"),
        app.doc.scene().param(bend, "fields")
    );
}
