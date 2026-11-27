//! Attribute manager (docs/specs/UI.md §12; brief 150): widgets per parameter kind, tabs,
//! multi-edit with mixed values, scrubbing maths. Every edit runs `param.edit`.

use std::sync::Arc;

use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use forge_core::{
    Category, NodeId, NodeType, ParamDesc, Plugin, Registry, RegistryError, Unit, Value,
};
use forge_ui::app::Editor;
use forge_ui::attributes::{AttrRow, Widget, attribute_rows_multi, split_group};
use forge_ui::commands::execute;
use forge_ui::state::EditorState;
use forge_ui::widgets::link_list::moved;
use forge_ui::widgets::scrub::{self, Mods};
use serde_json::json;

struct Plug;

impl Plugin for Plug {
    fn id(&self) -> &str {
        "am"
    }

    fn register(&self, r: &mut Registry) -> Result<(), RegistryError> {
        r.register(
            NodeType::new("am.thing", "Thing", Category::Generator)
                .param(
                    ParamDesc::float("angle", "Angle", 0.5)
                        .unit(Unit::Angle)
                        .group("Basic/Main"),
                )
                .param(
                    ParamDesc::int("count", "Count", 3)
                        .range(1, 50)
                        .group("Basic/Main"),
                )
                .param(ParamDesc::bool("flag", "Flag", false).group("Basic/Main"))
                .param(
                    ParamDesc::enumeration("mode", "Mode", "a", &[("a", "Alpha"), ("b", "Beta")])
                        .group("Basic/Main"),
                )
                .param(ParamDesc::vec3("pos", "Pos", [1.0, 2.0, 3.0]).group("Basic/Main"))
                .param(ParamDesc::color("tint", "Tint", [1.0, 0.5, 0.25]).group("Extra/Look"))
                .param(ParamDesc::string("note", "Note", "").group("Extra/Look"))
                .param(
                    ParamDesc::link("target", "Target", &[Category::Generator])
                        .group("Extra/Links"),
                )
                .param(
                    ParamDesc::link_list("targets", "Targets", &[Category::Generator])
                        .group("Extra/Links"),
                ),
        )?;
        r.register(
            NodeType::new("am.plain", "Plain", Category::Generator)
                .param(ParamDesc::float("angle", "Angle", 0.0).unit(Unit::Angle)),
        )?;
        Ok(())
    }
}

fn state() -> EditorState {
    let mut r = Registry::with_builtins();
    r.install(&Plug).unwrap();
    EditorState::new(Arc::new(r))
}

fn make(s: &mut EditorState, ty: &str) -> NodeId {
    execute(s, "object.create", json!({"type": ty})).unwrap();
    s.selection.active.unwrap()
}

fn harness(state: EditorState) -> Harness<'static, Editor> {
    Harness::builder()
        .with_size([1280.0, 1400.0])
        .build_ui_state(|ui, e: &mut Editor| e.show(ui, None), Editor::new(state))
}

fn param(h: &Harness<'_, Editor>, id: NodeId, p: &str) -> Value {
    h.state().state.doc.scene().param(id, p).cloned().unwrap()
}

fn undo_steps(h: &Harness<'_, Editor>) -> usize {
    h.state().state.doc.history().undo_labels().len()
}

fn undo(h: &mut Harness<'_, Editor>) {
    execute(&mut h.state_mut().state, "edit.undo", json!({})).unwrap();
    h.run();
}

fn key_menu(h: &mut Harness<'_, Editor>, marker: &str) {
    let p = h.get_by_label(marker).rect().center();
    h.hover_at(p);
    h.run();
    for pressed in [true, false] {
        h.event(egui::Event::PointerButton {
            pos: p,
            button: egui::PointerButton::Secondary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        });
        h.run();
    }
}

#[test]
fn attribute_key_indicator_stages_a_change_then_keys_it_and_navigates_the_row() {
    let mut st = state();
    let id = make(&mut st, "am.thing");
    let mut h = harness(st);
    h.run();
    let click = |h: &mut Harness<'_, Editor>, name| {
        h.get_by_label(name).click();
        h.run();
    };
    click(&mut h, "Flag unkeyed");
    h.get_by_label("Flag keyed");
    let n = undo_steps(&h);
    click(&mut h, "Flag");
    h.get_by_label("Flag changed");
    assert_eq!(undo_steps(&h), n, "draft is not an undoable edit");
    let key_value = |h: &Harness<'_, Editor>| {
        h.state().state.doc.scene().node(id).unwrap().tracks["flag"].keys[0]
    };
    assert_eq!(key_value(&h).value, 0.0);
    click(&mut h, "Flag changed");
    assert_eq!(key_value(&h).value, 1.0);
    assert_eq!(key_value(&h).interp, forge_core::Interp::Step);
    assert_eq!(undo_steps(&h), n + 1);

    let st = &mut h.state_mut().state;
    execute(
        st,
        "key.set_parameter",
        json!({"node": id.0, "param": "flag", "frame": 20, "value": false}),
    )
    .unwrap();
    execute(
        st,
        "key.set",
        json!({"node": id.0, "track": "angle", "frame": 10, "value": 10}),
    )
    .unwrap();
    h.run();
    key_menu(&mut h, "Flag keyed");
    click(&mut h, "Next Key on Parameter");
    assert_eq!(h.state().state.timeline.current, 20);
    key_menu(&mut h, "Flag keyed");
    click(&mut h, "Delete Key on Parameter");
    h.get_by_label("Flag animated");
    assert_eq!(
        h.state().state.doc.scene().node(id).unwrap().tracks["flag"]
            .keys
            .len(),
        1
    );
}

