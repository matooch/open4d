//! docs/specs/CORE.md §6: native scene file.

mod common;

use forge_core::{Document, EditError, LoadError, NodeId, Value, file};

fn rich_doc() -> Document {
    let mut doc = common::doc();
    doc.transact("build", |tx| {
        let root = tx.create("forge.null", None, None)?;
        tx.rename(root, "Root \"quoted\" ünïcode")?;
        let b = tx.create("test.box", Some(root), None)?;
        tx.create("test.bend", Some(b), None)?;
        let t = tx.attach_tag("test.sel", b, None)?;
        tx.set_param(t, "set", Value::String("line1\nline2".into()))?;
        tx.set_param(b, "size", Value::Float(0.1))?;
        tx.set_param(b, "segments", Value::Int(7))?;
        tx.set_param(b, "mode", Value::Enum("wire".into()))?;
        tx.set_param(b, "target", Value::Link(Some(root)))?;
        tx.set_param(b, "enabled", Value::Bool(false))?;
        tx.set_param(b, "offset", Value::Vec3([1.5, -2.25, 1e-9]))?;
        tx.set_param(b, "tint", Value::Color([0.2, 0.4, 8.0]))?;
        tx.create("forge.null", None, None)?;
        Ok(())
    })
    .unwrap();
    doc
}

#[test]
fn save_is_tagged_and_deterministic() {
    let a = file::save(rich_doc().scene());
    let b = file::save(rich_doc().scene());
    assert_eq!(a, b);
    assert!(a.contains("format: \"forge3d-scene\""), "{a}");
    assert!(a.contains("version: 1"), "{a}");
}

#[test]
fn round_trip_is_lossless() {
    let doc = rich_doc();
    let text = file::save(doc.scene());
    let (scene, report) = file::load(common::registry(), &text).unwrap();
    assert!(report.warnings.is_empty(), "{:?}", report.warnings);
    assert_eq!(file::save(&scene), text);
    let b = scene.roots()[0];
    let bx = scene.node(b).unwrap().children[0];
    assert_eq!(scene.param(bx, "size"), Some(&Value::Float(0.1)));
    assert_eq!(scene.param(bx, "target"), Some(&Value::Link(Some(b))));
}

#[test]
fn id_counter_resumes_after_max_id() {
    let text = r#"(format: "forge3d-scene", version: 1, roots: [
        (id: 7, type_id: "forge.null", name: "a", params: {}, tags: [], children: []),
        (id: 3, type_id: "forge.null", name: "b", params: {}, tags: [], children: []),
    ])"#;
    let (scene, _) = file::load(common::registry(), text).unwrap();
    let mut doc = Document::from_scene(scene);
    let n = doc
        .transact("n", |tx| tx.create("forge.null", None, None))
        .unwrap();
    assert_eq!(n, NodeId(8));
    assert!(doc.history().undo_labels().len() == 1);
}

fn load_err(text: &str) -> LoadError {
    file::load(common::registry(), text).unwrap_err()
}

#[test]
fn structural_problems_are_errors() {
    assert!(matches!(load_err("not ron at all ("), LoadError::Parse(_)));
    assert!(matches!(
        load_err(r#"(format: "other", version: 1, roots: [])"#),
        LoadError::WrongFormat(_)
    ));
    assert!(matches!(
        load_err(r#"(format: "forge3d-scene", version: 2, roots: [])"#),
        LoadError::UnsupportedVersion(2)
    ));
    assert!(matches!(
        load_err(
            r#"(format: "forge3d-scene", version: 1, roots: [
            (id: 1, type_id: "forge.null", name: "a", params: {}, tags: [], children: [
                (id: 1, type_id: "forge.null", name: "b", params: {}, tags: [], children: []),
            ]),
        ])"#
        ),
        LoadError::DuplicateId(NodeId(1))
    ));
    assert!(matches!(
        load_err(
            r#"(format: "forge3d-scene", version: 1, roots: [
            (id: 1, type_id: "test.sel", name: "tag in tree", params: {}, tags: [], children: []),
        ])"#
        ),
        LoadError::Structure(_)
    ));
    assert!(matches!(
        load_err(
            r#"(format: "forge3d-scene", version: 1, roots: [
            (id: 1, type_id: "forge.null", name: "a", params: {}, children: [], tags: [
                (id: 2, type_id: "forge.null", name: "not a tag", params: {}, tags: [], children: []),
            ]),
        ])"#
        ),
        LoadError::Structure(_)
    ));
}

#[test]
fn recoverable_problems_become_warnings() {
    let text = r#"(format: "forge3d-scene", version: 1, roots: [
        (id: 1, type_id: "test.box", name: "b", tags: [], children: [], params: {
            "size": Bool(true),
            "segments": Int(99999),
            "retired_param": Float(1.0),
        }),
    ])"#;
    let (scene, report) = file::load(common::registry(), text).unwrap();
    let id = NodeId(1);
    assert_eq!(
        scene.param(id, "size"),
        Some(&Value::Float(100.0)),
        "mismatch -> default"
    );
    assert_eq!(
        scene.param(id, "segments"),
        Some(&Value::Int(1000)),
        "clamped"
    );
    assert_eq!(scene.param(id, "retired_param"), None, "dropped");
    assert_eq!(
        scene.param(id, "mode"),
        Some(&Value::Enum("solid".into())),
        "filled"
    );
    assert_eq!(report.warnings.len(), 3, "{:?}", report.warnings);
    assert!(
        report.warnings.iter().all(|w| w.contains("node 1")),
        "{:?}",
        report.warnings
    );
}

#[test]
fn unknown_types_are_kept_verbatim_but_not_editable() {
    let text = r#"(format: "forge3d-scene", version: 1, roots: [
        (id: 4, type_id: "gone.plugin", name: "orphan", tags: [], params: {"weird": Vec3((1.0, 2.0, 3.0))}, children: [
            (id: 5, type_id: "forge.null", name: "kid", params: {}, tags: [], children: []),
        ]),
    ])"#;
    let (scene, report) = file::load(common::registry(), text).unwrap();
    assert_eq!(report.warnings.len(), 1, "{:?}", report.warnings);
    assert!(report.warnings[0].contains("gone.plugin"));
    assert_eq!(
        scene.param(NodeId(4), "weird"),
        Some(&Value::Vec3([1.0, 2.0, 3.0]))
    );
    assert_eq!(scene.node(NodeId(4)).unwrap().children, vec![NodeId(5)]);

    let resaved = file::save(&scene);
    let (again, _) = file::load(common::registry(), &resaved).unwrap();
    assert_eq!(file::save(&again), resaved);

    let mut doc = Document::from_scene(scene);
    let e = doc
        .transact("x", |tx| {
            tx.set_param(NodeId(4), "weird", Value::Vec3([0.0; 3]))
        })
        .unwrap_err();
    assert!(matches!(e, EditError::UnknownType(_)));
    // structure edits on unknown nodes still work
    doc.transact("mv", |tx| tx.move_node(NodeId(5), None, None))
        .unwrap();
    doc.transact("rn", |tx| tx.rename(NodeId(4), "renamed"))
        .unwrap();
}
