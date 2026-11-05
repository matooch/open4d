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
    // the three tools of briefs 020/021, then the mesh commands of brief 022 (MODELING.md §4)
    assert_eq!(
        ids,
        [
            "extrude",
            "extrude_inner",
            "bevel",
            "subdivide",
            "triangulate",
            "untriangulate",
            "reverse_normals",
            "optimize",
            "weld",
            "melt",
            "dissolve",
            "split",
            "disconnect",
            "connect_points",
            "mirror",
            "center_axis"
        ]
    );
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

/// An editable cube with the top-front edge (points 1 and 3) selected, stored as edge ids.
fn setup_edge() -> (EditorState, NodeId) {
    let (mut st, id) = setup();
    let edge = match st.doc.scene().param(id, "mesh") {
        Some(Value::Mesh(m)) => forge_generators::to_poly_mesh(m)
            .edge_ids()
            .get(&forge_geometry::Edge::new(1, 3))
            .copied()
            .unwrap(),
        _ => panic!("mesh"),
    };
    st.doc
        .transact("sel", |tx| {
            tx.set_param(id, "selection.polys", Value::IndexList(Vec::new()))?;
            tx.set_param(id, "selection.edges", Value::IndexList(vec![edge]))
        })
        .unwrap();
    (st, id)
}

#[test]
fn bevel_previews_the_stored_edges_and_applies_in_one_step() {
    let (mut st, id) = setup_edge();
    let before = steps(&st);
    assert!(activate_tool(&mut st, "bevel"));
    let ts = st.tool.as_mut().unwrap();
    assert_eq!(ts.values().float("offset"), 10.0);
    assert_eq!(ts.values().choice("bevel_mode"), "chamfer");
    ts.set("offset", Value::Float(20.0)).unwrap();
    ts.set("subdivision", Value::Int(2)).unwrap();
    ts.set("offset_mode", Value::Enum("proportional".into()))
        .unwrap();
    assert!(
        ts.set("offset_mode", Value::Enum("sideways".into()))
            .is_err()
    );
    ts.set("offset_mode", Value::Enum("fixed_distance".into()))
        .unwrap();
    let (_, preview) = tool_preview(&st).expect("preview");
    assert_eq!((preview.points.len(), preview.polys.len()), (14, 9));
    assert_eq!(counts(&st, id), (8, 6), "the document is untouched");
    // a drag edits the offset and applies as one command
    let drag = ToolDrag::begin(&st, 400.0, 800.0).expect("a target");
    drag.update(&mut st, 380.0);
    drag.finish(&mut st).unwrap();
    assert_eq!(counts(&st, id), (14, 9));
    assert_eq!(steps(&st), before + 1);
    // an unsupported option leaves no preview and no change
    st.tool
        .as_mut()
        .unwrap()
        .set("shape", Value::Enum("user".into()))
        .unwrap();
    assert!(tool_preview(&st).is_none());
    assert!(apply_tool(&mut st).is_err());
    assert_eq!(steps(&st), before + 1);
}

#[test]
fn the_bevel_panel_has_enum_rows_and_the_menu_entry() {
    let (st, id) = setup_edge();
    let before = steps(&st);
    let mut h = harness(st);
    h.get_by_label("Mesh").click();
    h.run();
    h.get_by_label("Bevel").click();
    h.run();
    h.get_by_label("Bevel Mode");
    h.get_by_label("Offset Mode");
    h.get_by_label("Subdivision");
    h.get_by_label("Apply").click();
    h.run();
    assert_eq!(counts(&h.state().state, id), (10, 7));
    assert_eq!(steps(&h.state().state), before + 1);
}

#[test]
fn mesh_commands_apply_without_a_drag() {
    let (mut st, id) = setup();
    let before = steps(&st);
    // Reverse Normals has no setting to drag: a drag does not start, Apply runs the command
    assert!(activate_tool(&mut st, "reverse_normals"));
    assert!(ToolDrag::begin(&st, 400.0, 800.0).is_none());
    assert_eq!(
        st.tool.as_ref().unwrap().tool().settings().len(),
        0,
        "no settings"
    );
    let (_, preview) = tool_preview(&st).expect("preview of the selected polygon");
    assert_eq!(preview.polys[4], vec![1, 3, 5, 7]);
    apply_tool(&mut st).unwrap();
    assert_eq!(steps(&st), before + 1);
    // Mirror: vector settings are rows, Subdivide applies to the whole mesh
    assert!(activate_tool(&mut st, "mirror"));
    let ts = st.tool.as_ref().unwrap();
    let rows = tool_rows(ts.tool().settings(), ts.values());
    let normal = rows.iter().find(|r| r.param == "plane_normal").unwrap();
    assert_eq!(normal.value, Value::Vec3([1.0, 0.0, 0.0]));
    // Hyper Subdivide works on the whole mesh: a part of the polygons is refused, none is all
    assert!(activate_tool(&mut st, "subdivide"));
    st.tool
        .as_mut()
        .unwrap()
        .set("hyper", Value::Bool(true))
        .unwrap();
    assert!(apply_tool(&mut st).is_err());
    st.doc
        .transact("sel", |tx| {
            tx.set_param(id, "selection.polys", Value::IndexList(vec![]))
        })
        .unwrap();
    apply_tool(&mut st).unwrap();
    assert_eq!(counts(&st, id), (26, 24));
    execute(&mut st, "edit.undo", json!({})).unwrap();
    assert_eq!(counts(&st, id), (8, 6));
}

#[test]
fn the_mesh_menu_lists_the_commands_and_their_panels_show() {
    let (st, id) = setup();
    let before = steps(&st);
    let mut h = harness(st);
    h.get_by_label("Mesh").click();
    h.run();
    h.get_by_label("Weld");
    h.get_by_label("Untriangulate");
    h.get_by_label("Mirror").click();
    h.run();
    assert!(h.query_all_by_label("Plane Normal").count() > 0);
    h.get_by_label("Duplicate");
    h.get_by_label("Apply").click();
    h.run();
    assert_eq!(steps(&h.state().state), before + 1);
    assert_eq!(counts(&h.state().state, id), (8, 6));
}