/// Clicks a key button and lets the next frames show the result without further input.
fn click_key_button(h: &mut Harness<'_, Editor>, name: &str) {
    h.get_by_label(name).click();
    h.step();
    h.step();
}

fn track_len(h: &Harness<'_, Editor>, id: NodeId, track: &str) -> usize {
    h.state()
        .state
        .doc
        .scene()
        .node(id)
        .unwrap()
        .tracks
        .get(track)
        .map_or(0, |t| t.keys.len())
}

#[test]
fn the_key_button_toggles_a_key_at_the_current_frame() {
    // owner note N3 (UI.md §12): add when no key is here, remove when one is
    let mut st = state();
    let id = make(&mut st, "am.thing");
    let mut h = harness(st);
    h.run();
    let n = undo_steps(&h);

    // no track: adds
    click_key_button(&mut h, "Flag unkeyed");
    h.get_by_label("Flag keyed");
    assert_eq!(track_len(&h, id, "flag"), 1);
    assert_eq!(undo_steps(&h), n + 1);

    // a key here: removes, in one undo step; the last key takes the track with it
    click_key_button(&mut h, "Flag keyed");
    h.get_by_label("Flag unkeyed");
    assert!(
        !h.state()
            .state
            .doc
            .scene()
            .node(id)
            .unwrap()
            .tracks
            .contains_key("flag"),
        "the empty track is removed"
    );
    assert_eq!(undo_steps(&h), n + 2);

    // undo restores the key, and the button shows it
    undo(&mut h);
    assert_eq!(track_len(&h, id, "flag"), 1);
    h.get_by_label("Flag keyed");
    assert_eq!(undo_steps(&h), n + 1);

    // a frame between keys adds a key, a second click removes just that one
    execute(
        &mut h.state_mut().state,
        "key.set_parameter",
        json!({"node": id.0, "param": "flag", "frame": 20, "value": true}),
    )
    .unwrap();
    h.state_mut().state.timeline.current = 10;
    h.run();
    h.get_by_label("Flag animated");
    click_key_button(&mut h, "Flag animated");
    h.get_by_label("Flag keyed");
    assert_eq!(track_len(&h, id, "flag"), 3);
    click_key_button(&mut h, "Flag keyed");
    h.get_by_label("Flag animated");
    assert_eq!(track_len(&h, id, "flag"), 2);
    let frames: Vec<f64> = h.state().state.doc.scene().node(id).unwrap().tracks["flag"]
        .keys
        .iter()
        .map(|k| k.time * 30.0)
        .collect();
    assert_eq!(frames, [0.0, 20.0]);
}

#[test]
fn toggling_a_vector_row_off_removes_all_its_component_keys_in_one_step() {
    let mut st = state();
    let id = make(&mut st, "am.thing");
    let mut h = harness(st);
    h.run();
    click_key_button(&mut h, "Pos unkeyed");
    let n = undo_steps(&h);
    for axis in ["x", "y", "z"] {
        assert_eq!(track_len(&h, id, &format!("pos.{axis}")), 1);
    }
    click_key_button(&mut h, "Pos keyed");
    h.get_by_label("Pos unkeyed");
    assert_eq!(undo_steps(&h), n + 1);
    for axis in ["x", "y", "z"] {
        assert_eq!(track_len(&h, id, &format!("pos.{axis}")), 0);
    }
    undo(&mut h);
    h.get_by_label("Pos keyed");
    for axis in ["x", "y", "z"] {
        assert_eq!(track_len(&h, id, &format!("pos.{axis}")), 1);
    }
}

#[test]
fn a_changed_row_updates_its_key_instead_of_removing_it() {
    let mut st = state();
    let id = make(&mut st, "am.thing");
    let mut h = harness(st);
    h.run();
    click_key_button(&mut h, "Flag unkeyed");
    h.get_by_label("Flag").click();
    h.run();
    h.get_by_label("Flag changed");
    click_key_button(&mut h, "Flag changed");
    h.get_by_label("Flag keyed");
    assert_eq!(track_len(&h, id, "flag"), 1);
    let key = h.state().state.doc.scene().node(id).unwrap().tracks["flag"].keys[0];
    assert_eq!(key.value, 1.0);
}

