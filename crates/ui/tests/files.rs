//! docs/specs/IO.md §2: the File menu, driven with injected dialogs.

mod common;

use std::cell::RefCell;
use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use common::create;
use egui::{Key, Modifiers};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use forge_core::Value;
use forge_ui::app::{Dialogs, Editor, FileKind, Unsaved};
use forge_ui::commands::{Command, execute};

#[derive(Default)]
struct Script {
    open: VecDeque<Option<PathBuf>>,
    save: VecDeque<Option<PathBuf>>,
    unsaved: VecDeque<Unsaved>,
    /// What was asked, in order.
    log: Vec<String>,
}

struct Scripted(Rc<RefCell<Script>>);

impl Dialogs for Scripted {
    fn pick_open(&mut self, kind: FileKind) -> Option<PathBuf> {
        let mut s = self.0.borrow_mut();
        s.log.push(format!("open {kind:?}"));
        s.open.pop_front().expect("unexpected open dialog")
    }

    fn pick_save(&mut self, kind: FileKind, default_name: &str) -> Option<PathBuf> {
        let mut s = self.0.borrow_mut();
        s.log.push(format!("save {kind:?} {default_name}"));
        s.save.pop_front().expect("unexpected save dialog")
    }

    fn confirm_unsaved(&mut self, name: &str) -> Unsaved {
        let mut s = self.0.borrow_mut();
        s.log.push(format!("unsaved {name}"));
        s.unsaved.pop_front().expect("unexpected prompt")
    }
}

fn editor() -> (Editor, Rc<RefCell<Script>>) {
    let script = Rc::new(RefCell::new(Script::default()));
    let mut e = Editor::new(common::state());
    e.set_dialogs(Box::new(Scripted(script.clone())));
    (e, script)
}

/// A fresh empty directory for one test.
fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("forge-files-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn add_box(e: &mut Editor) -> forge_core::NodeId {
    create(&mut e.state, "test.box")
}

fn names(e: &Editor) -> Vec<String> {
    let s = e.state.doc.scene();
    s.roots()
        .iter()
        .map(|r| s.node(*r).unwrap().name.clone())
        .collect()
}

