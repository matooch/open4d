//! docs/specs/UI.md §15.2: the "crashed last time" notice on the start after a crash report.

mod common;

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use forge_platform::crash::unseen;
use forge_ui::app::Editor;
use forge_ui::crash_notice::CrashNotice;

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("forge-crash-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn editor_with(dir: &std::path::Path) -> (Editor, Arc<Mutex<Vec<PathBuf>>>) {
    let opened = Arc::new(Mutex::new(Vec::new()));
    let mut e = Editor::new(common::state());
    e.state.crash_notice = CrashNotice::start(dir.to_path_buf());
    let log = opened.clone();
    e.state
        .crash_notice
        .set_opener(Box::new(move |p| log.lock().unwrap().push(p.to_path_buf())));
    (e, opened)
}

fn harness(e: Editor) -> Harness<'static, Editor> {
    Harness::builder()
        .with_size([1280.0, 800.0])
        .build_ui_state(|ui, e: &mut Editor| e.show(ui, None), e)
}

#[test]
fn no_notice_without_a_report() {
    let dir = temp_dir("none");
    let (e, _) = editor_with(&dir);
    let mut h = harness(e);
    h.step();
    assert!(h.query_by_label("Forge3D crashed last time").is_none());
}

#[test]
fn a_new_report_offers_the_folder_and_is_shown_once() {
    let dir = temp_dir("found");
    std::fs::write(dir.join("crash-1730000000-42-1.txt"), "report").unwrap();
    let (e, opened) = editor_with(&dir);
    let mut h = harness(e);
    h.step();
    h.get_by_label("Forge3D crashed last time");
    h.get_by_label("crash-1730000000-42-1.txt");
    assert!(
        h.query_by_label("Autosaved work may be offered in the recovery window.")
            .is_none(),
        "no recovery entries, no hint"
    );
    h.get_by_label("Open folder").click();
    h.step();
    h.step();
    assert!(h.query_by_label("Forge3D crashed last time").is_none());
    assert_eq!(*opened.lock().unwrap(), vec![dir.clone()]);
    // the report stays on disk but is not offered again
    assert!(dir.join("crash-1730000000-42-1.txt").exists());
    assert!(unseen(&dir).is_empty());
    let (next, _) = editor_with(&dir);
    assert!(!next.state.crash_notice.is_open());
}

#[test]
fn dismiss_marks_the_reports_seen_without_opening_anything() {
    let dir = temp_dir("dismiss");
    std::fs::write(dir.join("crash-1-1-1.txt"), "a").unwrap();
    let (e, opened) = editor_with(&dir);
    let mut h = harness(e);
    h.step();
    h.get_by_label("Dismiss").click();
    h.step();
    h.step();
    assert!(h.query_by_label("Forge3D crashed last time").is_none());
    assert!(opened.lock().unwrap().is_empty());
    assert!(unseen(&dir).is_empty());
    // a later crash is new again
    std::fs::write(dir.join("crash-2-1-1.txt"), "b").unwrap();
    assert_eq!(unseen(&dir).len(), 1);
}

#[test]
fn the_notice_points_to_the_recovery_window_when_there_is_something_to_recover() {
    let dir = temp_dir("recover");
    std::fs::write(dir.join("crash-1-1-1.txt"), "a").unwrap();
    let (mut e, _) = editor_with(&dir);
    // an autosave of a crashed session waiting for an answer
    let autosave_dir = temp_dir("recover-autosave");
    let clock = forge_platform::autosave::ManualClock::new(100);
    let mut first = forge_ui::autosave::Autosave::start(
        autosave_dir.clone(),
        forge_platform::autosave::Settings::default(),
        Box::new(clock.clone()),
    );
    let scene = e.state.doc.scene().clone();
    first.queue_now(&scene, None).unwrap();
    first.flush();
    first.simulate_crash();
    drop(first);
    e.state.autosave = forge_ui::autosave::Autosave::start(
        autosave_dir,
        forge_platform::autosave::Settings::default(),
        Box::new(forge_platform::autosave::ManualClock::new(200)),
    );
    assert_eq!(e.recoverable_count(), 1);
    let mut h = harness(e);
    h.step();
    h.get_by_label("Recover unsaved work");
    h.get_by_label("Autosaved work may be offered in the recovery window.");
}