#[test]
fn keying_a_vector_row_keys_every_component_in_one_step() {
    let mut st = state();
    let id = make(&mut st, "am.thing");
    let mut h = harness(st);
    h.run();
    let n = undo_steps(&h);
    h.get_by_label("Pos unkeyed").click();
    h.run();
    let node = h.state().state.doc.scene().node(id).unwrap();
    for (axis, v) in [("x", 1.0), ("y", 2.0), ("z", 3.0)] {
        assert_eq!(node.tracks[&format!("pos.{axis}")].keys[0].value, v);
    }
    assert_eq!(undo_steps(&h), n + 1);
    undo(&mut h);
    h.get_by_label("Pos unkeyed");
}

fn combo(h: &mut Harness<'_, Editor>, label: &str) {
    h.get_by_role_and_label(egui::accesskit::Role::ComboBox, label)
        .click();
    h.run();
}

/// Clicks the entry of an open drop-down (drawn last, after the object manager's rows).
fn pick(h: &mut Harness<'_, Editor>, label: &str) {
    h.get_all_by_label(label).last().unwrap().click();
    h.run();
}

fn tab(h: &mut Harness<'_, Editor>, name: &str) {
    h.get_by_label(name).click();
    h.run();
}

#[test]
fn float_edit_runs_a_command_in_display_units_and_undoes() {
    let mut s = state();
    let a = make(&mut s, "am.thing");
    let mut h = harness(s);
    h.run();
    let before = undo_steps(&h);
    h.get_by_label("Angle").set_value_accesskit(90.0);
    h.run();
    let Value::Float(v) = param(&h, a, "angle") else {
        panic!()
    };
    assert!((v - std::f64::consts::FRAC_PI_2).abs() < 1e-9, "{v}");
    assert_eq!(undo_steps(&h), before + 1);
    undo(&mut h);
    assert_eq!(param(&h, a, "angle"), Value::Float(0.5));
}

#[test]
fn int_edit_is_clamped_by_the_range_and_undoes() {
    let mut s = state();
    let a = make(&mut s, "am.thing");
    let mut h = harness(s);
    h.run();
    h.get_by_label("Count").set_value_accesskit(7.0);
    h.run();
    assert_eq!(param(&h, a, "count"), Value::Int(7));
    undo(&mut h);
    assert_eq!(param(&h, a, "count"), Value::Int(3));
}

#[test]
fn bool_edit_and_undo() {
    let mut s = state();
    let a = make(&mut s, "am.thing");
    let mut h = harness(s);
    h.run();
    h.get_by_label("Flag").click();
    h.run();
    assert_eq!(param(&h, a, "flag"), Value::Bool(true));
    undo(&mut h);
    assert_eq!(param(&h, a, "flag"), Value::Bool(false));
}

#[test]
fn enum_edit_and_undo() {
    let mut s = state();
    let a = make(&mut s, "am.thing");
    let mut h = harness(s);
    h.run();
    combo(&mut h, "Mode");
    pick(&mut h, "Beta");
    assert_eq!(param(&h, a, "mode"), Value::Enum("b".into()));
    undo(&mut h);
    assert_eq!(param(&h, a, "mode"), Value::Enum("a".into()));
}

#[test]
fn vector_edit_sets_one_component_and_undoes() {
    let mut s = state();
    let a = make(&mut s, "am.thing");
    let mut h = harness(s);
    h.run();
    let fields: Vec<_> = h.get_all_by_label("Pos").collect();
    assert_eq!(fields.len(), 3);
    fields[1].set_value_accesskit(9.0);
    h.run();
    assert_eq!(param(&h, a, "pos"), Value::Vec3([1.0, 9.0, 3.0]));
    undo(&mut h);
    assert_eq!(param(&h, a, "pos"), Value::Vec3([1.0, 2.0, 3.0]));
}

#[test]
fn tabs_split_the_groups_and_switch_on_click() {
    let mut s = state();
    let a = make(&mut s, "am.thing");
    let view = attribute_rows_multi(s.doc.scene(), &[a]).unwrap();
    assert_eq!(view.tabs(), vec!["Basic", "Transform", "Extra"]);
    let names: Vec<&str> = view.groups_of("Extra").iter().map(|g| g.0).collect();
    assert_eq!(names, vec!["Look", "Links"]);
    assert_eq!(split_group("Shape"), ("Basic", "Shape"));
    assert_eq!(split_group("Tab/Shape"), ("Tab", "Shape"));
    let mut h = harness(s);
    h.run();
    assert!(h.query_by_label("Angle").is_some());
    assert!(h.query_by_label("Note").is_none());
    tab(&mut h, "Extra");
    assert!(h.query_by_label("Note").is_some());
    assert!(h.query_by_label("Angle").is_none());
}

