//! The editor runs every action through the command registry (docs/specs/COMMANDS.md).

mod common;

use common::{create, state};
use forge_ui::commands::execute;
use forge_viewport::render::Shading;
use serde_json::json;

#[test]
fn the_editor_registry_holds_the_builtin_commands() {
    let st = state();
    for id in [
        "object.create",
        "timeline.settings",
        "param.set",
        "file.save_as",
    ] {
        assert!(st.commands.get(id).is_some(), "missing {id}");
    }
}

#[test]
fn unknown_command_and_bad_arguments_are_status_errors() {
    let mut st = state();
    assert!(
        execute(&mut st, "nope.nothing", json!({}))
            .unwrap_err()
            .contains("unknown command")
    );
    assert!(execute(&mut st, "object.create", json!({"typo": 1})).is_err());
    assert!(st.doc.history().undo_labels().is_empty());
}

#[test]
fn view_commands_change_view_state_only() {
    let mut st = state();
    execute(&mut st, "view.shading", json!({"mode": "wire"})).unwrap();
    assert_eq!(st.shading, Shading::Wire);
    let a = create(&mut st, "forge.null");
    let steps = st.doc.history().undo_labels().len();
    execute(&mut st, "view.toggle_expanded", json!({"node": a.0})).unwrap();
    assert!(st.expanded.contains(&a));
    execute(&mut st, "view.frame_all", json!({})).unwrap();
    assert_eq!(st.doc.history().undo_labels().len(), steps);
}

#[test]
fn transform_and_param_commands_are_single_undo_steps() {
    let mut st = state();
    let a = create(&mut st, "test.box");
    let steps = st.doc.history().undo_labels().len();
    execute(
        &mut st,
        "transform.set",
        json!({"node": a.0, "scale": [2.0, 2.0, 2.0]}),
    )
    .unwrap();
    execute(
        &mut st,
        "param.set",
        json!({"node": a.0, "param": "twist", "value": 90.0}),
    )
    .unwrap();
    assert_eq!(st.doc.history().undo_labels().len(), steps + 2);
    let twist = st.doc.scene().param(a, "twist").cloned();
    assert_eq!(
        twist,
        Some(forge_core::Value::Float(std::f64::consts::FRAC_PI_2))
    );
}

#[test]
fn key_commands_record_jump_and_toggle_autokey() {
    let mut st = state();
    create(&mut st, "forge.cube");
    execute(&mut st, "timeline.set_frame", json!({"frame": 10})).unwrap();
    let r = execute(&mut st, "key.record", json!({})).unwrap();
    assert_eq!(r["keys"], 9);
    execute(&mut st, "timeline.set_frame", json!({"frame": 0})).unwrap();
    let r = execute(&mut st, "key.jump", json!({"forward": true})).unwrap();
    assert_eq!(r["frame"], 10);
    assert_eq!(
        execute(&mut st, "key.autokey", json!({})).unwrap()["enabled"],
        true
    );
    assert!(st.autokey);
}

#[test]
fn file_new_resets_the_document_but_keeps_the_dock() {
    let mut st = state();
    create(&mut st, "forge.cube");
    st.dock.close(forge_ui::dock::Panel::Timeline);
    execute(&mut st, "file.new", json!({})).unwrap();
    assert!(st.doc.scene().is_empty());
    assert!(st.dock.area_of(forge_ui::dock::Panel::Timeline).is_none());
    assert!(st.commands.get("object.create").is_some());
}
