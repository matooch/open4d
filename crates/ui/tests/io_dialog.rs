//! docs/specs/UI.md §27: the import/export options window.

mod common;

use std::cell::RefCell;
use std::collections::VecDeque;
use std::path::PathBuf;
use std::rc::Rc;

use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use forge_core::Value;
use forge_ui::app::{Dialogs, Editor, FileKind, Unsaved};
use forge_ui::commands::execute;
use serde_json::json;

#[derive(Default)]
struct Script {
    open: VecDeque<Option<PathBuf>>,
    save: VecDeque<Option<PathBuf>>,
}

struct Scripted(Rc<RefCell<Script>>);

impl Dialogs for Scripted {
    fn pick_open(&mut self, _kind: FileKind) -> Option<PathBuf> {
        self.0.borrow_mut().open.pop_front().expect("open dialog")
    }

    fn pick_save(&mut self, _kind: FileKind, _name: &str) -> Option<PathBuf> {
        self.0.borrow_mut().save.pop_front().expect("save dialog")
    }

    fn confirm_unsaved(&mut self, _name: &str) -> Unsaved {
        Unsaved::Cancel
    }
}

fn editor() -> (Editor, Rc<RefCell<Script>>) {
    let script = Rc::new(RefCell::new(Script::default()));
    let mut e = Editor::new(common::state());
    e.set_dialogs(Box::new(Scripted(script.clone())));
    (e, script)
}

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("forge-iodlg-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn add_cube(e: &mut Editor) {
    execute(&mut e.state, "object.create", json!({"type": "forge.cube"})).unwrap();
}

const TRI: &str = "o T\nv 0 0 1\nv 1 0 1\nv 0 1 1\nf 1 2 3\n";

#[test]
fn export_dialog_opens_after_the_chooser_and_options_change_the_file() {
    let dir = temp_dir("export");
    let (mut e, script) = editor();
    add_cube(&mut e);
    script.borrow_mut().save.push_back(Some(dir.join("a")));
    e.begin_export("obj", false);
    let d = e.io_dialog().expect("options window pending");
    assert!(d.export && !d.selected);
    assert_eq!(d.path, dir.join("a.obj"));
    assert_eq!(d.values["normals"], Value::Bool(true));
    assert!(!dir.join("a.obj").exists(), "nothing written before OK");
    e.io_dialog_set("normals", Value::Bool(false));
    e.io_dialog_set("nope", Value::Bool(false));
    e.io_dialog_confirm();
    assert!(e.io_dialog().is_none());
    let text = std::fs::read_to_string(dir.join("a.obj")).unwrap();
    assert!(!text.contains("vn "), "normals off");
    assert!(e.state.status.is_empty(), "{}", e.state.status);
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn import_dialog_flip_z_and_cancel() {
    let dir = temp_dir("import");
    let obj = dir.join("t.obj");
    std::fs::write(&obj, TRI).unwrap();
    let (mut e, script) = editor();
    script.borrow_mut().open.push_back(Some(obj.clone()));
    e.begin_import("obj");
    assert!(e.io_dialog().is_some());
    e.io_dialog_cancel();
    assert!(e.state.doc.scene().roots().is_empty());

    script.borrow_mut().open.push_back(Some(obj));
    e.begin_import("obj");
    e.io_dialog_set("flip_z", Value::Bool(true));
    e.io_dialog_confirm();
    let id = e.state.selection.active.unwrap();
    let Some(Value::Mesh(m)) = e.state.doc.scene().param(id, "mesh") else {
        panic!("mesh");
    };
    assert_eq!(m.points[0][2], -1.0);
    assert_eq!(e.state.doc.history().undo_labels(), ["Import OBJ"]);
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn a_cancelled_chooser_and_nothing_to_export_open_no_window() {
    let (mut e, script) = editor();
    e.begin_export("obj", false);
    assert_eq!(e.state.status, "Nothing to export");
    assert!(e.io_dialog().is_none());
    add_cube(&mut e);
    script.borrow_mut().save.push_back(None);
    e.begin_export("obj", false);
    assert!(e.io_dialog().is_none());
    script.borrow_mut().open.push_back(None);
    e.begin_import("obj");
    assert!(e.io_dialog().is_none());
    e.begin_import("stl");
    assert!(e.state.status.contains("unknown format"));
}

#[test]
fn window_shows_the_controls_and_buttons_drive_it() {
    let dir = temp_dir("window");
    let (mut editor, script) = editor();
    add_cube(&mut editor);
    script.borrow_mut().save.push_back(Some(dir.join("w.obj")));
    let mut h = Harness::builder()
        .with_size([1280.0, 800.0])
        .build_ui_state(|ui, e: &mut Editor| e.show(ui, None), editor);
    h.run();
    // the request command opens the chooser and then the window
    h.state_mut().begin_export("obj", false);
    h.run();
    assert!(h.query_by_label("Export Wavefront OBJ Options").is_some());
    h.get_by_label("Write normals").click();
    h.run();
    assert_eq!(
        h.state().io_dialog().unwrap().values["normals"],
        Value::Bool(false)
    );
    assert!(h.query_by_label("Merge equal points").is_some());
    h.get_by_label("Export").click();
    h.run();
    assert!(h.state().io_dialog().is_none());
    assert!(
        !std::fs::read_to_string(dir.join("w.obj"))
            .unwrap()
            .contains("vn ")
    );

    // Cancel and Escape
    script.borrow_mut().save.push_back(Some(dir.join("c.obj")));
    h.state_mut().begin_export("obj", false);
    h.run();
    h.get_by_label("Cancel").click();
    h.run();
    assert!(h.state().io_dialog().is_none());
    assert!(!dir.join("c.obj").exists());
    script.borrow_mut().save.push_back(Some(dir.join("c.obj")));
    h.state_mut().begin_export("obj", false);
    h.run();
    h.key_press(egui::Key::Escape);
    h.run();
    assert!(h.state().io_dialog().is_none());
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn dialog_commands_open_the_window() {
    let dir = temp_dir("request");
    let obj = dir.join("t.obj");
    std::fs::write(&obj, TRI).unwrap();
    let (mut e, script) = editor();
    script.borrow_mut().open.push_back(Some(obj));
    execute(&mut e.state, "file.import_dialog", json!({})).unwrap();
    // the window serves requests when it runs a frame
    let mut h = Harness::builder()
        .with_size([1280.0, 800.0])
        .build_ui_state(|ui, e: &mut Editor| e.show(ui, None), e);
    h.run();
    let d = h
        .state()
        .io_dialog()
        .expect("the request opened the window");
    assert!(!d.export);
    assert_eq!(d.format, "obj");
    std::fs::remove_dir_all(dir).ok();
}
