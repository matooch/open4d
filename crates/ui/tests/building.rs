//! docs/specs/MODELING.md §6.7: the topology tools in the editor, without a GPU.

mod common;

use serde_json::json;

use common::{create, state};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use forge_core::{NodeId, Value};
use forge_geometry::PolyMesh;
use forge_ui::app::Editor;
use forge_ui::building::{apply_build, border_edge_ids, build_preview, click, hover, hud};
use forge_ui::commands::execute;
use forge_ui::state::EditorState;
use forge_ui::tools::{activate_tool, apply_tool, building_tools, tool_preview};
use forge_viewport::picking::EdgeHit;

const W: f64 = 800.0;
const H: f64 = 600.0;

fn mesh_of(st: &EditorState, id: NodeId) -> PolyMesh {
    match st.doc.scene().param(id, "mesh") {
        Some(Value::Mesh(m)) => forge_generators::to_poly_mesh(m),
        _ => panic!("mesh"),
    }
}

/// An editable cube, optionally with polygon 4 removed (an open side).
fn setup(open: bool) -> (EditorState, NodeId) {
    let mut st = state();
    create(&mut st, "forge.cube");
    execute(&mut st, "mesh.make_editable", json!({})).unwrap();
    let id = st.selection.active.unwrap();
    if open {
        let mut m = mesh_of(&st, id);
        m.polys.remove(4);
        let data = std::sync::Arc::new(forge_generators::from_poly_mesh(&m));
        st.doc
            .transact("open", |tx| tx.set_param(id, "mesh", Value::Mesh(data)))
            .unwrap();
    }
    (st, id)
}

fn counts(st: &EditorState, id: NodeId) -> (usize, usize) {
    let m = mesh_of(st, id);
    (m.points.len(), m.polys.len())
}

fn steps(st: &EditorState) -> usize {
    st.doc.history().undo_labels().len()
}

/// Moves the pointer over the midpoint of each edge in `ids` until the tool hovers one.
fn hover_edge_of(st: &mut EditorState, id: NodeId, ids: &[u32]) -> bool {
    let mesh = mesh_of(st, id);
    for &e in ids {
        let e = mesh.edges()[e as usize];
        let mid = (mesh.points[e.0 as usize] + mesh.points[e.1 as usize]) * 0.5;
        if let Some(p) = st.camera.project(mid, W, H) {
            hover(st, p.x, p.y, W, H);
            if st.tool.as_ref().unwrap().interaction.hover.is_some() {
                return true;
            }
        }
    }
    false
}

fn set_hover(st: &mut EditorState, edge: u32) {
    st.tool.as_mut().unwrap().interaction.hover = Some(EdgeHit { edge, t: 0.5 });
}

#[test]
fn the_building_tools_are_listed() {
    let ids: Vec<_> = building_tools().iter().map(|t| t.id()).collect();
    assert_eq!(ids, ["bridge", "close_hole", "stitch", "polygon_pen"]);
    let (mut st, _) = setup(false);
    for id in ids {
        assert!(activate_tool(&mut st, id), "{id}");
    }
}

#[test]
fn close_hole_previews_on_hover_and_commits_on_click() {
    let (mut st, id) = setup(true);
    let before = steps(&st);
    activate_tool(&mut st, "close_hole");
    assert!(build_preview(&st).is_none(), "no hover, no preview");
    assert_eq!(hud(&st).unwrap(), "Close Polygon Hole: hover a border edge");
    assert!(click(&mut st).is_err());
    let border = border_edge_ids(&mesh_of(&st, id));
    assert_eq!(border.len(), 4);
    assert!(
        hover_edge_of(&mut st, id, &border),
        "a hole edge is visible"
    );
    let (node, preview) = build_preview(&st).expect("preview");
    assert_eq!((node, preview.polys.len()), (id, 6));
    assert_eq!(tool_preview(&st).unwrap().1, preview);
    assert_eq!(counts(&st, id), (8, 5));
    assert_eq!(steps(&st), before);
    assert!(hud(&st).unwrap().contains("click to close the hole"));
    let out = click(&mut st).unwrap();
    assert_eq!(out["polys"], 6);
    assert_eq!(counts(&st, id), (8, 6));
    assert_eq!(steps(&st), before + 1);
    execute(&mut st, "edit.undo", json!({})).unwrap();
    assert_eq!(counts(&st, id), (8, 5));
}

