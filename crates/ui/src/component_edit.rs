//! Moving, rotating and scaling components (docs/specs/EDITABLE.md §3). UI-free: the egui
//! code in `app` only turns pointer input into these calls.

use std::sync::Arc;

use forge_core::transform::rotation_part;
use forge_core::{Document, EditError, MeshData, NodeId, Scene, Value};
use forge_generators::{POLYGON, to_poly_mesh};
use forge_geometry::selection::{IndexSet, Selection, edges_to_points, polys_to_points};
use forge_modeling::ComponentPlan;
use forge_viewport::gizmo::DragDelta;
use glam::{DMat3, DMat4, DVec3};

use crate::attributes::Gesture;
use crate::components::{EditMode, stored_selection, targets};
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

/// `mesh` with `m` applied through a soft selection / symmetry `plan` (EDITABLE.md §5.3).
pub fn transform_points_planned(mesh: &MeshData, plan: &ComponentPlan, m: DMat4) -> MeshData {
    let start: Vec<DVec3> = mesh.points.iter().map(|p| DVec3::from_array(*p)).collect();
    let mut out = mesh.clone();
    for (slot, p) in out.points.iter_mut().zip(plan.apply(&start, m)) {
        *slot = p.to_array();
    }
    out
}

/// The world-space matrix of a gizmo `delta` about the world point `p`; `frame` is the world
/// matrix whose axes a Scale follows (EDITABLE.md §3.4, §6.4).
pub fn world_delta_matrix(frame: &DMat4, p: DVec3, delta: DragDelta) -> DMat4 {
    let about_p = |linear: DMat4| DMat4::from_translation(p) * linear * DMat4::from_translation(-p);
    match delta {
        DragDelta::Translate(w) => DMat4::from_translation(w),
        DragDelta::Rotate { axis, angle } => about_p(DMat4::from_axis_angle(axis, angle)),
        DragDelta::Scale(f) => {
            let a = rotation_part(frame);
            about_p(DMat4::from_mat3(
                a * DMat3::from_diagonal(f) * a.transpose(),
            ))
        }
    }
}

/// The object-space matrix for a world-space gizmo `delta` about the pivot `c` (object
/// space) of a node with world matrix `world`: `W⁻¹ · M · W` (EDITABLE.md §3.4).
pub fn local_matrix(world: &DMat4, c: DVec3, delta: DragDelta) -> DMat4 {
    let m = world_delta_matrix(world, world.transform_point3(c), delta);
    world.inverse() * m * *world
}

/// The stored mesh of a polygon object.
fn stored_mesh(scene: &Scene, id: NodeId) -> Option<Arc<MeshData>> {
    match scene.param(id, "mesh") {
        Some(Value::Mesh(m)) if scene.node(id)?.type_id == POLYGON => Some(m.clone()),
        _ => None,
    }
}

/// One target of a component edit: its stored mesh and the points the selection moves.
#[derive(Debug, Clone)]
pub struct EditPart {
    pub id: NodeId,
    pub mesh: Arc<MeshData>,
    pub points: IndexSet,
}

/// Every component target with a non-empty set of affected points, in ascending id order
/// (EDITABLE.md §6.1, §6.4). Empty without a target.
pub fn edit_parts(s: &EditorState) -> Vec<EditPart> {
    let scene = s.doc.scene();
    targets(s)
        .into_iter()
        .filter_map(|id| {
            let mesh = stored_mesh(scene, id)?;
            let points = affected_points(&mesh, s.mode, &stored_selection(scene, id));
            (!points.is_empty()).then_some(EditPart { id, mesh, points })
        })
        .collect()
}

/// The mean world position of every affected point of every part (`world` gives each
/// part's world matrix), EDITABLE.md §6.4.
fn world_centroid(parts: &[EditPart], world: impl Fn(NodeId) -> Option<DMat4>) -> Option<DVec3> {
    let (mut sum, mut n) = (DVec3::ZERO, 0usize);
    for part in parts {
        let w = world(part.id)?;
        for i in part.points.iter() {
            sum += w.transform_point3(DVec3::from_array(*part.mesh.points.get(i as usize)?));
            n += 1;
        }
    }
    (n > 0).then(|| sum / n as f64)
}

/// The points the current mode's selection moves, with the stored mesh and its pivot (the
/// first target, kept for single-object callers). `None` without a target or with an empty
/// set.
pub fn edit_set(s: &EditorState) -> Option<(NodeId, Arc<MeshData>, IndexSet, DVec3)> {
    let part = edit_parts(s).into_iter().next()?;
    let c = pivot(&part.mesh, &part.points)?;
    Some((part.id, part.mesh, part.points, c))
}

