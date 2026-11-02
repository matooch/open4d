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
    assert_eq!(view.tabs(), vec!["Basic", "Extra"]);
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
    assert_eq!(scrub::factor(shift), 0.1);
    assert_eq!(scrub::factor(ctrl), 10.0);
    assert_eq!(scrub::factor(both), 1.0);
    assert_eq!(scrub::factor(alt), 1.0);
    assert!((scrub::step(0.01, shift) - 0.001).abs() < 1e-15);
    assert!((scrub::step(0.01, ctrl) - 0.1).abs() < 1e-15);
    // egui divides by ten itself while Shift alone is held, so it gets the base speed
    assert_eq!(scrub::drag_speed(0.01, shift), 0.01);
    assert_eq!(scrub::drag_speed(0.01, ctrl), 0.1);
    assert_eq!(scrub::drag_speed(0.01, none), 0.01);
    assert!((scrub::scrubbed(1.0, 10.0, 0.01, none) - 1.1).abs() < 1e-12);
    assert!((scrub::scrubbed(1.0, 10.0, 0.01, ctrl) - 2.0).abs() < 1e-12);
    assert!((scrub::scrubbed(1.0, -10.0, 0.01, shift) - 0.99).abs() < 1e-12);
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
