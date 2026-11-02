//! docs/specs/PLATFORM.md §7 and UI.md §15: autosave schedule, clean exit, crash and
//! recovery, driven with a manual clock and temp directories.

mod common;

use std::path::{Path, PathBuf};

use common::create;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use forge_core::file::{SaveOptions, save_with};
use forge_platform::autosave::{
    ManualClock, SessionMarker, Settings, crashed_sessions, data_path, list_entries,
};
use forge_ui::app::Editor;
use forge_ui::autosave::Autosave;
use forge_ui::commands::execute;
use serde_json::json;

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("forge-autosave-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn settings(interval: u64) -> Settings {
    Settings {
        interval_seconds: interval,
        ..Settings::default()
    }
}

fn editor(dir: &Path, clock: &ManualClock, s: Settings) -> Editor {
    let mut e = Editor::new(common::state());
    e.state.autosave = Autosave::start(dir.to_path_buf(), s, Box::new(clock.clone()));
    e
}

fn entry_files(dir: &Path) -> Vec<String> {
    list_entries(dir).into_iter().map(|e| e.key).collect()
}

fn scene_bytes(e: &Editor) -> Vec<u8> {
    save_with(e.state.doc.scene(), &SaveOptions::default())
}

#[test]
fn autosaves_only_when_changed_and_the_interval_passed() {
    let dir = temp_dir("schedule");
    let clock = ManualClock::new(1_730_000_000);
    let mut e = editor(&dir, &clock, settings(60));
    create(&mut e.state, "test.box");
    let expected = scene_bytes(&e);
    let mut h = Harness::builder()
        .with_size([1280.0, 800.0])
        .build_ui_state(|ui, e: &mut Editor| e.show(ui, None), e);
    h.step();
    clock.advance(59.0);
    h.step();
    h.state_mut().state.autosave.flush();
    assert!(entry_files(&dir).is_empty(), "too early");

    clock.advance(1.0);
    h.step();
    assert!(h.state_mut().state.autosave.flush().is_empty());
    let entries = list_entries(&dir);
    assert_eq!(entries.len(), 1);
    assert!(entries[0].key.starts_with("untitled-"));
    assert_eq!(std::fs::read(&entries[0].data).unwrap(), expected);
    assert_eq!(entries[0].meta.as_ref().unwrap().original, None);
    assert_eq!(entries[0].meta.as_ref().unwrap().saved_at, 1_730_000_060);

    // unchanged since the last autosave: nothing is written, even much later
    std::fs::remove_file(&entries[0].data).unwrap();
    clock.advance(3600.0);
    h.step();
    h.state_mut().state.autosave.flush();
    assert!(entry_files(&dir).is_empty());

    // an edit and another interval: written again
    create(&mut h.state_mut().state, "test.box");
    clock.advance(60.0);
    h.step();
    h.state_mut().state.autosave.flush();
    assert_eq!(entry_files(&dir).len(), 1);
}

#[test]
fn disabled_autosave_does_not_run_but_the_command_still_does() {
    let dir = temp_dir("disabled");
    let clock = ManualClock::new(5);
    let mut e = editor(
        &dir,
        &clock,
        Settings {
            enabled: false,
            ..settings(1)
        },
    );
    create(&mut e.state, "test.box");
    let mut h = Harness::builder().build_ui_state(|ui, e: &mut Editor| e.show(ui, None), e);
    clock.advance(1000.0);
    h.step();
    h.state_mut().state.autosave.flush();
    assert!(entry_files(&dir).is_empty());

    let out = execute(
        &mut h.state_mut().state,
        "file.autosave_now",
        json!({"wait": true}),
    )
    .unwrap();
    assert_eq!(out["queued"], true);
    let path = out["path"].as_str().unwrap();
    assert!(Path::new(path).exists(), "wait flushed the write");
    assert_eq!(entry_files(&dir).len(), 1);
}

#[test]
fn the_command_fails_without_an_autosave_directory() {
    let mut e = Editor::new(common::state());
    let err = execute(&mut e.state, "file.autosave_now", json!({})).unwrap_err();
    assert!(err.contains("autosave unavailable"), "{err}");
}