#[test]
fn only_border_edges_are_accepted() {
    let (mut st, id) = setup(true);
    activate_tool(&mut st, "close_hole");
    let mesh = mesh_of(&st, id);
    let border = border_edge_ids(&mesh);
    let inner: Vec<u32> = (0..mesh.edges().len() as u32)
        .filter(|e| !border.contains(e))
        .collect();
    // an interior edge under the cursor is refused (no hover, a note in the HUD)
    let mut refused = false;
    for e in inner {
        let ed = mesh.edges()[e as usize];
        let mid = (mesh.points[ed.0 as usize] + mesh.points[ed.1 as usize]) * 0.5;
        if let Some(p) = st.camera.project(mid, W, H) {
            hover(&mut st, p.x, p.y, W, H);
            let ts = st.tool.as_ref().unwrap();
            if ts.interaction.build.rejected {
                refused = true;
                assert!(ts.interaction.hover.is_none());
                break;
            }
        }
    }
    assert!(refused);
    assert!(hud(&st).unwrap().contains("not a border edge"));
    assert!(click(&mut st).is_err());
    assert_eq!(counts(&st, id), (8, 5));
}

#[test]
fn bridge_picks_two_edges_then_commits_one_undo_step() {
    let (mut st, id) = setup(true);
    // a second open cube next to it, so that two border loops exist
    let mut m = mesh_of(&st, id);
    let base = m.points.len() as u32;
    let shift = glam::DVec3::new(500.0, 0.0, 0.0);
    let more: Vec<_> = m.points.iter().map(|p| *p + shift).collect();
    let polys: Vec<Vec<u32>> = m
        .polys
        .iter()
        .map(|p| p.iter().map(|i| i + base).collect())
        .collect();
    m.points.extend(more);
    m.polys.extend(polys);
    let data = std::sync::Arc::new(forge_generators::from_poly_mesh(&m));
    st.doc
        .transact("two", |tx| tx.set_param(id, "mesh", Value::Mesh(data)))
        .unwrap();
    let border = border_edge_ids(&mesh_of(&st, id));
    assert_eq!(border.len(), 8);
    let (a, b) = (border[0], border[4]);
    let before = steps(&st);
    activate_tool(&mut st, "bridge");
    assert!(hud(&st).unwrap().contains("hover a border edge"));
    set_hover(&mut st, a);
    assert!(build_preview(&st).is_none(), "one edge is not enough");
    assert_eq!(click(&mut st).unwrap()["picked"], 1);
    assert!(hud(&st).unwrap().contains("second edge"));
    // the same edge again is refused and keeps the pick
    assert!(click(&mut st).is_err());
    set_hover(&mut st, b);
    let (_, preview) = build_preview(&st).expect("preview with two edges");
    assert_eq!(preview.polys.len(), 10 + 4);
    assert_eq!(steps(&st), before);
    let out = click(&mut st).unwrap();
    assert_eq!(out["polys"], 10 + 4);
    assert_eq!(counts(&st, id), (16, 14));
    assert_eq!(steps(&st), before + 1);
    // the picks are cleared
    assert!(
        st.tool
            .as_ref()
            .unwrap()
            .interaction
            .build
            .picked
            .is_empty()
    );
    execute(&mut st, "edit.undo", json!({})).unwrap();
    assert_eq!(counts(&st, id), (16, 10));
    // Apply with exactly two selected border edges
    st.doc
        .transact("sel", |tx| {
            tx.set_param(id, "selection.edges", Value::IndexList(vec![a, b]))
        })
        .unwrap();
    st.tool
        .as_mut()
        .unwrap()
        .set("segments", Value::Int(2))
        .unwrap();
    apply_tool(&mut st).unwrap();
    assert_eq!(counts(&st, id), (16 + 4, 10 + 8));
}

