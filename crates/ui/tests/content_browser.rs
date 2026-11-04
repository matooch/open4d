//! Content browser and recent files (docs/specs/UI.md §26; brief 154): the pure helpers and
//! egui_kittest interactions. Every action is a `browser.*` command; the browser state is
//! isolated in a temporary folder.

mod common;

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

use common::{create, state};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use forge_ui::app::{Dialogs, Editor, FileKind, Unsaved};
use forge_ui::commands::execute;
use forge_ui::content_browser::{load_command, menu_entries, preset_label, save_command};
use forge_ui::dock::{Area, Dock, Panel};
use forge_ui::state::EditorState;
use serde_json::json;

fn isolated(tmp: &tempfile::TempDir) -> EditorState {
    let mut s = state();
    s.app.browser.use_files(
        Some(tmp.path().join("settings.json")),
        Some(tmp.path().join("presets")),
    );
    s
}

fn harness(st: EditorState) -> Harness<'static, Editor> {
    let mut editor = Editor::new(st);
    editor.state.dock.open(Panel::ContentBrowser);
    let mut h = Harness::builder()
        .with_size([1400.0, 1000.0])
        .build_ui_state(|ui, editor: &mut Editor| editor.show(ui, None), editor);
    h.run();
    h
}

fn st<'a>(h: &'a Harness<'_, Editor>) -> &'a EditorState {
    &h.state().state
}

fn click(h: &mut Harness<'_, Editor>, label: &str) {
    h.get_by_label(label).click();
    h.run();
}

fn node_count(s: &EditorState) -> usize {
    let sc = s.doc.scene();
    sc.roots()
        .iter()
        .map(|r| 1 + sc.descendants(*r).len())
        .sum()
}

#[test]
fn pure_helpers() {
    assert_eq!(
        load_command("builtin:table"),
        ("browser.load", json!({"id": "builtin:table"}))
    );
    assert_eq!(preset_label("Table"), "Load Table");
    assert_eq!(
        save_command("  My Chair "),
        Some(("browser.save_preset", json!({"name": "My Chair"})))
    );
    assert_eq!(save_command(" !! "), None);
    let mut b = forge_command::browser::Browser::default();
    assert!(menu_entries(&b).is_empty());
    b.record_recent("/x/a.forge");
    b.record_recent("/x/b.forge");
    assert_eq!(
        menu_entries(&b),
        [(0, "b.forge".to_string()), (1, "a.forge".to_string())]
    );
}

#[test]
fn the_panel_is_closed_by_default_and_opens_from_the_window_menu() {
    assert_eq!(Dock::default().area_of(Panel::ContentBrowser), None);
    assert_eq!(Panel::ContentBrowser.title(), "Content Browser");
    let tmp = tempfile::tempdir().unwrap();
    let editor = Editor::new(isolated(&tmp));
    let mut h = Harness::builder()
        .with_size([1400.0, 1000.0])
        .build_ui_state(|ui, editor: &mut Editor| editor.show(ui, None), editor);
    h.run();
    assert!(h.query_by_label("Load Table").is_none());
    click(&mut h, "Window");
    click(&mut h, "Content Browser panel");
    assert_eq!(
        st(&h).dock.area_of(Panel::ContentBrowser),
        Some(Area::RightBottom)
    );
    h.get_by_label("Load Table");
    h.get_by_label("Load Pedestal");
    h.get_by_label("Load Snowman");
}

#[test]
fn clicking_a_preset_loads_it_with_one_undo_step() {
    let tmp = tempfile::tempdir().unwrap();
    let mut h = harness(isolated(&tmp));
    let depth = st(&h).doc.undo_depth();
    click(&mut h, "Load Table");
    assert_eq!(st(&h).doc.undo_depth(), depth + 1);
    assert_eq!(node_count(st(&h)), 6);
    assert_eq!(st(&h).selection.nodes.len(), 1);
    assert_eq!(
        st(&h).doc.history().undo_labels().last(),
        Some(&"Load Preset")
    );
    execute(&mut h.state_mut().state, "edit.undo", json!({})).unwrap();
    h.run();
    assert_eq!(node_count(st(&h)), 0);
}

#[test]
fn the_search_box_filters_the_presets() {
    let tmp = tempfile::tempdir().unwrap();
    let mut h = harness(isolated(&tmp));
    h.state_mut().state.browser_panel.query = "snow".into();
    h.run();
    h.get_by_label("Load Snowman");
    assert!(h.query_by_label("Load Table").is_none());
    h.state_mut().state.browser_panel.query = "zzz".into();
    h.run();
    h.get_by_label("No presets");
}

