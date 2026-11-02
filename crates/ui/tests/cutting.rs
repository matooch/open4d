//! docs/specs/MODELING.md §5.5: the cutting tools in the editor, without a GPU.

mod common;

use serde_json::json;

use common::{create, state};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use forge_core::{NodeId, Value};
use forge_ui::app::Editor;
use forge_ui::commands::execute;
use forge_ui::cutting::{
    apply_cut, click, cut_preview, drag_begin, drag_finish, drag_update, hover, hud,
};
use forge_ui::state::EditorState;
use forge_ui::tools::{ToolDrag, activate_tool, apply_tool, cutting_tools, tool_preview};

const W: f64 = 800.0;
const H: f64 = 600.0;

fn setup() -> (EditorState, NodeId) {
    let mut st = state();
    create(&mut st, "forge.cube");
    execute(&mut st, "mesh.make_editable", json!({})).unwrap();
    let id = st.selection.active.unwrap();
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

/// Moves the pointer over each edge midpoint until the tool finds an edge to hover.
fn hover_some_edge(st: &mut EditorState, id: NodeId) {
    let mesh = match st.doc.scene().param(id, "mesh") {
        Some(Value::Mesh(m)) => forge_generators::to_poly_mesh(m),
        _ => panic!("mesh"),
    };
    for e in mesh.edges() {
        let mid = (mesh.points[e.0 as usize] + mesh.points[e.1 as usize]) * 0.5;
        if let Some(p) = st.camera.project(mid, W, H) {
            hover(st, p.x, p.y, W, H);
            if st.tool.as_ref().unwrap().interaction.hover.is_some() {
                return;
            }
        }
    }
    panic!("no edge could be hovered");
}

#[test]
fn the_cutting_tools_are_listed() {
    let ids: Vec<_> = cutting_tools().iter().map(|t| t.id()).collect();
    assert_eq!(
        ids,
        [
            "loop_cut",
            "knife_line",
            "knife_plane",
            "knife_loop",
            "slide"
        ]
    );
    let (mut st, _) = setup();
    for id in ids {
        assert!(activate_tool(&mut st, id), "{id}");
    }
}

#[test]
fn hover_previews_without_touching_the_document() {
    let (mut st, id) = setup();
    let before = steps(&st);
    assert!(activate_tool(&mut st, "loop_cut"));
    assert!(cut_preview(&st).is_none(), "no hover, no preview");
    assert_eq!(hud(&st).unwrap(), "Loop/Path Cut: hover an edge");
    hover_some_edge(&mut st, id);
    let (node, preview) = cut_preview(&st).expect("preview");
    assert_eq!(node, id);
    assert_eq!((preview.points.len(), preview.polys.len()), (12, 10));
    // the same through the generic preview used for drawing
    assert_eq!(tool_preview(&st).unwrap().1, preview);
    assert_eq!(counts(&st, id), (8, 6));
    assert_eq!(steps(&st), before);
    assert!(
        hud(&st)
            .unwrap()
            .starts_with("Loop/Path Cut: 1 cut, offset ")
    );
    // moving off the mesh clears it
    hover(&mut st, 2.0, 2.0, W, H);
    assert!(cut_preview(&st).is_none());
}

#[test]
fn click_commits_one_undo_step() {
    let (mut st, id) = setup();
    let before = steps(&st);
    activate_tool(&mut st, "loop_cut");
    assert!(click(&mut st).is_err(), "nothing hovered");
    hover_some_edge(&mut st, id);
    let out = click(&mut st).unwrap();
    assert_eq!(out["polys"], 10);
    assert_eq!(counts(&st, id), (12, 10));
    assert_eq!(steps(&st), before + 1);
    // the tool stays active
    assert!(st.tool.is_some());
    execute(&mut st, "edit.undo", json!({})).unwrap();
    assert_eq!(counts(&st, id), (8, 6));
    assert_eq!(steps(&st), before);
}

#[test]
fn count_and_fixed_offset_settings() {
    let (mut st, id) = setup();
    activate_tool(&mut st, "loop_cut");
    st.tool
        .as_mut()
        .unwrap()
        .set("count", Value::Int(3))
        .unwrap();
    hover_some_edge(&mut st, id);
    assert!(
        hud(&st)
            .unwrap()
            .contains("3 cuts, offset 0.25, 0.50, 0.75")
    );
    click(&mut st).unwrap();
    assert_eq!(counts(&st, id), (20, 18));
    execute(&mut st, "edit.undo", json!({})).unwrap();
    // one cut at a fixed offset (the edge only chooses the ring)
    let ts = st.tool.as_mut().unwrap();
    ts.set("count", Value::Int(1)).unwrap();
    ts.set("lock_offset", Value::Bool(true)).unwrap();
    ts.set("offset", Value::Float(0.25)).unwrap();
    hover_some_edge(&mut st, id);
    click(&mut st).unwrap();
    assert_eq!(counts(&st, id), (12, 10));
}

#[test]
fn knife_loop_cuts_once_at_the_hover() {
    let (mut st, id) = setup();
    activate_tool(&mut st, "knife_loop");
    hover_some_edge(&mut st, id);
    click(&mut st).unwrap();
    assert_eq!(counts(&st, id), (12, 10));
}

#[test]
fn knife_plane_applies_the_panel_numbers() {
    let (mut st, id) = setup();
    let before = steps(&st);
    activate_tool(&mut st, "knife_plane");
    // the plane x = 0 is previewed from the panel values
    assert_eq!(cut_preview(&st).unwrap().1.polys.len(), 10);
    assert_eq!(steps(&st), before);
    st.tool
        .as_mut()
        .unwrap()
        .set("plane_normal", Value::Vec3([0.0, 1.0, 0.0]))
        .unwrap();
    apply_tool(&mut st).unwrap();
    assert_eq!(counts(&st, id), (12, 10));
    assert_eq!(steps(&st), before + 1);
    // a plane that misses the mesh is an error and changes nothing
    st.tool
        .as_mut()
        .unwrap()
        .set("plane_point", Value::Vec3([0.0, 9999.0, 0.0]))
        .unwrap();
    assert!(apply_cut(&mut st).is_err());
    assert_eq!(steps(&st), before + 1);
}

#[test]
fn knife_drag_previews_then_cuts_on_release() {
    let (mut st, id) = setup();
    let before = steps(&st);
    activate_tool(&mut st, "knife_line");
    let c = st.camera.project(glam::DVec3::ZERO, W, H).unwrap();
    // a vertical line through the cube's centre on screen
    drag_begin(&mut st, c.x, c.y - 250.0, [W, H]);
    drag_update(&mut st, c.x, c.y + 250.0);
    let (_, preview) = cut_preview(&st).expect("preview while dragging");
    assert!(preview.polys.len() > 6);
    assert_eq!(counts(&st, id), (8, 6));
    assert_eq!(steps(&st), before);
    let out = drag_finish(&mut st, c.x, c.y + 250.0).unwrap().unwrap();
    assert_eq!(out["polys"].as_u64().unwrap() as usize, preview.polys.len());
    assert_eq!(steps(&st), before + 1);
    execute(&mut st, "edit.undo", json!({})).unwrap();
    assert_eq!(counts(&st, id), (8, 6));
    // a click without a drag does nothing
    drag_begin(&mut st, c.x, c.y, [W, H]);
    assert!(drag_finish(&mut st, c.x + 1.0, c.y).unwrap().is_none());
    assert_eq!(steps(&st), before);
    // the plane tool takes the same drag
    activate_tool(&mut st, "knife_plane");
    drag_begin(&mut st, c.x, c.y - 250.0, [W, H]);
    let out = drag_finish(&mut st, c.x, c.y + 250.0).unwrap().unwrap();
    assert!(out["polys"].as_u64().unwrap() > 6);
}

#[test]
fn slide_drag_sets_the_amount_and_applies() {
    let (mut st, id) = setup();
    // the top-front edge (points 1 and 3), as a stored edge selection
    let mesh = match st.doc.scene().param(id, "mesh") {
        Some(Value::Mesh(m)) => forge_generators::to_poly_mesh(m),
        _ => panic!("mesh"),
    };
    let e = mesh.edge_ids()[&forge_geometry::Edge::new(1, 3)];
    st.doc
        .transact("sel", |tx| {
            tx.set_param(id, "selection.edges", Value::IndexList(vec![e]))
        })
        .unwrap();
    let before = steps(&st);
    activate_tool(&mut st, "slide");
    let drag = ToolDrag::begin(&st, 400.0, 600.0).expect("a target");
    drag.update(&mut st, 300.0);
    // 100 of 600 pixels is a third of the 2.0 range
    let got = st.tool.as_ref().unwrap().values().float("amount");
    assert!((got - 100.0 / 600.0 * 2.0).abs() < 1e-9, "{got}");
    let (_, preview) = tool_preview(&st).expect("preview");
    assert_ne!(preview.points, mesh.points);
    assert_eq!(steps(&st), before);
    drag.finish(&mut st).unwrap();
    assert_eq!(steps(&st), before + 1);
    assert_eq!(counts(&st, id), (8, 6));
}

#[test]
fn the_mesh_menu_has_the_cutting_tools() {
    let (st, id) = setup();
    let before = steps(&st);
    let mut h = Harness::builder()
        .with_size([1280.0, 800.0])
        .build_ui_state(|ui, e: &mut Editor| e.show(ui, None), Editor::new(st));
    h.run();
    h.get_by_label("Mesh").click();
    h.run();
    h.get_by_label("Knife Line");
    h.get_by_label("Knife Loop");
    h.get_by_label("Slide");
    h.get_by_label("Loop/Path Cut");
    h.get_by_label("Knife Plane").click();
    h.run();
    assert!(h.query_all_by_label("Plane Normal").count() > 0);
    h.get_by_label("Apply").click();
    h.run();
    assert_eq!(counts(&h.state().state, id), (12, 10));
    assert_eq!(steps(&h.state().state), before + 1);
    h.get_by_label("Done").click();
    h.run();
    assert!(h.state().state.tool.is_none());
}