#[test]
fn saving_replacing_and_a_clean_exit_remove_the_autosave() {
    let dir = temp_dir("clean");
    let clock = ManualClock::new(10);
    let mut e = editor(&dir, &clock, settings(60));
    create(&mut e.state, "test.box");
    execute(&mut e.state, "file.autosave_now", json!({"wait": true})).unwrap();
    assert_eq!(entry_files(&dir).len(), 1);

    // explicit save: the untitled autosave goes
    let scene = dir.join("work.forge");
    execute(
        &mut e.state,
        "file.save_as",
        json!({"path": scene.to_string_lossy()}),
    )
    .unwrap();
    e.state.autosave.flush();
    assert!(entry_files(&dir).is_empty());

    // a saved document autosaves under a name from its path, and a later save removes it
    create(&mut e.state, "test.box");
    execute(&mut e.state, "file.autosave_now", json!({"wait": true})).unwrap();
    let keys = entry_files(&dir);
    assert_eq!(keys.len(), 1);
    assert!(keys[0].starts_with("work-"), "{keys:?}");
    assert_eq!(
        list_entries(&dir)[0]
            .meta
            .as_ref()
            .unwrap()
            .original
            .as_deref(),
        Some(scene.to_str().unwrap())
    );
    execute(&mut e.state, "file.save", json!({})).unwrap();
    e.state.autosave.flush();
    assert!(entry_files(&dir).is_empty());

    // file.new discards the autosave of the document it replaces
    create(&mut e.state, "test.box");
    execute(&mut e.state, "file.autosave_now", json!({"wait": true})).unwrap();
    execute(&mut e.state, "file.new", json!({})).unwrap();
    e.state.autosave.flush();
    assert!(entry_files(&dir).is_empty());

    // clean exit: entries and marker are gone
    create(&mut e.state, "test.box");
    execute(&mut e.state, "file.autosave_now", json!({"wait": true})).unwrap();
    assert!(dir.join("sessions").read_dir().unwrap().next().is_some());
    e.state.autosave.finish();
    assert!(entry_files(&dir).is_empty());
    assert!(dir.join("sessions").read_dir().unwrap().next().is_none());
    // and the next start offers nothing
    let next = editor(&dir, &ManualClock::new(500), settings(60));
    assert_eq!(next.recoverable_count(), 0);
}

#[test]
fn crash_then_recover_gives_the_same_scene_bytes() {
    let dir = temp_dir("crash");
    let clock = ManualClock::new(1_000);
    let mut a = editor(&dir, &clock, settings(60));
    create(&mut a.state, "test.box");
    create(&mut a.state, "test.box");
    execute(&mut a.state, "file.autosave_now", json!({"wait": true})).unwrap();
    let bytes = scene_bytes(&a);
    // a crash: nothing is cleaned up
    a.state.autosave.simulate_crash();
    drop(a);
    assert_eq!(crashed_sessions(&dir).len(), 1);

    // a session that is still running is never offered
    let other_clock = ManualClock::new(2_000);
    let mut running = editor(&dir, &other_clock, settings(60));
    assert_eq!(running.recoverable_count(), 1, "sees the crashed one only");
    create(&mut running.state, "test.box");
    execute(
        &mut running.state,
        "file.autosave_now",
        json!({"wait": true}),
    )
    .unwrap();

    let mut b = editor(&dir, &ManualClock::new(3_000), settings(60));
    assert_eq!(
        b.recoverable_count(),
        1,
        "the running session's entry is not offered"
    );
    let mut h = Harness::builder()
        .with_size([1280.0, 800.0])
        .build_ui_state(|ui, e: &mut Editor| e.show(ui, None), {
            b.state.status.clear();
            b
        });
    h.step();
    h.get_by_label("Recover unsaved work");
    h.get_by_label("Recover").click();
    h.run();
    let b = h.state_mut();
    assert_eq!(b.recoverable_count(), 0);
    assert!(b.is_dirty(), "recovered documents are unsaved");
    assert!(b.file_path().is_none());
    assert_eq!(scene_bytes(b), bytes);
    b.state.autosave.flush();
    // the crashed entry was replaced by this session's own autosave of the same scene
    let entries = list_entries(&dir);
    let mine: Vec<_> = entries
        .iter()
        .filter(|e| e.meta.as_ref().unwrap().session == b.state.autosave.session())
        .collect();
    assert_eq!(mine.len(), 1);
    assert_eq!(std::fs::read(&mine[0].data).unwrap(), bytes);
    assert_eq!(entries.len(), 2, "plus the running session's entry");
    assert!(crashed_sessions(&dir).is_empty(), "stale marker cleared");
    drop(running);
}

