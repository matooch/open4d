//! docs/specs/UI.md §4–5: attribute panel and commands.

mod common;

use common::{create, state};
use forge_core::{Category, NodeId, ParamKind, Unit, Value};
use forge_ui::attributes::{Gesture, Widget, apply_edit, attribute_rows, from_display, to_display};
use forge_ui::commands::{Command, create_menu, execute};

#[test]
fn rows_follow_descriptors_and_groups() {
    let mut st = state();
    let b = create(&mut st, "test.box");
    let view = attribute_rows(st.doc.scene(), b).unwrap();
    assert_eq!(view.name, "Box");
    assert_eq!(view.type_label, "Box");
    assert!(view.editable);
    let groups: Vec<&str> = view.groups.iter().map(|(g, _)| g.as_str()).collect();
    assert_eq!(groups, ["Coordinates", "Object", "Shape", "Parameters"]);
    let labels = |g: usize| {
        view.groups[g]
            .1
            .iter()
            .map(|r| r.label.as_str())
            .collect::<Vec<_>>()
    };
    assert_eq!(labels(0), ["Position", "Rotation", "Scale"]);
    assert_eq!(labels(1), ["Size", "Twist", "Segments"]);
    assert_eq!(labels(2), ["Ratio", "Enabled"]);
    assert_eq!(
        labels(3),
        ["Mode", "Target", "Offset", "Tint", "Note"],
        "hidden param omitted"
    );

    let row = |p: &str| {
        view.groups
            .iter()
            .flat_map(|(_, r)| r)
            .find(|r| r.param == p)
            .unwrap()
    };
    assert_eq!(row("enabled").widget, Widget::Checkbox);
    assert_eq!(row("segments").widget, Widget::IntDrag { min: 1, max: 100 });
    assert_eq!(
        row("size").widget,
        Widget::FloatDrag {
            min: 0.0,
            max: 1e6,
            unit: Unit::Distance
        }
    );
    assert!(matches!(row("offset").widget, Widget::Vec3Drag { .. }));
    assert_eq!(row("tint").widget, Widget::Color);
    let Widget::Combo { options } = &row("mode").widget else {
        panic!()
    };
    assert_eq!(options.len(), 2);
    assert_eq!(
        row("target").widget,
        Widget::Link {
            accepts: vec![Category::Object]
        }
    );
    assert_eq!(row("note").widget, Widget::Text);
    assert_eq!(row("size").value, Value::Float(2.0));
    assert!(attribute_rows(st.doc.scene(), NodeId(999)).is_none());
}

#[test]
fn display_units_round_trip() {
    let v = 0.5;
    assert!((to_display(Unit::Angle, v) - 0.5f64.to_degrees()).abs() < 1e-12);
    assert_eq!(to_display(Unit::Percent, 0.25), 25.0);
    assert_eq!(to_display(Unit::Distance, 3.0), 3.0);
    for unit in [
        Unit::None,
        Unit::Distance,
        Unit::Angle,
        Unit::Percent,
        Unit::Time,
    ] {
        for x in [-3.5, 0.0, 0.123, 720.0] {
            assert!(
                (from_display(unit, to_display(unit, x)) - x).abs() < 1e-12,
                "{unit:?}"
            );
        }
    }
    let _ = ParamKind::Bool;
}

#[test]
fn drag_is_one_undo_step_commit_is_another() {
    let mut st = state();
    let b = create(&mut st, "test.box");
    for v in [3.0, 4.0, 5.0] {
        apply_edit(&mut st.doc, b, "size", Value::Float(v), Gesture::Drag(1)).unwrap();
    }
    apply_edit(&mut st.doc, b, "segments", Value::Int(7), Gesture::Commit).unwrap();
    let labels = st.doc.history().undo_labels();
    assert_eq!(labels[labels.len() - 2..], ["Set Size", "Set Segments"]);
    execute(&mut st, Command::Undo).unwrap();
    execute(&mut st, Command::Undo).unwrap();
    assert_eq!(st.doc.scene().param(b, "size"), Some(&Value::Float(2.0)));

    // a new drag gesture is a separate step
    apply_edit(&mut st.doc, b, "size", Value::Float(3.0), Gesture::Drag(2)).unwrap();
    apply_edit(&mut st.doc, b, "size", Value::Float(4.0), Gesture::Drag(3)).unwrap();
    let n = st.doc.history().undo_labels().len();
    assert_eq!(
        st.doc.history().undo_labels()[n - 2..],
        ["Set Size", "Set Size"]
    );
}

#[test]
fn invalid_edits_change_nothing() {
    let mut st = state();
    let b = create(&mut st, "test.box");
    let before = st.doc.history().undo_labels().len();
    assert!(
        apply_edit(
            &mut st.doc,
            b,
            "mode",
            Value::Enum("nope".into()),
            Gesture::Commit
        )
        .is_err()
    );
    assert!(apply_edit(&mut st.doc, b, "size", Value::Bool(true), Gesture::Commit).is_err());
    assert_eq!(st.doc.history().undo_labels().len(), before);
}

#[test]
fn create_menu_lists_placeable_types() {
    let st = state();
    let menu = create_menu(st.doc.scene().registry());
    let cats: Vec<Category> = menu.iter().map(|(c, _)| *c).collect();
    assert_eq!(
        cats,
        [
            Category::Object,
            Category::Generator,
            Category::Deformer,
            Category::Tag
        ]
    );
    assert_eq!(menu[0].1, [("forge.null".to_string(), "Null".to_string())]);
}

#[test]
fn create_tag_needs_an_active_node() {
    let mut st = state();
    assert!(execute(&mut st, Command::Create("test.sel".into())).is_err());
    let b = create(&mut st, "test.box");
    execute(&mut st, Command::Create("test.sel".into())).unwrap();
    assert_eq!(st.doc.scene().node(b).unwrap().tags.len(), 1);
    assert_eq!(
        st.selection.active,
        Some(b),
        "tag creation keeps the host selected"
    );
}

#[test]
fn delete_selected_skips_nested_and_is_one_step() {
    let mut st = state();
    let a = create(&mut st, "forge.null");
    let b = create(&mut st, "forge.null");
    let c = create(&mut st, "forge.null");
    execute(&mut st, Command::Reparent(b, Some(a))).unwrap();
    st.selection.click(a);
    st.selection.toggle(b);
    st.selection.toggle(c);
    let steps = st.doc.history().undo_labels().len();
    execute(&mut st, Command::DeleteSelected).unwrap();
    assert!(st.doc.scene().is_empty());
    assert_eq!(st.doc.history().undo_labels().len(), steps + 1);
    execute(&mut st, Command::Undo).unwrap();
    assert_eq!(st.doc.scene().len(), 3);
}

#[test]
fn reparent_cycles_are_reported() {
    let mut st = state();
    let a = create(&mut st, "forge.null");
    let b = create(&mut st, "forge.null");
    execute(&mut st, Command::Reparent(b, Some(a))).unwrap();
    assert!(execute(&mut st, Command::Reparent(a, Some(b))).is_err());
    execute(&mut st, Command::Reparent(b, None)).unwrap();
    assert_eq!(st.doc.scene().roots(), [a, b]);
    execute(&mut st, Command::Rename(a, "Rig".into())).unwrap();
    assert_eq!(st.doc.scene().node(a).unwrap().name, "Rig");
    execute(&mut st, Command::ToggleExpanded(a)).unwrap();
    assert!(st.expanded.contains(&a));
}
