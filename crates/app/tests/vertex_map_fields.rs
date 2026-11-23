//! O49 vertex-map readback and restricted deformation.

use forge_app::App;
use forge_core::{EvalCtx, LinkRow, NodeId, Value};
use forge_io::{export_scene, obj};
use forge_parity_harness::{Samples, Tolerance, compare_meshes, compare_samples, parse_obj};
use glam::DVec3;

fn fixture(case: &str, file: &str) -> String {
    std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/golden/fields")
            .join(case)
            .join(file),
    )
    .unwrap()
}

fn setup(case: &str) -> (App, NodeId, NodeId, NodeId, toml::Value) {
    let m: toml::Value = toml::from_str(&fixture(case, "case.toml")).unwrap();
    let p = &m["params"];
    let mut app = App::new_default();
    let (cube, tag, field) = app
        .doc
        .transact("O49 setup", |tx| {
            let cube = tx.create("forge.cube", None, None)?;
            for axis in ["x", "y", "z"] {
                let key = format!("size_{axis}");
                tx.set_param(cube, &key, Value::Float(p[&key].as_float().unwrap()))?;
                let key = format!("segments_{axis}");
                tx.set_param(cube, &key, Value::Int(p[&key].as_integer().unwrap()))?;
            }
            let tag = tx.attach_tag(forge_core::tags::VERTEX_MAP, cube, None)?;
            let field = tx.create(forge_fields::LINEAR, None, None)?;
            tx.set_param(field, "direction", Value::Enum("y_pos".into()))?;
            tx.set_param(
                field,
                "length",
                Value::Float(p["field"]["length"].as_float().unwrap()),
            )?;
            tx.set_param(
                tag,
                "fields",
                Value::LinkRows(vec![LinkRow::new(field, "normal")]),
            )?;
            tx.set_param(tag, "use_fields", Value::Bool(true))?;
            Ok((cube, tag, field))
        })
        .unwrap();
    (app, cube, tag, field, m)
}

fn weights(app: &App, cube: NodeId, tag: NodeId) -> Vec<f64> {
    let s = app.doc.scene();
    let points = forge_generators::evaluate(s, cube, &EvalCtx::default())
        .unwrap()
        .unwrap()
        .points;
    forge_fields::evaluate_vertex_map(s, tag, &points, &EvalCtx::default()).unwrap()
}

fn check_mesh(app: &App, cube: NodeId, case: &str) {
    let bytes = obj::write(
        &export_scene(app.doc.scene(), Some(&[cube])),
        &obj::WriteOptions {
            weld: true,
            normals: false,
        },
    );
    let expect = parse_obj(&fixture(case, "mesh.obj")).unwrap();
    let got = parse_obj(&bytes).unwrap();
    let result = compare_meshes(&expect, &got, &Tolerance::default());
    assert!(result.passed(), "{case}: {:?}", result.failures.first());
}

#[test]
fn field_weights_and_base_mesh_match_o49() {
    let name = "vmap-from-field";
    let (app, cube, tag, _, _) = setup(name);
    let want =
        forge_parity_harness::samples::parse_samples(&fixture(name, "samples.json")).unwrap();
    let Samples::Track {
        param,
        fps,
        tolerance,
        ..
    } = &want
    else {
        panic!("expected track");
    };
    let got = Samples::Track {
        param: param.clone(),
        fps: *fps,
        tolerance: *tolerance,
        samples: weights(&app, cube, tag)
            .into_iter()
            .enumerate()
            .map(|(i, w)| (i as f64, w))
            .collect(),
    };
    let result = compare_samples(&want, &got);
    assert!(result.passed(), "{:?}", result.failures.first());
    check_mesh(&app, cube, name);
}

#[test]
fn o49_frozen_restriction_matches_live_restriction() {
    let name = "restrict-bend";
    let (mut app, cube, tag, field, m) = setup(name);
    let frozen = weights(&app, cube, tag);
    let p = &m["params"];
    app.doc
        .transact("Add restricted bend", |tx| {
            tx.rename(tag, p["restriction_name_1"].as_str().unwrap())?;
            let bend = tx.create("forge.bend", Some(cube), None)?;
            tx.set_param(
                bend,
                "strength",
                Value::Float(p["bend_strength"].as_float().unwrap()),
            )?;
            // The exported displacement direction distinguishes 90 radians from 90°.
            tx.set_param(bend, "angle", Value::Float(90.0))?;
            let restrict = tx.attach_tag(forge_core::tags::RESTRICTION, bend, None)?;
            tx.set_param(
                restrict,
                "selection",
                Value::String(p["restriction_name_1"].as_str().unwrap().into()),
            )
        })
        .unwrap();
    let live = app.eval_mesh(cube, 0).unwrap();
    app.doc
        .transact("Move live field", |tx| {
            tx.set_param(field, "transform.pos", Value::Vec3([0.0, 1000.0, 0.0]))
        })
        .unwrap();
    assert_ne!(live, app.eval_mesh(cube, 0).unwrap());
    app.doc
        .transact("Freeze and move field", |tx| {
            tx.set_param(tag, "weights", Value::FloatList(frozen))?;
            tx.set_param(tag, "use_fields", Value::Bool(false))?;
            tx.set_param(
                field,
                "transform.pos",
                Value::Vec3(DVec3::splat(1000.0).to_array()),
            )
        })
        .unwrap();
    assert_eq!(live, app.eval_mesh(cube, 0).unwrap());
    check_mesh(&app, cube, name);
}
