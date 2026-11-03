//! Gizmo drags → transform params, and object picking (docs/specs/MANIPULATE.md §3, §5).

use forge_core::transform::{Transform, hpb_from_rotation, rotation_part};
use forge_core::{Document, EditError, EvalCtx, NodeId, Scene, Tx};
use forge_generators::axis::AxisEdit;
use forge_viewport::camera::Camera;
use forge_viewport::gizmo::DragDelta;
use forge_viewport::picking::{pick_instances, pick_poly};
use glam::DMat3;

use crate::animate::AnimEdit;
use crate::attributes::Gesture;
use crate::viewport_panel::{DrawItem, instance_input, scene_draw_items_at};

pub(crate) use forge_app::coords::write_transform;

/// Sets the node's transform to `start` changed by `delta` (one undo step per drag).
pub fn apply_drag(
    doc: &mut Document,
    id: NodeId,
    start: &Transform,
    delta: DragDelta,
    gesture: Gesture,
) -> Result<(), EditError> {
    apply_drag_anim(doc, id, start, delta, gesture, None)
}

/// [`apply_drag`] on an animated scene: `start` and the parent matrix come from the
/// evaluated scene and the result is keyed per autokey / rule 5.
pub fn apply_drag_anim(
    doc: &mut Document,
    id: NodeId,
    start: &Transform,
    delta: DragDelta,
    gesture: Gesture,
    anim: Option<&AnimEdit>,
) -> Result<(), EditError> {
    let parent = anim
        .map_or(doc.scene(), |a| a.eval)
        .parent_world_matrix(id)
        .ok_or(EditError::UnknownNode(id))?;
    let (label, next) = dragged_transform(start, &parent, delta);
    let edit = |tx: &mut Tx| write_transform(tx, id, &next, anim);
    match gesture {
        Gesture::Commit => doc.transact(label, edit),
        Gesture::Drag(n) => doc.transact_merged(label, &format!("gizmo:{id}:{n}"), edit),
    }
}

/// The transform a gizmo `delta` makes of `start` under a parent with world matrix `parent`
/// (the rules of [`apply_drag`]) and the undo label.
pub fn dragged_transform(
    start: &Transform,
    parent: &glam::DMat4,
    delta: DragDelta,
) -> (&'static str, Transform) {
    let rp = rotation_part(parent);
    match delta {
        DragDelta::Translate(w) => (
            "Move",
            Transform {
                pos: start.pos + parent.inverse().transform_vector3(w),
                ..*start
            },
        ),
        DragDelta::Rotate { axis, angle } => {
            let r = rp.inverse() * DMat3::from_axis_angle(axis, angle) * rp * start.rotation();
            (
                "Rotate",
                Transform {
                    rot: hpb_from_rotation(r),
                    ..*start
                },
            )
        }
        DragDelta::Scale(f) => (
            "Scale",
            Transform {
                scale: start.scale * f,
                ..*start
            },
        ),
    }
}

/// Sets the axis of the captured object to the start axis changed by `delta`; the points and
/// children keep their world place (MANIPULATE.md §7.4). One undo step per drag.
pub fn apply_axis_drag(
    doc: &mut Document,
    edit: &AxisEdit,
    delta: DragDelta,
    gesture: Gesture,
) -> Result<(), EditError> {
    let id = edit.node;
    let parent = doc
        .scene()
        .parent_world_matrix(id)
        .ok_or(EditError::UnknownNode(id))?;
    let (label, next) = dragged_transform(&edit.start, &parent, delta);
    let label = format!("{label} axis");
    let run = |tx: &mut Tx| edit.apply(tx, &next);
    match gesture {
        Gesture::Commit => doc.transact(&label, run),
        Gesture::Drag(n) => doc.transact_merged(&label, &format!("axis:{id}:{n}"), run),
    }
}

/// What a pick hit: the object and, for an instanced cloner, the clone under the cursor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PickHit {
    pub node: NodeId,
    /// Clone index (into the cloner's clone data) when `node` is a cloner drawn by instancing.
    pub clone: Option<u32>,
}

/// The object nearest under the cursor; cloners are picked through their instances without
/// expanding the clone geometry (docs/specs/VIEWPORT.md §10.7).
pub fn pick_object(scene: &Scene, cam: &Camera, x: f64, y: f64, w: f64, h: f64) -> Option<PickHit> {
    let ray = cam.ray(x, y, w, h);
    scene_draw_items_at(scene, &Default::default(), &EvalCtx::default())
        .into_iter()
        .filter_map(|item| match item {
            DrawItem::Mesh { id, mesh, .. } => pick_poly(&mesh, &ray).map(|hit| (hit.t, id, None)),
            DrawItem::Instanced {
                id,
                instances,
                world,
                ..
            } => pick_instances(
                &instances.prototypes,
                &instance_input(&instances),
                &world,
                &ray,
            )
            .map(|hit| (hit.t, id, Some(hit.clone))),
        })
        .min_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)))
        .map(|(_, node, clone)| PickHit { node, clone })
}

/// The generator node whose world-space mesh (or instance) is nearest under the cursor.
pub fn pick_node(scene: &Scene, cam: &Camera, x: f64, y: f64, w: f64, h: f64) -> Option<NodeId> {
    pick_object(scene, cam, x, y, w, h).map(|hit| hit.node)
}
