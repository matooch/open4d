//! Workspaces and saved layouts (docs/specs/UI.md §21; brief 156): switching through the
//! commands and the menu keeps the document untouched; files round trip; broken files fall
//! back to the standard layout with a warning.

mod common;

use common::{create, state};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use forge_ui::app::Editor;
use forge_ui::commands::execute;
use forge_ui::dock::{Area, Panel};
use forge_ui::layout::Layout;
use forge_ui::state::EditorState;
use forge_ui::workspaces::{LayoutStore, Workspace};
use serde_json::json;

fn harness(state: EditorState, dir: &std::path::Path) -> Harness<'static, Editor> {
    let mut editor = Editor::new(state);
    editor.set_layout_store(LayoutStore::new(Some(dir.join("layouts"))));
    let mut h = Harness::builder()
        .with_size([1280.0, 900.0])
        .build_ui_state(|ui, e: &mut Editor| e.show(ui, None), editor);
    h.run();
    h
}

fn cmd(h: &mut Harness<'_, Editor>, id: &str, args: serde_json::Value) {
    execute(&mut h.state_mut().state, id, args).unwrap();
    h.run();
}

fn status(h: &Harness<'_, Editor>) -> String {
    h.state().state.status.clone()
}

/// A scene with a cube, to see that nothing changes it.
fn scene_state() -> EditorState {
    let mut s = state();
    create(&mut s, "forge.cube");
    create(&mut s, "forge.sphere");
    s
}

#[test]
fn every_workspace_switches_the_layout_and_leaves_the_document_alone() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = harness(scene_state(), dir.path());
    let revision = h.state().state.doc.scene().revision();
    let nodes = h.state().state.doc.scene().len();
    let undo = h.state().state.doc.history().undo_labels().len();
    let selection = h.state().state.selection.clone();
    for w in Workspace::ALL {
        cmd(&mut h, "ui.workspace", json!({"name": w.name()}));
        assert_eq!(h.state().layout(), w.layout(), "{}", w.name());
        assert_eq!(h.state().workspace(), w.name());
        assert_eq!(h.state().state.doc.scene().revision(), revision);
        assert_eq!(h.state().state.doc.scene().len(), nodes);
        assert_eq!(h.state().state.doc.history().undo_labels().len(), undo);
        assert_eq!(h.state().state.selection, selection);
        assert!(status(&h).is_empty(), "{}", status(&h));
    }
    // the names are matched without case; unknown names report and change nothing
    cmd(&mut h, "ui.workspace", json!({"name": "uv"}));
    assert_eq!(h.state().layout(), Workspace::Uv.layout());
    cmd(&mut h, "ui.workspace", json!({"name": "Sculpt"}));
    assert!(status(&h).contains("no workspace named"), "{}", status(&h));
    assert_eq!(h.state().layout(), Workspace::Uv.layout());
}

#[test]
fn the_node_workspace_shows_the_node_editor_in_the_centre() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = harness(scene_state(), dir.path());
    cmd(&mut h, "ui.workspace", json!({"name": "Node"}));
    let dock = &h.state().state.dock;
    assert_eq!(dock.active(Area::Center), Some(Panel::NodeEditor));
    assert!(dock.tabs(Area::Bottom).contains(&Panel::Console));
    assert!(h.query_by_label("Node Editor").is_some());
}

#[test]
fn the_workspace_menu_switches_and_resets() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = harness(scene_state(), dir.path());
    h.get_by_label("Workspace").click();
    h.run();
    // the open menu is drawn last; "Modeling" is also a palette group
    h.get_by_role_and_label(egui::accesskit::Role::Button, "Modeling")
        .click();
    h.run();
    assert_eq!(h.state().layout(), Workspace::Modeling.layout());
    h.get_by_label("Workspace").click();
    h.run();
    h.get_by_label("Reset layout").click();
    h.run();
    assert_eq!(h.state().layout(), Layout::standard());
}

