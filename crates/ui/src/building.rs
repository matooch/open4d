//! The topology tools in the viewport: Bridge, Close Polygon Hole, Stitch and Sew and Polygon Pen
//! (docs/specs/MODELING.md §6.7). Hover and picks only compute a preview with the pure functions
//! of `forge-modeling`; the click or Apply runs the registry command, so one application is one
//! undo step. Own design, parity-pending (GUI session G3).

use std::collections::HashMap;

use forge_core::{NodeId, ParamDesc};
use forge_geometry::mesh::poly_edges;
use forge_geometry::selection::Selection;
use forge_geometry::{Edge, PolyMesh};
use forge_modeling::{
    BridgeEnd, PenCorner, Settings, StitchTo, ToolDesc, bridge, close_hole, pen, stitch,
    stitch_edges,
};
use forge_viewport::picking::{pick_edge_param, pick_point};
use glam::DVec3;
use serde_json::{Value as Json, json};

use crate::commands::execute;
use crate::components::{EDGE_RADIUS, POINT_RADIUS, pick_mesh};
use crate::cutting::cut_target;
use crate::state::EditorState;
use crate::tools::{Tool, ToolState};

/// A topology tool of `forge-modeling`'s descriptors.
pub struct BuildTool {
    pub desc: ToolDesc,
}

impl Tool for BuildTool {
    fn id(&self) -> &'static str {
        self.desc.id
    }

    fn label(&self) -> &'static str {
        self.desc.label
    }

    fn command_id(&self) -> &'static str {
        self.desc.command
    }

    fn settings(&self) -> Vec<ParamDesc> {
        self.desc.params.clone()
    }

    fn drag_param(&self) -> &'static str {
        ""
    }

    fn preview(&self, _: &PolyMesh, _: &Selection, _: &Settings) -> Result<PolyMesh, String> {
        Err("interactive tool".into())
    }
}

/// True for the tools that are driven by hover and click.
pub fn is_build_tool(id: &str) -> bool {
    matches!(id, "bridge" | "close_hole" | "stitch" | "polygon_pen")
}

/// Picks, pen corners and the cursor of the active topology tool (editor state: not saved).
#[derive(Debug, Clone, Default)]
pub struct BuildState {
    /// Border edges picked so far (Bridge, Stitch and Sew).
    pub picked: Vec<u32>,
    /// Polygon Pen corners so far.
    pub corners: Vec<PenCorner>,
    /// The cursor `[x, y, w, h]` in render pixels (Polygon Pen).
    pub cursor: Option<[f64; 4]>,
    /// The hovered edge is not a border edge.
    pub rejected: bool,
}

fn active(state: &EditorState) -> Option<&ToolState> {
    state.tool.as_ref().filter(|t| is_build_tool(t.tool().id()))
}

/// Border edges of `mesh` (used by one polygon), as edge ids.
pub fn border_edge_ids(mesh: &PolyMesh) -> Vec<u32> {
    let mut uses: HashMap<Edge, u32> = HashMap::new();
    for p in &mesh.polys {
        for e in poly_edges(p) {
            *uses.entry(e).or_default() += 1;
        }
    }
    mesh.edges()
        .iter()
        .enumerate()
        .filter(|(_, e)| uses[e] == 1)
        .map(|(i, _)| i as u32)
        .collect()
}

/// Updates the hover for the cursor at pixel `(x, y)` of a `w x h` viewport.
pub fn hover(state: &mut EditorState, x: f64, y: f64, w: f64, h: f64) {
    let Some(id) = active(state).map(|t| t.tool().id()) else {
        return;
    };
    if id == "polygon_pen" {
        if let Some(ts) = state.tool.as_mut() {
            ts.interaction.build.cursor = Some([x, y, w, h]);
        }
        return;
    }
    let hit = cut_target(state).and_then(|(node, mesh)| {
        let picked = pick_mesh(state.doc.scene(), node)?;
        let hit = pick_edge_param(&picked, &state.camera, x, y, w, h, EDGE_RADIUS, true)?;
        Some((hit, border_edge_ids(&mesh).contains(&hit.edge)))
    });
    if let Some(ts) = state.tool.as_mut() {
        ts.interaction.hover = hit.filter(|(_, border)| *border).map(|(hit, _)| hit);
        ts.interaction.build.rejected = hit.is_some_and(|(_, border)| !border);
    }
}

