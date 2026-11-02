//! docs/specs/UI.md §10: the Script Manager panel.

mod common;

use std::path::PathBuf;

use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use forge_ui::app::{Dialogs, Editor, FileKind, Unsaved};
use forge_ui::commands::execute;
use forge_ui::dock::{Area, Dock, Panel};
use forge_ui::script_panel::{LineKind, MAX_RECENT, ScriptPanelState};
use serde_json::json;

struct Pick {
    open: Option<PathBuf>,
    save: Option<PathBuf>,
}

impl Dialogs for Pick {
    fn pick_open(&mut self, kind: FileKind) -> Option<PathBuf> {
        assert_eq!(kind, FileKind::Script);
        self.open.take()
    }

    fn pick_save(&mut self, kind: FileKind, _default_name: &str) -> Option<PathBuf> {
        assert_eq!(kind, FileKind::Script);
        self.save.take()
    }

    fn confirm_unsaved(&mut self, _name: &str) -> Unsaved {
        Unsaved::Cancel
    }
}

fn temp(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("forge-ui-script-{}-{name}", std::process::id()));
    std::fs::remove_dir_all(&d).ok();
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn harness_with(dialogs: Pick) -> Harness<'static, Editor> {
    let mut editor = Editor::new(common::state());
    editor.set_dialogs(Box::new(dialogs));
    editor.state.dock.open(Panel::Script);
    Harness::builder()
        .with_size([1280.0, 800.0])
        .build_ui_state(|ui, editor: &mut Editor| editor.show(ui, None), editor)
}

fn harness() -> Harness<'static, Editor> {
    harness_with(Pick {
        open: None,
        save: None,
    })
}

fn click(h: &mut Harness<'_, Editor>, label: &str) {
    h.get_by_label(label).click();
    h.run();
}

fn lines(h: &Harness<'_, Editor>) -> Vec<(String, LineKind)> {
    h.state()
        .state
        .script
        .console
        .iter()
        .map(|l| (l.text.clone(), l.kind))
        .collect()
}

#[test]
fn the_panel_is_closed_by_default_and_opens_from_the_window_menu() {
    assert_eq!(Dock::default().area_of(Panel::Script), None);
    assert_eq!(Panel::Script.title(), "Script Manager");
    let editor = Editor::new(common::state());
    let mut h = Harness::builder()
        .with_size([1280.0, 800.0])
        .build_ui_state(|ui, editor: &mut Editor| editor.show(ui, None), editor);
    click(&mut h, "Window");
    click(&mut h, "Script Manager panel");
    assert_eq!(
        h.state().state.dock.area_of(Panel::Script),
        Some(Area::Bottom)
    );
    assert_eq!(
        h.state().state.dock.active(Area::Bottom),
        Some(Panel::Script)
    );
    h.get_by_label("Run");
    click(&mut h, "Window");
    click(&mut h, "Reset layout");
    assert_eq!(h.state().state.dock.area_of(Panel::Script), None);
}

#[test]
fn run_creates_nodes_prints_and_is_one_undo_step() {
    let mut h = harness();
    h.state_mut().state.script.source =
        "let a = create(\"test.box\"); create(\"test.box\"); print(`made ${a}`);".into();
    click(&mut h, "Run");
    assert_eq!(h.state().state.doc.scene().roots().len(), 2);
    assert_eq!(
        lines(&h),
        [
            ("made 1".to_string(), LineKind::Output),
            ("ran 2 command(s)".to_string(), LineKind::Info)
        ]
    );
    assert_eq!(h.state().state.doc.history().undo_labels(), ["Run Script"]);
    // the console shows the lines
    h.get_by_label("made 1");
    execute(&mut h.state_mut().state, "edit.undo", json!({})).unwrap();
    assert!(h.state().state.doc.scene().is_empty());
}