#[test]
fn layouts_are_saved_loaded_and_deleted_through_commands() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = harness(scene_state(), dir.path());
    cmd(&mut h, "ui.workspace", json!({"name": "Animate"}));
    h.state_mut()
        .state
        .dock
        .move_panel(Panel::Script, Area::RightBottom);
    let mine = h.state().layout();
    cmd(&mut h, "ui.layout_save", json!({"name": "Mine"}));
    assert!(status(&h).is_empty(), "{}", status(&h));
    assert!(dir.path().join("layouts/Mine.json").exists());
    assert_eq!(h.state().saved_layouts(), ["Mine"]);
    cmd(&mut h, "ui.workspace", json!({"name": "Rigging"}));
    cmd(&mut h, "ui.layout_load", json!({"name": "Mine"}));
    assert_eq!(h.state().layout(), mine);
    // the menu lists it
    h.get_by_label("Workspace").click();
    h.run();
    assert!(h.query_by_label("Mine").is_some());
    h.key_press(egui::Key::Escape);
    h.run();
    cmd(&mut h, "ui.layout_delete", json!({"name": "Mine"}));
    assert!(h.state().saved_layouts().is_empty());
    cmd(&mut h, "ui.layout_load", json!({"name": "Mine"}));
    assert!(status(&h).contains("no layout named"), "{}", status(&h));
    // bad names are refused and write nothing
    cmd(&mut h, "ui.layout_save", json!({"name": "../x"}));
    assert!(status(&h).contains("bad layout name"), "{}", status(&h));
    cmd(&mut h, "ui.layout_save", json!({"name": "Modeling"}));
    assert!(status(&h).contains("workspace"), "{}", status(&h));
    assert!(h.state().saved_layouts().is_empty());
}

#[test]
fn a_broken_file_loads_the_standard_layout_with_a_warning() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = harness(scene_state(), dir.path());
    cmd(&mut h, "ui.workspace", json!({"name": "Node"}));
    std::fs::create_dir_all(dir.path().join("layouts")).unwrap();
    std::fs::write(dir.path().join("layouts/Broken.json"), "{\"version\": 7").unwrap();
    let old = Layout::standard()
        .to_json()
        .replace("\"version\": 1", "\"version\": 0");
    std::fs::write(dir.path().join("layouts/Old.json"), old).unwrap();
    for name in ["Broken", "Old"] {
        cmd(&mut h, "ui.workspace", json!({"name": "Node"}));
        cmd(&mut h, "ui.layout_load", json!({"name": name}));
        assert_eq!(h.state().layout(), Layout::standard(), "{name}");
        assert!(
            status(&h).contains("standard layout"),
            "{name}: {}",
            status(&h)
        );
    }
}

#[test]
fn the_last_layout_is_restored_at_the_next_start() {
    let dir = tempfile::tempdir().unwrap();
    let store = LayoutStore::new(Some(dir.path().join("layouts")));
    store.save_last(&Workspace::Rigging.layout()).unwrap();
    let mut editor = Editor::new(scene_state());
    editor.set_layout_store(store.clone());
    editor.restore_last_layout();
    assert_eq!(editor.layout(), Workspace::Rigging.layout());
    assert!(editor.state.status.is_empty());
    // a broken remembered file: standard layout and a warning
    std::fs::write(dir.path().join("layouts/_last.json"), "nonsense").unwrap();
    editor.restore_last_layout();
    assert_eq!(editor.layout(), Layout::standard());
    assert!(editor.state.status.contains("standard layout"));
}

#[test]
fn the_save_window_saves_under_the_typed_name() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = harness(scene_state(), dir.path());
    h.get_by_label("Workspace").click();
    h.run();
    h.get_by_label("Save layout as\u{2026}").click();
    h.run();
    h.get_by_role_and_label(egui::accesskit::Role::TextInput, "Name")
        .click();
    h.run();
    h.get_by_role_and_label(egui::accesskit::Role::TextInput, "Name")
        .type_text("Desk");
    h.run();
    h.run();
    h.get_by_label("Save").click();
    h.run();
    assert_eq!(
        h.state().saved_layouts(),
        ["Desk"],
        "status: {}",
        status(&h)
    );
}