/// The Polygon Pen corner under the cursor: an existing point within the pick radius, else a point
/// on the plane through the camera target facing the viewer, in object space.
fn pen_corner(state: &EditorState) -> Result<PenCorner, String> {
    let ts = active(state).ok_or("no topology tool is active")?;
    let [x, y, w, h] = ts
        .interaction
        .build
        .cursor
        .ok_or("move the pointer over the view")?;
    let (node, _) = cut_target(state).ok_or("select a polygon object")?;
    let picked = pick_mesh(state.doc.scene(), node).ok_or("no mesh")?;
    if let Some(p) = pick_point(&picked, &state.camera, x, y, w, h, POINT_RADIUS, false) {
        return Ok(PenCorner::Point(p));
    }
    let cam = &state.camera;
    let ray = cam.ray(x, y, w, h);
    let normal = (cam.target_world() - cam.eye_world()).normalize_or_zero();
    let den = ray.dir.dot(normal);
    if den.abs() < 1e-9 {
        return Err("the pointer ray is parallel to the drawing plane".into());
    }
    let t = (cam.target_world() - ray.origin).dot(normal) / den;
    let world = ray.origin + ray.dir * t;
    let inv = state
        .doc
        .scene()
        .world_matrix(node)
        .ok_or("no transform")?
        .inverse();
    Ok(PenCorner::New(inv.transform_point3(world)))
}

fn corners_json(corners: &[PenCorner]) -> Json {
    Json::Array(
        corners
            .iter()
            .map(|c| match c {
                PenCorner::Point(p) => json!({"point": p}),
                PenCorner::New(v) => json!({"pos": vec(*v)}),
            })
            .collect(),
    )
}

fn vec(v: DVec3) -> Json {
    json!([v.x, v.y, v.z])
}

fn clear_picks(state: &mut EditorState) {
    if let Some(ts) = state.tool.as_mut() {
        ts.interaction.build.picked.clear();
        ts.interaction.build.corners.clear();
        ts.interaction.hover = None;
    }
}

/// The command and arguments for the picked edges (two) of Bridge or Stitch and Sew.
fn pair_args(ts: &ToolState, edges: [u32; 2]) -> (&'static str, Json) {
    if ts.tool().id() == "bridge" {
        (
            "mesh.bridge",
            json!({"edges": edges, "segments": ts.values().int("segments")}),
        )
    } else {
        (
            "mesh.stitch",
            json!({"edges": [edges], "position": ts.values().choice("position")}),
        )
    }
}

/// A click: picks the hovered border edge (Bridge, Stitch), closes the hovered hole, or adds a pen
/// corner. The command runs when the click completes the input; one command, one undo step.
pub fn click(state: &mut EditorState) -> Result<Json, String> {
    let ts = active(state).ok_or("no topology tool is active")?;
    let id = ts.tool().id();
    if id == "polygon_pen" {
        let corner = pen_corner(state)?;
        let ts = state.tool.as_mut().ok_or("no tool")?;
        let corners = &mut ts.interaction.build.corners;
        if corners.len() >= 3 && corners[0] == corner {
            return finish_pen(state);
        }
        if let PenCorner::Point(p) = corner
            && corners.contains(&corner)
        {
            return Err(format!("point {p} is already a corner"));
        }
        corners.push(corner);
        return Ok(json!({"corners": corners.len()}));
    }
    let hit = ts
        .interaction
        .hover
        .ok_or(if ts.interaction.build.rejected {
            "that is not a border edge"
        } else {
            "hover a border edge first"
        })?;
    if id == "close_hole" {
        return execute(state, "mesh.close_hole", json!({"edge": hit.edge}));
    }
    let picked = &ts.interaction.build.picked;
    if picked.first() == Some(&hit.edge) {
        return Err("pick a second, different border edge".into());
    }
    let Some(&first) = picked.first() else {
        let ts = state.tool.as_mut().ok_or("no tool")?;
        ts.interaction.build.picked.push(hit.edge);
        return Ok(json!({"picked": 1}));
    };
    let (command, args) = pair_args(ts, [first, hit.edge]);
    let result = execute(state, command, args);
    clear_picks(state);
    result
}

fn finish_pen(state: &mut EditorState) -> Result<Json, String> {
    let ts = active(state).ok_or("no topology tool is active")?;
    let corners = corners_json(&ts.interaction.build.corners);
    let result = execute(state, "mesh.pen", json!({"corners": corners}));
    if result.is_ok() {
        clear_picks(state);
    }
    result
}

