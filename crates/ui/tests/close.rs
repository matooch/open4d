//! docs/specs/UI.md §15.1: closing the window with unsaved changes asks first.

mod common;

use std::cell::RefCell;
use std::collections::VecDeque;
use std::path::PathBuf;
use std::rc::Rc;

use common::create;
use egui::{ViewportCommand, ViewportEvent, ViewportId};
use egui_kittest::Harness;
use forge_ui::app::{Dialogs, Editor, FileKind, Unsaved};

#[derive(Default)]
struct Script {
    save: VecDeque<Option<PathBuf>>,
    unsaved: VecDeque<Unsaved>,
    asked: usize,
}

struct Scripted(Rc<RefCell<Script>>);

impl Dialogs for Scripted {
    fn pick_open(&mut self, _: FileKind) -> Option<PathBuf> {
        None
    }

    fn pick_save(&mut self, _: FileKind, _: &str) -> Option<PathBuf> {
        self.0.borrow_mut().save.pop_front().expect("save dialog")
    }

    fn confirm_unsaved(&mut self, _: &str) -> Unsaved {
        let mut s = self.0.borrow_mut();
        s.asked += 1;
        s.unsaved.pop_front().expect("prompt")
    }
}

/// Builds a harness, requests a close, and returns whether the editor cancelled it.
fn close(
    dirty: bool,
    answer: Option<Unsaved>,
    save: Option<PathBuf>,
) -> (bool, Rc<RefCell<Script>>, bool) {
    let script = Rc::new(RefCell::new(Script::default()));
    script.borrow_mut().unsaved.extend(answer);
    script.borrow_mut().save.push_back(save);
    let mut e = Editor::new(common::state());
    e.set_dialogs(Box::new(Scripted(script.clone())));
    if dirty {
        create(&mut e.state, "test.box");
    }
    let mut h = Harness::builder()
        .with_size([800.0, 600.0])
        .build_ui_state(|ui, e: &mut Editor| e.show(ui, None), e);
    h.step();
    h.input_mut()
        .viewports
        .get_mut(&ViewportId::ROOT)
        .unwrap()
        .events
        .push(ViewportEvent::Close);
    h.step();
    let cancelled = h.output().viewport_output[&ViewportId::ROOT]
        .commands
        .iter()
        .any(|c| matches!(c, ViewportCommand::CancelClose));
    let still_dirty = h.state().is_dirty();
    (cancelled, script, still_dirty)
}

fn temp(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("forge-close-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

#[test]
fn a_clean_document_closes_without_asking() {
    let (cancelled, script, _) = close(false, None, None);
    assert!(!cancelled);
    assert_eq!(script.borrow().asked, 0);
}

#[test]
fn dont_save_closes() {
    let (cancelled, script, dirty) = close(true, Some(Unsaved::DontSave), None);
    assert!(!cancelled);
    assert_eq!(script.borrow().asked, 1);
    assert!(dirty, "nothing was saved");
}

#[test]
fn cancel_keeps_the_window_open() {
    let (cancelled, script, dirty) = close(true, Some(Unsaved::Cancel), None);
    assert!(cancelled);
    assert_eq!(script.borrow().asked, 1);
    assert!(dirty);
}

#[test]
fn save_saves_then_closes() {
    let path = temp("save").join("scene.forge");
    let (cancelled, _, dirty) = close(true, Some(Unsaved::Save), Some(path.clone()));
    assert!(!cancelled);
    assert!(!dirty);
    assert!(path.exists());
}

#[test]
fn save_with_a_cancelled_save_dialog_stays_open() {
    let (cancelled, _, dirty) = close(true, Some(Unsaved::Save), None);
    assert!(cancelled);
    assert!(dirty);
}