#[test]
fn stitch_sews_two_picked_border_edges() {
    let (mut st, id) = setup(false);
    execute(&mut st, "mesh.disconnect", json!({"polys": [4]})).unwrap();
    let mesh = mesh_of(&st, id);
    let border = border_edge_ids(&mesh);
    assert_eq!(border.len(), 8);
    // a loose-polygon edge and the hole edge it faces
    let pos = |p: u32| mesh.points[p as usize];
    let edges = mesh.edges();
    let loose: Vec<u32> = border
        .iter()
        .copied()
        .filter(|e| {
            mesh.polys[4].contains(&edges[*e as usize].0)
                && mesh.polys[4].contains(&edges[*e as usize].1)
        })
        .collect();
    let a = loose[0];
    let ea = edges[a as usize];
    let b = *border
        .iter()
        .find(|e| {
            !loose.contains(e) && {
                let eb = edges[**e as usize];
                let same = |p, q| (pos(p) - pos(q)).length() < 1e-9;
                (same(ea.0, eb.0) && same(ea.1, eb.1)) || (same(ea.0, eb.1) && same(ea.1, eb.0))
            }
        })
        .unwrap();
    let before = steps(&st);
    activate_tool(&mut st, "stitch");
    set_hover(&mut st, a);
    click(&mut st).unwrap();
    set_hover(&mut st, b);
    assert_eq!(build_preview(&st).unwrap().1.points.len(), 10);
    click(&mut st).unwrap();
    assert_eq!(counts(&st, id), (10, 6));
    assert_eq!(steps(&st), before + 1);
    // Apply without picks or a selection explains what is missing
    assert!(apply_build(&mut st).is_err());
}

#[test]
fn pen_collects_corners_and_snaps_to_points() {
    let (mut st, id) = setup(false);
    let before = steps(&st);
    activate_tool(&mut st, "polygon_pen");
    assert!(click(&mut st).is_err(), "no pointer position yet");
    assert!(hud(&st).unwrap().starts_with("Polygon Pen: 0 corners"));
    // snapping: the pointer over a cube point picks that point
    let corner = st.camera.project(mesh_of(&st, id).points[0], W, H).unwrap();
    hover(&mut st, corner.x, corner.y, W, H);
    click(&mut st).unwrap();
    assert_eq!(
        st.tool.as_ref().unwrap().interaction.build.corners,
        vec![forge_modeling::PenCorner::Point(0)]
    );
    // the same point twice is refused
    assert!(click(&mut st).is_err());
    // two free corners in empty space
    for (x, y) in [(60.0, 80.0), (120.0, 80.0)] {
        hover(&mut st, x, y, W, H);
        click(&mut st).unwrap();
    }
    assert!(hud(&st).unwrap().starts_with("Polygon Pen: 3 corners"));
    assert!(tool_preview(&st).is_some(), "three corners and the cursor");
    assert_eq!(steps(&st), before);
    // Apply finishes the outline
    apply_tool(&mut st).unwrap();
    assert_eq!(counts(&st, id), (10, 7));
    assert_eq!(steps(&st), before + 1);
    assert!(
        st.tool
            .as_ref()
            .unwrap()
            .interaction
            .build
            .corners
            .is_empty()
    );
    execute(&mut st, "edit.undo", json!({})).unwrap();
    assert_eq!(counts(&st, id), (8, 6));
}

#[test]
fn clicking_the_first_corner_closes_the_pen_outline() {
    let (mut st, id) = setup(false);
    activate_tool(&mut st, "polygon_pen");
    let spots = [(60.0, 80.0), (140.0, 80.0), (100.0, 160.0)];
    for (x, y) in spots {
        hover(&mut st, x, y, W, H);
        click(&mut st).unwrap();
    }
    hover(&mut st, spots[0].0, spots[0].1, W, H);
    let out = click(&mut st).unwrap();
    assert_eq!(out["polys"], 7);
    assert_eq!(counts(&st, id), (11, 7));
    // Reset Values discards unfinished corners
    hover(&mut st, 60.0, 80.0, W, H);
    click(&mut st).unwrap();
    st.tool.as_mut().unwrap().reset();
    assert!(
        st.tool
            .as_ref()
            .unwrap()
            .interaction
            .build
            .corners
            .is_empty()
    );
}

#[test]
fn the_mesh_menu_has_the_topology_tools() {
    let (st, _) = setup(false);
    let mut h = Harness::builder()
        .with_size([1280.0, 800.0])
        .build_ui_state(|ui, e: &mut Editor| e.show(ui, None), Editor::new(st));
    h.run();
    h.get_by_label("Mesh").click();
    h.run();
    h.get_by_label("Bridge");
    h.get_by_label("Close Polygon Hole");
    h.get_by_label("Stitch and Sew");
    h.get_by_label("Polygon Pen").click();
    h.run();
    assert_eq!(
        h.state().state.tool.as_ref().unwrap().tool().id(),
        "polygon_pen"
    );
    h.get_by_label("Done").click();
    h.run();
    assert!(h.state().state.tool.is_none());
}