#[test]
fn built_in_objects_remember_inspector_tabs_per_type() {
    let mut s = EditorState::new(Arc::new(forge_app::default_registry().unwrap()));
    make(&mut s, "forge.sphere");
    let mut h = harness(s);
    h.run();
    assert!(h.query_by_label("Radius").is_some());
    assert!(h.query_all_by_label("Position").next().is_none());
    tab(&mut h, "Transform");
    assert!(h.query_all_by_label("Position").next().is_some());
    assert!(h.query_by_label("Radius").is_none());

    let cloner = make(&mut h.state_mut().state, "forge.cloner");
    h.run();
    let view = attribute_rows_multi(h.state().state.doc.scene(), &[cloner]).unwrap();
    assert_eq!(
        view.tabs(),
        ["Transform", "Object", "Clone Transform", "Effectors"]
    );
    h.get_by_label("Effectors");
    assert!(h.query_by_label("Clones").is_some());
    make(&mut h.state_mut().state, "forge.sphere");
    h.run();
    assert!(h.query_all_by_label("Position").next().is_some());
    assert!(h.query_by_label("Radius").is_none());
}

#[test]
fn string_edit_merges_into_one_undo_step() {
    let mut s = state();
    let a = make(&mut s, "am.thing");
    let mut h = harness(s);
    h.run();
    tab(&mut h, "Extra");
    let before = undo_steps(&h);
    let note = h.get_by_label("Note");
    note.focus();
    h.run();
    h.get_by_label("Note").type_text("hi");
    h.run();
    assert_eq!(param(&h, a, "note"), Value::String("hi".into()));
    assert_eq!(undo_steps(&h), before + 1);
    undo(&mut h);
    assert_eq!(param(&h, a, "note"), Value::String(String::new()));
}

#[test]
fn colour_edit_goes_through_param_edit_and_the_row_shows() {
    let mut s = state();
    let a = make(&mut s, "am.thing");
    let mut h = harness(s);
    h.run();
    tab(&mut h, "Extra");
    h.get_by_label("Tint");
    let st = &mut h.state_mut().state;
    execute(
        st,
        "param.edit",
        json!({"param": "tint", "value": [0.1, 0.2, 0.3], "nodes": [a.0]}),
    )
    .unwrap();
    assert_eq!(
        st.doc.scene().param(a, "tint"),
        Some(&Value::Color([0.1, 0.2, 0.3]))
    );
    execute(st, "edit.undo", json!({})).unwrap();
    assert_eq!(
        st.doc.scene().param(a, "tint"),
        Some(&Value::Color([1.0, 0.5, 0.25]))
    );
}

#[test]
fn link_edit_and_clear() {
    let mut s = state();
    let other = make(&mut s, "am.plain");
    execute(
        &mut s,
        "object.rename",
        json!({"node": other.0, "name": "Other"}),
    )
    .unwrap();
    let a = make(&mut s, "am.thing");
    let mut h = harness(s);
    h.run();
    tab(&mut h, "Extra");
    combo(&mut h, "Target");
    pick(&mut h, "Other");
    assert_eq!(param(&h, a, "target"), Value::Link(Some(other)));
    h.get_by_label("Clear Target").click();
    h.run();
    assert_eq!(param(&h, a, "target"), Value::Link(None));
    undo(&mut h);
    assert_eq!(param(&h, a, "target"), Value::Link(Some(other)));
}

#[test]
fn link_list_add_move_remove() {
    let mut s = state();
    let x = make(&mut s, "am.plain");
    execute(
        &mut s,
        "object.rename",
        json!({"node": x.0, "name": "Xray"}),
    )
    .unwrap();
    let y = make(&mut s, "am.plain");
    execute(
        &mut s,
        "object.rename",
        json!({"node": y.0, "name": "Yankee"}),
    )
    .unwrap();
    let a = make(&mut s, "am.thing");
    let mut h = harness(s);
    h.run();
    tab(&mut h, "Extra");
    for name in ["Xray", "Yankee"] {
        combo(&mut h, "Targets");
        pick(&mut h, name);
    }
    assert_eq!(param(&h, a, "targets"), Value::LinkList(vec![x, y]));
    h.get_by_label("Move Yankee up").click();
    h.run();
    assert_eq!(param(&h, a, "targets"), Value::LinkList(vec![y, x]));
    h.get_by_label("Remove Yankee").click();
    h.run();
    assert_eq!(param(&h, a, "targets"), Value::LinkList(vec![x]));
    undo(&mut h);
    assert_eq!(param(&h, a, "targets"), Value::LinkList(vec![y, x]));
}

