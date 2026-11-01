//! Key map (docs/specs/UI.md §14; brief 152): chord text, defaults, overrides, conflicts
//! (pure) and the editor driven with egui_kittest (shortcuts run commands; rebinding).

mod common;

use std::path::PathBuf;

use egui::{Key, Modifiers};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use forge_command::{CommandContext, schema};
use forge_ui::app::Editor;
use forge_ui::commands::execute;
use forge_ui::keymap::{Binding, Chord, KeyMap, KeyMapState, Scope, read_settings};
use forge_ui::keymap_editor::rows;
use serde_json::{Value, json};

/// The same bindings, whatever the order.
fn same(a: &KeyMap, b: &KeyMap) -> bool {
    let sorted = |m: &KeyMap| {
        let mut v: Vec<String> = m.bindings().iter().map(|b| format!("{b:?}")).collect();
        v.sort();
        v
    };
    sorted(a) == sorted(b)
}

fn chord(text: &str) -> Chord {
    Chord::parse(text).unwrap_or_else(|e| panic!("{text}: {e}"))
}

fn bind(text: &str, command: &str) -> Binding {
    Binding::new(chord(text), command, json!({}))
}

// ---- chord text ---------------------------------------------------------------------

#[test]
fn chord_text_round_trips_for_every_key_and_modifier_combination() {
    for key in Key::ALL {
        for bits in 0..8u8 {
            let c = Chord {
                command: bits & 1 != 0,
                alt: bits & 2 != 0,
                shift: bits & 4 != 0,
                key: *key,
            };
            let text = c.to_string();
            assert_eq!(Chord::parse(&text), Ok(c), "{text}");
        }
    }
}

#[test]
fn chord_text_is_canonical_and_accepts_aliases() {
    assert_eq!(chord("Mod+Shift+Z").to_string(), "Mod+Shift+Z");
    for alias in [
        "ctrl+shift+z",
        "Cmd+Shift+Z",
        "Command+SHIFT+z",
        "shift+mod+Z",
    ] {
        assert_eq!(chord(alias).to_string(), "Mod+Shift+Z", "{alias}");
    }
    assert_eq!(chord("alt+ctrl+shift+F2").to_string(), "Mod+Alt+Shift+F2");
    assert_eq!(chord("Comma").key, Key::Comma);
    assert_eq!(chord(" Mod + A ").to_string(), "Mod+A");
}

#[test]
fn bad_chord_text_is_an_error() {
    for bad in [
        "",
        "Mod+",
        "+A",
        "Mod+Mod+A",
        "Hyper+A",
        "Mod+Shift",
        "Mod+Nope",
        "A+B",
    ] {
        assert!(Chord::parse(bad).is_err(), "'{bad}' parsed");
    }
}

#[test]
fn matching_is_exact_on_modifiers() {
    let c = chord("Mod+A");
    assert!(c.matches(&Modifiers::COMMAND, Key::A));
    assert!(!c.matches(&(Modifiers::COMMAND | Modifiers::SHIFT), Key::A));
    assert!(!c.matches(&Modifiers::NONE, Key::A));
    assert!(!c.matches(&Modifiers::COMMAND, Key::B));
    assert!(chord("Shift+K").matches(&Modifiers::SHIFT, Key::K));
    assert_eq!(
        chord("Mod+Shift+Z").modifiers(),
        Modifiers::COMMAND | Modifiers::SHIFT
    );
    assert_eq!(Chord::from_press(&Modifiers::COMMAND, Key::A), c);
}

// ---- defaults -----------------------------------------------------------------------

#[test]
fn default_map_loads_has_no_conflicts_and_every_command_exists() {
    let map = KeyMap::defaults();
    assert!(!map.bindings().is_empty());
    assert!(map.conflicts().is_empty(), "{:?}", map.conflicts());
    let registry = common::state().app.commands.clone();
    assert!(map.unknown_commands(&registry).is_empty());
    for b in map.bindings() {
        let h = registry
            .get(&b.command)
            .unwrap_or_else(|| panic!("{} is not a command", b.command));
        schema::validate(&h.args_schema(), &b.args)
            .unwrap_or_else(|e| panic!("{} {}: {e}", b.command, b.args));
    }
}

