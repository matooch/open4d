//! The interactive cutting tools in the viewport: Loop/Path Cut, Knife Line, Knife Plane and
//! Knife Loop (docs/specs/MODELING.md §5.5). Hover and drag only compute a preview with the pure
//! functions of `forge-modeling`; the click or release runs the registry command, so one cut is one
//! undo step. Own design, parity-pending (GUI session G3).

use forge_core::{NodeId, ParamDesc, Value};
use forge_generators::{POLYGON, to_poly_mesh};
use forge_geometry::PolyMesh;
use forge_geometry::selection::{IndexSet, Selection};
use forge_modeling::{
    KnifeLine, Plane, Settings, ToolDesc, View, even_offsets, knife_line, knife_plane, loop_cut,
};
use forge_viewport::camera::Projection;
use forge_viewport::picking::{EdgeHit, pick_edge_param};
use glam::DVec3;
use serde_json::{Value as Json, json};

use crate::commands::execute;
use crate::components::{EDGE_RADIUS, pick_mesh};
use crate::state::EditorState;
use crate::tools::{Tool, ToolState};

/// A drag shorter than this many pixels is a click and cuts nothing.
pub const MIN_DRAG_PX: f64 = 3.0;

/// Hover and drag state of the active cutting tool (editor state: not saved, not undoable).
#[derive(Debug, Clone, Default)]
pub struct Interaction {
    /// The edge under the cursor and the position along it.
    pub hover: Option<EdgeHit>,
    /// The knife line being dragged, `[x0, y0, x1, y1]` in render pixels, and the viewport size.
    pub drag: Option<([f64; 4], [f64; 2])>,
}

/// An interactive cutting tool of `forge-modeling`'s descriptors.
pub struct CutTool {
    pub desc: ToolDesc,
}