/// World position of the pivot (where the gizmo is drawn), if there is one: the mean world
/// position of the affected points of all targets.
pub fn pivot_world(s: &EditorState) -> Option<DVec3> {
    let parts = edit_parts(s);
    let eval = s.evaluated();
    world_centroid(&parts, |id| eval.world_matrix(id))
}

/// The pivot in object space (`world = false`, one target) or world space, for the
/// Coordinates panel. With several targets it is always the world pivot.
pub fn pivot_view(s: &EditorState, world: bool) -> Option<DVec3> {
    if world || edit_parts(s).len() > 1 {
        pivot_world(s)
    } else {
        edit_set(s).map(|(_, _, _, c)| c)
    }
}

/// What the Coordinates panel shows for the component selection (EDITABLE.md §6.7).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ComponentCoords {
    pub pivot: DVec3,
    /// Extent of the bounding box of the affected points.
    pub size: DVec3,
}

/// The pivot and size of the selection. `world = false` with one target is object space;
/// otherwise world space (the size is then the world axis-aligned extent).
pub fn component_coords(s: &EditorState, world: bool) -> Option<ComponentCoords> {
    let parts = edit_parts(s);
    let object = !world && parts.len() == 1;
    let eval = s.evaluated();
    let (mut lo, mut hi) = (DVec3::splat(f64::MAX), DVec3::splat(f64::MIN));
    for part in &parts {
        let w = if object {
            DMat4::IDENTITY
        } else {
            eval.world_matrix(part.id)?
        };
        for i in part.points.iter() {
            let p = w.transform_point3(DVec3::from_array(part.mesh.points[i as usize]));
            lo = lo.min(p);
            hi = hi.max(p);
        }
    }
    (!parts.is_empty()).then(|| ComponentCoords {
        pivot: pivot_view(s, world).unwrap_or(DVec3::ZERO),
        size: hi - lo,
    })
}

/// A typed value of the Coordinates panel (EDITABLE.md §6.7).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CoordEntry {
    /// The new pivot position.
    Position(DVec3),
    /// A rotation about the pivot, `(h, p, b)` radians.
    Rotation(DVec3),
    /// The new extent of the selection.
    Size(DVec3),
}

/// One target of a drag, fixed at its start.
#[derive(Debug, Clone)]
struct DragPart {
    id: NodeId,
    start: Arc<MeshData>,
    points: IndexSet,
    /// The pivot in this object's space (the mean of its points).
    pivot: DVec3,
    /// Soft selection weights and mirror partners, fixed at the start; `None` when both
    /// features are off (the plain §3.3 path). Built per object (EDITABLE.md §6.4).
    plan: Option<ComponentPlan>,
}

/// A component drag in progress: everything is fixed at the start, each frame recomputes
/// the meshes from the start meshes (EDITABLE.md §3.5, §6.4).
#[derive(Debug, Clone)]
pub struct ComponentDrag {
    /// The first target.
    pub id: NodeId,
    parts: Vec<DragPart>,
    /// The common world pivot.
    pivot: DVec3,
    /// The object whose axes a Scale follows: the active target, else the first.
    frame: NodeId,
}

impl ComponentDrag {
    /// Starts a drag on the current selection; `None` if there is nothing to move.
    pub fn begin(s: &EditorState) -> Option<ComponentDrag> {
        let parts = edit_parts(s);
        let settings = &s.app.component_settings;
        let world = |id| s.doc.scene().world_matrix(id);
        let pivot = world_centroid(&parts, world)?;
        let frame = s
            .selection
            .active
            .filter(|a| parts.iter().any(|p| p.id == *a))
            .or_else(|| parts.first().map(|p| p.id))?;
        let drag_parts: Vec<DragPart> = parts
            .into_iter()
            .map(|p| {
                let c = self::pivot(&p.mesh, &p.points)?;
                let plan = (settings.soft.enabled || settings.symmetry.enabled).then(|| {
                    let poly = to_poly_mesh(&p.mesh);
                    let selected: Vec<u32> = p.points.iter().collect();
                    ComponentPlan::new(&poly, &selected, settings)
                });
                Some(DragPart {
                    id: p.id,
                    start: p.mesh,
                    points: p.points,
                    pivot: c,
                    plan,
                })
            })
            .collect::<Option<_>>()?;
        Some(ComponentDrag {
            id: drag_parts.first()?.id,
            parts: drag_parts,
            pivot,
            frame,
        })
    }

    /// The selected points the drag moves, per object (they are not snap targets,
    /// MANIPULATE.md §6.7).
    pub fn moved(&self) -> Vec<(NodeId, Vec<u32>)> {
        self.parts
            .iter()
            .map(|p| (p.id, p.points.iter().collect()))
            .collect()
    }

