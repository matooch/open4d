//! docs/specs/UI.md §18: the command palette and the undo history panel.

mod common;

use egui::{Key, Modifiers};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use forge_core::Value;
use forge_ui::app::Editor;
use forge_ui::commands::execute;
use forge_ui::dock::{Area, Dock, Panel};
use serde_json::json;

fn harness() -> Harness<'static, Editor> {
    let mut editor = Editor::new(common::state());
    editor.set_dialogs(Box::new(NoDialogs));
    Harness::builder()
        .with_size([1280.0, 800.0])
        .build_ui_state(|ui, editor: &mut Editor| editor.show(ui, None), editor)
}

struct NoDialogs;

impl forge_ui::app::Dialogs for NoDialogs {
    fn pick_open(&mut self, _: forge_ui::app::FileKind) -> Option<std::path::PathBuf> {
        None
    }
    fn pick_save(&mut self, _: forge_ui::app::FileKind, _: &str) -> Option<std::path::PathBuf> {
        None
    }
    fn confirm_unsaved(&mut self, _: &str) -> forge_ui::app::Unsaved {
        forge_ui::app::Unsaved::Cancel
    }
}

fn press(h: &mut Harness<'_, Editor>, mods: Modifiers, key: Key) {
    h.key_press_modifiers(mods, key);
    h.run();
}

fn open_palette(h: &mut Harness<'_, Editor>) {
    press(h, Modifiers::COMMAND | Modifiers::SHIFT, Key::P);
    assert!(h.state().palette().open, "Mod+Shift+P opens the palette");
    h.run();
}

fn type_query(h: &mut Harness<'_, Editor>, text: &str) {
    h.event(egui::Event::Text(text.to_string()));
    h.run();
}

fn undo_labels(h: &Harness<'_, Editor>) -> Vec<String> {
    h.state()
        .state
        .doc
        .history()
        .undo_labels()
        .iter()
        .map(|s| s.to_string())
        .collect()
}

fn click(h: &mut Harness<'_, Editor>, label: &str) {
    h.get_by_label(label).click();
    h.run();
}

#[test]
fn the_palette_opens_with_its_key_and_closes_with_escape() {
    let mut h = harness();
    h.run();
    assert!(!h.state().palette().open);
    open_palette(&mut h);
    h.get_by_label("Undo");
    press(&mut h, Modifiers::NONE, Key::Escape);
    assert!(!h.state().palette().open);
    // the key does nothing else: no document change, no status message
    assert!(undo_labels(&h).is_empty());
    assert!(h.state().state.status.is_empty());
    // Window > Command palette... opens it too
    click(&mut h, "Window");
    click(&mut h, "Command palette\u{2026}");
    assert!(h.state().palette().open);
}

#[test]
fn search_then_enter_runs_a_command_with_no_arguments() {
    let mut h = harness();
    let id = common::create(&mut h.state_mut().state, "test.box");
    h.run();
    open_palette(&mut h);
    type_query(&mut h, "object.delete");
    assert_eq!(h.state().palette().visible()[0].0, "object.delete");
    press(&mut h, Modifiers::NONE, Key::Enter);
    assert!(!h.state().palette().open);
    assert!(!h.state().state.doc.scene().contains(id));
    assert_eq!(undo_labels(&h).len(), 2); // create, delete
    // and it is one undo step
    press(&mut h, Modifiers::COMMAND, Key::Z);
    assert!(h.state().state.doc.scene().contains(id));
}

#[test]
fn clicking_a_row_runs_it_and_a_fuzzy_query_finds_by_label_and_chord() {
    let mut h = harness();
    common::create(&mut h.state_mut().state, "test.box");
    execute(&mut h.state_mut().state, "edit.undo", json!({})).unwrap();
    h.run();
    open_palette(&mut h);
    type_query(&mut h, "rdo");
    let hits = h.state().palette().visible();
    assert!(hits.iter().any(|(id, _)| id == "edit.redo"), "{hits:?}");
    h.get_all_by_label("Redo").last().unwrap().click();
    h.run();
    assert!(!h.state().palette().open);
    assert_eq!(h.state().state.doc.scene().roots().len(), 1);

    // the chord is searchable and shown
    open_palette(&mut h);
    type_query(&mut h, "mod+shift+p");
    assert_eq!(h.state().palette().visible()[0].0, "ui.command_palette");
    assert!(
        !h.get_all_by_label_contains("Ctrl+Shift+P").next().is_none(),
        "the chord is shown in the row"
    );
}

#[test]
fn a_preset_runs_a_bound_command_with_its_arguments() {
    let mut h = harness();
    open_palette(&mut h);
    type_query(&mut h, "shading wire");
    let first = h.state().palette().visible()[0].clone();
    assert_eq!(first, ("view.shading".to_string(), json!({"mode": "wire"})));
    press(&mut h, Modifiers::NONE, Key::Enter);
    assert_eq!(
        h.state().state.shading,
        forge_viewport::render::Shading::Wire
    );
    assert!(h.state().state.status.is_empty());
}

