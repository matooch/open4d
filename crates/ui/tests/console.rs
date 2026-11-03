//! docs/specs/UI.md §19: the Console panel (log and Rhai REPL).

mod common;

use egui::accesskit::Role;
use egui::{Key, Modifiers};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use forge_ui::app::Editor;
use forge_ui::commands::execute;
use forge_ui::console::ReplKind;
use forge_ui::dock::{Area, Dock, Panel};
use serde_json::json;

const LOG: [&str; 5] = [
    "2026-11-03T10:00:00Z  INFO forge_ui: window opened",
    "2026-11-03T10:00:01Z DEBUG forge_command::registry: run command=object.create",
    "2026-11-03T10:00:02Z  WARN forge_command::registry: command failed error=boom",
    "2026-11-03T10:00:03Z ERROR forge_rpc::server: handler panic",
    "  while serving a request",
];

fn harness() -> Harness<'static, Editor> {
    let mut editor = Editor::new(common::state());
    editor.state.dock.open(Panel::Console);
    editor.state.console.log_source = Box::new(|| LOG.iter().map(|s| s.to_string()).collect());
    Harness::builder()
        .with_size([1280.0, 800.0])
        .build_ui_state(|ui, editor: &mut Editor| editor.show(ui, None), editor)
}

fn click(h: &mut Harness<'_, Editor>, label: &str) {
    h.get_by_label(label).click();
    h.run();
}

fn input(h: &Harness<'_, Editor>) -> String {
    h.state().state.console.input.clone()
}

fn role_of(field: &str) -> Role {
    if field == "REPL input" {
        Role::MultilineTextInput
    } else {
        Role::TextInput
    }
}

fn type_in(h: &mut Harness<'_, Editor>, field: &str, text: &str) {
    h.get_by_role_and_label(role_of(field), field).focus();
    h.run();
    h.get_by_role_and_label(role_of(field), field)
        .type_text(text);
    h.run();
}

fn press(h: &mut Harness<'_, Editor>, mods: Modifiers, key: Key) {
    h.key_press_modifiers(mods, key);
    h.run();
}

fn transcript(h: &Harness<'_, Editor>) -> Vec<(String, ReplKind)> {
    h.state()
        .state
        .console
        .repl
        .iter()
        .map(|l| (l.text.clone(), l.kind))
        .collect()
}