/// The Apply button: Pen finishes its outline; Bridge and Stitch use the picked edges, or exactly
/// two selected border edges; Close Polygon Hole the hovered hole or the one selected border edge.
pub fn apply_build(state: &mut EditorState) -> Result<Json, String> {
    let ts = active(state).ok_or("no topology tool is active")?;
    let id = ts.tool().id();
    if id == "polygon_pen" {
        return finish_pen(state);
    }
    let selected: Vec<u32> = {
        let (node, _) = cut_target(state).ok_or("select a polygon object")?;
        match state
            .doc
            .scene()
            .param(node, forge_generators::SELECTION_EDGES)
        {
            Some(forge_core::Value::IndexList(l)) => l.clone(),
            _ => Vec::new(),
        }
    };
    if id == "close_hole" {
        let edge = ts
            .interaction
            .hover
            .map(|h| h.edge)
            .or(match selected[..] {
                [one] => Some(one),
                _ => None,
            })
            .ok_or("hover a border edge or select one")?;
        return execute(state, "mesh.close_hole", json!({"edge": edge}));
    }
    let edges: [u32; 2] = match (&ts.interaction.build.picked[..], &selected[..]) {
        ([a, b], _) | (_, [a, b]) => [*a, *b],
        _ => return Err("pick two border edges, or select exactly two".into()),
    };
    let (command, args) = pair_args(ts, edges);
    let result = execute(state, command, args);
    clear_picks(state);
    result
}

/// The tool's result for the current picks and hover, for drawing in place of the target; the
/// document is not touched.
pub fn build_preview(state: &EditorState) -> Option<(NodeId, PolyMesh)> {
    let ts = active(state)?;
    let (node, mesh) = cut_target(state)?;
    let hover = ts.interaction.hover.map(|h| h.edge);
    let out = match ts.tool().id() {
        "close_hole" => close_hole(&mesh, hover?).ok()?,
        "bridge" => {
            let a = *ts.interaction.build.picked.first()?;
            bridge(
                &mesh,
                BridgeEnd::Edge(a),
                BridgeEnd::Edge(hover?),
                ts.values().int("segments") as usize,
            )
            .ok()?
        }
        "stitch" => {
            let a = *ts.interaction.build.picked.first()?;
            let pairs = stitch_edges(&mesh, &[(a, hover?)]).ok()?;
            let to = if ts.values().choice("position") == "first" {
                StitchTo::First
            } else {
                StitchTo::Middle
            };
            stitch(&mesh, &pairs, to).ok()?
        }
        _ => {
            let mut corners = ts.interaction.build.corners.clone();
            if corners.len() < 3 {
                return None;
            }
            if let Ok(c) = pen_corner(state)
                && c != corners[0]
                && !corners.contains(&c)
            {
                corners.push(c);
            }
            pen(&mesh, &corners).ok()?
        }
    };
    Some((node, out.mesh))
}

/// The heads-up line for the viewport.
pub fn hud(state: &EditorState) -> Option<String> {
    let ts = active(state)?;
    let label = ts.tool().label();
    let b = &ts.interaction.build;
    Some(match ts.tool().id() {
        "close_hole" => hover_line(ts, label, "click to close the hole"),
        "polygon_pen" => format!(
            "{label}: {} corner{}  (click to add; click the first corner or Apply to finish)",
            b.corners.len(),
            if b.corners.len() == 1 { "" } else { "s" }
        ),
        _ if b.picked.is_empty() => hover_line(ts, label, "click to pick the first edge"),
        _ => hover_line(ts, label, "click to pick the second edge"),
    })
}

fn hover_line(ts: &ToolState, label: &str, hint: &str) -> String {
    if ts.interaction.hover.is_some() {
        format!("{label}: {hint}")
    } else if ts.interaction.build.rejected {
        format!("{label}: that is not a border edge")
    } else {
        format!("{label}: hover a border edge")
    }
}

/// The screen positions (render pixels) of the Polygon Pen corners placed so far, for drawing.
pub fn pen_corner_pixels(state: &EditorState, w: f64, h: f64) -> Vec<[f64; 2]> {
    let Some(ts) = active(state).filter(|t| t.tool().id() == "polygon_pen") else {
        return Vec::new();
    };
    let Some((node, mesh)) = cut_target(state) else {
        return Vec::new();
    };
    let Some(world) = state.doc.scene().world_matrix(node) else {
        return Vec::new();
    };
    ts.interaction
        .build
        .corners
        .iter()
        .filter_map(|c| {
            let p = match *c {
                PenCorner::Point(i) => *mesh.points.get(i as usize)?,
                PenCorner::New(v) => v,
            };
            let s = state.camera.project(world.transform_point3(p), w, h)?;
            Some([s.x, s.y])
        })
        .collect()
}
