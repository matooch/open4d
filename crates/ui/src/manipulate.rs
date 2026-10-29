//! Gizmo drags → transform params, and object picking (docs/specs/MANIPULATE.md §3, §5).

use forge_core::transform::{self, Transform, hpb_from_rotation, rotation_part};
use forge_core::{Document, EditError, NodeId, Scene, Tx, Value};
use forge_viewport::camera::Camera;
use forge_viewport::gizmo::DragDelta;
use forge_viewport::picking::pick_poly;
use glam::DMat3;

use crate::animate::{AnimEdit, unchanged, write_param};
use crate::attributes::Gesture;
use crate::viewport_panel::scene_meshes;

/// Writes the three transform params. With `anim` the params that already hold the
/// shown value are skipped and animated ones get keys (docs/specs/ANIM.md §2.4); without
/// it all three are set.
pub(crate) fn write_transform(
    tx: &mut Tx,
    id: NodeId,
    t: &Transform,
    anim: Option<&AnimEdit>,
) -> Result<(), EditError> {
    for (param, v) in [
        (transform::POS, t.pos),
        (transform::ROT, t.rot),
        (transform::SCALE, t.scale),
    ] {
        let value = Value::Vec3(v.to_array());
        if unchanged(anim, id, param, &value) {
            continue;
        }
        write_param(tx, id, param, value, anim.map(|a| &a.ctx))?;
    }
    Ok(())
}

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
    let rp = rotation_part(&parent);
    let (label, next) = match delta {
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
    };
    let edit = |tx: &mut Tx| write_transform(tx, id, &next, anim);
    match gesture {
        Gesture::Commit => doc.transact(label, edit),
        Gesture::Drag(n) => doc.transact_merged(label, &format!("gizmo:{id}:{n}"), edit),
    }
}

/// The generator node whose world-space mesh is nearest under the cursor.
pub fn pick_node(scene: &Scene, cam: &Camera, x: f64, y: f64, w: f64, h: f64) -> Option<NodeId> {
    let ray = cam.ray(x, y, w, h);
    scene_meshes(scene, &Default::default())
        .into_iter()
        .filter_map(|(id, mesh, _, _)| pick_poly(&mesh, &ray).map(|hit| (hit.t, id)))
        .min_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)))
        .map(|(_, id)| id)
}
