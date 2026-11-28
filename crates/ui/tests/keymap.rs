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
        .map(|b| (b.keys_text(), b.command.clone(), b.args.to_string()))
        .collect();
    let want = [
        ("Mod+O", "file.open_dialog", "{}"),
        ("Mod+S", "file.save_dialog", "{}"),
        ("Mod+Shift+S", "file.save_as_dialog", "{}"),
        // observed, G2 item 1: undo / redo, delete, selection
        ("Mod+Z", "edit.undo", "{}"),
        ("Mod+Y", "edit.redo", "{}"),
        ("Delete", "object.delete", "{}"),
        ("Backspace", "object.delete", "{}"),
        ("Mod+A", "select.all", "{}"),
        ("Mod+Shift+A", "select.none", "{}"),
        ("U I", "select.invert", "{}"),
        ("U Y", "select.grow", "{}"),
        ("U K", "select.shrink", "{}"),
        ("U W", "select.connected", "{}"),
        ("U L", "select.loop", "{}"),
        ("U B", "select.ring", "{}"),
        // ours: the reference has no mode keys
        ("Alt+1", "select.mode", r#"{"mode":"model"}"#),
        ("Alt+2", "select.mode", r#"{"mode":"points"}"#),
        ("Alt+3", "select.mode", r#"{"mode":"edges"}"#),
        ("Alt+4", "select.mode", r#"{"mode":"polys"}"#),
        ("C", "mesh.make_editable", "{}"),
        ("Alt+G", "object.group", "{}"),
        ("Mod+Shift+P", "ui.command_palette", "{}"),
        ("Mod+Shift+1", "view.shading", r#"{"mode":"smooth"}"#),
        ("Mod+Shift+2", "view.shading", r#"{"mode":"flat"}"#),
        ("Mod+Shift+3", "view.shading", r#"{"mode":"wire"}"#),
        ("Mod+Shift+4", "view.shading", r#"{"mode":"smooth_wire"}"#),
        ("Mod+Shift+5", "view.shading", r#"{"mode":"flat_wire"}"#),
        ("Mod+Shift+6", "view.shading", r#"{"mode":"quick"}"#),
        ("Mod+Shift+7", "view.shading", r#"{"mode":"constant"}"#),
        ("Mod+Shift+8", "view.shading", r#"{"mode":"hidden_line"}"#),
        ("Mod+Shift+9", "view.shading", r#"{"mode":"bounding_box"}"#),
        // observed: F1 Perspective, F2 Top, F3 Right, F4 Front, F5 all views
        ("F1", "view.set_view", r#"{"view":"perspective"}"#),
        ("F2", "view.set_view", r#"{"view":"top"}"#),
        ("F3", "view.set_view", r#"{"view":"right"}"#),
        ("F4", "view.set_view", r#"{"view":"front"}"#),
        ("F5", "view.layout", "{}"),
        ("Mod+M", "view.maximize", "{}"),
        // observed: H geometry, S elements, O objects
        ("H", "view.frame_all", "{}"),
        ("S", "view.frame_selected", "{}"),
        ("O", "view.frame_selected", "{}"),
        ("Mod+1", "view.set_view", r#"{"view":"perspective"}"#),
        ("Mod+2", "view.set_view", r#"{"view":"front"}"#),
        ("Mod+3", "view.set_view", r#"{"view":"right"}"#),
        ("Mod+4", "view.set_view", r#"{"view":"top"}"#),
        ("Mod+5", "view.set_view", r#"{"view":"back"}"#),
        ("Mod+6", "view.set_view", r#"{"view":"left"}"#),
        ("Mod+7", "view.set_view", r#"{"view":"bottom"}"#),
        ("E", "view.gizmo_mode", r#"{"mode":"move"}"#),
        ("R", "view.gizmo_mode", r#"{"mode":"rotate"}"#),
        ("T", "view.gizmo_mode", r#"{"mode":"scale"}"#),
        ("Q", "view.gizmo_space", "{}"),
        ("Shift+S", "snap.toggle", "{}"),
        ("L", "edit.axis_mode", "{}"),
        // observed: F8 / F7 play and stop, F9 record, Mod+F9 autokey, G / F frames, Mod+G / Mod+F keys
        ("F8", "timeline.play", "{}"),
        ("F7", "timeline.stop", "{}"),
        ("F9", "key.record", "{}"),
        ("Mod+F9", "key.autokey", "{}"),
        ("G", "timeline.step", r#"{"delta":1}"#),
        ("F", "timeline.step", r#"{"delta":-1}"#),
        ("Mod+G", "key.jump", r#"{"forward":true}"#),
        ("Mod+F", "key.jump", r#"{"forward":false}"#),
        ("Space", "timeline.toggle_play", "{}"),
        ("Shift+M", "marker.add", "{}"),
        ("Alt+Comma", "marker.jump", r#"{"forward":false}"#),
        ("Alt+Period", "marker.jump", r#"{"forward":true}"#),
        // observed two-key sequences
        ("M T", "mesh.extrude", "{}"),
        ("M W", "mesh.extrude_inner", "{}"),
        ("M S", "mesh.bevel", "{}"),
        ("M B", "mesh.bridge", "{}"),
        ("M Q", "mesh.weld", "{}"),
        ("M P", "mesh.stitch", "{}"),
        ("M N", "mesh.dissolve", "{}"),
        ("M H", "mesh.mirror", "{}"),
        ("M M", "mesh.connect_points", "{}"),
        ("M O", "mesh.slide", "{}"),
        ("U S", "mesh.subdivide", "{}"),
        ("U O", "mesh.optimize", "{}"),
        ("U Z", "mesh.melt", "{}"),
        ("U P", "mesh.split", "{}"),
        ("U D", "mesh.disconnect", "{}"),
        ("U U", "mesh.untriangulate", "{}"),
        ("U R", "mesh.reverse_normals", "{}"),
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
        map.lookup(&Modifiers::NONE, Key::F7, false)
            .unwrap()
            .command,
        "timeline.stop"
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
    assert_eq!(keys.map.for_command("edit.redo").count(), 1);
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
    assert_eq!(rows(&keys, &registry, "mod+y", &shown).len(), 1);
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

/// Lets the frame clock run past the sequence time-out (the editor's capture waits that
/// long for a second key, UI.md §14.7).
fn settle(h: &mut Harness<'_, Editor>) {
    for _ in 0..10 {
        h.step();
    }
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
    // Ctrl+Y redoes; Ctrl+Shift+Z is Undo View (observed, G2 item 1), which has no command
    // yet, so it does nothing and in particular does not redo
    press(&mut h, Modifiers::COMMAND | Modifiers::SHIFT, Key::Z);
    assert!(h.state().state.doc.scene().contains(a));
    press(&mut h, Modifiers::COMMAND, Key::Y);
    assert!(!h.state().state.doc.scene().contains(a));
    press(&mut h, Modifiers::COMMAND, Key::Z);
    // Backspace deletes like Delete
    h.state_mut().state.selection.click(a);
    press(&mut h, Modifiers::NONE, Key::Backspace);
    assert!(!h.state().state.doc.scene().contains(a));
    press(&mut h, Modifiers::COMMAND, Key::Z);
    assert!(h.state().state.doc.scene().contains(a));

    // select all / none / invert
    press(&mut h, Modifiers::COMMAND, Key::A);
    assert_eq!(h.state().state.selection.nodes.len(), 2);
    press(&mut h, Modifiers::COMMAND | Modifiers::SHIFT, Key::A);
    assert!(h.state().state.selection.nodes.is_empty());
    h.state_mut().state.selection.click(b);
    // invert is the sequence U, I (observed)
    press(&mut h, Modifiers::NONE, Key::U);
    assert!(
        !h.state().state.selection.nodes.contains(&a),
        "U alone does nothing"
    );
    press(&mut h, Modifiers::NONE, Key::I);
    assert!(h.state().state.selection.nodes.contains(&a));
    assert!(!h.state().state.selection.nodes.contains(&b));

    // view and gizmo state
    // owner note N1: the bare digits do nothing (they are navigation hold keys)
    let shading0 = h.state().state.shading;
    press(&mut h, Modifiers::NONE, Key::Num3);
    assert_eq!(h.state().state.shading, shading0);
    let shift_cmd = Modifiers::COMMAND | Modifiers::SHIFT;
    press(&mut h, shift_cmd, Key::Num3);
    assert_eq!(
        h.state().state.shading,
        forge_viewport::render::Shading::Wire
    );
    press(&mut h, shift_cmd, Key::Num1);
    assert_eq!(
        h.state().state.shading,
        forge_viewport::render::Shading::Smooth
    );
    // owner note N2: E / R / T = move / rotate / scale; W is free
    press(&mut h, Modifiers::NONE, Key::R);
    assert_eq!(
        h.state().state.gizmo_mode,
        forge_viewport::gizmo::GizmoMode::Rotate
    );
    press(&mut h, Modifiers::NONE, Key::T);
    assert_eq!(
        h.state().state.gizmo_mode,
        forge_viewport::gizmo::GizmoMode::Scale
    );
    press(&mut h, Modifiers::NONE, Key::W);
    assert_eq!(
        h.state().state.gizmo_mode,
        forge_viewport::gizmo::GizmoMode::Scale
    );
    press(&mut h, Modifiers::NONE, Key::E);
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

    // observed, G2 item 1: F2 is the Top view now, not rename (rename is the object
    // manager's double-click and menu entry)
    h.state_mut().state.selection.click(a);
    press(&mut h, Modifiers::NONE, Key::F2);
    assert!(h.state().objects().renaming.is_none());
    assert!(matches!(
        h.state().state.camera.projection,
        forge_viewport::camera::Projection::Orthographic { .. }
    ));
}

#[test]
fn a_shortcut_that_fails_reports_in_the_status_bar() {
    let mut e = Editor::new(common::state());
    e.state.selection.clear();
    let mut h = harness(e);
    press(&mut h, Modifiers::NONE, Key::F9); // key.record with nothing selected
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
    h.get_by_label("Key map…").click();
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
    settle(&mut h); // a single key is bound when no second key follows
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
    // rebind redo's Ctrl+Y to Ctrl+Z (undo's)
    h.get_by_label("Ctrl+Y").click();
    h.run();
    press(&mut h, Modifiers::COMMAND, Key::Z);
    settle(&mut h);
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
    settle(&mut h);
    h.get_by_label("Replace").click();
    h.run();
    let keys = &h.state().state.keys;
    assert!(keys.map.conflicts().is_empty());
    assert_eq!(keys.map.chord_of("edit.undo", &json!({})), None);
    assert_eq!(keys.map.for_command("edit.redo").count(), 1);
    assert_eq!(
        keys.map
            .lookup(&Modifiers::COMMAND, Key::Z, false)
            .unwrap()
            .command,
        "edit.redo"
    );

    // Escape while capturing changes nothing
    let before = h.state().state.keys.map.clone();
    h.get_by_label("Ctrl+Z").click();
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
    assert_eq!(read_settings(&path)["keymap"]["U I"], Value::Null);
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

// ---- owner notes N1 / N2 (brief 223) ------------------------------------------------

#[test]
fn no_default_binding_sits_on_a_bare_navigation_digit() {
    let map = KeyMap::defaults();
    for b in map.bindings() {
        assert!(
            !forge_ui::keymap::is_reserved(b.chord),
            "{} is bound to a navigation key",
            b.chord
        );
    }
    for d in ["1", "2", "3"] {
        assert!(forge_ui::keymap::is_reserved(chord(d)), "{d}");
    }
    // 4 .. 9, modified digits and the old X / C / V hold keys are not reserved
    for c in [
        "4",
        "9",
        "Alt+1",
        "Mod+1",
        "Mod+Shift+1",
        "Shift+1",
        "X",
        "C",
        "V",
    ] {
        assert!(!forge_ui::keymap::is_reserved(chord(c)), "{c}");
    }
    // E / R / T move, rotate, scale; W is free
    assert!(map.lookup(&Modifiers::NONE, Key::W, false).is_none());
    for (k, mode) in [(Key::E, "move"), (Key::R, "rotate"), (Key::T, "scale")] {
        let b = map.lookup(&Modifiers::NONE, k, false).unwrap();
        assert_eq!(
            (b.command.as_str(), &b.args),
            ("view.gizmo_mode", &json!({"mode": mode}))
        );
    }
}

#[test]
fn old_override_files_migrate_once() {
    // an old file: digits rebound or unbound, other bindings and keys of the user
    let path = temp_file("migrate");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(
        &path,
        r#"{"rpc": {"port": 9}, "keymap": {
            "1": {"command": "view.shading", "args": {"mode": "wire"}},
            "2": null,
            "3": {"command": "select.all"},
            "7": {"command": "select.none"},
            "E": {"command": "edit.undo"},
            "F9": {"command": "select.all"},
            "Hyper+Q": {"command": "edit.undo"}}}"#,
    )
    .unwrap();
    let mut keys = KeyMapState::default();
    keys.use_settings_file(Some(path.clone()));
    // three digit entries dropped with a warning each, the broken one still warns
    let digit_warnings: Vec<_> = keys
        .warnings
        .iter()
        .filter(|w| w.contains("navigation keys"))
        .collect();
    assert_eq!(digit_warnings.len(), 3, "{:?}", keys.warnings);
    assert_eq!(keys.warnings.len(), 4, "{:?}", keys.warnings);
    for k in [Key::Num1, Key::Num2, Key::Num3] {
        assert!(keys.map.lookup(&Modifiers::NONE, k, true).is_none());
    }
    // the user's other bindings are kept
    let cmd = |m: Modifiers, k: Key| keys.map.lookup(&m, k, false).map(|b| b.command.clone());
    assert_eq!(
        cmd(Modifiers::NONE, Key::Num7).as_deref(),
        Some("select.none")
    );
    assert_eq!(cmd(Modifiers::NONE, Key::E).as_deref(), Some("edit.undo"));
    assert_eq!(cmd(Modifiers::NONE, Key::F9).as_deref(), Some("select.all"));
    // the new shading defaults and tool keys are there
    assert_eq!(
        cmd(Modifiers::COMMAND | Modifiers::SHIFT, Key::Num1).as_deref(),
        Some("view.shading")
    );
    assert_eq!(
        cmd(Modifiers::NONE, Key::T).as_deref(),
        Some("view.gizmo_mode")
    );
    // the file lost exactly the digit entries and kept the rest, so a second load is silent
    let file = read_settings(&path);
    assert_eq!(file["rpc"]["port"], 9);
    let km = file["keymap"].as_object().unwrap();
    assert_eq!(km.len(), 4, "{km:?}");
    assert!(["1", "2", "3"].iter().all(|d| !km.contains_key(*d)));
    let mut again = KeyMapState::default();
    again.use_settings_file(Some(path.clone()));
    assert_eq!(again.warnings.len(), 1, "{:?}", again.warnings);
    assert!(again.warnings[0].contains("Hyper+Q"));
    // a file with only digit entries loses its keymap key altogether
    std::fs::write(&path, r#"{"keymap": {"1": null}}"#).unwrap();
    again.use_settings_file(Some(path.clone()));
    assert_eq!(again.warnings.len(), 1);
    assert!(read_settings(&path).get("keymap").is_none());
    assert_eq!(again.map, KeyMap::defaults());
    std::fs::remove_dir_all(path.parent().unwrap()).ok();
}

#[test]
fn the_editor_refuses_to_bind_a_navigation_digit() {
    let (mut e, path) = editor_with_file("reserved");
    e.state.selection.clear();
    let mut h = harness(e);
    open_editor_window(&mut h, "edit.undo");
    h.get_by_label("Ctrl+Z").click();
    h.run();
    assert!(h.state().keymap_editor().is_capturing());
    press(&mut h, Modifiers::NONE, Key::Num2);
    assert!(!h.state().keymap_editor().is_capturing());
    assert!(!h.state().keymap_editor().message.is_empty());
    let keys = &h.state().state.keys;
    assert_eq!(
        keys.map.chord_of("edit.undo", &json!({})),
        Some(chord("Mod+Z"))
    );
    assert!(read_settings(&path).get("keymap").is_none());
    std::fs::remove_dir_all(path.parent().unwrap()).ok();
}

// ---- observed defaults (brief 233, G2 item 1) ---------------------------------------

#[test]
fn observed_keys_run_their_commands() {
    use forge_viewport::panes::Layout;
    let mut e = Editor::new(common::state());
    let cube = common::create(&mut e.state, "forge.cube");
    let a = common::create(&mut e.state, "forge.null");
    e.state.selection.click(cube);
    let mut h = harness(e);

    // C makes the selected generator editable (a polygon object now)
    let cubes = |h: &Harness<'_, Editor>| {
        let scene = h.state().state.doc.scene();
        scene
            .roots()
            .iter()
            .filter(|r| scene.node(**r).is_some_and(|n| n.type_id == "forge.cube"))
            .count()
    };
    assert_eq!(cubes(&h), 1);
    press(&mut h, Modifiers::NONE, Key::C);
    assert_eq!(cubes(&h), 0, "the generator was made editable");
    let _ = cube;

    // F5 opens the four views, F5 again closes them
    press(&mut h, Modifiers::NONE, Key::F5);
    assert_eq!(h.state().state.views.layout, Layout::Four);
    press(&mut h, Modifiers::NONE, Key::F5);
    assert_eq!(h.state().state.views.layout, Layout::Single);
    // F1 Perspective, F3 Right, F4 Front: orthographic except the first
    press(&mut h, Modifiers::NONE, Key::F3);
    assert!(matches!(
        h.state().state.camera.projection,
        forge_viewport::camera::Projection::Orthographic { .. }
    ));
    press(&mut h, Modifiers::NONE, Key::F1);
    assert!(matches!(
        h.state().state.camera.projection,
        forge_viewport::camera::Projection::Perspective { .. }
    ));

    // F8 plays, F7 stops
    // (playing repaints every frame, so step instead of run)
    h.key_press(Key::F8);
    h.step();
    h.step();
    assert!(h.state().state.timeline.playing);
    h.key_press(Key::F7);
    h.step();
    h.step();
    assert!(!h.state().state.timeline.playing);

    // Alt+G groups the selection
    h.state_mut().state.selection.click(a);
    let roots = h.state().state.doc.scene().roots().len();
    press(&mut h, Modifiers::ALT, Key::G);
    assert_eq!(h.state().state.doc.scene().roots().len(), roots);
    let grouped = h.state().state.doc.scene().node(a).unwrap().parent;
    assert!(grouped.is_some(), "the null is inside a new group");
}

#[test]
fn bindings_with_no_command_yet_stay_unbound() {
    // observed in G2 but without an argument-free command: nothing may sit on them
    let map = KeyMap::defaults();
    for (text, what) in [
        ("9", "Live Selection"),
        ("0", "Rectangle Selection"),
        ("Mod+R", "Render View"),
        ("Shift+R", "Render to Picture Viewer"),
        ("F6", "Play backwards"),
        ("Alt+F9", "Record Animated"),
        ("Shift+G", "Ungroup"),
        ("Mod+Shift+Y", "Redo View"),
        ("Mod+Shift+Z", "Undo View"),
        ("K K", "Line Cut"),
        ("K L", "Loop/Path Cut"),
        ("K J", "Plane Cut"),
    ] {
        let (c, t) = forge_ui::keymap::parse_keys(text).unwrap();
        assert!(
            map.conflicts_with_keys(c, t, Scope::Global).is_empty(),
            "{what} ({text}) must stay unbound until its command exists"
        );
    }
}

// ---- two-key sequences (UI.md §14.7) --------------------------------------------------

#[test]
fn key_sequence_text_parses_prints_and_round_trips() {
    use forge_ui::keymap::{keys_text, parse_keys};
    assert_eq!(parse_keys("M T"), Ok((chord("M"), Some(chord("T")))));
    assert_eq!(parse_keys("  m   t "), Ok((chord("M"), Some(chord("T")))));
    assert_eq!(
        parse_keys("Mod + K  Mod + S"),
        Ok((chord("Mod+K"), Some(chord("Mod+S"))))
    );
    assert_eq!(parse_keys("Mod+Shift+Z"), Ok((chord("Mod+Shift+Z"), None)));
    for bad in ["", "M T U", "M Nope", "M Hyper+T", "M +"] {
        assert!(parse_keys(bad).is_err(), "'{bad}' parsed");
    }
    assert_eq!(keys_text(chord("M"), Some(chord("T"))), "M T");
    assert_eq!(keys_text(chord("ctrl+z"), None), "Mod+Z");
    let b = Binding::sequence(chord("U"), chord("Mod+S"), "mesh.subdivide", json!({}));
    assert_eq!(b.keys_text(), "U Mod+S");
    // a TOML default may be a sequence; three keys are refused
    let map =
        KeyMap::from_toml("version = 1\n[[bind]]\nchord = \"M T\"\ncommand = \"a.b\"\n").unwrap();
    assert_eq!(map.bindings()[0].then, Some(chord("T")));
    assert!(
        KeyMap::from_toml("version = 1\n[[bind]]\nchord = \"M T U\"\ncommand = \"a.b\"\n").is_err()
    );
}

fn step<'a>(
    map: &'a KeyMap,
    armed: &mut Option<forge_ui::keymap::Armed>,
    key: &str,
    hot: bool,
    now: f64,
) -> forge_ui::keymap::Step<'a> {
    map.step(chord(key), hot, armed, now)
}

fn runs(s: forge_ui::keymap::Step<'_>) -> Option<String> {
    match s {
        forge_ui::keymap::Step::Run(b) => Some(b.command.clone()),
        _ => None,
    }
}

#[test]
fn a_prefix_arms_then_the_second_key_runs() {
    use forge_ui::keymap::Step;
    let map = KeyMap::defaults();
    let mut armed = None;
    // M alone runs nothing and arms
    assert_eq!(step(&map, &mut armed, "M", false, 0.0), Step::Armed);
    assert_eq!(armed.map(|a| a.chord), Some(chord("M")));
    // the second key runs the command and disarms
    let s = step(&map, &mut armed, "T", false, 0.4);
    assert_eq!(runs(s).as_deref(), Some("mesh.extrude"));
    assert!(armed.is_none());
    // the same second key without the prefix is an ordinary press (T = scale)
    let s = step(&map, &mut armed, "T", false, 1.0);
    assert_eq!(runs(s).as_deref(), Some("view.gizmo_mode"));
    // the other prefixes and a repeated key
    step(&map, &mut armed, "U", false, 2.0);
    assert_eq!(
        runs(step(&map, &mut armed, "S", false, 2.1)).as_deref(),
        Some("mesh.subdivide")
    );
    step(&map, &mut armed, "M", false, 3.0);
    assert_eq!(
        runs(step(&map, &mut armed, "M", false, 3.1)).as_deref(),
        Some("mesh.connect_points")
    );
    // modifiers count: Mod+T after M is not M T
    step(&map, &mut armed, "M", false, 4.0);
    assert!(runs(step(&map, &mut armed, "Mod+T", false, 4.1)).is_none());
    assert!(armed.is_none());
}

#[test]
fn a_prefix_times_out_and_escape_cancels() {
    use forge_ui::keymap::{SEQUENCE_TIMEOUT, Step};
    let map = KeyMap::defaults();
    let mut armed = None;
    // within the timeout: runs; just after it: the second key is an ordinary press
    step(&map, &mut armed, "M", false, 10.0);
    assert_eq!(
        runs(step(
            &map,
            &mut armed,
            "T",
            false,
            10.0 + SEQUENCE_TIMEOUT - 0.01
        ))
        .as_deref(),
        Some("mesh.extrude")
    );
    step(&map, &mut armed, "M", false, 20.0);
    assert_eq!(
        runs(step(
            &map,
            &mut armed,
            "T",
            false,
            20.0 + SEQUENCE_TIMEOUT + 0.01
        ))
        .as_deref(),
        Some("view.gizmo_mode"),
        "after the timeout T is the scale key"
    );
    // a pending prefix expires for the status bar too
    let state = KeyMapState {
        armed: Some(forge_ui::keymap::Armed {
            chord: chord("M"),
            at: 5.0,
        }),
        ..KeyMapState::default()
    };
    assert_eq!(state.pending_prefix(5.5), Some(chord("M")));
    assert_eq!(state.pending_prefix(5.0 + SEQUENCE_TIMEOUT + 0.1), None);
    // Escape cancels and is consumed; the next key is ordinary again
    step(&map, &mut armed, "M", false, 30.0);
    assert_eq!(
        step(&map, &mut armed, "Escape", false, 30.1),
        Step::Cancelled
    );
    assert!(armed.is_none());
    assert_eq!(step(&map, &mut armed, "Escape", false, 30.2), Step::Pass);
    // a key that continues nothing disarms and is handled as a fresh press
    step(&map, &mut armed, "M", false, 40.0);
    assert_eq!(step(&map, &mut armed, "X", false, 40.1), Step::Pass);
    assert!(armed.is_none());
    // ... which may itself be a prefix
    step(&map, &mut armed, "M", false, 50.0);
    assert_eq!(step(&map, &mut armed, "U", false, 50.1), Step::Armed);
    assert_eq!(armed.map(|a| a.chord), Some(chord("U")));
}

#[test]
fn a_prefix_and_a_single_key_on_the_same_chord_conflict() {
    let sequence = |a: &str, b: &str, c: &str| Binding::sequence(chord(a), chord(b), c, json!({}));
    // the defaults have none: M and U only start sequences
    assert!(KeyMap::defaults().conflicts().is_empty());
    let map = KeyMap::new(vec![
        bind("M", "marker.add"),
        sequence("M", "T", "mesh.extrude"),
        sequence("M", "W", "mesh.extrude_inner"),
        sequence("U", "S", "mesh.subdivide"),
        sequence("U", "S", "mesh.optimize"),
    ]);
    let c = map.conflicts();
    assert_eq!(c.len(), 2, "{c:?}");
    // the single key against its sequences: one group on the prefix
    assert_eq!((c[0].chord, c[0].then), (chord("M"), None));
    assert_eq!(c[0].bindings, vec![0, 1, 2]);
    // two sequences on the same two keys
    assert_eq!((c[1].chord, c[1].then), (chord("U"), Some(chord("S"))));
    assert_eq!(c[1].bindings, vec![3, 4]);
    // dispatch is deterministic: the prefix arms, the single key never runs
    let mut armed = None;
    assert_eq!(
        step(&map, &mut armed, "M", false, 0.0),
        forge_ui::keymap::Step::Armed
    );
    // different second keys, different scopes
    let mut viewport = sequence("K", "K", "mesh.knife");
    viewport.scope = Scope::Viewport;
    let map = KeyMap::new(vec![viewport, bind("K", "key.record")]);
    assert_eq!(map.conflicts().len(), 1);
    let mut armed = None;
    assert_eq!(
        runs(step(&map, &mut armed, "K", false, 0.0)).as_deref(),
        Some("key.record"),
        "outside the viewport the single key runs"
    );
    assert_eq!(
        step(&map, &mut armed, "K", true, 1.0),
        forge_ui::keymap::Step::Armed
    );
    // M T and M W do not conflict with each other
    assert!(
        KeyMap::new(vec![
            sequence("M", "T", "a.b"),
            sequence("M", "W", "a.c"),
            sequence("U", "T", "a.d"),
        ])
        .conflicts()
        .is_empty()
    );
}

#[test]
fn binding_over_sequences_and_a_prefix_replaces_what_it_collides_with() {
    let mut map = KeyMap::defaults();
    // a single key on a prefix removes every sequence it starts
    let taken = map.conflicts_with(chord("M"), Scope::Global);
    assert_eq!(taken.len(), 10);
    let removed = map.bind(bind("M", "marker.add"));
    assert_eq!(removed.len(), 10);
    assert!(map.conflicts().is_empty());
    // a sequence on a single key removes the single key
    let mut map = KeyMap::new(vec![
        bind("G", "timeline.step"),
        bind("Mod+A", "select.all"),
    ]);
    let removed = map.bind(Binding::sequence(chord("G"), chord("G"), "a.b", json!({})));
    assert_eq!(removed.len(), 1);
    assert_eq!(removed[0].command, "timeline.step");
    // the same two keys replace the old sequence
    let removed = map.bind(Binding::sequence(chord("G"), chord("G"), "a.c", json!({})));
    assert_eq!(removed.len(), 1);
    // a different second key is free
    assert!(
        map.conflicts_with_keys(chord("G"), Some(chord("H")), Scope::Global)
            .is_empty()
    );
}

#[test]
fn sequence_overrides_follow_the_settings_file_syntax() {
    let defaults = KeyMap::defaults();
    let mut map = defaults.clone();
    let warnings = map.apply_overrides(&json!({
        "M T": null,
        "M  X": {"command": "mesh.extrude", "args": {}},
        "Mod+K Mod+S": {"command": "select.all"},
        "U S": {"command": "mesh.optimize"},
    }));
    assert!(warnings.is_empty(), "{warnings:?}");
    let mut armed = None;
    step(&map, &mut armed, "M", false, 0.0);
    assert_ne!(
        runs(step(&map, &mut armed, "T", false, 0.1)).as_deref(),
        Some("mesh.extrude"),
        "M T is unbound (T alone is the scale key)"
    );
    step(&map, &mut armed, "M", false, 1.0);
    assert_eq!(
        runs(step(&map, &mut armed, "X", false, 1.1)).as_deref(),
        Some("mesh.extrude")
    );
    step(&map, &mut armed, "Mod+K", false, 2.0);
    assert_eq!(
        runs(step(&map, &mut armed, "Mod+S", false, 2.1)).as_deref(),
        Some("select.all")
    );
    step(&map, &mut armed, "U", false, 3.0);
    assert_eq!(
        runs(step(&map, &mut armed, "S", false, 3.1)).as_deref(),
        Some("mesh.optimize"),
        "the entry replaced the default on U S"
    );
    // the minimal inverse uses canonical sequence text and round-trips
    let o = map.overrides(&defaults);
    let keys: Vec<&String> = o.as_object().unwrap().keys().collect();
    assert!(keys.iter().any(|k| *k == "M T"), "{keys:?}");
    assert!(keys.iter().any(|k| *k == "Mod+K Mod+S"), "{keys:?}");
    let mut again = defaults.clone();
    assert!(again.apply_overrides(&o).is_empty());
    assert!(same(&map, &again));
    // a single-key entry on a prefix replaces the sequences; unbinding it keeps them
    let mut m2 = defaults.clone();
    m2.apply_overrides(&json!({"M": null}));
    assert!(
        same(&m2, &defaults),
        "null on M unbinds only the single key M"
    );
    m2.apply_overrides(&json!({"M": {"command": "marker.add"}}));
    assert!(m2.conflicts().is_empty());
    assert!(
        m2.bindings()
            .iter()
            .all(|b| b.chord != chord("M") || b.then.is_none())
    );
    let o2 = m2.overrides(&defaults);
    let mut m3 = defaults.clone();
    assert!(m3.apply_overrides(&o2).is_empty());
    assert!(same(&m2, &m3));
    // bare navigation digits are refused as a first or second key
    let mut m4 = defaults.clone();
    let w = m4.apply_overrides(
        &json!({"1 A": {"command": "select.all"}, "A 2": {"command": "select.all"}}),
    );
    assert_eq!(w.len(), 2, "{w:?}");
    assert!(same(&m4, &defaults));
    // three keys are a warning
    assert_eq!(
        m4.apply_overrides(&json!({"A B C": {"command": "select.all"}}))
            .len(),
        1
    );
}

#[test]
fn a_sequence_runs_in_the_window_and_the_prefix_alone_does_nothing() {
    let mut e = Editor::new(common::state());
    let a = common::create(&mut e.state, "forge.null");
    let b = common::create(&mut e.state, "forge.null");
    e.state.selection.click(a);
    e.state.keys.map.bind(Binding::sequence(
        chord("X"),
        chord("Y"),
        "select.all",
        json!({}),
    ));
    let mut h = harness(e);
    press(&mut h, Modifiers::NONE, Key::X);
    assert_eq!(
        h.state().state.selection.nodes.len(),
        1,
        "the prefix alone runs nothing"
    );
    assert!(h.state().state.keys.armed.is_some());
    h.get_by_label_contains("X \u{2026}");
    press(&mut h, Modifiers::NONE, Key::Y);
    assert_eq!(h.state().state.selection.nodes.len(), 2);
    assert!(h.state().state.selection.nodes.contains(&b));
    assert!(h.state().state.keys.armed.is_none());
    // Escape drops the prefix; the Y that follows does nothing
    h.state_mut().state.selection.click(a);
    press(&mut h, Modifiers::NONE, Key::X);
    press(&mut h, Modifiers::NONE, Key::Escape);
    press(&mut h, Modifiers::NONE, Key::Y);
    assert_eq!(h.state().state.selection.nodes.len(), 1);
}

#[test]
fn the_editor_captures_single_keys_and_sequences() {
    use forge_ui::keymap_editor::{Captured, KeyMapEditor};
    let mut keys = KeyMapState::default();
    let registry = common::state().app.commands.clone();
    let row_of = |keys: &KeyMapState, command: &str| {
        rows(keys, &registry, "", &|c| c.to_string())
            .into_iter()
            .find(|r| r.command == command)
            .unwrap()
    };
    let mut ed = KeyMapEditor::default();
    // a free first key waits for a second one; a second key within the time makes a sequence
    let row = row_of(&keys, "edit.symmetry");
    assert!(row.chord.is_none());
    ed.start_capture(row.clone());
    ed.captured(Captured::Chord(chord("J")), &mut keys, 0.0);
    assert_eq!(ed.first_key(), Some(chord("J")));
    assert!(
        keys.map
            .bindings()
            .iter()
            .all(|b| b.command != "edit.symmetry")
    );
    ed.captured(Captured::Chord(chord("X")), &mut keys, 0.5);
    let b = keys.map.binding_of("edit.symmetry", &json!({})).unwrap();
    assert_eq!((b.chord, b.then), (chord("J"), Some(chord("X"))));
    assert!(keys.map.conflicts().is_empty());
    // the row now shows the sequence, searchable by its text
    let r = row_of(&keys, "edit.symmetry");
    assert_eq!((r.chord, r.then), (Some(chord("J")), Some(chord("X"))));
    assert!(r.changed);
    let found = rows(&keys, &registry, "j x", &|c| c.to_string());
    assert!(found.iter().any(|r| r.command == "edit.symmetry"));
    assert!(
        rows(&keys, &registry, "m t", &|c| c.to_string())
            .iter()
            .any(|r| r.command == "mesh.extrude" && r.then == Some(chord("T")))
    );
    // without a second key the time-out binds the single key
    let mut keys = KeyMapState::default();
    let row = row_of(&keys, "edit.symmetry");
    ed.start_capture(row);
    ed.captured(Captured::Chord(chord("J")), &mut keys, 10.0);
    assert!(!ed.tick(&mut keys, 11.0) || ed.first_key().is_some());
    assert!(keys.map.binding_of("edit.symmetry", &json!({})).is_none());
    ed.tick(&mut keys, 11.6);
    let b = keys.map.binding_of("edit.symmetry", &json!({})).unwrap();
    assert_eq!((b.chord, b.then), (chord("J"), None));
    // a late second key is dropped and the first stands alone
    let mut keys = KeyMapState::default();
    let row = row_of(&keys, "edit.soft_selection");
    ed.start_capture(row);
    ed.captured(Captured::Chord(chord("J")), &mut keys, 20.0);
    ed.captured(Captured::Chord(chord("X")), &mut keys, 22.0);
    let b = keys
        .map
        .binding_of("edit.soft_selection", &json!({}))
        .unwrap();
    assert_eq!((b.chord, b.then), (chord("J"), None));
    // Escape cancels at either step
    let mut keys = KeyMapState::default();
    let row = row_of(&keys, "edit.symmetry");
    ed.start_capture(row.clone());
    ed.captured(Captured::Chord(chord("J")), &mut keys, 0.0);
    ed.captured(Captured::Cancel, &mut keys, 0.1);
    assert!(!ed.is_capturing() && ed.first_key().is_none());
    ed.start_capture(row);
    ed.captured(Captured::Cancel, &mut keys, 0.2);
    assert!(!ed.is_capturing());
    assert_eq!(keys.map, KeyMap::defaults());
    // a single key on a prefix asks first, naming the sequences; Replace removes them
    let row = row_of(&keys, "marker.add");
    ed.start_capture(row);
    ed.captured(Captured::Chord(chord("M")), &mut keys, 0.0);
    ed.tick(&mut keys, 2.0);
    assert!(ed.has_pending());
    assert!(
        keys.map
            .bindings()
            .iter()
            .any(|b| b.then == Some(chord("T")))
    );
    ed.replace_pending(&mut keys);
    assert!(keys.map.conflicts().is_empty());
    assert!(
        keys.map
            .bindings()
            .iter()
            .all(|b| b.then.is_none() || b.chord != chord("M"))
    );
    assert_eq!(
        keys.map.binding_of("marker.add", &json!({})).unwrap().chord,
        chord("M")
    );
    // a navigation digit is refused as the first key
    let row = row_of(&keys, "edit.symmetry");
    ed.start_capture(row);
    ed.captured(Captured::Chord(chord("2")), &mut keys, 0.0);
    assert!(!ed.is_capturing() && ed.first_key().is_none());
    assert!(!ed.message.is_empty());
}