    /// The pivot of one part in world space: the common pivot, or for a single target the
    /// mean of its points.
    fn world_pivot_of(&self, part: &DragPart, world: &DMat4) -> DVec3 {
        if self.parts.len() == 1 {
            world.transform_point3(part.pivot)
        } else {
            self.pivot
        }
    }

    /// Writes every part's mesh: its start mesh changed by `local(part, world)`, in one
    /// undo step.
    fn write(
        &self,
        doc: &mut Document,
        label: &str,
        key: &str,
        gesture: Gesture,
        local: impl Fn(&DragPart, &DMat4) -> DMat4,
    ) -> Result<(), EditError> {
        let mut meshes = Vec::new();
        for part in &self.parts {
            let world = doc
                .scene()
                .world_matrix(part.id)
                .ok_or(EditError::UnknownNode(part.id))?;
            let m = local(part, &world);
            meshes.push((
                part.id,
                Arc::new(match &part.plan {
                    Some(plan) => transform_points_planned(&part.start, plan, m),
                    None => transform_points(&part.start, &part.points, m),
                }),
            ));
        }
        let edit = |tx: &mut forge_core::Tx| {
            for (id, mesh) in &meshes {
                tx.set_param(*id, "mesh", Value::Mesh(mesh.clone()))?;
            }
            Ok(())
        };
        match gesture {
            Gesture::Commit => doc.transact(label, edit),
            Gesture::Drag(n) => doc.transact_merged(label, &format!("{key}:{n}"), edit),
        }
    }

    /// Sets the meshes to the start meshes changed by `delta` (one undo step per drag).
    pub fn apply(
        &self,
        doc: &mut Document,
        delta: DragDelta,
        gesture: Gesture,
    ) -> Result<(), EditError> {
        let frame = doc
            .scene()
            .world_matrix(self.frame)
            .ok_or(EditError::UnknownNode(self.frame))?;
        let label = match delta {
            DragDelta::Translate(_) => "Move",
            DragDelta::Rotate { .. } => "Rotate",
            DragDelta::Scale(_) => "Scale",
        };
        let key = format!("gizmo:{}", self.id);
        self.write(doc, label, &key, gesture, |part, world| {
            let p = self.world_pivot_of(part, world);
            let single_frame = if self.parts.len() == 1 { world } else { &frame };
            world.inverse() * world_delta_matrix(single_frame, p, delta) * *world
        })
    }

    /// The pivot and size at the start of the drag (see [`component_coords`]); `object`
    /// selects object space, which needs a single target.
    fn start_coords(&self, doc: &Document, object: bool) -> Option<(DVec3, DVec3)> {
        let (mut lo, mut hi) = (DVec3::splat(f64::MAX), DVec3::splat(f64::MIN));
        for part in &self.parts {
            let w = if object {
                DMat4::IDENTITY
            } else {
                doc.scene().world_matrix(part.id)?
            };
            for i in part.points.iter() {
                let p = w.transform_point3(DVec3::from_array(part.start.points[i as usize]));
                lo = lo.min(p);
                hi = hi.max(p);
            }
        }
        let pivot = if object {
            self.parts.first()?.pivot
        } else {
            self.parts
                .first()
                .filter(|_| self.parts.len() == 1)
                .and_then(|p| Some(doc.scene().world_matrix(p.id)?.transform_point3(p.pivot)))
                .unwrap_or(self.pivot)
        };
        Some((pivot, hi - lo))
    }

    /// Applies a typed Coordinates-panel value (EDITABLE.md §6.7). `world = false` with one
    /// target works in the object's space, otherwise in world space.
    pub fn apply_entry(
        &self,
        doc: &mut Document,
        entry: CoordEntry,
        world: bool,
        gesture: Gesture,
    ) -> Result<(), EditError> {
        let object = !world && self.parts.len() == 1;
        let (pivot, size) = self
            .start_coords(doc, object)
            .ok_or(EditError::UnknownNode(self.id))?;
        let about = |linear: DMat4| {
            DMat4::from_translation(pivot) * linear * DMat4::from_translation(-pivot)
        };
        let m = match entry {
            CoordEntry::Position(v) => DMat4::from_translation(v - pivot),
            CoordEntry::Rotation(hpb) => about(DMat4::from_mat3(
                forge_core::transform::Transform {
                    rot: hpb,
                    ..forge_core::transform::Transform::IDENTITY
                }
                .rotation(),
            )),
            CoordEntry::Size(v) => about(DMat4::from_scale(DVec3::from_array(
                std::array::from_fn(|k| {
                    if size[k].abs() > 1e-12 {
                        v[k] / size[k]
                    } else {
                        1.0
                    }
                }),
            ))),
        };
        let key = format!("compcoords:{}", self.id);
        self.write(doc, "Set coordinates", &key, gesture, |_, w| {
            if object { m } else { w.inverse() * m * *w }
        })
    }
}
