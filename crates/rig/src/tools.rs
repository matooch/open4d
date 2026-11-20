//! Joint tools: chain creation and align (docs/specs/RIG.md §1.3).

use forge_core::transform::{Transform, hpb_from_rotation};
use forge_core::{EditError, NodeId, Tx, Value};
use glam::{DMat3, DMat4, DQuat, DVec3};

use crate::joint::{Axis, JOINT, first_child_joint, is_joint};

/// Splits an affine matrix into position, rotation (h, p, b) and scale (column lengths).
/// Shear is dropped.
pub fn decompose(m: &DMat4) -> Transform {
    let c0 = m.x_axis.truncate();
    let c1 = m.y_axis.truncate();
    let c2 = m.z_axis.truncate();
    let scale = DVec3::new(c0.length(), c1.length(), c2.length());
    let rot = DMat3::from_cols(
        c0.normalize_or_zero(),
        c1.normalize_or_zero(),
        c2.normalize_or_zero(),
    );
    Transform {
        pos: m.w_axis.truncate(),
        rot: hpb_from_rotation(rot),
        scale,
    }
}

/// Writes a transform to a node's three transform params.
pub fn set_transform(tx: &mut Tx<'_>, id: NodeId, t: &Transform) -> Result<(), EditError> {
    tx.set_param(
        id,
        forge_core::transform::POS,
        Value::Vec3(t.pos.to_array()),
    )?;
    tx.set_param(
        id,
        forge_core::transform::ROT,
        Value::Vec3(t.rot.to_array()),
    )?;
    tx.set_param(
        id,
        forge_core::transform::SCALE,
        Value::Vec3(t.scale.to_array()),
    )
}

/// Creates a chain of joints through `points` (positions in `parent`'s space): the first joint
/// at `points[0]`, each next one a child of the previous at the difference of the points. The
/// joints are named `prefix1`, `prefix2`, ... With `align = Some(axis)`, every joint but the last
/// turns that local axis to its child (see [`align_joint`]). Returns the ids, root first.
///
/// # Errors
/// Fails when a node cannot be created or edited.
pub fn create_chain(
    tx: &mut Tx<'_>,
    parent: Option<NodeId>,
    points: &[DVec3],
    prefix: &str,
    align: Option<Axis>,
) -> Result<Vec<NodeId>, EditError> {
    let mut ids: Vec<NodeId> = Vec::new();
    let mut prev: Option<DVec3> = None;
    for (i, p) in points.iter().enumerate() {
        let id = tx.create(JOINT, ids.last().copied().or(parent), None)?;
        tx.rename(id, &format!("{prefix}{}", i + 1))?;
        let local = *p - prev.unwrap_or(DVec3::ZERO);
        tx.set_param(
            id,
            forge_core::transform::POS,
            Value::Vec3(local.to_array()),
        )?;
        prev = Some(*p);
        ids.push(id);
    }
    if let Some(axis) = align {
        for id in ids.iter().take(ids.len().saturating_sub(1)) {
            align_joint(tx, *id, axis)?;
        }
    }
    Ok(ids)
}

/// Turns `joint` so its local `axis` points at its first child joint, by the shortest rotation
/// from the current orientation (twist is kept). Every child keeps its world matrix. Does
/// nothing without a child joint or when the child sits on the joint.
///
/// # Errors
/// Fails when the joint does not exist.
pub fn align_joint(tx: &mut Tx<'_>, joint: NodeId, axis: Axis) -> Result<(), EditError> {
    let scene = tx.scene();
    if !is_joint(scene, joint) {
        return Err(EditError::UnknownNode(joint));
    }
    let Some(child) = first_child_joint(scene, joint) else {
        return Ok(());
    };
    let (Some(world), Some(child_world), Some(parent_world)) = (
        scene.world_matrix(joint),
        scene.world_matrix(child),
        scene.parent_world_matrix(joint),
    ) else {
        return Ok(());
    };
    let dir = child_world.w_axis.truncate() - world.w_axis.truncate();
    let current = world.transform_vector3(axis.unit());
    if dir.length() < 1e-12 || current.length() < 1e-12 {
        return Ok(());
    }
    let delta = DQuat::from_rotation_arc(current.normalize(), dir.normalize());
    let pos = world.w_axis.truncate();
    let rotated = DMat4::from_translation(pos)
        * DMat4::from_quat(delta)
        * DMat4::from_translation(-pos)
        * world;
    let kids: Vec<(NodeId, DMat4)> = scene
        .node(joint)
        .map(|n| n.children.clone())
        .unwrap_or_default()
        .into_iter()
        .filter_map(|c| scene.world_matrix(c).map(|m| (c, m)))
        .collect();
    set_transform(tx, joint, &decompose(&(parent_world.inverse() * rotated)))?;
    // Recompute the children from the transform that was stored, not from `rotated`, so a
    // dropped shear cannot move them.
    let stored = tx.scene().world_matrix(joint).unwrap_or(rotated);
    for (c, m) in kids {
        set_transform(tx, c, &decompose(&(stored.inverse() * m)))?;
    }
    Ok(())
}