#[test]
fn moving_list_entries_is_bounded() {
    let ids = [NodeId(1), NodeId(2), NodeId(3)];
    assert_eq!(moved(&ids, 2, -1), [NodeId(1), NodeId(3), NodeId(2)]);
    assert_eq!(moved(&ids, 0, -1), ids);
    assert_eq!(moved(&ids, 2, 1), ids);
}

#[test]
fn multi_edit_shows_mixed_and_a_set_applies_to_all_in_one_step() {
    let mut s = state();
    let a = make(&mut s, "am.thing");
    let b = make(&mut s, "am.thing");
    execute(
        &mut s,
        "param.edit",
        json!({"param": "angle", "value": 10.0, "nodes": [a.0]}),
    )
    .unwrap();
    execute(
        &mut s,
        "param.edit",
        json!({"param": "angle", "value": 20.0, "nodes": [b.0]}),
    )
    .unwrap();
    execute(
        &mut s,
        "param.edit",
        json!({"param": "pos", "value": 7.0, "component": 1, "nodes": [b.0]}),
    )
    .unwrap();
    s.selection.nodes.insert(a);
    s.selection.nodes.insert(b);
    s.selection.active = Some(b);
    let view = attribute_rows_multi(s.doc.scene(), &[b, a]).unwrap();
    assert_eq!(view.count, 2);
    let row = |p: &str| -> &AttrRow {
        view.groups
            .iter()
            .flat_map(|g| &g.1)
            .find(|r| r.param == p)
            .unwrap()
    };
    assert!(row("angle").mixed);
    assert!(!row("count").mixed);
    assert_eq!(row("pos").mixed_axes, [false, true, false]);
    assert_eq!(
        row("angle").widget,
        Widget::FloatDrag {
            min: f64::MIN,
            max: f64::MAX,
            unit: Unit::Angle
        }
    );

    let mut h = harness(s);
    h.run();
    let before = undo_steps(&h);
    h.get_by_label("Angle").set_value_accesskit(45.0);
    h.run();
    for n in [a, b] {
        let Value::Float(v) = param(&h, n, "angle") else {
            panic!()
        };
        assert!((v - std::f64::consts::FRAC_PI_4).abs() < 1e-9);
    }
    assert_eq!(undo_steps(&h), before + 1);
    // editing one vector component keeps the other components of each object
    h.get_all_by_label("Pos")
        .next()
        .unwrap()
        .set_value_accesskit(5.0);
    h.run();
    assert_eq!(param(&h, a, "pos"), Value::Vec3([5.0, 2.0, 3.0]));
    assert_eq!(param(&h, b, "pos"), Value::Vec3([5.0, 7.0, 3.0]));
    undo(&mut h);
    undo(&mut h);
    let Value::Float(v) = param(&h, a, "angle") else {
        panic!()
    };
    assert!((v - 10f64.to_radians()).abs() < 1e-9);
    let Value::Float(v) = param(&h, b, "angle") else {
        panic!()
    };
    assert!((v - 20f64.to_radians()).abs() < 1e-9);
}

#[test]
fn multi_edit_lists_only_common_parameters_and_skips_others() {
    let mut s = state();
    let a = make(&mut s, "am.thing");
    let p = make(&mut s, "am.plain");
    let view = attribute_rows_multi(s.doc.scene(), &[a, p]).unwrap();
    let params: Vec<&str> = view
        .groups
        .iter()
        .flat_map(|g| &g.1)
        .map(|r| r.param.as_str())
        .collect();
    // the transform is common to every object
    assert_eq!(
        params,
        vec!["transform.pos", "transform.rot", "transform.scale", "angle"]
    );
    // the command skips objects without the parameter and fails when none has it
    execute(
        &mut s,
        "param.edit",
        json!({"param": "count", "value": 9, "nodes": [a.0, p.0]}),
    )
    .unwrap();
    assert_eq!(s.doc.scene().param(a, "count"), Some(&Value::Int(9)));
    let e = execute(
        &mut s,
        "param.edit",
        json!({"param": "count", "value": 9, "nodes": [p.0]}),
    );
    assert!(e.is_err());
}

#[test]
fn a_gesture_merges_edits_into_one_undo_step_and_autokey_keys() {
    let mut s = state();
    let a = make(&mut s, "am.thing");
    let before = s.doc.history().undo_labels().len();
    for v in [1.0, 2.0, 3.0] {
        execute(
            &mut s,
            "param.edit",
            json!({"param": "angle", "value": v, "nodes": [a.0], "gesture": 4}),
        )
        .unwrap();
    }
    assert_eq!(s.doc.history().undo_labels().len(), before + 1);
    execute(&mut s, "key.autokey", json!({"enabled": true})).unwrap();
    execute(
        &mut s,
        "param.edit",
        json!({"param": "angle", "value": 30.0, "nodes": [a.0]}),
    )
    .unwrap();
    assert!(s.doc.scene().node(a).unwrap().tracks.contains_key("angle"));
}

