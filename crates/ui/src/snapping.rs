//! Snapping in the editor: the sources in view, a snapped gizmo drag step, and the snapped
//! pixel for the cutting tools (docs/specs/MANIPULATE.md §6.7). UI-free (no egui).

use std::collections::BTreeSet;

use forge_core::{EvalCtx, NodeId};
use forge_modeling::snap::{SnapHit, SnapSettings, SnapSource};
use forge_viewport::camera::Camera;
use forge_viewport::gizmo::{Drag, DragDelta, GizmoMode};
use forge_viewport::snap::snap_cursor;
use glam::DVec3;

use crate::state::EditorState;
use crate::viewport_panel::{DrawItem, scene_draw_items_at};

/// The snap sources of a scene, with the node each one came from.
#[derive(Debug, Clone, Default)]
pub struct SnapScene {
    pub nodes: Vec<NodeId>,
    pub sources: Vec<SnapSource>,
}

impl SnapScene {
    /// The node of the source a hit landed on.
    pub fn node_of(&self, hit: &SnapHit) -> Option<NodeId> {
        hit.source.and_then(|s| self.nodes.get(s)).copied()
    }
}

/// The meshes in view (world space) except `exclude` and their descendants; the points
/// `skip.1` of node `skip.0` are marked as moving (they are not targets).
pub fn collect_sources(
    state: &EditorState,
    exclude: &[NodeId],
    skip: Option<(NodeId, Vec<u32>)>,
) -> SnapScene {
    let skips: Vec<(NodeId, Vec<u32>)> = skip.into_iter().collect();
    collect_sources_multi(state, exclude, &skips)
}

/// [`collect_sources`] with the moving points of several objects (a multi-object component
/// drag, EDITABLE.md §6.6).
pub fn collect_sources_multi(
    state: &EditorState,
    exclude: &[NodeId],
    skips: &[(NodeId, Vec<u32>)],
) -> SnapScene {
    let scene = state.evaluated();
    let mut hidden: BTreeSet<NodeId> = BTreeSet::new();
    for id in exclude {
        hidden.insert(*id);
        hidden.extend(scene.descendants(*id));
    }
    let mut out = SnapScene::default();
    for item in scene_draw_items_at(&scene, &BTreeSet::new(), &EvalCtx::default()) {
        let DrawItem::Mesh { id, mesh, .. } = item else {
            continue;
        };
        if hidden.contains(&id) {
            continue;
        }
        let mut skip_points: Vec<u32> = skips
            .iter()
            .filter(|(node, _)| *node == id)
            .flat_map(|(_, points)| points.iter().copied())
            .collect();
        skip_points.sort_unstable();
        skip_points.dedup();
        out.nodes.push(id);
        out.sources.push(SnapSource {
            mesh,
            origin: scene.world_matrix(id).map(|m| m.w_axis.truncate()),
            skip_points,
        });
    }
    out
}

#[allow(clippy::too_many_arguments)] // settings, sources, drag, camera, cursor, size, mode and Ctrl are independent
/// One pointer step of a gizmo drag: a target hit (translate drags, when snapping is on),
/// else the quantized or free drag (MANIPULATE.md §6.6, §6.7).
pub fn drag_step(
    settings: &SnapSettings,
    sources: &[SnapSource],
    drag: &mut Drag,
    cam: &Camera,
    cursor: (f64, f64),
    size: (f64, f64),
    mode: GizmoMode,
    ctrl: bool,
) -> (DragDelta, Option<SnapHit>) {
    if settings.enabled
        && mode == GizmoMode::Translate
        && let Some(hit) = snap_cursor(sources, cam, cursor, size, settings)
    {
        return (drag.translate_to(hit.position), Some(hit));
    }
    let step = settings.quantize_steps(ctrl).map(|s| match mode {
        GizmoMode::Translate => s.translate,
        GizmoMode::Rotate => s.rotate,
        GizmoMode::Scale => s.scale,
    });
    let (x, y) = cursor;
    (drag.update(cam, x, y, size.0, size.1, step), None)
}

/// The snap hit under the pixel, if snapping is on.
pub fn hit_at(
    state: &EditorState,
    scene: &SnapScene,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
) -> Option<SnapHit> {
    let s = &state.app.transform_settings.snap;
    s.enabled
        .then(|| snap_cursor(&scene.sources, &state.camera, (x, y), (w, h), s))
        .flatten()
}

/// The pixel the cutting tools use: the projection of the snap hit, else `(x, y)`.
pub fn snap_pixel(state: &EditorState, x: f64, y: f64, w: f64, h: f64) -> (f64, f64) {
    if !state.app.transform_settings.snap.enabled {
        return (x, y);
    }
    let scene = collect_sources(state, &[], None);
    hit_at(state, &scene, x, y, w, h)
        .and_then(|hit| pixel_of(&state.camera, hit.position, w, h))
        .unwrap_or((x, y))
}

/// Pixel of a world position.
pub fn pixel_of(cam: &Camera, p: DVec3, w: f64, h: f64) -> Option<(f64, f64)> {
    cam.project(p, w, h).map(|s| (s.x, s.y))
}