#[test]
fn save_then_open_with_injected_paths_and_dirty_marker() {
    let dir = temp_dir("save-open");
    let path = dir.join("scene.forge");
    let (mut e, script) = editor();
    assert!(!e.is_dirty());
    assert_eq!(e.window_title(), "Untitled - Forge3D");
    let id = add_box(&mut e);
    execute(&mut e.state, Command::Rename(id, "My box".into())).unwrap();
    assert!(e.is_dirty());
    assert_eq!(e.window_title(), "Untitled* - Forge3D");

    // Save with no file yet asks for a path (an extension is added)
    script.borrow_mut().save.push_back(Some(dir.join("scene")));
    assert!(e.save());
    assert!(!e.is_dirty());
    assert_eq!(e.file_path(), Some(path.as_path()));
    assert_eq!(e.window_title(), "scene.forge - Forge3D");
    assert_eq!(script.borrow().log, ["save Scene Untitled.forge"]);

    // a further edit marks it dirty, Save writes without asking and clears the marker
    execute(&mut e.state, Command::Rename(id, "Renamed".into())).unwrap();
    assert_eq!(e.window_title(), "scene.forge* - Forge3D");
    assert!(e.save());
    assert!(!e.is_dirty());
    assert_eq!(script.borrow().log.len(), 1, "no second dialog");

    // load it into another editor through Open
    let (mut other, script2) = editor();
    script2.borrow_mut().open.push_back(Some(path.clone()));
    other.open_scene();
    assert_eq!(names(&other), ["Renamed"]);
    assert!(!other.is_dirty());
    assert_eq!(other.file_path(), Some(path.as_path()));
    assert!(other.state.status.is_empty(), "{}", other.state.status);
    // the loaded scene equals the saved one
    assert_eq!(
        forge_core::file::save(other.state.doc.scene()),
        forge_core::file::save(e.state.doc.scene())
    );
    // loading is not an undoable edit
    assert!(other.state.doc.history().undo_labels().is_empty());
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn save_as_changes_the_current_file_and_cancel_changes_nothing() {
    let dir = temp_dir("save-as");
    let (mut e, script) = editor();
    add_box(&mut e);
    script.borrow_mut().save.push_back(None);
    assert!(!e.save_as());
    assert!(e.is_dirty());
    assert_eq!(e.file_path(), None);
    let a = dir.join("a.forge");
    let b = dir.join("b.forge");
    script.borrow_mut().save.push_back(Some(a.clone()));
    assert!(e.save());
    script.borrow_mut().save.push_back(Some(b.clone()));
    assert!(e.save_as());
    assert_eq!(e.file_path(), Some(b.as_path()));
    assert!(a.exists() && b.exists());
    // the second dialog proposed the file name of the first
    assert_eq!(script.borrow().log.last().unwrap(), "save Scene a.forge");
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn failed_save_and_failed_open_leave_things_alone() {
    let dir = temp_dir("errors");
    let (mut e, script) = editor();
    add_box(&mut e);
    script
        .borrow_mut()
        .save
        .push_back(Some(dir.join("missing-dir").join("x.forge")));
    assert!(!e.save());
    assert!(e.is_dirty());
    assert!(
        e.state.status.starts_with("Save failed"),
        "{}",
        e.state.status
    );

    let bad = dir.join("bad.forge");
    std::fs::write(&bad, "this is not a scene").unwrap();
    script.borrow_mut().unsaved.push_back(Unsaved::DontSave);
    script.borrow_mut().open.push_back(Some(bad));
    e.open_scene();
    assert!(
        e.state.status.starts_with("Open failed"),
        "{}",
        e.state.status
    );
    assert_eq!(names(&e).len(), 1, "the document stays");
    assert!(e.is_dirty());
    assert_eq!(e.file_path(), None);

    // a missing file
    e.open_path(&dir.join("nothing.forge"));
    assert!(e.state.status.starts_with("Open failed"));
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn new_and_open_ask_about_unsaved_changes() {
    let dir = temp_dir("prompt");
    let other = dir.join("other.forge");
    // a saved file to open
    {
        let (mut e, script) = editor();
        add_box(&mut e);
        script.borrow_mut().save.push_back(Some(other.clone()));
        assert!(e.save());
    }
    let (mut e, script) = editor();
    // clean: no prompt
    e.new_scene();
    assert!(script.borrow().log.is_empty());

    add_box(&mut e);
    // Cancel keeps everything
    script.borrow_mut().unsaved.push_back(Unsaved::Cancel);
    e.new_scene();
    assert_eq!(names(&e).len(), 1);
    assert!(e.is_dirty());

    // Don't Save discards
    script.borrow_mut().unsaved.push_back(Unsaved::DontSave);
    e.new_scene();
    assert!(names(&e).is_empty());
    assert!(!e.is_dirty());

    // Open asks too; Save saves (here with a path from the dialog) and then opens
    add_box(&mut e);
    let mine = dir.join("mine.forge");
    script.borrow_mut().open.push_back(Some(other.clone()));
    script.borrow_mut().unsaved.push_back(Unsaved::Save);
    script.borrow_mut().save.push_back(Some(mine.clone()));
    e.open_scene();
    assert!(mine.exists(), "saved before opening");
    assert_eq!(e.file_path(), Some(other.as_path()));
    assert!(!e.is_dirty());

    // Save with the save dialog cancelled: the whole action is cancelled
    let (mut e, script) = editor();
    add_box(&mut e);
    script.borrow_mut().open.push_back(Some(other.clone()));
    script.borrow_mut().unsaved.push_back(Unsaved::Save);
    script.borrow_mut().save.push_back(None);
    e.open_scene();
    assert_eq!(e.file_path(), None, "not opened");
    assert!(e.is_dirty());
    assert_eq!(names(&e).len(), 1);
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn shortcuts_save_and_open() {
    let dir = temp_dir("shortcuts");
    let path = dir.join("keys.forge");
    let (mut editor, script) = editor();
    add_box(&mut editor);
    script.borrow_mut().save.push_back(Some(path.clone()));
    let mut h = Harness::builder()
        .with_size([1280.0, 800.0])
        .build_ui_state(|ui, e: &mut Editor| e.show(ui, None), editor);
    h.run();
    h.key_press_modifiers(Modifiers::COMMAND, Key::S);
    h.run();
    assert!(path.exists(), "Ctrl+S saved");
    assert!(!h.state().is_dirty());
    assert_eq!(script.borrow().log.len(), 1);

    // Ctrl+Shift+S always asks
    let copy = dir.join("copy.forge");
    script.borrow_mut().save.push_back(Some(copy.clone()));
    h.key_press_modifiers(Modifiers::COMMAND | Modifiers::SHIFT, Key::S);
    h.run();
    assert!(copy.exists());
    assert_eq!(h.state().file_path(), Some(copy.as_path()));

    // Ctrl+O opens
    script.borrow_mut().open.push_back(Some(path.clone()));
    h.key_press_modifiers(Modifiers::COMMAND, Key::O);
    h.run();
    assert_eq!(h.state().file_path(), Some(path.as_path()));
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn file_menu_items_and_export_selected_needs_a_selection() {
    let dir = temp_dir("menu");
    let (editor, script) = editor();
    let mut h = Harness::builder()
        .with_size([1280.0, 800.0])
        .build_ui_state(|ui, e: &mut Editor| e.show(ui, None), editor);
    h.get_by_label("File").click();
    h.run();
    for label in [
        "New",
        "Open\u{2026}",
        "Save",
        "Save As\u{2026}",
        "Import OBJ\u{2026}",
        "Export OBJ\u{2026}",
        "Export Selected as OBJ\u{2026}",
    ] {
        assert!(
            h.query_all_by_label_contains(label).next().is_some(),
            "{label}"
        );
    }
    // nothing selected: the item is disabled, so clicking does nothing
    h.get_by_label_contains("Export Selected as OBJ\u{2026}")
        .click();
    h.run();
    assert!(script.borrow().log.is_empty());

    // Save from the menu
    let path = dir.join("menu.forge");
    script.borrow_mut().save.push_back(Some(path.clone()));
    // reopen the menu if the click on the disabled item closed it
    if h.query_by_label_contains("Import OBJ").is_none() {
        h.get_by_label("File").click();
        h.run();
    }
    let items: Vec<_> = h.query_all_by_label_contains("Save").collect();
    println!("{} save items", items.len());
    // "Save" comes before "Save As..." in the menu
    items[0].click();
    h.run();
    assert!(path.exists());
    std::fs::remove_dir_all(dir).ok();
}

const TRI_OBJ: &str = "# triangle pair\no Left\nv 0 0 0\nv 10 0 0\nv 0 10 0\nf 1 2 3\n\
o Right\nv 20 0 0\nv 30 0 0\nv 20 10 5\nf 4 5 6\n";

#[test]
fn import_obj_creates_polygon_objects_in_one_undo_step() {
    let dir = temp_dir("import");
    let obj = dir.join("two.obj");
    std::fs::write(&obj, TRI_OBJ).unwrap();
    let (mut e, script) = editor();
    script.borrow_mut().open.push_back(Some(obj));
    e.import_obj();
    assert_eq!(names(&e), ["Left", "Right"]);
    let scene = e.state.doc.scene();
    let first = scene.roots()[0];
    assert_eq!(scene.node(first).unwrap().type_id, "forge.polygon");
    assert_eq!(scene.node(first).unwrap().tags.len(), 1, "a smoothing tag");
    let Some(Value::Mesh(m)) = scene.param(first, "mesh") else {
        panic!("mesh param")
    };
    assert_eq!(m.points.len(), 3);
    assert_eq!(e.state.selection.nodes.len(), 2);
    assert!(e.is_dirty());
    assert_eq!(e.state.doc.history().undo_labels(), ["Import OBJ"]);
    assert!(e.state.doc.undo());
    assert!(names(&e).is_empty(), "one undo removes both objects");

    // errors and empty files go to the status bar
    std::fs::write(dir.join("bad.obj"), "v 0 0 0\nf 1 2 3\n").unwrap();
    e.import_obj_path(&dir.join("bad.obj"));
    assert!(
        e.state.status.starts_with("Import failed"),
        "{}",
        e.state.status
    );
    std::fs::write(dir.join("empty.obj"), "# nothing\n").unwrap();
    e.import_obj_path(&dir.join("empty.obj"));
    assert_eq!(e.state.status, "Import: no polygons found");
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn export_obj_whole_scene_and_selection_then_import_back() {
    let dir = temp_dir("export");
    let (mut e, script) = editor();
    // nothing to export yet
    e.export_obj(false);
    assert_eq!(e.state.status, "Nothing to export");
    assert!(script.borrow().log.is_empty());

    execute(&mut e.state, Command::Create("forge.cube".into())).unwrap();
    let cube = e.state.selection.active.unwrap();
    execute(&mut e.state, Command::Create("forge.plane".into())).unwrap();
    let all = dir.join("all");
    script.borrow_mut().save.push_back(Some(all.clone()));
    e.export_obj(false);
    assert!(e.state.status.is_empty(), "{}", e.state.status);
    let text = std::fs::read_to_string(dir.join("all.obj")).unwrap();
    assert_eq!(text.lines().filter(|l| l.starts_with("o ")).count(), 2);
    assert_eq!(script.borrow().log, ["save Obj Untitled.obj"]);

    // only the selection
    e.state.selection.click(cube);
    let one = dir.join("one.obj");
    script.borrow_mut().save.push_back(Some(one.clone()));
    e.export_obj(true);
    let text = std::fs::read_to_string(&one).unwrap();
    assert_eq!(text.lines().filter(|l| l.starts_with("o ")).count(), 1);
    // a default cube: 8 points, 6 faces
    assert_eq!(text.lines().filter(|l| l.starts_with("v ")).count(), 8);
    assert_eq!(text.lines().filter(|l| l.starts_with("f ")).count(), 6);

    // the exported file imports back as a polygon object with the same mesh
    let before = e.state.doc.scene().len();
    e.import_obj_path(&one);
    assert!(e.state.doc.scene().len() > before);
    let new = e.state.selection.active.unwrap();
    let Some(Value::Mesh(m)) = e.state.doc.scene().param(new, "mesh") else {
        panic!("mesh param")
    };
    assert_eq!((m.points.len(), m.polys.len()), (8, 6));
    let _: &Path = &one;
    std::fs::remove_dir_all(dir).ok();
}