#[test]
fn saving_the_selection_adds_a_user_preset() {
    let tmp = tempfile::tempdir().unwrap();
    let mut s = isolated(&tmp);
    create(&mut s, "forge.cube");
    s.browser_panel.save_name = "My Cube".into();
    let mut h = harness(s);
    click(&mut h, "Save selection as preset");
    assert!(tmp.path().join("presets/objects/my-cube.forge").is_file());
    h.get_by_label("Load My Cube");
    // without a library folder the row is absent
    let h2 = harness(state());
    assert!(h2.query_by_label("Save selection as preset").is_none());
}

#[test]
fn the_recent_tab_opens_a_file_and_clears() {
    let tmp = tempfile::tempdir().unwrap();
    let mut s = isolated(&tmp);
    create(&mut s, "forge.cube");
    let path = tmp.path().join("scene one.forge");
    execute(
        &mut s,
        "file.save_as",
        json!({"path": path.to_string_lossy()}),
    )
    .unwrap();
    execute(&mut s, "file.new", json!({})).unwrap();
    assert_eq!(node_count(&s), 0);
    let mut h = harness(s);
    click(&mut h, "Recent");
    h.get_by_label("Recent scene one.forge");
    click(&mut h, "Recent scene one.forge");
    assert_eq!(node_count(st(&h)), 1);
    assert_eq!(st(&h).file_path.as_deref(), Some(&*path.to_string_lossy()));
    click(&mut h, "Clear recent");
    h.get_by_label("No recent files");
    assert!(st(&h).app.browser.recent.is_empty());
}

#[test]
fn the_file_menu_has_open_recent() {
    let tmp = tempfile::tempdir().unwrap();
    let mut s = isolated(&tmp);
    create(&mut s, "forge.cube");
    let path = tmp.path().join("menu.forge");
    execute(
        &mut s,
        "file.save_as",
        json!({"path": path.to_string_lossy()}),
    )
    .unwrap();
    execute(&mut s, "file.new", json!({})).unwrap();
    let mut h = harness(s);
    click(&mut h, "File");
    click(&mut h, "Open Recent");
    click(&mut h, "menu.forge");
    assert_eq!(node_count(st(&h)), 1);
}

struct Choose(Rc<RefCell<Vec<Unsaved>>>);

impl Dialogs for Choose {
    fn pick_open(&mut self, _: FileKind) -> Option<PathBuf> {
        panic!("no open dialog expected")
    }
    fn pick_save(&mut self, _: FileKind, _: &str) -> Option<PathBuf> {
        panic!("no save dialog expected")
    }
    fn confirm_unsaved(&mut self, _: &str) -> Unsaved {
        self.0.borrow_mut().remove(0)
    }
}

#[test]
fn opening_a_recent_file_asks_about_unsaved_changes() {
    let tmp = tempfile::tempdir().unwrap();
    let mut s = isolated(&tmp);
    let path = tmp.path().join("clean.forge");
    execute(
        &mut s,
        "file.save_as",
        json!({"path": path.to_string_lossy()}),
    )
    .unwrap();
    create(&mut s, "forge.cube"); // unsaved change
    let answers = Rc::new(RefCell::new(vec![Unsaved::Cancel, Unsaved::DontSave]));
    let mut editor = Editor::new(s);
    editor.set_dialogs(Box::new(Choose(answers.clone())));
    editor.state.dock.open(Panel::ContentBrowser);
    editor.state.browser_panel.tab = forge_ui::content_browser::Tab::Recent;
    let mut h = Harness::builder()
        .with_size([1400.0, 1000.0])
        .build_ui_state(|ui, editor: &mut Editor| editor.show(ui, None), editor);
    h.run();
    click(&mut h, "Recent clean.forge");
    assert_eq!(node_count(st(&h)), 1, "cancelled: the document stays");
    click(&mut h, "Recent clean.forge");
    assert_eq!(node_count(st(&h)), 0, "discarded: the file is opened");
    assert!(answers.borrow().is_empty());
}

#[test]
fn the_commands_work_in_a_window_less_state_too() {
    let tmp = tempfile::tempdir().unwrap();
    let mut s = isolated(&tmp);
    let r = execute(&mut s, "browser.load", json!({"id": "builtin:snowman"})).unwrap();
    assert_eq!(r["nodes"].as_array().unwrap().len(), 1);
    let r = execute(&mut s, "browser.list", json!({"query": "ped"})).unwrap();
    assert_eq!(r["presets"][0]["id"], "builtin:pedestal");
    let mut plain = state();
    let r = execute(&mut plain, "browser.recent", json!({})).unwrap();
    assert_eq!(r["files"].as_array().unwrap().len(), 0);
}