impl Tool for CutTool {
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

/// True for the tools that are driven by hover and click or drag.
pub fn is_cut_tool(id: &str) -> bool {
    matches!(id, "loop_cut" | "knife_line" | "knife_plane" | "knife_loop")
}

fn is_loop(id: &str) -> bool {
    matches!(id, "loop_cut" | "knife_loop")
}

/// The active polygon object and its stored (object space) mesh. No selection is needed.
pub fn cut_target(state: &EditorState) -> Option<(NodeId, PolyMesh)> {
    let scene = state.doc.scene();
    let id = state.selection.active?;
    if scene.node(id)?.type_id != POLYGON {
        return None;
    }
    match scene.param(id, "mesh") {
        Some(Value::Mesh(m)) => Some((id, to_poly_mesh(m))),
        _ => None,
    }
}

fn active(state: &EditorState) -> Option<&ToolState> {
    state.tool.as_ref().filter(|t| is_cut_tool(t.tool().id()))
}

/// The cut offsets of a loop tool at the current hover (`None` without a hover edge).
fn offsets(ts: &ToolState, hover: Option<&EdgeHit>) -> Option<Vec<f64>> {
    let v = ts.values();
    let id = ts.tool().id();
    if id == "loop_cut" && v.int("count") > 1 {
        return Some(even_offsets(v.int("count") as usize));
    }
    if v.bool("lock_offset") {
        return Some(vec![v.float("offset")]);
    }
    hover.map(|h| vec![h.t.clamp(0.001, 0.999)])
}

/// Updates the hover edge for the cursor at pixel `(x, y)` of a `w x h` viewport. Does nothing for
/// the line and plane knife.
pub fn hover(state: &mut EditorState, x: f64, y: f64, w: f64, h: f64) {
    let Some(id) = active(state).map(|t| t.tool().id()) else {
        return;
    };
    if !is_loop(id) {
        return;
    }
    let hit = cut_target(state).and_then(|(node, _)| {
        let mesh = pick_mesh(state.doc.scene(), node)?;
        pick_edge_param(&mesh, &state.camera, x, y, w, h, EDGE_RADIUS, true)
    });
    if let Some(ts) = state.tool.as_mut() {
        ts.interaction.hover = hit;
    }
}

/// The command and arguments of a loop cut at the current hover.
fn loop_args(ts: &ToolState) -> Result<(&'static str, Json), String> {
    let hit = ts.interaction.hover.ok_or("hover an edge first")?;
    let offs = offsets(ts, Some(&hit)).ok_or("no offset")?;
    Ok(if ts.tool().id() == "loop_cut" {
        ("mesh.loop_cut", json!({"edge": hit.edge, "offsets": offs}))
    } else {
        (
            "mesh.knife",
            json!({"mode": "loop", "edge": hit.edge, "offset": offs[0]}),
        )
    })
}

/// A click: cuts at the hover edge (loop tools). One command, one undo step.
pub fn click(state: &mut EditorState) -> Result<Json, String> {
    let ts = active(state).ok_or("no cutting tool is active")?;
    let (command, args) = loop_args(ts)?;
    execute(state, command, args)
}

fn vec(v: DVec3) -> Json {
    json!([v.x, v.y, v.z])
}

/// The knife line of a drag in object space.
fn drag_line(
    state: &EditorState,
    node: NodeId,
    rect: [f64; 4],
    size: [f64; 2],
    visible_only: bool,
) -> Option<KnifeLine> {
    let cam = &state.camera;
    let inv = state.doc.scene().world_matrix(node)?.inverse();
    let (r0, r1) = (
        cam.ray(rect[0], rect[1], size[0], size[1]),
        cam.ray(rect[2], rect[3], size[0], size[1]),
    );
    let (from, to, view) = match cam.projection {
        Projection::Perspective { .. } => {
            let eye = cam.eye_world();
            let depth = (cam.target_world() - eye).length().max(1.0);
            (
                eye + r0.dir * depth,
                eye + r1.dir * depth,
                View::Eye(inv.transform_point3(eye)),
            )
        }
        Projection::Orthographic { .. } => (
            r0.origin,
            r1.origin,
            View::Direction(inv.transform_vector3(r0.dir)),
        ),
    };
    Some(KnifeLine {
        from: inv.transform_point3(from),
        to: inv.transform_point3(to),
        view,
        visible_only,
    })
}

/// Starts a knife drag at pixel `(x, y)`.
pub fn drag_begin(state: &mut EditorState, x: f64, y: f64, size: [f64; 2]) {
    if let Some(ts) = state.tool.as_mut().filter(|t| !is_loop(t.tool().id())) {
        ts.interaction.drag = Some(([x, y, x, y], size));
    }
}

/// Moves the end of the knife line.
pub fn drag_update(state: &mut EditorState, x: f64, y: f64) {
    if let Some((rect, _)) = state
        .tool
        .as_mut()
        .and_then(|t| t.interaction.drag.as_mut())
    {
        rect[2] = x;
        rect[3] = y;
    }
}

/// Ends the drag at `(x, y)` and cuts. A drag shorter than [`MIN_DRAG_PX`] cuts nothing.
pub fn drag_finish(state: &mut EditorState, x: f64, y: f64) -> Result<Option<Json>, String> {
    drag_update(state, x, y);
    let Some(ts) = state.tool.as_mut() else {
        return Ok(None);
    };
    let Some((rect, size)) = ts.interaction.drag.take() else {
        return Ok(None);
    };
    if (rect[2] - rect[0]).hypot(rect[3] - rect[1]) < MIN_DRAG_PX {
        return Ok(None);
    }
    let (node, _) = cut_target(state).ok_or("select a polygon object")?;
    let ts = active(state).ok_or("no cutting tool is active")?;
    let visible = ts.values().bool("visible_only");
    let line = drag_line(state, node, rect, size, visible).ok_or("no line")?;
    let args = if ts.tool().id() == "knife_line" {
        let (key, view) = match line.view {
            View::Eye(e) => ("eye", e),
            View::Direction(d) => ("direction", d),
        };
        json!({"mode": "line", "from": vec(line.from), "to": vec(line.to),
               key: vec(view), "visible_only": visible})
    } else {
        let plane = line.plane().map_err(|e| e.to_string())?;
        json!({"mode": "plane", "point": vec(plane.point), "normal": vec(plane.normal)})
    };
    execute(state, "mesh.knife", args).map(Some)
}

/// The Apply button: Knife Plane with the panel's numbers, or a loop cut at the hover edge.
pub fn apply_cut(state: &mut EditorState) -> Result<Json, String> {
    let ts = active(state).ok_or("no cutting tool is active")?;
    if ts.tool().id() == "knife_line" {
        return Err("Knife Line: drag a line in the viewport".into());
    }
    let (command, args) = if is_loop(ts.tool().id()) {
        loop_args(ts)?
    } else {
        (
            "mesh.knife",
            json!({"mode": "plane",
                   "point": vec(ts.values().vec3("plane_point")),
                   "normal": vec(ts.values().vec3("plane_normal"))}),
        )
    };
    execute(state, command, args)
}

/// The result of the cut under the pointer, for drawing in place of the target; the document is
/// not touched.
pub fn cut_preview(state: &EditorState) -> Option<(NodeId, PolyMesh)> {
    let ts = active(state)?;
    let (node, mesh) = cut_target(state)?;
    let id = ts.tool().id();
    let none = IndexSet::new();
    let out = if is_loop(id) {
        let hit = ts.interaction.hover?;
        loop_cut(&mesh, hit.edge, &offsets(ts, Some(&hit))?).ok()?
    } else if let Some((rect, size)) = ts.interaction.drag {
        let line = drag_line(state, node, rect, size, ts.values().bool("visible_only"))?;
        if id == "knife_line" {
            knife_line(&mesh, &none, &line).ok()?
        } else {
            knife_plane(&mesh, &none, line.plane().ok()?).ok()?
        }
    } else if id == "knife_plane" {
        let plane = Plane {
            point: ts.values().vec3("plane_point"),
            normal: ts.values().vec3("plane_normal"),
        };
        knife_plane(&mesh, &none, plane).ok()?
    } else {
        return None;
    };
    Some((node, out.mesh))
}

/// The heads-up line for the viewport: what the tool will do and the count and offset.
pub fn hud(state: &EditorState) -> Option<String> {
    let ts = active(state)?;
    let label = ts.tool().label();
    Some(match ts.tool().id() {
        "loop_cut" | "knife_loop" => match offsets(ts, ts.interaction.hover.as_ref()) {
            Some(o) if ts.interaction.hover.is_some() || ts.values().bool("lock_offset") => {
                let shown: Vec<String> = o.iter().map(|t| format!("{t:.2}")).collect();
                format!(
                    "{label}: {} cut{}, offset {}  (click to cut)",
                    o.len(),
                    if o.len() == 1 { "" } else { "s" },
                    shown.join(", ")
                )
            }
            _ => format!("{label}: hover an edge"),
        },
        "knife_line" => format!("{label}: drag a line across the mesh"),
        _ => format!("{label}: drag a line, or set the plane and press Apply"),
    })
}