#[test]
fn recover_keeps_the_original_path_and_discard_removes_files() {
    let dir = temp_dir("path");
    let clock = ManualClock::new(100);
    let scene = dir.join("my scene.forge");
    let mut a = editor(&dir, &clock, settings(60));
    create(&mut a.state, "test.box");
    execute(
        &mut a.state,
        "file.save_as",
        json!({"path": scene.to_string_lossy()}),
    )
    .unwrap();
    create(&mut a.state, "test.box");
    execute(&mut a.state, "file.autosave_now", json!({"wait": true})).unwrap();
    let bytes = scene_bytes(&a);
    let saved_on_disk = std::fs::read(&scene).unwrap();
    a.state.autosave.simulate_crash();
    drop(a);

    let mut b = editor(&dir, &ManualClock::new(200), settings(60));
    assert_eq!(b.recoverable_count(), 1);
    assert!(b.recover_autosave(0));
    assert_eq!(b.file_path(), Some(scene.as_path()));
    assert_eq!(b.window_title(), "my scene.forge* - Forge3D");
    assert_eq!(scene_bytes(&b), bytes);
    assert_eq!(
        std::fs::read(&scene).unwrap(),
        saved_on_disk,
        "the file itself is untouched"
    );
    // saving writes the original path
    assert!(b.save());
    assert!(!b.is_dirty());
    assert_ne!(std::fs::read(&scene).unwrap(), saved_on_disk);
    b.state.autosave.flush();
    assert!(entry_files(&dir).is_empty());
    b.state.autosave.finish();

    // a crash again, then Discard all
    let mut c = editor(&dir, &ManualClock::new(300), settings(60));
    create(&mut c.state, "test.box");
    execute(&mut c.state, "file.autosave_now", json!({"wait": true})).unwrap();
    let key = c.state.autosave.key_for(None);
    c.state.autosave.simulate_crash();
    drop(c);
    let mut d = editor(&dir, &ManualClock::new(400), settings(60));
    assert_eq!(d.recoverable_count(), 1);
    assert!(data_path(&dir, &key).exists());
    d.discard_autosave(0);
    assert_eq!(d.recoverable_count(), 0);
    assert!(entry_files(&dir).is_empty());
    assert!(crashed_sessions(&dir).is_empty());
}

#[test]
fn a_damaged_autosave_reports_and_stays_listed() {
    let dir = temp_dir("damaged");
    let mut a = editor(&dir, &ManualClock::new(100), settings(60));
    create(&mut a.state, "test.box");
    execute(&mut a.state, "file.autosave_now", json!({"wait": true})).unwrap();
    let data = list_entries(&dir)[0].data.clone();
    a.state.autosave.simulate_crash();
    drop(a);
    std::fs::write(&data, b"garbage").unwrap();
    let mut b = editor(&dir, &ManualClock::new(200), settings(60));
    assert!(!b.recover_autosave(0));
    assert!(
        b.state.status.starts_with("Recover failed"),
        "{}",
        b.state.status
    );
    assert_eq!(b.recoverable_count(), 1);
}

#[test]
fn live_sessions_are_not_crashes() {
    let dir = temp_dir("live");
    let _live = SessionMarker::acquire(&dir, "someone-else").unwrap();
    let e = editor(&dir, &ManualClock::new(1), settings(60));
    assert_eq!(e.recoverable_count(), 0);
    assert!(crashed_sessions(&dir).is_empty());
}