#[test]
fn a_failing_script_shows_its_error_and_changes_nothing() {
    let mut h = harness();
    h.state_mut().state.script.source =
        "create(\"test.box\");\nprint(\"hi\");\nrun(\"no.such\");".into();
    click(&mut h, "Run");
    assert!(h.state().state.doc.scene().is_empty());
    assert!(h.state().state.doc.history().undo_labels().is_empty());
    let l = lines(&h);
    assert_eq!(l[0], ("hi".to_string(), LineKind::Output));
    assert_eq!(l[1].1, LineKind::Error);
    assert!(l[1].0.starts_with("line 3: "), "{l:?}");
    // a syntax error too
    h.state_mut().state.script.source = "let = ;".into();
    click(&mut h, "Run");
    assert_eq!(lines(&h).last().unwrap().1, LineKind::Error);
    // Clear empties the console
    click(&mut h, "Clear");
    assert!(lines(&h).is_empty());
}

#[test]
fn open_and_save_use_the_dialogs_and_fill_the_recent_list() {
    let d = temp("files");
    let script = d.join("a.rhai");
    std::fs::write(&script, "create(\"test.box\");").unwrap();
    let out = d.join("b.rhai");
    let mut h = harness_with(Pick {
        open: Some(script.clone()),
        save: Some(out.clone()),
    });
    h.state_mut().state.script.settings = Some(d.join("settings.json"));
    click(&mut h, "Open...");
    assert_eq!(h.state().state.script.source, "create(\"test.box\");");
    h.state_mut().state.script.source.push_str(" // edited");
    click(&mut h, "Save...");
    assert_eq!(
        std::fs::read_to_string(&out).unwrap(),
        "create(\"test.box\"); // edited"
    );
    let recent = h.state().state.script.recent.clone();
    assert_eq!(recent, [out.to_string_lossy(), script.to_string_lossy()]);
}

#[test]
fn recent_list_is_stored_in_the_settings_file_next_to_other_keys() {
    let d = temp("recent");
    let settings = d.join("sub/settings.json");
    std::fs::create_dir_all(settings.parent().unwrap()).unwrap();
    std::fs::write(&settings, r#"{"rpc": {"token": "secret"}}"#).unwrap();
    let mut s = ScriptPanelState::default();
    s.use_settings_file(Some(settings.clone()));
    assert!(s.recent.is_empty());
    for i in 0..12 {
        let p = d.join(format!("s{i}.rhai"));
        std::fs::write(&p, "1").unwrap();
        assert!(s.load_file(&p));
    }
    s.load_file(&d.join("s5.rhai")); // moves to the front, no duplicate
    assert_eq!(s.recent.len(), MAX_RECENT);
    assert_eq!(s.recent[0], d.join("s5.rhai").to_string_lossy());
    assert_eq!(
        s.recent.iter().filter(|p| p.ends_with("s5.rhai")).count(),
        1
    );

    let stored: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&settings).unwrap()).unwrap();
    assert_eq!(stored["rpc"]["token"], "secret");
    assert_eq!(
        stored["scripts"]["recent"].as_array().unwrap().len(),
        MAX_RECENT
    );

    // a fresh panel reads it back
    let mut t = ScriptPanelState::default();
    t.use_settings_file(Some(settings));
    assert_eq!(t.recent, s.recent);

    // a missing file reports an error in the console and keeps the source
    t.source = "keep".into();
    assert!(!t.load_file(&d.join("missing.rhai")));
    assert_eq!(t.source, "keep");
    assert_eq!(t.console.last().unwrap().kind, LineKind::Error);
}

#[test]
fn clicking_a_recent_script_loads_it() {
    let d = temp("click");
    let script = d.join("c.rhai");
    std::fs::write(&script, "print(\"from recent\");").unwrap();
    let mut h = harness();
    h.state_mut().state.script.recent = vec![script.to_string_lossy().into_owned()];
    h.run();
    click(&mut h, "Recent scripts");
    click(&mut h, &script.to_string_lossy());
    assert_eq!(h.state().state.script.source, "print(\"from recent\");");
}