#[test]
fn default_bindings_table() {
    let got: Vec<(String, String, String)> = KeyMap::defaults()
        .bindings()
        .iter()
        .map(|b| (b.chord.to_string(), b.command.clone(), b.args.to_string()))
        .collect();
    let want = [
        ("Mod+O", "file.open_dialog", "{}"),
        ("Mod+S", "file.save_dialog", "{}"),
        ("Mod+Shift+S", "file.save_as_dialog", "{}"),
        ("Mod+Z", "edit.undo", "{}"),
        ("Mod+Shift+Z", "edit.redo", "{}"),
        ("Mod+Y", "edit.redo", "{}"),
        ("Delete", "object.delete", "{}"),
        ("F2", "object.begin_rename", "{}"),
        ("Mod+A", "select.all", "{}"),
        ("Mod+Shift+A", "select.none", "{}"),
        ("Mod+I", "select.invert", "{}"),
        ("Alt+1", "select.mode", r#"{"mode":"model"}"#),
        ("Alt+2", "select.mode", r#"{"mode":"points"}"#),
        ("Alt+3", "select.mode", r#"{"mode":"edges"}"#),
        ("Alt+4", "select.mode", r#"{"mode":"polys"}"#),
        ("Mod+Shift+E", "mesh.make_editable", "{}"),
        ("1", "view.shading", r#"{"mode":"smooth"}"#),
        ("2", "view.shading", r#"{"mode":"flat"}"#),
        ("3", "view.shading", r#"{"mode":"wire"}"#),
        ("4", "view.shading", r#"{"mode":"smooth_wire"}"#),
        ("5", "view.shading", r#"{"mode":"flat_wire"}"#),
        ("6", "view.shading", r#"{"mode":"quick"}"#),
        ("7", "view.shading", r#"{"mode":"constant"}"#),
        ("8", "view.shading", r#"{"mode":"hidden_line"}"#),
        ("9", "view.shading", r#"{"mode":"bounding_box"}"#),
        ("F4", "view.layout", "{}"),
        ("Mod+M", "view.maximize", "{}"),
        ("H", "view.frame_all", "{}"),
        ("S", "view.frame_selected", "{}"),
        ("Mod+1", "view.set_view", r#"{"view":"perspective"}"#),
        ("Mod+2", "view.set_view", r#"{"view":"front"}"#),
        ("Mod+3", "view.set_view", r#"{"view":"right"}"#),
        ("Mod+4", "view.set_view", r#"{"view":"top"}"#),
        ("Mod+5", "view.set_view", r#"{"view":"back"}"#),
        ("Mod+6", "view.set_view", r#"{"view":"left"}"#),
        ("Mod+7", "view.set_view", r#"{"view":"bottom"}"#),
        ("W", "view.gizmo_mode", r#"{"mode":"move"}"#),
        ("E", "view.gizmo_mode", r#"{"mode":"rotate"}"#),
        ("R", "view.gizmo_mode", r#"{"mode":"scale"}"#),
        ("Q", "view.gizmo_space", "{}"),
        ("K", "key.record", "{}"),
        ("Shift+K", "key.autokey", "{}"),
        ("Space", "timeline.toggle_play", "{}"),
        ("Comma", "timeline.step", r#"{"delta":-1}"#),
        ("Period", "timeline.step", r#"{"delta":1}"#),
        ("Shift+Comma", "key.jump", r#"{"forward":false}"#),
        ("Shift+Period", "key.jump", r#"{"forward":true}"#),
    ];
    let want: Vec<(String, String, String)> = want
        .iter()
        .map(|(a, b, c)| (a.to_string(), b.to_string(), c.to_string()))
        .collect();
    assert_eq!(got, want);
    let space = KeyMap::defaults();
    let b = space.lookup(&Modifiers::NONE, Key::Space, true).unwrap();
    assert_eq!(b.scope, Scope::Viewport);
    assert!(space.lookup(&Modifiers::NONE, Key::Space, false).is_none());
}

#[test]
fn key_map_file_errors() {
    let ok = |t: &str| KeyMap::from_toml(t);
    assert!(ok("version = 2").is_err());
    assert!(ok("version = 1\n[[bind]]\nchord = \"Nope\"\ncommand = \"a.b\"").is_err());
    assert!(ok("version = 1\n[[bind]]\nchord = \"A\"\ncommand = \"a.b\"\nscope = \"x\"").is_err());
    assert!(ok("version = 1\n[[bind]]\nchord = \"A\"\ncommand = \"a.b\"\nargs = 3").is_err());
    assert!(ok("version = 1\n[[bind]]\nchord = \"A\"").is_err());
    let m =
        ok("version = 1\n[[bind]]\nchord = \"A\"\ncommand = \"a.b\"\nargs = { x = 1 }").unwrap();
    assert_eq!(m.bindings()[0].args, json!({"x": 1}));
}

// ---- conflicts ----------------------------------------------------------------------

#[test]
fn conflicts_need_the_same_chord_and_overlapping_scopes() {
    let mut viewport = bind("Space", "timeline.toggle_play");
    viewport.scope = Scope::Viewport;
    let map = KeyMap::new(vec![
        bind("Mod+A", "select.all"),
        bind("Mod+A", "select.none"),
        bind("Mod+B", "select.all"),
        viewport.clone(),
        bind("Space", "timeline.play"),
    ]);
    let c = map.conflicts();
    assert_eq!(c.len(), 2);
    assert_eq!(c[0].chord, chord("Mod+A"));
    assert_eq!(c[0].bindings, vec![0, 1]);
    assert_eq!(c[1].chord, chord("Space"));
    assert_eq!(c[1].bindings, vec![3, 4]);

    let mut other_viewport = bind("Space", "timeline.stop");
    other_viewport.scope = Scope::Viewport;
    assert_eq!(
        KeyMap::new(vec![viewport.clone(), other_viewport])
            .conflicts()
            .len(),
        1
    );
    assert!(Scope::Viewport.overlaps(Scope::Global));
    assert!(
        !KeyMap::new(vec![bind("Mod+A", "a.b"), bind("Mod+Shift+A", "a.c")])
            .conflicts()
            .iter()
            .any(|_| true)
    );
}

#[test]
fn conflicts_with_and_bind_replace() {
    let mut map = KeyMap::defaults();
    let taken = map.conflicts_with(chord("Mod+Z"), Scope::Global);
    assert_eq!(taken.len(), 1);
    assert_eq!(taken[0].command, "edit.undo");
    let removed = map.bind(bind("Mod+Z", "select.all"));
    assert_eq!(removed.len(), 1);
    assert_eq!(removed[0].command, "edit.undo");
    assert!(map.conflicts().is_empty());
    assert!(map.chord_of("edit.undo", &json!({})).is_none());
    assert_eq!(map.chord_of("select.all", &json!({})), Some(chord("Mod+A")));
    assert_eq!(
        map.lookup(&Modifiers::COMMAND, Key::Z, false)
            .unwrap()
            .command,
        "select.all"
    );
}

// ---- overrides ----------------------------------------------------------------------

#[test]
fn overrides_replace_unbind_and_add() {
    let mut map = KeyMap::defaults();
    let warnings = map.apply_overrides(&json!({
        "Mod+Z": null,
        "ctrl+u": {"command": "edit.undo"},
        "Mod+Shift+Z": {"command": "select.all", "args": {}, "scope": "viewport"},
        "F9": {"command": "view.shading", "args": {"mode": "wire"}},
    }));
    assert!(warnings.is_empty(), "{warnings:?}");
    assert!(map.lookup(&Modifiers::COMMAND, Key::Z, true).is_none());
    assert_eq!(
        map.lookup(&Modifiers::COMMAND, Key::U, false)
            .unwrap()
            .command,
        "edit.undo"
    );
    // replaced: Mod+Shift+Z is now a viewport-scope select.all
    assert!(
        map.lookup(&(Modifiers::COMMAND | Modifiers::SHIFT), Key::Z, false)
            .is_none()
    );
    assert_eq!(
        map.lookup(&(Modifiers::COMMAND | Modifiers::SHIFT), Key::Z, true)
            .unwrap()
            .command,
        "select.all"
    );
    assert_eq!(
        map.lookup(&Modifiers::NONE, Key::F9, false).unwrap().args,
        json!({"mode": "wire"})
    );
    // untouched defaults stay
    assert_eq!(
        map.lookup(&Modifiers::NONE, Key::K, false).unwrap().command,
        "key.record"
    );
    // Mod+Y (redo) was not overridden
    assert_eq!(
        map.lookup(&Modifiers::COMMAND, Key::Y, false)
            .unwrap()
            .command,
        "edit.redo"
    );
}

#[test]
fn bad_override_entries_are_skipped_with_a_warning() {
    let mut map = KeyMap::defaults();
    let before = map.clone();
    let warnings = map.apply_overrides(&json!({
        "Hyper+Q": {"command": "edit.undo"},
        "F1": {"args": {}},
        "F3": {"command": "edit.undo", "args": 5},
        "F4": {"command": "edit.undo", "scope": "nowhere"},
        "F5": 7,
    }));
    assert_eq!(warnings.len(), 5, "{warnings:?}");
    assert_eq!(map, before);
    assert_eq!(map.apply_overrides(&json!(3)).len(), 1);
    assert!(map.apply_overrides(&Value::Null).is_empty());
}

#[test]
fn overrides_are_the_minimal_inverse_of_the_merge() {
    let defaults = KeyMap::defaults();
    assert_eq!(defaults.overrides(&defaults), json!({}));
    let mut map = defaults.clone();
    map.bind(bind("Mod+U", "edit.undo")); // Mod+Z stays bound? no: edit.undo moves below
    let removed_default = map
        .bindings()
        .iter()
        .position(|b| b.chord == chord("Mod+Z"))
        .unwrap();
    map.remove(removed_default);
    map.bind(bind("Mod+Y", "select.all")); // replaces redo's second chord
    let o = map.overrides(&defaults);
    let o = o.as_object().unwrap();
    assert_eq!(o.len(), 3, "{o:?}");
    assert_eq!(o["Mod+Z"], Value::Null);
    assert_eq!(o["Mod+U"]["command"], "edit.undo");
    assert_eq!(o["Mod+Y"]["command"], "select.all");
    // applying them to the defaults gives back the same bindings (order aside)
    let mut again = defaults.clone();
    assert!(again.apply_overrides(&Value::Object(o.clone())).is_empty());
    let sort = |m: &KeyMap| {
        let mut v: Vec<String> = m
            .bindings()
            .iter()
            .map(|b| format!("{} {}", b.chord, b.command))
            .collect();
        v.sort();
        v
    };
    assert_eq!(sort(&again), sort(&map));
}

fn temp_file(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("forge-keymap-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir.join("settings.json")
}

#[test]
fn settings_file_keeps_other_keys_and_drops_keymap_when_default() {
    let path = temp_file("settings");
    let mut keys = KeyMapState::default();
    // no file, nothing to store: nothing is created
    keys.use_settings_file(Some(path.clone()));
    keys.save().unwrap();
    assert!(!path.exists());

    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(
        &path,
        r#"{"rpc": {"port": 9}, "keymap": {"Mod+Z": null, "Mod+Q": {"command": "edit.undo"}}}"#,
    )
    .unwrap();
    keys.use_settings_file(Some(path.clone()));
    assert!(keys.warnings.is_empty());
    assert!(
        keys.map
            .lookup(&Modifiers::COMMAND, Key::Z, false)
            .is_none()
    );
    assert_eq!(
        keys.map
            .lookup(&Modifiers::COMMAND, Key::Q, false)
            .unwrap()
            .command,
        "edit.undo"
    );

    keys.map.bind(bind("F6", "select.all"));
    keys.save().unwrap();
    let file = read_settings(&path);
    assert_eq!(file["rpc"]["port"], 9);
    assert_eq!(file["keymap"]["F6"]["command"], "select.all");

    keys.reset_all();
    keys.save().unwrap();
    let file = read_settings(&path);
    assert_eq!(file["rpc"]["port"], 9);
    assert!(file.get("keymap").is_none());

    // a broken file means defaults, not a crash
    std::fs::write(&path, "not json").unwrap();
    keys.use_settings_file(Some(path.clone()));
    assert_eq!(keys.map, KeyMap::defaults());
    std::fs::remove_dir_all(path.parent().unwrap()).ok();
}

#[test]
fn reset_binding_restores_a_moved_default() {
    let mut keys = KeyMapState::default();
    keys.map.bind(bind("Mod+Y", "select.all")); // takes redo's second chord
    keys.map.bind(bind("F7", "edit.undo"));
    keys.reset_binding("edit.undo", &json!({}));
    keys.reset_binding("edit.redo", &json!({}));
    assert_eq!(keys.map.conflicts().len(), 0);
    assert_eq!(
        keys.map.chord_of("edit.undo", &json!({})),
        Some(chord("Mod+Z"))
    );
    assert_eq!(keys.map.for_command("edit.redo").count(), 2);
    assert!(keys.map.for_command("edit.undo").count() == 1);
}

// ---- editor rows --------------------------------------------------------------------

#[test]
fn editor_rows_list_commands_search_and_flag_changes() {
    let st = common::state();
    let registry = st.app.commands.clone();
    let mut keys = KeyMapState::default();
    let shown = |c: Chord| c.to_string();
    let all = rows(&keys, &registry, "", &shown);
    assert!(all.len() >= registry.len(), "a row per command at least");
    assert!(all.iter().all(|r| r.known && !r.changed));
    // search by id, label, chord and args
    let undo = rows(&keys, &registry, "  EDIT.undo ", &shown);
    assert_eq!(undo.len(), 1);
    assert_eq!(undo[0].chord, Some(chord("Mod+Z")));
    assert_eq!(rows(&keys, &registry, "mod+shift+z", &shown).len(), 1);
    assert_eq!(rows(&keys, &registry, "mode=wire", &shown).len(), 1);
    assert!(rows(&keys, &registry, "no such thing", &shown).is_empty());
    // a cleared default shows as an unbound, changed row
    keys.map.remove(
        keys.map
            .bindings()
            .iter()
            .position(|b| b.command == "edit.undo")
            .unwrap(),
    );
    let undo = rows(&keys, &registry, "edit.undo", &shown);
    assert_eq!(
        (undo.len(), undo[0].chord, undo[0].changed),
        (1, None, true)
    );
    // a binding to a command the registry lacks is listed as unknown
    keys.map.bind(bind("F8", "plugin.missing"));
    let missing = rows(&keys, &registry, "plugin.missing", &shown);
    assert_eq!(missing.len(), 1);
    assert!(!missing[0].known && missing[0].changed);
    assert_eq!(
        keys.map.unknown_commands(&registry),
        vec!["plugin.missing".to_string()]
    );
}

// ---- kittest: shortcuts run commands ------------------------------------------------

fn harness(editor: Editor) -> Harness<'static, Editor> {
    let mut h = Harness::builder()
        .with_size([1280.0, 800.0])
        .build_ui_state(|ui, e: &mut Editor| e.show(ui, None), editor);
    h.run();
    h
}

fn press(h: &mut Harness<'_, Editor>, mods: Modifiers, key: Key) {
    h.key_press_modifiers(mods, key);
    h.run();
}

fn labels(h: &Harness<'_, Editor>) -> Vec<String> {
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
fn default_shortcuts_run_their_commands_and_undo_undoes() {
    let mut e = Editor::new(common::state());
    let a = common::create(&mut e.state, "forge.null");
    let b = common::create(&mut e.state, "forge.null");
    e.state.selection.click(a);
    let mut h = harness(e);
    let steps = labels(&h).len();

    // Delete runs object.delete (one undo step); Ctrl+Z brings it back
    press(&mut h, Modifiers::NONE, Key::Delete);
    assert!(!h.state().state.doc.scene().contains(a));
    assert_eq!(labels(&h).len(), steps + 1);
    press(&mut h, Modifiers::COMMAND, Key::Z);
    assert!(h.state().state.doc.scene().contains(a));
    assert_eq!(labels(&h).len(), steps);
    // Ctrl+Shift+Z and Ctrl+Y redo
    press(&mut h, Modifiers::COMMAND | Modifiers::SHIFT, Key::Z);
    assert!(!h.state().state.doc.scene().contains(a));
    press(&mut h, Modifiers::COMMAND, Key::Z);
    press(&mut h, Modifiers::COMMAND, Key::Y);
    assert!(!h.state().state.doc.scene().contains(a));
    press(&mut h, Modifiers::COMMAND, Key::Z);

    // select all / none / invert
    press(&mut h, Modifiers::COMMAND, Key::A);
    assert_eq!(h.state().state.selection.nodes.len(), 2);
    press(&mut h, Modifiers::COMMAND | Modifiers::SHIFT, Key::A);
    assert!(h.state().state.selection.nodes.is_empty());
    h.state_mut().state.selection.click(b);
    press(&mut h, Modifiers::COMMAND, Key::I);
    assert!(h.state().state.selection.nodes.contains(&a));
    assert!(!h.state().state.selection.nodes.contains(&b));

    // view and gizmo state
    press(&mut h, Modifiers::NONE, Key::Num3);
    assert_eq!(
        h.state().state.shading,
        forge_viewport::render::Shading::Wire
    );
    press(&mut h, Modifiers::NONE, Key::Num1);
    assert_eq!(
        h.state().state.shading,
        forge_viewport::render::Shading::Smooth
    );
    press(&mut h, Modifiers::NONE, Key::E);
    assert_eq!(
        h.state().state.gizmo_mode,
        forge_viewport::gizmo::GizmoMode::Rotate
    );
    press(&mut h, Modifiers::NONE, Key::R);
    assert_eq!(
        h.state().state.gizmo_mode,
        forge_viewport::gizmo::GizmoMode::Scale
    );
    press(&mut h, Modifiers::NONE, Key::W);
    assert_eq!(
        h.state().state.gizmo_mode,
        forge_viewport::gizmo::GizmoMode::Translate
    );
    assert_eq!(
        h.state().state.gizmo_space,
        forge_viewport::gizmo::Space::Object
    );
    press(&mut h, Modifiers::NONE, Key::Q);
    assert_eq!(
        h.state().state.gizmo_space,
        forge_viewport::gizmo::Space::World
    );
    press(&mut h, Modifiers::NONE, Key::Q);
    assert_eq!(
        h.state().state.gizmo_space,
        forge_viewport::gizmo::Space::Object
    );

    // edit mode
    press(&mut h, Modifiers::ALT, Key::Num2);
    assert_eq!(h.state().state.edit_mode(), forge_command::ModeArg::Points);
    press(&mut h, Modifiers::ALT, Key::Num1);
    assert_eq!(h.state().state.edit_mode(), forge_command::ModeArg::Model);

    // F2 starts the rename of the active object
    h.state_mut().state.selection.click(a);
    press(&mut h, Modifiers::NONE, Key::F2);
    assert_eq!(h.state().objects().renaming.as_ref().map(|r| r.0), Some(a));
}

#[test]
fn a_shortcut_that_fails_reports_in_the_status_bar() {
    let mut e = Editor::new(common::state());
    e.state.selection.clear();
    let mut h = harness(e);
    press(&mut h, Modifiers::NONE, Key::K); // key.record with nothing selected
    assert!(!h.state().state.status.is_empty());
}

#[test]
fn unbound_chords_do_nothing() {
    let mut e = Editor::new(common::state());
    let a = common::create(&mut e.state, "forge.null");
    e.state.selection.click(a);
    let mut h = harness(e);
    press(&mut h, Modifiers::COMMAND | Modifiers::ALT, Key::Z);
    assert!(h.state().state.doc.scene().contains(a));
    assert_eq!(h.state().state.doc.scene().len(), 1);
}

#[test]
fn nothing_runs_while_a_text_field_has_the_keyboard() {
    let mut e = Editor::new(common::state());
    let a = common::create(&mut e.state, "forge.null");
    e.state.selection.click(a);
    let mut h = harness(e);
    open_editor_window(&mut h, "");
    // focus the search field of the key map window, then press Delete
    h.get_by_role_and_label(egui::accesskit::Role::TextInput, "Search")
        .click();
    h.run();
    press(&mut h, Modifiers::NONE, Key::Delete);
    assert!(
        h.state().state.doc.scene().contains(a),
        "Delete went to the text field"
    );
    assert_eq!(h.state().state.doc.scene().len(), 1);
}

// ---- kittest: the editor window -----------------------------------------------------

fn editor_with_file(name: &str) -> (Editor, PathBuf) {
    let path = temp_file(name);
    let mut e = Editor::new(common::state());
    e.state.keys.use_settings_file(Some(path.clone()));
    (e, path)
}

fn open_editor_window(h: &mut Harness<'_, Editor>, filter: &str) {
    h.get_by_label("Window").click();
    h.run();
    h.get_by_label_contains("Key map").click();
    h.run();
    h.state_mut().keymap_editor_mut().filter = filter.to_string();
    h.run();
}

#[test]
fn rebinding_in_the_editor_takes_effect_and_is_stored() {
    let (mut e, path) = editor_with_file("rebind");
    let a = common::create(&mut e.state, "forge.null");
    e.state.selection.click(a);
    let mut h = harness(e);
    open_editor_window(&mut h, "edit.undo");
    assert!(h.state().keymap_editor().open);
    // delete something so there is something to undo
    execute(&mut h.state_mut().state, "object.delete", json!({})).unwrap();
    h.run();
    assert!(!h.state().state.doc.scene().contains(a));

    // rebind Undo to Ctrl+U
    h.get_by_label("Ctrl+Z").click();
    h.run();
    assert!(h.state().keymap_editor().is_capturing());
    h.get_by_label("Press keys\u{2026}");
    press(&mut h, Modifiers::COMMAND, Key::U);
    assert!(!h.state().keymap_editor().is_capturing());
    h.get_by_label("Ctrl+U");
    let keys = &h.state().state.keys;
    assert_eq!(
        keys.map.chord_of("edit.undo", &json!({})),
        Some(chord("Mod+U"))
    );
    assert!(keys.map.conflicts().is_empty());

    // the old chord is dead, the new one undoes
    h.state_mut().keymap_editor_mut().filter = String::new();
    h.run();
    press(&mut h, Modifiers::COMMAND, Key::Z);
    assert!(
        !h.state().state.doc.scene().contains(a),
        "Ctrl+Z is unbound now"
    );
    press(&mut h, Modifiers::COMMAND, Key::U);
    assert!(h.state().state.doc.scene().contains(a), "Ctrl+U undoes");

    // stored in the settings file
    let file = read_settings(&path);
    assert_eq!(file["keymap"]["Mod+U"]["command"], "edit.undo");
    assert_eq!(file["keymap"]["Mod+Z"], Value::Null);
    std::fs::remove_dir_all(path.parent().unwrap()).ok();
}

#[test]
fn a_taken_chord_asks_first_and_escape_cancels_capture() {
    let (e, path) = editor_with_file("conflict");
    let mut h = harness(e);
    open_editor_window(&mut h, "edit.redo");
    // redo has two rows (Ctrl+Shift+Z and Ctrl+Y); rebind the Ctrl+Y one to Ctrl+Z (undo's)
    h.get_by_label("Ctrl+Y").click();
    h.run();
    press(&mut h, Modifiers::COMMAND, Key::Z);
    assert!(h.state().keymap_editor().has_pending());
    h.get_by_label_contains("is bound to edit.undo");
    // nothing changed yet
    assert_eq!(h.state().state.keys.map, KeyMap::defaults());
    h.get_by_label("Cancel").click();
    h.run();
    assert!(!h.state().keymap_editor().has_pending());
    assert_eq!(h.state().state.keys.map, KeyMap::defaults());

    // again, then Replace: undo loses Ctrl+Z, no conflict remains
    h.get_by_label("Ctrl+Y").click();
    h.run();
    press(&mut h, Modifiers::COMMAND, Key::Z);
    h.get_by_label("Replace").click();
    h.run();
    let keys = &h.state().state.keys;
    assert!(keys.map.conflicts().is_empty());
    assert_eq!(keys.map.chord_of("edit.undo", &json!({})), None);
    assert_eq!(keys.map.for_command("edit.redo").count(), 2);
    assert_eq!(
        keys.map
            .lookup(&Modifiers::COMMAND, Key::Z, false)
            .unwrap()
            .command,
        "edit.redo"
    );

    // Escape while capturing changes nothing
    let before = h.state().state.keys.map.clone();
    h.get_by_label("Ctrl+Shift+Z").click();
    h.run();
    press(&mut h, Modifiers::NONE, Key::Escape);
    assert!(!h.state().keymap_editor().is_capturing());
    assert_eq!(h.state().state.keys.map, before);
    std::fs::remove_dir_all(path.parent().unwrap()).ok();
}

#[test]
fn clear_reset_and_reset_all() {
    let (e, path) = editor_with_file("reset");
    let mut h = harness(e);
    open_editor_window(&mut h, "select.invert");
    // Clear removes the chord, Reset brings the default back
    h.get_by_label("Clear").click();
    h.run();
    assert!(
        h.state()
            .state
            .keys
            .map
            .chord_of("select.invert", &json!({}))
            .is_none()
    );
    assert_eq!(read_settings(&path)["keymap"]["Mod+I"], Value::Null);
    h.get_by_label("(none)");
    h.get_by_label("Reset").click();
    h.run();
    assert!(same(&h.state().state.keys.map, &KeyMap::defaults()));
    assert!(read_settings(&path).get("keymap").is_none());

    // Reset all after two changes
    h.state_mut().state.keys.map.bind(bind("F8", "select.all"));
    h.state_mut().state.keys.map.bind(bind("F9", "edit.undo"));
    h.get_by_label("Reset all").click();
    h.run();
    assert!(same(&h.state().state.keys.map, &KeyMap::defaults()));
    std::fs::remove_dir_all(path.parent().unwrap()).ok();
}
