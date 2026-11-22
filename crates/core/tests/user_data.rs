//! docs/specs/CORE.md §3: per-node user data (brief 110).

mod common;

use forge_core::{Document, ParamDesc, Value, file};

fn null(doc: &mut Document) -> forge_core::NodeId {
    doc.transact("add", |tx| tx.create("forge.null", None, None))
        .unwrap()
}

#[test]
fn add_user_data_creates_a_parameter_with_default_value() {
    let mut doc = common::doc();
    let id = null(&mut doc);
    doc.transact("add field", |tx| {
        tx.add_user_data(
            id,
            ParamDesc::float("amount", "Amount", 1.0).range(0.0, 10.0),
        )
    })
    .unwrap();
    let scene = doc.scene();
    assert_eq!(scene.param(id, "user_data.0"), Some(&Value::Float(1.0)));
    let desc = scene.param_desc(id, "user_data.0").unwrap();
    assert_eq!(desc.label, "Amount");
    assert_eq!(desc.ui.group, "User Data");
}

#[test]
fn set_param_validates_user_data_ranges() {
    let mut doc = common::doc();
    let id = null(&mut doc);
    doc.transact("add field", |tx| {
        tx.add_user_data(
            id,
            ParamDesc::float("amount", "Amount", 1.0).range(0.0, 10.0),
        )
    })
    .unwrap();
    doc.transact("set", |tx| {
        tx.set_param(id, "user_data.0", Value::Float(25.0))
    })
    .unwrap();
    assert_eq!(
        doc.scene().param(id, "user_data.0"),
        Some(&Value::Float(10.0))
    );
}

#[test]
fn user_data_survives_save_and_load() {
    let mut doc = common::doc();
    let id = null(&mut doc);
    let target = doc
        .transact("add target", |tx| tx.create("forge.null", None, None))
        .unwrap();
    doc.transact("add fields", |tx| {
        tx.add_user_data(
            id,
            ParamDesc::float("amount", "Amount", 1.0).range(0.0, 10.0),
        )?;
        tx.set_param(id, "user_data.0", Value::Float(7.5))?;
        tx.add_user_data(
            id,
            ParamDesc::link("target", "Target", &[forge_core::Category::Object]),
        )?;
        tx.set_param(id, "user_data.1", Value::Link(Some(target)))?;
        Ok(())
    })
    .unwrap();

    let text = file::save(doc.scene());
    let (scene, report) = file::load(common::registry(), &text).unwrap();
    assert!(report.warnings.is_empty(), "{:?}", report.warnings);
    assert_eq!(scene.param(id, "user_data.0"), Some(&Value::Float(7.5)));
    assert_eq!(
        scene.param(id, "user_data.1"),
        Some(&Value::Link(Some(target)))
    );
    let desc = scene.param_desc(id, "user_data.0").unwrap();
    assert_eq!(desc.label, "Amount");
    assert_eq!(file::save(&scene), text);
}

#[test]
fn user_data_animates_like_a_regular_float() {
    let mut doc = common::doc();
    let id = null(&mut doc);
    doc.transact("add field", |tx| {
        tx.add_user_data(id, ParamDesc::float("amount", "Amount", 0.0))
    })
    .unwrap();
    doc.transact("track", |tx| {
        tx.set_track(
            id,
            "user_data.0",
            Some(forge_core::Track::new(vec![
                forge_core::Key::new(0.0, 0.0),
                forge_core::Key::new(1.0, 5.0),
            ])),
        )
    })
    .unwrap();
    let track = doc
        .scene()
        .node(id)
        .unwrap()
        .tracks
        .get("user_data.0")
        .cloned();
    assert!(track.is_some());
}

#[test]
fn undo_removes_user_data_field() {
    let mut doc = common::doc();
    let id = null(&mut doc);
    doc.transact("add field", |tx| {
        tx.add_user_data(id, ParamDesc::float("amount", "Amount", 1.0))
    })
    .unwrap();
    assert!(doc.scene().param_desc(id, "user_data.0").is_some());
    doc.undo();
    assert!(doc.scene().param_desc(id, "user_data.0").is_none());
    assert!(doc.scene().param(id, "user_data.0").is_none());
}
