//! Moving, rotating and scaling components (docs/specs/EDITABLE.md §3). UI-free: the egui
//! code in `app` only turns pointer input into these calls.

use std::sync::Arc;

use forge_core::transform::rotation_part;
use forge_core::{Document, EditError, MeshData, NodeId, Scene, Value};
use forge_generators::{POLYGON, to_poly_mesh};
use forge_geometry::selection::{IndexSet, Selection, edges_to_points, polys_to_points};
use forge_viewport::gizmo::DragDelta;
use glam::{DMat3, DMat4, DVec3};

use crate::attributes::Gesture;
use crate::components::{EditMode, stored_selection, target};
use crate::state::EditorState;

/// The points a selection in `mode` moves: the point set, the points of the edges, or the
/// points of the polygons. Indices past the element count are ignored.
pub fn affected_points(mesh: &MeshData, mode: EditMode, sets: &Selection) -> IndexSet {
    let poly = to_poly_mesh(mesh);
    let all = match mode {
        EditMode::Model => IndexSet::new(),
        EditMode::Points => sets.points.clone(),
        EditMode::Edges => edges_to_points(&poly, &sets.edges),
        EditMode::Polys => polys_to_points(&poly, &sets.polys),
    };
    all.iter()
        .filter(|i| (*i as usize) < mesh.points.len())
        .collect()
}

/// The mean position of `points` (object space); `None` when the set is empty.
pub fn pivot(mesh: &MeshData, points: &IndexSet) -> Option<DVec3> {
    let sum: DVec3 = points
        .iter()
        .filter_map(|i| mesh.points.get(i as usize))
        .map(|p| DVec3::from_array(*p))
        .sum();
    let n = points
        .iter()
        .filter(|i| (*i as usize) < mesh.points.len())
        .count();
    (n > 0).then(|| sum / n as f64)
}

/// `mesh` with `m` applied to the listed points only (object space); indices past the
/// point count are ignored, polygons are copied.
pub fn transform_points(mesh: &MeshData, points: &IndexSet, m: DMat4) -> MeshData {
    let mut out = mesh.clone();
    for i in points.iter() {
        if let Some(p) = out.points.get_mut(i as usize) {
            *p = m.transform_point3(DVec3::from_array(*p)).to_array();
        }
    }
    out
}

/// The object-space matrix for a world-space gizmo `delta` about the pivot `c` (object
/// space) of a node with world matrix `world`: `W⁻¹ · M · W` (EDITABLE.md §3.4).
pub fn local_matrix(world: &DMat4, c: DVec3, delta: DragDelta) -> DMat4 {
    let p = world.transform_point3(c);
    let about_p = |linear: DMat4| DMat4::from_translation(p) * linear * DMat4::from_translation(-p);
    let m = match delta {
        DragDelta::Translate(w) => DMat4::from_translation(w),
        DragDelta::Rotate { axis, angle } => about_p(DMat4::from_axis_angle(axis, angle)),
        DragDelta::Scale(f) => {
            let a = rotation_part(world);
            about_p(DMat4::from_mat3(
                a * DMat3::from_diagonal(f) * a.transpose(),
            ))
        }
    };
    world.inverse() * m * *world
}

/// The stored mesh of a polygon object.
fn stored_mesh(scene: &Scene, id: NodeId) -> Option<Arc<MeshData>> {
    match scene.param(id, "mesh") {
        Some(Value::Mesh(m)) if scene.node(id)?.type_id == POLYGON => Some(m.clone()),
        _ => None,
    }
}

/// The points the current mode's selection moves, with the stored mesh and its pivot.
/// `None` without a component target or with an empty set.
pub fn edit_set(s: &EditorState) -> Option<(NodeId, Arc<MeshData>, IndexSet, DVec3)> {
    let id = target(s)?;
    let scene = s.doc.scene();
    let mesh = stored_mesh(scene, id)?;
    let points = affected_points(&mesh, s.mode, &stored_selection(scene, id));
    let c = pivot(&mesh, &points)?;
    Some((id, mesh, points, c))
}

/// World position of the pivot (where the gizmo is drawn), if there is one.
pub fn pivot_world(s: &EditorState) -> Option<DVec3> {
    let (id, _, _, c) = edit_set(s)?;
    Some(s.doc.scene().world_matrix(id)?.transform_point3(c))
}

/// The pivot in object space (`world = false`) or world space, for the Coordinates panel.
pub fn pivot_view(s: &EditorState, world: bool) -> Option<DVec3> {
    let (id, _, _, c) = edit_set(s)?;
    if world {
        Some(s.doc.scene().world_matrix(id)?.transform_point3(c))
    } else {
        Some(c)
    }
}

/// A component drag in progress: everything is fixed at the start, each frame recomputes
/// the mesh from the start mesh (EDITABLE.md §3.5).
#[derive(Debug, Clone)]
pub struct ComponentDrag {
    pub id: NodeId,
    start: Arc<MeshData>,
    points: IndexSet,
    pivot: DVec3,
}

impl ComponentDrag {
    /// Starts a drag on the current selection; `None` if there is nothing to move.
    pub fn begin(s: &EditorState) -> Option<ComponentDrag> {
        let (id, start, points, pivot) = edit_set(s)?;
        Some(ComponentDrag {
            id,
            start,
            points,
            pivot,
        })
    }

    /// Sets the mesh to the start mesh changed by `delta` (one undo step per drag).
    pub fn apply(
        &self,
        doc: &mut Document,
        delta: DragDelta,
        gesture: Gesture,
    ) -> Result<(), EditError> {
        let world = doc
            .scene()
            .world_matrix(self.id)
            .ok_or(EditError::UnknownNode(self.id))?;
        let m = local_matrix(&world, self.pivot, delta);
        let mesh = Arc::new(transform_points(&self.start, &self.points, m));
        let label = match delta {
            DragDelta::Translate(_) => "Move",
            DragDelta::Rotate { .. } => "Rotate",
            DragDelta::Scale(_) => "Scale",
        };
        let id = self.id;
        let edit = |tx: &mut forge_core::Tx| tx.set_param(id, "mesh", Value::Mesh(mesh));
        match gesture {
            Gesture::Commit => doc.transact(label, edit),
            Gesture::Drag(n) => doc.transact_merged(label, &format!("gizmo:{id}:{n}"), edit),
        }
    }
}
