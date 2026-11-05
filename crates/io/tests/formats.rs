//! docs/specs/IO.md §3: format table, options and text entry points.

use forge_core::{Document, Plugin, Value};
use forge_generators::Primitives;
use forge_io::formats::{
    IoError, default_options, export_options, export_text, format_by_id, format_for_path, formats,
    import_options, obj_read_options, obj_write_options, read_text, resolve_options,
};
use forge_io::obj::{ReadOptions, WriteOptions};
use serde_json::{Map, json};

fn map(v: serde_json::Value) -> Map<String, serde_json::Value> {
    v.as_object().cloned().unwrap()
}

fn cube_scene() -> forge_core::Scene {
    let mut registry = forge_core::Registry::new();
    Primitives.register(&mut registry).unwrap();
    let mut doc = Document::new(std::sync::Arc::new(registry));
    doc.transact("cube", |tx| tx.create("forge.cube", None, None))
        .unwrap();
    doc.scene().clone()
}

#[test]
fn table_and_extension_lookup() {
    assert_eq!(formats().len(), 1);
    let obj = format_by_id("obj").unwrap();
    assert!(obj.import && obj.export);
    assert_eq!(obj.extensions, &["obj"]);
    assert_eq!(format_for_path("a/b/Model.OBJ").unwrap().id, "obj");
    assert!(format_for_path("model.stl").is_none());
    assert!(format_for_path("noext").is_none());
    assert!(format_by_id("OBJ").is_none());
}

#[test]
fn option_defaults_equal_struct_defaults() {
    let read = obj_read_options(&default_options(&import_options("obj").unwrap()));
    assert_eq!(read, ReadOptions::default());
    let write = obj_write_options(&default_options(&export_options("obj").unwrap()));
    assert_eq!(write, WriteOptions::default());
    assert!(import_options("nope").is_none() && export_options("nope").is_none());
}

#[test]
fn resolve_options_rules() {
    let descs = export_options("obj").unwrap();
    let o = resolve_options(&descs, &map(json!({"normals": false}))).unwrap();
    assert_eq!(o["normals"], Value::Bool(false));
    assert_eq!(o["weld"], Value::Bool(true));
    let e = resolve_options(&descs, &map(json!({"nope": 1}))).unwrap_err();
    assert_eq!(e.to_string(), "unknown option \"nope\"");
    let e = resolve_options(&descs, &map(json!({"weld": 1}))).unwrap_err();
    assert_eq!(e.to_string(), "option \"weld\" must be a boolean");
}

#[test]
fn export_text_options_change_the_file() {
    let scene = cube_scene();
    let on = default_options(&export_options("obj").unwrap());
    let (text, n) = export_text("obj", &scene, None, &on).unwrap();
    assert_eq!(n, 1);
    assert!(text.lines().any(|l| l.starts_with("vn ")));
    assert!(
        text.lines()
            .any(|l| l.starts_with("f ") && l.contains("//"))
    );
    let off = resolve_options(
        &export_options("obj").unwrap(),
        &map(json!({"normals": false, "weld": false})),
    )
    .unwrap();
    let (plain, _) = export_text("obj", &scene, None, &off).unwrap();
    assert!(!plain.lines().any(|l| l.starts_with("vn ")));
    assert!(
        plain
            .lines()
            .filter(|l| l.starts_with("f "))
            .all(|l| !l.contains('/'))
    );
    assert_eq!(plain.lines().filter(|l| l.starts_with("v ")).count(), 8);
}

#[test]
fn read_text_flip_z_and_errors() {
    let text = "o T\nv 0 0 1\nv 1 0 1\nv 0 1 1\nf 1 2 3\n";
    let none = default_options(&import_options("obj").unwrap());
    let a = read_text("obj", text, &none).unwrap();
    assert_eq!(a[0].mesh.polys, vec![vec![0, 1, 2]]);
    let flip = resolve_options(
        &import_options("obj").unwrap(),
        &map(json!({"flip_z": true})),
    )
    .unwrap();
    let b = read_text("obj", text, &flip).unwrap();
    assert_eq!(b[0].mesh.polys, vec![vec![2, 1, 0]]);
    assert_eq!(b[0].mesh.points[0].z, -1.0);
    assert!(matches!(
        read_text("obj", "f 1 2 3", &none),
        Err(IoError::Parse(_))
    ));
    assert_eq!(
        read_text("stl", "", &none).unwrap_err(),
        IoError::UnknownFormat("stl".into())
    );
}

#[test]
fn export_text_errors() {
    let empty = Document::new(std::sync::Arc::new(forge_core::Registry::new()));
    let none = default_options(&export_options("obj").unwrap());
    assert_eq!(
        export_text("obj", empty.scene(), None, &none).unwrap_err(),
        IoError::NothingToExport
    );
    assert!(matches!(
        export_text("zzz", empty.scene(), None, &none),
        Err(IoError::UnknownFormat(_))
    ));
}