#[test]
fn arrow_keys_move_the_selection() {
    let mut h = harness();
    open_palette(&mut h);
    type_query(&mut h, "select");
    let hits = h.state().palette().visible();
    assert!(hits.len() > 2);
    press(&mut h, Modifiers::NONE, Key::ArrowDown);
    press(&mut h, Modifiers::NONE, Key::ArrowDown);
    press(&mut h, Modifiers::NONE, Key::ArrowUp);
    assert_eq!(h.state().palette().selected, 1);
    // typing more resets the selection to the first row
    type_query(&mut h, " all");
    assert_eq!(h.state().palette().selected, 0);
}

#[test]
fn a_command_with_required_arguments_opens_a_form() {
    let mut h = harness();
    open_palette(&mut h);
    type_query(&mut h, "object.create");
    assert_eq!(h.state().palette().visible()[0].0, "object.create");
    press(&mut h, Modifiers::NONE, Key::Enter);
    // the form replaces the list and the palette stays open
    assert!(h.state().palette().open && h.state().palette().form_open());
    h.get_all_by_label("type *").next().unwrap();
    h.get_by_label("Back");
    // Run with the field empty: an error, nothing happens
    click(&mut h, "Run");
    assert!(h.state().palette().form_open());
    assert_eq!(h.state().palette().form_error(), Some("type is required"));
    h.get_by_label("type is required");
    assert!(undo_labels(&h).is_empty());
    // fill in the field and run
    h.get_by_role_and_label(egui::accesskit::Role::TextInput, "type *")
        .focus();
    h.run();
    h.get_by_role_and_label(egui::accesskit::Role::TextInput, "type *")
        .type_text("test.box");
    h.run();
    click(&mut h, "Run");
    assert!(!h.state().palette().open);
    assert_eq!(h.state().state.doc.scene().roots().len(), 1);
    assert_eq!(undo_labels(&h).len(), 1);
    // the run is remembered under the generic entry too
    assert_eq!(
        h.state().palette().recent[0],
        r#"object.create {"type":"test.box"}"#
    );
    assert!(
        h.state()
            .palette()
            .recent
            .contains(&"object.create".to_string())
    );
}

#[test]
fn a_form_rejects_a_bad_value_and_back_returns_to_the_list() {
    let mut h = harness();
    open_palette(&mut h);
    type_query(&mut h, "timeline.set_frame");
    press(&mut h, Modifiers::NONE, Key::Enter);
    assert!(h.state().palette().form_open());
    h.get_by_role_and_label(egui::accesskit::Role::TextInput, "frame *")
        .focus();
    h.run();
    h.get_by_role_and_label(egui::accesskit::Role::TextInput, "frame *")
        .type_text("abc");
    h.run();
    click(&mut h, "Run");
    assert!(h.state().palette().open);
    let err = h.state().palette().form_error().unwrap().to_string();
    assert!(err.starts_with("frame:"), "{err}");
    click(&mut h, "Back");
    assert!(!h.state().palette().form_open() && h.state().palette().open);
}

#[test]
fn a_form_offers_the_values_of_an_enum() {
    let mut h = harness();
    open_palette(&mut h);
    type_query(&mut h, "view.gizmo_mode");
    press(&mut h, Modifiers::NONE, Key::Enter);
    assert!(h.state().palette().form_open());
    // the required choice starts at its first value; Run sends it
    click(&mut h, "Run");
    assert!(!h.state().palette().open);
    assert!(h.state().state.status.is_empty());
}

#[test]
fn recently_run_commands_come_first() {
    let mut h = harness();
    common::create(&mut h.state_mut().state, "test.box");
    h.run();
    press(&mut h, Modifiers::COMMAND, Key::Z);
    press(&mut h, Modifiers::COMMAND, Key::Y);
    open_palette(&mut h);
    let hits = h.state().palette().visible();
    assert_eq!(hits[0].0, "edit.redo");
    assert_eq!(hits[1].0, "edit.undo");
    // the palette's own command is not remembered
    assert!(
        h.state()
            .palette()
            .recent
            .iter()
            .all(|k| !k.contains("palette"))
    );
}

#[test]
fn shortcuts_do_not_fire_while_the_palette_is_open() {
    let mut h = harness();
    open_palette(&mut h);
    // `3` would set wire shading if it reached the key map
    type_query(&mut h, "3");
    assert_ne!(
        h.state().state.shading,
        forge_viewport::render::Shading::Wire
    );
}

// ---------------------------------------------------------------- history panel

fn history_harness() -> Harness<'static, Editor> {
    let mut h = harness();
    h.state_mut().state.dock.open(Panel::History);
    h.run();
    h
}