fn steps(h: &Harness<'_, Editor>) -> Vec<String> {
    h.state()
        .state
        .doc
        .history()
        .undo_labels()
        .iter()
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn the_panel_is_closed_by_default_and_opens_from_the_window_menu() {
    assert_eq!(Dock::default().area_of(Panel::Console), None);
    assert_eq!(Panel::Console.title(), "Console");
    let editor = Editor::new(common::state());
    let mut h = Harness::builder()
        .with_size([1280.0, 800.0])
        .build_ui_state(|ui, editor: &mut Editor| editor.show(ui, None), editor);
    click(&mut h, "Window");
    click(&mut h, "Console panel");
    assert_eq!(
        h.state().state.dock.area_of(Panel::Console),
        Some(Area::Bottom)
    );
    assert_eq!(
        h.state().state.dock.active(Area::Bottom),
        Some(Panel::Console)
    );
    h.get_by_label("Copy");
    click(&mut h, "Window");
    click(&mut h, "Reset layout");
    assert_eq!(h.state().state.dock.area_of(Panel::Console), None);
}

#[test]
fn the_log_shows_the_lines_of_its_source() {
    let mut h = harness();
    h.run();
    for line in LOG {
        h.get_by_label(line);
    }
    h.get_by_label("5 of 5 lines");
}

#[test]
fn the_level_drop_down_hides_lines_below_it() {
    let mut h = harness();
    h.run();
    h.get_by_role_and_label(Role::ComboBox, "Level").click();
    h.run();
    h.get_all_by_label("Warn").last().unwrap().click();
    h.run();
    assert!(h.query_by_label(LOG[0]).is_none());
    assert!(h.query_by_label(LOG[1]).is_none());
    h.get_by_label(LOG[2]);
    h.get_by_label(LOG[3]);
    h.get_by_label(LOG[4]); // continues the error
    h.get_by_label("3 of 5 lines");
    // back to everything
    h.get_by_role_and_label(Role::ComboBox, "Level").click();
    h.run();
    h.get_all_by_label("Trace (all)").last().unwrap().click();
    h.run();
    h.get_by_label(LOG[0]);
}

#[test]
fn the_search_field_filters_and_copy_takes_what_is_shown() {
    let mut h = harness();
    h.run();
    type_in(&mut h, "Search", "REGISTRY");
    assert!(h.query_by_label(LOG[0]).is_none());
    h.get_by_label(LOG[1]);
    h.get_by_label(LOG[2]);
    assert!(h.query_by_label(LOG[3]).is_none());
    h.get_by_label("2 of 5 lines");
    click(&mut h, "Copy");
    assert_eq!(
        h.state().state.console.last_copy,
        format!("{}\n{}", LOG[1], LOG[2])
    );
    // an unmatched search shows nothing and copies nothing
    h.state_mut().state.console.search = "zzz".into();
    h.run();
    h.get_by_label("0 of 5 lines");
    click(&mut h, "Copy");
    assert_eq!(h.state().state.console.last_copy, "");
}

#[test]
fn run_evaluates_a_command_prints_and_is_one_undo_step() {
    let mut h = harness();
    h.run();
    h.state_mut().state.console.input =
        "let a = create(\"test.box\"); create(\"test.box\"); print(`made ${a}`); a".into();
    click(&mut h, "Run");
    assert_eq!(h.state().state.doc.scene().roots().len(), 2);
    assert_eq!(steps(&h), ["Run Script"]);
    assert_eq!(
        transcript(&h),
        [
            (
                "> let a = create(\"test.box\"); create(\"test.box\"); print(`made ${a}`); a"
                    .to_string(),
                ReplKind::Input
            ),
            ("made 1".to_string(), ReplKind::Output),
            ("=> 1".to_string(), ReplKind::Result),
        ]
    );
    assert_eq!(input(&h), "");
    h.get_by_label("made 1");
    h.get_by_label("=> 1");
    // two commands, one undo
    execute(&mut h.state_mut().state, "edit.undo", json!({})).unwrap();
    assert!(h.state().state.doc.scene().is_empty());
}

#[test]
fn enter_runs_the_entry_and_shift_enter_adds_a_line() {
    let mut h = harness();
    h.run();
    type_in(&mut h, "REPL input", "let n = 20;");
    press(&mut h, Modifiers::SHIFT, Key::Enter);
    assert_eq!(input(&h), "let n = 20;\n", "Shift+Enter inserted a newline");
    assert!(transcript(&h).is_empty());
    h.get_by_role_and_label(Role::MultilineTextInput, "REPL input")
        .type_text("n + 1");
    h.run();
    assert_eq!(input(&h), "let n = 20;\nn + 1");
    press(&mut h, Modifiers::NONE, Key::Enter);
    assert_eq!(input(&h), "");
    assert_eq!(
        transcript(&h),
        [
            ("> let n = 20;".to_string(), ReplKind::Input),
            (". n + 1".to_string(), ReplKind::Input),
            ("=> 21".to_string(), ReplKind::Result),
        ]
    );
    // a blank entry does nothing
    press(&mut h, Modifiers::NONE, Key::Enter);
    assert_eq!(transcript(&h).len(), 3);
    assert!(h.state().state.console.history().len() == 1);
}

#[test]
fn enter_runs_a_command_and_keeps_the_field_focused_for_the_next_one() {
    let mut h = harness();
    h.run();
    type_in(&mut h, "REPL input", "create(\"test.box\");");
    press(&mut h, Modifiers::NONE, Key::Enter);
    assert_eq!(h.state().state.doc.scene().roots().len(), 1);
    h.get_by_role_and_label(Role::MultilineTextInput, "REPL input")
        .type_text("create(\"test.box\");");
    h.run();
    press(&mut h, Modifiers::NONE, Key::Enter);
    assert_eq!(h.state().state.doc.scene().roots().len(), 2);
    // one undo step per entry
    assert_eq!(steps(&h), ["Run Script", "Run Script"]);
    execute(&mut h.state_mut().state, "edit.undo", json!({})).unwrap();
    assert_eq!(h.state().state.doc.scene().roots().len(), 1);
}

#[test]
fn a_failing_entry_shows_its_error_and_changes_nothing() {
    let mut h = harness();
    h.run();
    h.state_mut().state.console.input =
        "create(\"test.box\");\nprint(\"hi\");\nrun(\"no.such\");".into();
    click(&mut h, "Run");
    assert!(h.state().state.doc.scene().is_empty());
    assert!(steps(&h).is_empty());
    let t = transcript(&h);
    assert_eq!(
        t[0],
        ("> create(\"test.box\");".to_string(), ReplKind::Input)
    );
    assert_eq!(t[3], ("hi".to_string(), ReplKind::Output));
    assert_eq!(t[4].1, ReplKind::Error);
    assert!(t[4].0.starts_with("line 3: "), "{t:?}");
    // a syntax error too, and the sandbox refuses what scripts may not do
    h.state_mut().state.console.input = "let = ;".into();
    click(&mut h, "Run");
    assert_eq!(transcript(&h).last().unwrap().1, ReplKind::Error);
    h.state_mut().state.console.input = "run(\"edit.undo\")".into();
    click(&mut h, "Run");
    assert_eq!(transcript(&h).last().unwrap().1, ReplKind::Error);
}

#[test]
fn up_and_down_recall_earlier_entries() {
    let mut h = harness();
    h.run();
    for entry in ["1 + 1", "2 + 2", "3 + 3"] {
        h.state_mut().state.console.input = entry.into();
        click(&mut h, "Run");
    }
    type_in(&mut h, "REPL input", "draft");
    press(&mut h, Modifiers::NONE, Key::ArrowUp);
    assert_eq!(input(&h), "3 + 3");
    press(&mut h, Modifiers::NONE, Key::ArrowUp);
    assert_eq!(input(&h), "2 + 2");
    press(&mut h, Modifiers::NONE, Key::ArrowUp);
    assert_eq!(input(&h), "1 + 1");
    press(&mut h, Modifiers::NONE, Key::ArrowDown);
    assert_eq!(input(&h), "2 + 2");
    press(&mut h, Modifiers::NONE, Key::ArrowDown);
    press(&mut h, Modifiers::NONE, Key::ArrowDown);
    assert_eq!(input(&h), "draft");
    // a recalled entry runs again
    press(&mut h, Modifiers::NONE, Key::ArrowUp);
    press(&mut h, Modifiers::NONE, Key::Enter);
    assert_eq!(
        transcript(&h).last().unwrap(),
        &("=> 6".to_string(), ReplKind::Result)
    );
}

#[test]
fn clear_empties_the_transcript() {
    let mut h = harness();
    h.run();
    h.state_mut().state.console.input = "40 + 2".into();
    click(&mut h, "Run");
    h.get_by_label("=> 42");
    click(&mut h, "Clear");
    assert!(transcript(&h).is_empty());
    assert!(h.query_by_label("=> 42").is_none());
}
