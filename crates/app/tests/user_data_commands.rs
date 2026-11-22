//! `user_data.add` command (brief 110).

use std::path::Path;

use forge_app::App;
use forge_core::{NodeId, ParamKind, Value};
use forge_io::export_scene;
use forge_io::obj::{WriteOptions, write};
use forge_parity_harness::{GoldenCase, Mesh, compare_meshes, discover_cases, parse_obj};
use serde_json::json;

#[test]
fn add_float_user_data_and_edit_it() {
    let mut app = App::new_default();
    app.run("object.create", json!({"type": "forge.null"}))
        .unwrap();
    let id = app.selection.active.unwrap();
    let r = app
        .run(
            "user_data.add",
            json!({
                "node": id.0,
                "name": "Amount",
                "kind": "float",
                "default": 1.0,
                "min": 0.0,
                "max": 10.0,
            }),
        )
        .unwrap();
    assert_eq!(r["param"], "user_data.0");
    assert_eq!(
        app.doc.scene().param(id, "user_data.0"),
        Some(&Value::Float(1.0))
    );

    app.run("param.edit", json!({"param": "user_data.0", "value": 7.5}))
        .unwrap();
    assert_eq!(
        app.doc.scene().param(id, "user_data.0"),
        Some(&Value::Float(7.5))
    );
}

#[test]
fn add_link_user_data_and_target_an_object() {
    let mut app = App::new_default();
    app.run("object.create", json!({"type": "forge.null"}))
        .unwrap();
    let holder = app.selection.active.unwrap();
    app.run("object.create", json!({"type": "forge.null"}))
        .unwrap();
    let target = app.selection.active.unwrap();
    let r = app
        .run(
            "user_data.add",
            json!({
                "node": holder.0,
                "name": "Target",
                "kind": "link",
                "default": target.0,
            }),
        )
        .unwrap();
    assert_eq!(r["param"], "user_data.0");
    assert_eq!(
        app.doc.scene().param(holder, "user_data.0"),
        Some(&Value::Link(Some(target)))
    );
}

fn user_data_case() -> GoldenCase {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden");
    discover_cases(&root)
        .unwrap()
        .into_iter()
        .find(|c| c.feature == "tags-2" && c.name == "user-data")
        .unwrap()
}

fn golden_mesh(case: &GoldenCase) -> Mesh {
    let src = std::fs::read_to_string(case.dir.join("mesh.obj")).unwrap();
    parse_obj(&src).unwrap()
}

fn exported(app: &App) -> Mesh {
    let objects = export_scene(app.doc.scene(), None);
    let opts = WriteOptions {
        normals: false,
        weld: false,
    };
    parse_obj(&write(&objects, &opts)).expect("our own file parses")
}

#[test]
fn o54_user_data_case_matches() {
    let case = user_data_case();
    let manifest = case.manifest().unwrap();
    let p = &manifest.params;
    let mut app = App::new_default();

    let holder = app
        .run("object.create", json!({"type": "forge.null"}))
        .unwrap();
    let holder = NodeId(holder["node"].as_u64().unwrap());
    let pos = p["object"]["position"].as_array().unwrap();
    app.run(
        "transform.set",
        json!({"node": holder.0, "pos": [pos[0], pos[1], pos[2]]}),
    )
    .unwrap();

    let cube = app
        .run("object.create", json!({"type": "forge.cube"}))
        .unwrap();
    let cube = NodeId(cube["node"].as_u64().unwrap());
    let pos2 = p["object_2"]["position"].as_array().unwrap();
    app.run(
        "transform.set",
        json!({"node": cube.0, "pos": [pos2[0], pos2[1], pos2[2]]}),
    )
    .unwrap();

    let ud = &p["user_data"];
    app.run(
        "user_data.add",
        json!({
            "node": holder.0,
            "name": "amount",
            "kind": "float",
            "default": ud["amount_default"],
            "min": ud["amount_min"],
            "max": ud["amount_max"],
        }),
    )
    .unwrap();
    app.run(
        "user_data.add",
        json!({
            "node": holder.0,
            "name": "target",
            "kind": "link",
            "default": cube.0,
        }),
    )
    .unwrap();

    let scene = app.doc.scene();
    let amount = scene.param_desc(holder, "user_data.0").unwrap();
    assert_eq!(amount.label, "amount");
    assert!(matches!(
        amount.kind,
        ParamKind::Float {
            min: 0.0,
            max: 10.0
        }
    ));
    assert_eq!(scene.param(holder, "user_data.0"), Some(&Value::Float(1.0)));

    let target = scene.param_desc(holder, "user_data.1").unwrap();
    assert_eq!(target.label, "target");
    assert!(matches!(target.kind, ParamKind::Link { .. }));
    assert_eq!(
        scene.param(holder, "user_data.1"),
        Some(&Value::Link(Some(cube)))
    );

    let golden = golden_mesh(&case);
    let ours = exported(&app);
    let r = compare_meshes(&golden, &ours, &manifest.tolerance);
    assert!(r.passed(), "{:?}", r.failures);
}
