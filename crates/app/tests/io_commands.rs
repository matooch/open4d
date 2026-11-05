//! docs/specs/IO.md §3.4: file.import, file.export, io.formats and the dialog requests.

use forge_app::App;
use forge_command::CommandError;
use forge_core::NodeId;
use serde_json::{Value, json};

fn dir(name: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("forge-io-cmd-{}-{name}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn p(d: &std::path::Path, name: &str) -> String {
    d.join(name).to_string_lossy().into_owned()
}

fn cube(app: &mut App) -> NodeId {
    let out = app
        .run("object.create", json!({"type": "forge.cube"}))
        .unwrap();
    NodeId(out["node"].as_u64().unwrap())
}

#[test]
fn export_matches_export_obj_and_infers_the_format() {
    let d = dir("same");
    let mut app = App::new_default();
    cube(&mut app);
    let r = app
        .run("file.export", json!({"path": p(&d, "a.OBJ")}))
        .unwrap();
    assert_eq!(r["format"], "obj");
    assert_eq!(r["objects"], 1);
    app.run("file.export_obj", json!({"path": p(&d, "b.obj")}))
        .unwrap();
    assert_eq!(
        std::fs::read(p(&d, "a.OBJ")).unwrap(),
        std::fs::read(p(&d, "b.obj")).unwrap()
    );
}

#[test]
fn export_options_apply() {
    let d = dir("opts");
    let mut app = App::new_default();
    cube(&mut app);
    app.run(
        "file.export",
        json!({"path": p(&d, "x.obj"), "options": {"normals": false, "weld": false}}),
    )
    .unwrap();
    let text = std::fs::read_to_string(p(&d, "x.obj")).unwrap();
    assert!(!text.contains("vn "));
    assert!(!text.contains("//"));
}

#[test]
fn import_options_apply_and_one_undo_step() {
    let d = dir("imp");
    std::fs::write(p(&d, "t.obj"), "o T\nv 0 0 1\nv 1 0 1\nv 0 1 1\nf 1 2 3\n").unwrap();
    let mut app = App::new_default();
    let r = app
        .run(
            "file.import",
            json!({"path": p(&d, "t.obj"), "options": {"flip_z": true}}),
        )
        .unwrap();
    assert_eq!(r["format"], "obj");
    let id = NodeId(r["nodes"][0].as_u64().unwrap());
    let mesh = app.eval_mesh(id, 0).unwrap();
    assert_eq!(mesh.points[0].z, -1.0);
    assert_eq!(mesh.polys, vec![vec![2, 1, 0]]);
    assert_eq!(app.doc.history().undo_labels().last(), Some(&"Import OBJ"));
    app.run("edit.undo", json!({})).unwrap();
    assert!(app.doc.scene().roots().is_empty());
}

#[test]
fn errors_keep_their_wording() {
    let d = dir("err");
    let mut app = App::new_default();
    let msg = |e: CommandError| e.to_string();
    let e = app
        .run("file.export", json!({"path": p(&d, "x.obj")}))
        .unwrap_err();
    assert!(msg(e).contains("Nothing to export"));
    cube(&mut app);
    let e = app
        .run("file.export", json!({"path": p(&d, "x.stl")}))
        .unwrap_err();
    let text = msg(e);
    assert!(text.contains("Export failed: unknown format"), "{text}");
    let e = app
        .run(
            "file.export",
            json!({"path": p(&d, "x.obj"), "options": {"zz": 1}}),
        )
        .unwrap_err();
    assert!(matches!(e, CommandError::InvalidArgs(_)));
    let e = app
        .run("file.import", json!({"path": p(&d, "missing.obj")}))
        .unwrap_err();
    assert!(msg(e).starts_with("Import failed"));
    std::fs::write(p(&d, "e.obj"), "# nothing\n").unwrap();
    let e = app
        .run("file.import", json!({"path": p(&d, "e.obj")}))
        .unwrap_err();
    assert!(msg(e).contains("no polygons"));
    let e = app
        .run(
            "file.import",
            json!({"path": p(&d, "e.obj"), "format": "x"}),
        )
        .unwrap_err();
    assert!(msg(e).contains("unknown format"));
}

#[test]
fn io_formats_describes_the_options() {
    let mut app = App::new_default();
    let r = app.run("io.formats", json!({})).unwrap();
    let obj = &r["formats"][0];
    assert_eq!(obj["id"], "obj");
    assert_eq!(obj["extensions"], json!(["obj"]));
    assert_eq!(obj["import"], true);
    let imp = &obj["import_options"][0];
    assert_eq!(
        (imp["id"].clone(), imp["kind"].clone()),
        (json!("flip_z"), json!("bool"))
    );
    assert_eq!(imp["default"], Value::Bool(false));
    let ids: Vec<&str> = obj["export_options"]
        .as_array()
        .unwrap()
        .iter()
        .map(|o| o["id"].as_str().unwrap())
        .collect();
    assert_eq!(ids, ["normals", "weld"]);
}

#[test]
fn dialog_commands_need_the_window() {
    let mut app = App::new_default();
    for (id, args) in [
        ("file.import_dialog", json!({})),
        ("file.export_dialog", json!({"selected": true})),
    ] {
        let e = app.run(id, args).unwrap_err();
        assert!(
            e.to_string().contains("needs the editor window"),
            "{id}: {e}"
        );
    }
    let e = app
        .run("file.import_dialog", json!({"format": "stl"}))
        .unwrap_err();
    assert!(matches!(e, CommandError::InvalidArgs(_)));
}
