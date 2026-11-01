//! Modeling tool framework: the active tool, its settings, a live preview and the apply step
//! (docs/specs/MODELING.md §0.2). Applying runs the tool's registry command, so one apply is
//! one undo step; the preview never touches the document.

use std::sync::Arc;

use forge_core::{NodeId, ParamDesc, Scene, Value};
use forge_generators::{
    POLYGON, SELECTION_EDGES, SELECTION_POINTS, SELECTION_POLYS, from_poly_mesh, to_poly_mesh,
};
use forge_geometry::PolyMesh;
use forge_geometry::selection::{IndexSet, Selection};
use forge_modeling::{Settings, ToolDesc, tool_descs, tool_fn};
use serde_json::{Value as Json, json};

use crate::commands::execute;
use crate::state::EditorState;

/// A modeling tool the editor can activate.
pub trait Tool: Send + Sync {
    fn id(&self) -> &'static str;
    fn label(&self) -> &'static str;
    /// The registry command that applies the tool.
    fn command_id(&self) -> &'static str;
    /// The settings shown in the Attribute panel.
    fn settings(&self) -> Vec<ParamDesc>;
    /// The setting an interactive drag edits.
    fn drag_param(&self) -> &'static str;
    /// The result of the tool on `mesh`; never writes the document.
    fn preview(&self, mesh: &PolyMesh, sel: &Selection, s: &Settings) -> Result<PolyMesh, String>;
}

/// A tool defined by a `forge-modeling` descriptor and pure function.
struct ModelingTool {
    desc: ToolDesc,
    apply: forge_modeling::ToolFn,
}

impl Tool for ModelingTool {
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
        self.desc.drag_param
    }

    fn preview(&self, mesh: &PolyMesh, sel: &Selection, s: &Settings) -> Result<PolyMesh, String> {
        (self.apply)(mesh, sel, s)
            .map(|(m, _)| m)
            .map_err(|e| e.to_string())
    }
}

/// Every tool, in menu order.
pub fn tools() -> Vec<Box<dyn Tool>> {
    tool_descs()
        .into_iter()
        .filter_map(|desc| {
            let apply = tool_fn(desc.id)?;
            Some(Box::new(ModelingTool { desc, apply }) as Box<dyn Tool>)
        })
        .collect()
}

/// The active tool and its current values (editor state: not saved, not undoable).
pub struct ToolState {
    tool: Box<dyn Tool>,
    desc: ToolDesc,
    values: Settings,
}

impl ToolState {
    pub fn tool(&self) -> &dyn Tool {
        self.tool.as_ref()
    }

    pub fn values(&self) -> &Settings {
        &self.values
    }

    /// Restores the factory defaults ("Reset Values").
    pub fn reset(&mut self) {
        self.values = Settings::defaults(&self.desc);
    }

    /// Sets one setting; numbers are clamped by the descriptor.
    pub fn set(&mut self, id: &str, value: Value) -> Result<(), String> {
        self.values
            .set(&self.desc, id, value)
            .map_err(|e| e.to_string())
    }

    /// The command arguments for the current values.
    pub fn command_args(&self) -> Json {
        json!({"settings": self.values.to_json()})
    }
}

/// Makes `id` the active tool with its factory defaults. False for an unknown id.
pub fn activate_tool(state: &mut EditorState, id: &str) -> bool {
    let Some(tool) = tools().into_iter().find(|t| t.id() == id) else {
        return false;
    };
    let Some(desc) = forge_modeling::tool_desc(id) else {
        return false;
    };
    let values = Settings::defaults(&desc);
    state.tool = Some(ToolState { tool, desc, values });
    true
}

pub fn deactivate_tool(state: &mut EditorState) {
    state.tool = None;
}

