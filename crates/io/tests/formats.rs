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
    assert_eq!(
        formats().iter().map(|f| f.id).collect::<Vec<_>>(),
        ["obj", "stl", "ply", "gltf", "usda", "fbx", "abc"]
    );
    let obj = format_by_id("obj").unwrap();
    assert!(obj.import && obj.export);
    assert_eq!(obj.extensions, &["obj"]);
    assert_eq!(format_for_path("a/b/Model.OBJ").unwrap().id, "obj");
    assert_eq!(format_for_path("model.STL").unwrap().id, "stl");
    assert_eq!(format_for_path("model.ply").unwrap().id, "ply");
    assert_eq!(format_for_path("model.GLB").unwrap().id, "gltf");
    assert_eq!(format_for_path("model.gltf").unwrap().id, "gltf");
    assert_eq!(format_for_path("model.USDA").unwrap().id, "usda");
    let abc = format_for_path("model.ABC").unwrap();
    assert_eq!(abc.id, "abc");
    assert!(abc.import && abc.export);
    assert!(format_for_path("model.dxf").is_none());
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
        read_text("dxf", "", &none).unwrap_err(),
        IoError::UnknownFormat("dxf".into())
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

#[test]
fn stl_and_ply_options_and_byte_entry_points() {
    use forge_io::formats::{export_bytes, ply_write_options, read_bytes, stl_write_options};
    for id in ["stl", "ply"] {
        let descs = export_options(id).unwrap();
        let opts = default_options(&descs);
        assert_eq!(opts["binary"], Value::Bool(true), "{id}");
        assert_eq!(opts["scale"], Value::Float(1.0), "{id}");
        assert_eq!(import_options(id).unwrap().len(), 1);
    }
    assert_eq!(
        stl_write_options(&default_options(&export_options("stl").unwrap())),
        forge_io::stl::WriteOptions::default()
    );
    assert_eq!(
        ply_write_options(&default_options(&export_options("ply").unwrap())),
        forge_io::ply::WriteOptions::default()
    );
    let scene = cube_scene();
    for id in ["stl", "ply"] {
        let opts = default_options(&export_options(id).unwrap());
        let (bytes, n) = export_bytes(id, &scene, None, &opts).unwrap();
        assert_eq!(n, 1);
        let back = read_bytes(id, &bytes, &default_options(&import_options(id).unwrap())).unwrap();
        assert_eq!(back.len(), 1, "{id}");
        assert!(!back[0].mesh.polys.is_empty());
        // a binary file is not text
        assert_eq!(
            export_text(id, &scene, None, &opts).unwrap_err(),
            IoError::Unsupported(id.into(), "text export"),
            "{id}"
        );
    }
    // the ASCII forms are text
    let off = resolve_options(
        &export_options("stl").unwrap(),
        &map(json!({"binary": false})),
    )
    .unwrap();
    let (text, _) = export_text("stl", &scene, None, &off).unwrap();
    assert!(text.starts_with("solid "));
    let off = resolve_options(
        &export_options("ply").unwrap(),
        &map(json!({"binary": false})),
    )
    .unwrap();
    let (text, _) = export_text("ply", &scene, None, &off).unwrap();
    assert!(text.starts_with("ply\nformat ascii 1.0\n"));
}
