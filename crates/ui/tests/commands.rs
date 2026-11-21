//! The editor runs every action through the command registry (docs/specs/COMMANDS.md).

mod common;

use common::{create, state};
use forge_ui::commands::{create_menu, execute};
use forge_ui::state::EditorState;
use forge_viewport::render::Shading;
use serde_json::json;
use std::sync::Arc;

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

#[test]
fn create_menu_is_grouped_like_the_reference() {
    let registry = Arc::new(forge_app::default_registry().unwrap());
    let st = EditorState::new(registry);
    let menu = create_menu(st.doc.scene().registry());
    let labels: Vec<_> = menu.iter().map(|(g, _)| g.as_str()).collect();

    // Reference order (Clone toolset is rendered separately by the UI).
    assert!(
        labels.windows(2).all(|w| {
            let order = |s: &str| match s {
                "Object" => 0,
                "Spline" => 1,
                "Generator" => 2,
                "Deformer" => 3,
                "Field" => 4,
                "Volume" => 5,
                "Camera" => 6,
                "Light" => 7,
                "Tag" => 8,
                _ => 9,
            };
            order(w[0]) <= order(w[1])
        }),
        "create menu groups out of order: {labels:?}"
    );

    fn has(menu: &[(String, Vec<(String, String)>)], group: &str, id: &str) -> bool {
        menu.iter()
            .find(|(g, _)| g == group)
            .map(|(_, items)| items.iter().any(|(tid, _)| tid == id))
            .unwrap_or(false)
    }

    assert!(
        has(&menu, "Object", "forge.cube"),
        "cube should be in Object"
    );
    assert!(
        has(&menu, "Spline", "forge.circle_spline"),
        "circle should be in Spline"
    );
    assert!(
        has(&menu, "Generator", "forge.extrude"),
        "extrude should be in Generator"
    );
    assert!(
        has(&menu, "Deformer", "forge.bend"),
        "bend should be in Deformer"
    );
    assert!(
        has(&menu, "Field", "forge.field.linear"),
        "linear field should be in Field"
    );
    assert!(
        has(&menu, "Volume", "forge.volume_builder"),
        "volume builder should be in Volume"
    );
    assert!(
        has(&menu, "Camera", "forge.camera"),
        "camera should be in Camera"
    );
    assert!(
        has(&menu, "Light", "forge.light"),
        "light should be in Light"
    );
    assert!(
        has(&menu, "Tag", "forge.polygon_selection"),
        "polygon selection tag should be in Tag"
    );

    // Each group is sorted by label.
    for (group, items) in &menu {
        let labels: Vec<_> = items.iter().map(|(_, l)| l.as_str()).collect();
        let mut sorted = labels.clone();
        sorted.sort();
        assert_eq!(labels, sorted, "{group} not sorted by label");
    }
}

#[test]
fn create_menu_does_not_list_effectors() {
    let registry = Arc::new(forge_app::default_registry().unwrap());
    let menu = create_menu(registry.as_ref());
    let effector_ids: Vec<_> = registry
        .types()
        .filter(|t| t.category == forge_core::Category::Effector && t.in_create_menu)
        .map(|t| t.id.clone())
        .collect();
    assert!(
        !effector_ids.is_empty(),
        "registry should have effectors to check"
    );
    for (group, items) in &menu {
        for id in &effector_ids {
            assert!(
                !items.iter().any(|(tid, _)| tid == id),
                "effector {id} should not appear in create_menu group {group}"
            );
        }
    }
}