/// Three renames of one box: original, create, one, two, three.
fn three_steps(h: &mut Harness<'_, Editor>) -> forge_core::NodeId {
    let id = common::create(&mut h.state_mut().state, "test.box");
    for name in ["one", "two", "three"] {
        execute(
            &mut h.state_mut().state,
            "object.rename",
            json!({"node": id.0, "name": name}),
        )
        .unwrap();
    }
    h.run();
    id
}

fn row(h: &Harness<'_, Editor>, position: usize) -> String {
    let labels = h.state().state.doc.history().undo_labels().to_vec();
    let redo = h.state().state.doc.history().redo_labels().to_vec();
    let all: Vec<&str> = std::iter::once("Original state")
        .chain(labels.iter().copied())
        .chain(redo.iter().rev().copied())
        .collect();
    format!("{position}. {}", all[position])
}

#[test]
fn the_history_panel_is_closed_by_default_and_opens_from_the_window_menu() {
    assert_eq!(Dock::default().area_of(Panel::History), None);
    assert_eq!(Panel::History.title(), "History");
    let mut h = harness();
    h.run();
    click(&mut h, "Window");
    click(&mut h, "History panel");
    assert_eq!(
        h.state().state.dock.area_of(Panel::History),
        Some(Area::RightBottom)
    );
    h.get_by_label("0. Original state");
    click(&mut h, "Window");
    click(&mut h, "Reset layout");
    assert_eq!(h.state().state.dock.area_of(Panel::History), None);
}

#[test]
fn rows_follow_edits_and_the_current_state_is_marked() {
    let mut h = history_harness();
    three_steps(&mut h);
    for p in 0..=4 {
        h.get_by_label(&row(&h, p));
    }
    assert_eq!(h.get_all_by_label("current").count(), 1);
    // undo moves the mark; the undone step stays listed
    press(&mut h, Modifiers::COMMAND, Key::Z);
    assert_eq!(h.state().state.doc.history().undo_labels().len(), 3);
    h.get_by_label(&row(&h, 4));
    assert_eq!(h.get_all_by_label("current").count(), 1);
    // a new edit drops the redo rows
    let id = h.state().state.selection.active.unwrap();
    execute(
        &mut h.state_mut().state,
        "object.rename",
        json!({"node": id.0, "name": "again"}),
    )
    .unwrap();
    h.run();
    assert!(h.state().state.doc.history().redo_labels().is_empty());
    for p in 0..=4 {
        h.get_by_label(&row(&h, p));
    }
    assert!(h.query_by_label_contains("5. ").is_none());
}

#[test]
fn clicking_a_row_jumps_with_undo_and_redo() {
    let mut h = history_harness();
    let id = three_steps(&mut h);
    let name = |h: &Harness<'_, Editor>| h.state().state.doc.scene().node(id).unwrap().name.clone();
    assert_eq!(name(&h), "three");
    // back to the state after the first rename
    let target = row(&h, 2);
    click(&mut h, &target);
    assert_eq!(name(&h), "one");
    assert_eq!(h.state().state.doc.history().undo_labels().len(), 2);
    assert_eq!(h.state().state.doc.history().redo_labels().len(), 2);
    // the mark is on the clicked row
    assert_eq!(h.get_all_by_label("current").count(), 1);
    // forward again: a dimmed future row redoes
    let target = row(&h, 4);
    click(&mut h, &target);
    assert_eq!(name(&h), "three");
    assert!(h.state().state.doc.history().redo_labels().is_empty());
    // all the way back: the node is gone
    let target = row(&h, 0);
    click(&mut h, &target);
    assert!(!h.state().state.doc.scene().contains(id));
    assert!(h.state().state.selection.active.is_none());
    // clicking the current row changes nothing
    let target = row(&h, 0);
    click(&mut h, &target);
    assert_eq!(h.state().state.doc.history().redo_labels().len(), 4);
    // the jump is a registry command with no history step of its own
    assert!(undo_labels(&h).is_empty());
}

#[test]
fn a_merged_gesture_is_one_row() {
    let mut h = history_harness();
    let id = common::create(&mut h.state_mut().state, "test.box");
    for size in [3.0, 4.0, 5.0] {
        h.state_mut()
            .state
            .doc
            .transact_merged("Drag Size", "drag:size", |tx| {
                tx.set_param(id, "size", Value::Float(size))
            })
            .unwrap();
    }
    h.run();
    assert_eq!(undo_labels(&h).len(), 2);
    assert_eq!(h.get_all_by_label_contains("Drag Size").count(), 1);
    // one jump over the whole gesture
    click(&mut h, "1. Create Box");
    assert_eq!(
        h.state().state.doc.scene().param(id, "size"),
        Some(&Value::Float(2.0))
    );
}