/// What the tool works on: the active polygon object, its stored mesh and its stored
/// selection (at least one of the three sets is non-empty).
pub fn tool_target(state: &EditorState) -> Option<(NodeId, PolyMesh, Selection)> {
    let scene = state.doc.scene();
    let id = state.selection.active?;
    if scene.node(id)?.type_id != POLYGON {
        return None;
    }
    let Some(Value::Mesh(m)) = scene.param(id, "mesh") else {
        return None;
    };
    let stored = |key: &str| -> IndexSet {
        match scene.param(id, key) {
            Some(Value::IndexList(l)) => l.iter().copied().collect(),
            _ => IndexSet::new(),
        }
    };
    let sel = Selection {
        polys: stored(SELECTION_POLYS),
        points: stored(SELECTION_POINTS),
        edges: stored(SELECTION_EDGES),
    };
    let any = !(sel.polys.is_empty() && sel.points.is_empty() && sel.edges.is_empty());
    any.then(|| (id, to_poly_mesh(m), sel))
}

/// The tool's result on the current target, or `None` without an active tool, a target, or
/// when the tool refuses the current values.
pub fn tool_preview(state: &EditorState) -> Option<(NodeId, PolyMesh)> {
    let ts = state.tool.as_ref()?;
    let (id, mesh, sel) = tool_target(state)?;
    let out = ts.tool.preview(&mesh, &sel, &ts.values).ok()?;
    Some((id, out))
}

/// The scene with the preview in place of the target's stored mesh (for drawing only).
pub fn with_preview(state: &EditorState, scene: &Scene) -> Option<Arc<Scene>> {
    let (id, mesh) = tool_preview(state)?;
    let data = Value::Mesh(Arc::new(from_poly_mesh(&mesh)));
    let (copy, skipped) = scene.with_params([(id, "mesh".to_string(), data)]);
    skipped.is_empty().then(|| Arc::new(copy))
}

/// Runs the active tool's command with the current values: one undo step. The tool stays
/// active with its values.
pub fn apply_tool(state: &mut EditorState) -> Result<Json, String> {
    let ts = state.tool.as_ref().ok_or("no tool is active")?;
    let (command, args) = (ts.tool.command_id(), ts.command_args());
    execute(state, command, args)
}

/// Distance for a vertical drag of `pixels` (up is positive): one viewport height is about
/// one diagonal of the target's bounding box.
pub fn drag_distance(pixels_up: f64, diagonal: f64, viewport_height: f64) -> f64 {
    pixels_up * diagonal / viewport_height.max(1.0)
}

/// Diagonal of the stored mesh's bounding box (1 for an empty mesh).
pub fn target_diagonal(mesh: &PolyMesh) -> f64 {
    let (mut lo, mut hi) = (glam::DVec3::splat(f64::MAX), glam::DVec3::splat(f64::MIN));
    for p in &mesh.points {
        lo = lo.min(*p);
        hi = hi.max(*p);
    }
    let d = (hi - lo).length();
    if d.is_finite() && d > 0.0 { d } else { 1.0 }
}

/// An interactive drag in progress: the offset when it started and the pointer's start row.
#[derive(Debug, Clone, Copy)]
pub struct ToolDrag {
    start_value: f64,
    start_y: f64,
    diagonal: f64,
    height: f64,
}

impl ToolDrag {
    /// Starts a drag at pixel row `y` in a viewport `height` pixels high. `None` without an
    /// active tool or target.
    pub fn begin(state: &EditorState, y: f64, height: f64) -> Option<ToolDrag> {
        let ts = state.tool.as_ref()?;
        // mesh commands (docs/specs/MODELING.md §4) have no drag setting: Apply only
        if ts.tool.drag_param().is_empty() {
            return None;
        }
        let (_, mesh, _) = tool_target(state)?;
        Some(ToolDrag {
            start_value: ts.values.float(ts.tool.drag_param()),
            start_y: y,
            diagonal: target_diagonal(&mesh),
            height,
        })
    }

    /// Sets the drag setting for the pointer at row `y`. The document is not touched.
    pub fn update(&self, state: &mut EditorState, y: f64) {
        let Some(ts) = state.tool.as_mut() else {
            return;
        };
        let d = drag_distance(self.start_y - y, self.diagonal, self.height);
        let id = ts.tool.drag_param();
        let _ = ts.set(id, Value::Float(self.start_value + d));
    }

    /// Ends the drag: applies the tool (one command, one undo step).
    pub fn finish(self, state: &mut EditorState) -> Result<Json, String> {
        apply_tool(state)
    }
}