#[test]
fn scrub_modifiers_scale_the_step() {
    let none = Mods::default();
    let shift = Mods {
        shift: true,
        ..none
    };
    let ctrl = Mods { ctrl: true, ..none };
    let both = Mods {
        shift: true,
        ctrl: true,
        alt: false,
    };
    let alt = Mods { alt: true, ..none };
    assert_eq!(scrub::factor(none), 1.0);
    assert_eq!(scrub::factor(shift), 10.0);
    assert_eq!(scrub::factor(ctrl), 10.0);
    assert_eq!(scrub::factor(both), 100.0);
    assert_eq!(scrub::factor(alt), 1.0);
    assert!((scrub::step(0.01, shift) - 0.1).abs() < 1e-15);
    assert!((scrub::step(0.01, ctrl) - 0.1).abs() < 1e-15);
    // egui divides by ten itself while Shift alone is held, so it gets the base speed
    assert_eq!(scrub::drag_speed(0.01, shift), 1.0);
    assert_eq!(scrub::drag_speed(0.01, ctrl), 0.1);
    assert_eq!(scrub::drag_speed(0.01, none), 0.01);
    assert!((scrub::scrubbed(1.0, 10.0, 0.01, none) - 1.1).abs() < 1e-12);
    assert!((scrub::scrubbed(1.0, 10.0, 0.01, ctrl) - 2.0).abs() < 1e-12);
    assert!((scrub::scrubbed(1.0, -10.0, 0.01, shift) - 0.0).abs() < 1e-12);
}

#[test]
fn base_step_follows_hint_kind_and_unit() {
    assert_eq!(scrub::base_step(Unit::None, true, None), 1.0);
    assert_eq!(scrub::base_step(Unit::None, false, None), 0.01);
    assert_eq!(scrub::base_step(Unit::Angle, false, None), 0.1);
    assert_eq!(scrub::base_step(Unit::Percent, false, None), 0.1);
    assert_eq!(scrub::base_step(Unit::Distance, false, Some(0.5)), 0.5);
    assert_eq!(scrub::base_step(Unit::Distance, false, Some(-1.0)), 0.01);
}

#[test]
fn user_data_rows_appear_in_the_attribute_panel() {
    let mut s = state();
    let a = make(&mut s, "am.thing");
    execute(
        &mut s,
        "user_data.add",
        json!({"node": a.0, "name": "Amount", "kind": "float", "default": 2.5}),
    )
    .unwrap();
    let view = attribute_rows_multi(s.doc.scene(), &[a]).unwrap();
    let row = view
        .groups
        .iter()
        .flat_map(|(_, rows)| rows.iter())
        .find(|r| r.param == "user_data.0")
        .unwrap();
    assert_eq!(row.label, "Amount");
    assert_eq!(row.value, Value::Float(2.5));
}

// ---- brief 233: chevrons, key indicator states, tab stacking -------------------------

use egui::{Key, Modifiers};
use forge_ui::attributes::TabSelection;
use forge_ui::key_indicator::KeyState;
use forge_ui::theme::{Theme, ThemeKind, contrast};
use forge_ui::widgets::number::{format_value, parse_value, step_value};

#[test]
fn a_chevron_steps_one_unit_and_stays_in_range() {
    // observed, G4 item 1: the right chevron took 200 cm to 201 cm, the left one back
    assert_eq!(scrub::chevron_step(Unit::Distance, false), 1.0);
    assert_eq!(scrub::chevron_step(Unit::Angle, false), 1.0);
    assert_eq!(scrub::chevron_step(Unit::Percent, false), 1.0);
    assert_eq!(scrub::chevron_step(Unit::None, true), 1.0);
    assert_eq!(scrub::chevron_step(Unit::None, false), 0.1);
    assert_eq!(step_value(200.0, 1, 1.0, None), 201.0);
    assert_eq!(step_value(201.0, -1, 1.0, None), 200.0);
    // repeated small steps carry no binary noise
    let mut v = 0.0;
    for _ in 0..3 {
        v = step_value(v, 1, 0.1, None);
    }
    assert_eq!(v, 0.3);
    // a step never leaves the range
    assert_eq!(step_value(50.0, 1, 1.0, Some((1.0, 50.0))), 50.0);
    assert_eq!(step_value(1.0, -1, 1.0, Some((1.0, 50.0))), 1.0);
    // typed text
    assert_eq!(format_value(201.0, false), "201");
    assert_eq!(format_value(0.5, false), "0.5");
    assert_eq!(format_value(-0.00001, false), "0");
    assert_eq!(format_value(7.4, true), "7");
}

