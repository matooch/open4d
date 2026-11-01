//! docs/specs/MODELING.md §0.2: the tool framework in the editor, without a GPU.

mod common;

use serde_json::json;

use common::{create, state};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use forge_core::{NodeId, Value};
use forge_ui::app::Editor;
use forge_ui::attributes::tool_rows;
use forge_ui::commands::execute;
use forge_ui::state::EditorState;
use forge_ui::tools::{
    ToolDrag, activate_tool, apply_tool, deactivate_tool, drag_distance, tool_preview, tools,
};

/// A made-editable default cube whose top polygon (4) is selected.
fn setup() -> (EditorState, NodeId) {
    let mut st = state();
    create(&mut st, "forge.cube");
    execute(&mut st, "mesh.make_editable", json!({})).unwrap();
    let id = st.selection.active.unwrap();
    st.doc
        .transact("sel", |tx| {
            tx.set_param(id, "selection.polys", Value::IndexList(vec![4]))
        })
        .unwrap();
    (st, id)
}

fn counts(st: &EditorState, id: NodeId) -> (usize, usize) {
    match st.doc.scene().param(id, "mesh") {
        Some(Value::Mesh(m)) => (m.points.len(), m.polys.len()),
        _ => panic!("mesh"),
    }
}

fn steps(st: &EditorState) -> usize {
    st.doc.history().undo_labels().len()
}

#[test]
fn tools_come_from_descriptors() {
    let all = tools();
    let ids: Vec<_> = all.iter().map(|t| t.id()).collect();
    assert_eq!(ids, ["extrude", "extrude_inner"]);
    let (mut st, _) = setup();
    assert!(!activate_tool(&mut st, "nope"));
    assert!(activate_tool(&mut st, "extrude"));
    let ts = st.tool.as_ref().unwrap();
    assert_eq!(ts.tool().command_id(), "mesh.extrude");
    let rows = tool_rows(ts.tool().settings(), ts.values());
    let offset = rows.iter().find(|r| r.param == "offset").unwrap();
    assert_eq!(offset.label, "Offset");
    assert_eq!(offset.value, Value::Float(5.0));
    assert!(rows.iter().any(|r| r.param == "preserve_groups"));
    deactivate_tool(&mut st);
    assert!(st.tool.is_none());
}

#[test]
fn drag_previews_then_applies_in_one_undo_step() {
    let (mut st, id) = setup();
    let before = steps(&st);
    assert!(activate_tool(&mut st, "extrude"));
    // the pointer starts at row 400 of an 800 pixel viewport and moves up by 200 pixels
    let drag = ToolDrag::begin(&st, 400.0, 800.0).expect("a target");
    for y in [380.0, 340.0, 300.0, 200.0] {
        drag.update(&mut st, y);
        // the document is untouched while dragging
        assert_eq!(counts(&st, id), (8, 6));
        assert_eq!(steps(&st), before);
        let (node, preview) = tool_preview(&st).expect("preview");
        assert_eq!(node, id);
        assert_eq!(preview.points.len(), 12);
    }
    // 200 px of 800 px is a quarter of the cube's diagonal (200 * sqrt 3), added to offset 5
    let want = 5.0 + drag_distance(200.0, 200.0 * 3f64.sqrt(), 800.0);
    let got = st.tool.as_ref().unwrap().values().float("offset");
    assert!((got - want).abs() < 1e-9, "{got} != {want}");
    let out = drag.finish(&mut st).unwrap();
    assert_eq!(out["polys"], 10);
    assert_eq!(counts(&st, id), (12, 10));
    assert_eq!(steps(&st), before + 1, "one undo step for the whole drag");
    // the tool stays active, the new cap polygon is the selection
    assert!(st.tool.is_some());
    execute(&mut st, "edit.undo", json!({})).unwrap();
    assert_eq!(counts(&st, id), (8, 6));
    assert_eq!(steps(&st), before);
}

#[test]
fn reset_values_and_apply_use_the_settings() {
    let (mut st, id) = setup();
    activate_tool(&mut st, "extrude");
    let ts = st.tool.as_mut().unwrap();
    ts.set("offset", Value::Float(50.0)).unwrap();
    ts.set("subdivision", Value::Int(2)).unwrap();
    ts.set("maximum_angle_deg", Value::Float(999.0)).unwrap();
    assert_eq!(ts.values().float("maximum_angle_deg"), 180.0, "clamped");
    assert!(ts.set("nope", Value::Float(1.0)).is_err());
    ts.reset();
    assert_eq!(ts.values().float("offset"), 5.0);
    assert_eq!(ts.values().int("subdivision"), 0);
    let ts = st.tool.as_mut().unwrap();
    ts.set("offset", Value::Float(50.0)).unwrap();
    ts.set("subdivision", Value::Int(2)).unwrap();
    apply_tool(&mut st).unwrap();
    assert_eq!(counts(&st, id), (20, 18), "extrude-subdiv numbers");
    // an inset applies through its own command
    activate_tool(&mut st, "extrude_inner");
    apply_tool(&mut st).unwrap();
    assert_eq!(counts(&st, id), (24, 22));
}

#[test]
fn no_target_or_tool_is_an_error_not_a_change() {
    let mut st = state();
    assert!(apply_tool(&mut st).is_err());
    create(&mut st, "forge.cube");
    activate_tool(&mut st, "extrude");
    let before = steps(&st);
    assert!(ToolDrag::begin(&st, 0.0, 100.0).is_none());
    assert!(
        apply_tool(&mut st).is_err(),
        "a cube is not a polygon object"
    );
    assert_eq!(steps(&st), before);
}

fn harness(st: EditorState) -> Harness<'static, Editor> {
    let mut h = Harness::builder()
        .with_size([1280.0, 800.0])
        .build_ui_state(|ui, e: &mut Editor| e.show(ui, None), Editor::new(st));
    h.run();
    h
}

#[test]
fn the_attribute_panel_shows_the_tool_and_apply_is_one_step() {
    let (st, id) = setup();
    let before = steps(&st);
    let mut h = harness(st);
    h.get_by_label("Mesh").click();
    h.run();
    h.get_by_label("Extrude").click();
    h.run();
    assert!(h.state().state.tool.is_some());
    // the panel lists the tool's settings and its buttons
    h.get_by_label("Offset");
    h.get_by_label("Preserve Groups");
    h.get_by_label("Reset Values");
    h.get_by_label("Apply").click();
    h.run();
    assert_eq!(counts(&h.state().state, id), (12, 10));
    assert_eq!(steps(&h.state().state), before + 1);
    h.get_by_label("Done").click();
    h.run();
    assert!(h.state().state.tool.is_none());
}