/// A double click: a frame of the test harness lasts 0.75 s of frame clock, longer than the
/// default double-click delay, so the delay is raised to 1 s; earlier clicks of the test are
/// first left behind (a third click soon after two others would be a triple click).
fn double_click(h: &mut Harness<'_, Editor>, label: &str) {
    h.ctx
        .options_mut(|o| o.input_options.max_double_click_delay = 1.0);
    for _ in 0..8 {
        h.step();
    }
    h.get_by_label(label).click();
    h.step();
    h.get_by_label(label).click();
    h.step();
    h.run();
}

#[test]
fn chevrons_and_typing_edit_a_number_field() {
    let mut s = state();
    let a = make(&mut s, "am.thing");
    let mut h = harness(s);
    h.run();
    let before = undo_steps(&h);
    // the right chevron adds one, the left one takes it back; each is its own undo step
    h.get_by_label("Increase Count").click();
    h.run();
    assert_eq!(param(&h, a, "count"), Value::Int(4));
    assert_eq!(undo_steps(&h), before + 1);
    h.get_by_label("Decrease Count").click();
    h.run();
    assert_eq!(param(&h, a, "count"), Value::Int(3));
    // a float in display units: one degree
    h.get_by_label("Decrease Angle").click();
    h.run();
    let Value::Float(v) = param(&h, a, "angle") else {
        panic!()
    };
    assert!((v - (0.5 - 1f64.to_radians())).abs() < 1e-9, "{v}");
    // the range holds: Count is 1 .. 50
    h.get_by_label("Decrease Count").click();
    h.run();
    h.get_by_label("Decrease Count").click();
    h.run();
    h.get_by_label("Decrease Count").click();
    h.run();
    assert_eq!(param(&h, a, "count"), Value::Int(1));
    // a vector field has chevrons per component
    h.get_by_label("Increase Y Pos").click();
    h.run();
    // unit-less floats step by 0.1
    assert_eq!(param(&h, a, "pos"), Value::Vec3([1.0, 2.1, 3.0]));

    // double-click edits the text, Return commits
    double_click(&mut h, "Count");
    h.get_by_label("Edit Count");
    h.key_press_modifiers(Modifiers::COMMAND, Key::A);
    h.get_by_label("Edit Count").type_text("8");
    h.run();
    assert_eq!(
        param(&h, a, "count"),
        Value::Int(1),
        "typing alone does not commit"
    );
    h.key_press(Key::Enter);
    h.run();
    assert_eq!(param(&h, a, "count"), Value::Int(8));
    assert!(
        h.query_by_label("Edit Count").is_none(),
        "the editor closed"
    );
    // Escape cancels an edit
    double_click(&mut h, "Count");
    h.get_by_label("Edit Count").type_text("9");
    h.key_press(Key::Escape);
    h.run();
    assert_eq!(param(&h, a, "count"), Value::Int(8));
    assert!(h.query_by_label("Edit Count").is_none());
}

#[test]
fn typed_text_is_read_without_its_unit() {
    use forge_ui::widgets::number::NumberField;
    let f = NumberField {
        base: 0.1,
        prefix: "X ".into(),
        suffix: "°",
        mixed: false,
        label: egui::Id::NULL,
        name: "Rot".into(),
        step: 1.0,
        range: None,
        is_int: false,
    };
    assert_eq!(parse_value("12.5", &f), Some(12.5));
    assert_eq!(parse_value(" X 12.5° ", &f), Some(12.5));
    assert_eq!(parse_value("12,5", &f), Some(12.5));
    assert_eq!(parse_value("abc", &f), None);
    assert_eq!(parse_value("", &f), None);
    assert_eq!(parse_value("inf", &f), None);
}

/// The hue in degrees of an sRGB colour (HSV on the bytes).
fn hue(c: egui::Color32) -> f32 {
    let [r, g, b] = [c.r(), c.g(), c.b()].map(|v| f32::from(v) / 255.0);
    let (max, min) = (r.max(g).max(b), r.min(g).min(b));
    let d = max - min;
    if d == 0.0 {
        return 0.0;
    }
    let h = if max == r {
        ((g - b) / d).rem_euclid(6.0)
    } else if max == g {
        (b - r) / d + 2.0
    } else {
        (r - g) / d + 4.0
    };
    h * 60.0
}

#[test]
fn the_key_diamond_has_four_distinct_states() {
    // observed, G4 item 2: hollow grey / solid key colour / solid modified colour / hollow
    // key colour
    assert_eq!(KeyState::of(false, false, false), KeyState::Unkeyed);
    assert_eq!(KeyState::of(false, true, true), KeyState::Keyed);
    assert_eq!(KeyState::of(false, false, true), KeyState::Animated);
    // an unkeyed draft shows as modified on a keyed frame and on a frame between keys
    assert_eq!(KeyState::of(true, true, true), KeyState::Modified);
    assert_eq!(KeyState::of(true, false, true), KeyState::Modified);
    let words: Vec<&str> = [
        KeyState::Unkeyed,
        KeyState::Keyed,
        KeyState::Modified,
        KeyState::Animated,
    ]
    .iter()
    .map(|s| s.word())
    .collect();
    assert_eq!(words, ["unkeyed", "keyed", "changed", "animated"]);
    for kind in ThemeKind::ALL {
        let t = Theme::of(kind);
        let clear = egui::Color32::TRANSPARENT;
        let (un, un_fill) = KeyState::Unkeyed.look(&t);
        let (kd, kd_fill) = KeyState::Keyed.look(&t);
        let (md, md_fill) = KeyState::Modified.look(&t);
        let (an, an_fill) = KeyState::Animated.look(&t);
        // hollow: no fill; solid: filled with the outline colour
        assert_eq!((un_fill, an_fill), (clear, clear), "{kind:?}");
        assert_eq!((kd_fill, md_fill), (kd, md), "{kind:?}");
        // grey has no hue worth naming; the others are the key colour and the modified colour
        assert_eq!(un, t.weak_text, "{kind:?}");
        assert_eq!((kd, an), (t.key, t.key), "{kind:?}");
        assert_eq!(md, t.modified, "{kind:?}");
        // the key colour and the modified colour are distinct hues, and both readable
        let mut d = (hue(t.key) - hue(t.modified)).abs();
        d = d.min(360.0 - d);
        assert!(
            d >= 30.0,
            "{kind:?}: hues {} / {}",
            hue(t.key),
            hue(t.modified)
        );
        for c in [t.key, t.modified] {
            assert!(contrast(c, t.panel) >= 3.0, "{kind:?}: {c:?}");
        }
        // the four are told apart: solid key vs solid modified differ, hollow key vs hollow grey
        assert_ne!(kd, md);
        assert_ne!(un, an);
    }
}

#[test]
fn shift_click_stacks_two_tab_bodies() {
    let tabs: Vec<String> = ["Basic", "Extra", "Third"].map(String::from).to_vec();
    let mut t = TabSelection::default();
    assert_eq!(t.shown(&tabs), ["Basic"], "the first tab shows by default");
    t.click("Extra", false, &tabs);
    assert_eq!(t.shown(&tabs), ["Extra"]);
    // Shift-click adds a body below, the first-active one stays on top
    t.click("Basic", true, &tabs);
    assert_eq!(t.shown(&tabs), ["Extra", "Basic"]);
    t.click("Third", true, &tabs);
    assert_eq!(t.shown(&tabs), ["Extra", "Basic", "Third"]);
    // a second Shift-click removes it; removing the top one promotes the next
    t.click("Basic", true, &tabs);
    assert_eq!(t.shown(&tabs), ["Extra", "Third"]);
    t.click("Extra", true, &tabs);
    assert_eq!(t.shown(&tabs), ["Third"]);
    t.click("Third", true, &tabs);
    assert_eq!(t.shown(&tabs), ["Third"], "the last tab stays");
    // a plain click shows just that tab
    t.click("Basic", true, &tabs);
    t.click("Extra", false, &tabs);
    assert_eq!(t.shown(&tabs), ["Extra"]);
    // tabs that no longer exist drop out
    t.click("Basic", true, &tabs);
    let fewer: Vec<String> = vec!["Basic".into()];
    assert_eq!(t.shown(&fewer), ["Basic"]);
    assert_eq!(t.shown(&[]), Vec::<String>::new());
}

#[test]
fn shift_clicking_a_second_tab_shows_both_bodies_in_one_scroll_area() {
    let mut s = state();
    make(&mut s, "am.thing");
    let mut h = harness(s);
    h.run();
    assert!(h.query_by_label("Note").is_none());
    // Shift-click the Extra tab with Basic active: both bodies, Basic on top
    h.get_by_label("Extra").click_modifiers(Modifiers::SHIFT);
    h.run();
    let angle = h.get_by_label("Angle").rect();
    let note = h.get_by_label("Note").rect();
    assert!(angle.center().y < note.center().y, "Basic above Extra");
    // a plain click on a tab shows only that tab again
    tab(&mut h, "Extra");
    assert!(h.query_by_label("Note").is_some());
    assert!(h.query_by_label("Angle").is_none());
    // Shift-click Basic while Extra is active: Extra stays on top
    h.get_by_label("Basic").click_modifiers(Modifiers::SHIFT);
    h.run();
    let angle = h.get_by_label("Angle").rect();
    let note = h.get_by_label("Note").rect();
    assert!(
        note.center().y < angle.center().y,
        "the first-active tab is on top"
    );
}
