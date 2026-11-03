//! Axis (pivot) editing: change an object's transform while its content stays where it is in
//! the world (docs/specs/MANIPULATE.md §7).

use std::sync::Arc;

use forge_core::transform::{POS, ROT, SCALE, Transform, hpb_from_rotation};
use forge_core::{EditError, MeshData, NodeId, Scene, Tx, Value};
use glam::{DMat3, DMat4, DVec3};

use crate::polygon::TYPE as POLYGON;

/// Rotation and scale params are rewritten only when they differ by more than this.
const KEEP: f64 = 1e-12;

/// `K = N⁻¹·O`: maps the old object space to the new one (MANIPULATE.md §7.1).
pub fn counter_matrix(old: &Transform, new: &Transform) -> DMat4 {
    new.matrix().inverse() * old.matrix()
}

/// Position, HPB rotation and scale of `m` (columns give the scale, negated on x when the
/// determinant is negative; exact for a rotation times a diagonal scale).
pub fn decompose(m: &DMat4) -> Transform {
    let m3 = DMat3::from_mat4(*m);
    let mut scale = DVec3::new(m3.x_axis.length(), m3.y_axis.length(), m3.z_axis.length());
    if m3.determinant() < 0.0 {
        scale.x = -scale.x;
    }
    let col = |c: DVec3, s: f64| if s == 0.0 { c } else { c / s };
    let rot = DMat3::from_cols(
        col(m3.x_axis, scale.x),
        col(m3.y_axis, scale.y),
        col(m3.z_axis, scale.z),
    );
    Transform {
        pos: m.w_axis.truncate(),
        rot: hpb_from_rotation(rot),
        scale,
    }
}

/// An object's axis at the start of an edit: its transform, stored points and children.
/// [`AxisEdit::apply`] recomputes everything from this start, so a drag never accumulates error.
#[derive(Debug, Clone)]
pub struct AxisEdit {
    pub node: NodeId,
    pub start: Transform,
    mesh: Option<Arc<MeshData>>,
    children: Vec<(NodeId, Transform)>,
}

impl AxisEdit {
    /// Captures `node` (not a tag). Children without transform params are left alone.
    pub fn capture(scene: &Scene, node: NodeId) -> Result<AxisEdit, EditError> {
        let n = scene
            .node(node)
            .filter(|n| !n.is_tag())
            .ok_or(EditError::UnknownNode(node))?;
        let mesh = match scene.param(node, "mesh") {
            Some(Value::Mesh(m)) if n.type_id == POLYGON => Some(m.clone()),
            _ => None,
        };
        let children = n
            .children
            .iter()
            .filter(|c| scene.param(**c, POS).is_some())
            .filter_map(|c| Some((*c, scene.local_transform(*c)?)))
            .collect();
        Ok(AxisEdit {
            node,
            start: scene
                .local_transform(node)
                .ok_or(EditError::UnknownNode(node))?,
            mesh,
            children,
        })
    }

    /// True when the object's own points are counter-transformed (polygon objects).
    pub fn has_points(&self) -> bool {
        self.mesh.is_some()
    }

    /// Sets the object's transform to `new`; the points and children are changed by
    /// `K = N⁻¹·O` so they keep their world place.
    pub fn apply(&self, tx: &mut Tx, new: &Transform) -> Result<(), EditError> {
        let k = counter_matrix(&self.start, new);
        write_transform(tx, self.node, new)?;
        if let Some(mesh) = &self.mesh {
            let moved = MeshData {
                points: mesh
                    .points
                    .iter()
                    .map(|p| k.transform_point3(DVec3::from_array(*p)).to_array())
                    .collect(),
                polys: mesh.polys.clone(),
            };
            tx.set_param(self.node, "mesh", Value::Mesh(Arc::new(moved)))?;
        }
        for (child, local) in &self.children {
            let next = decompose(&(k * local.matrix()));
            write_transform(tx, *child, &next)?;
        }
        Ok(())
    }
}

/// Writes `new` over what the node holds now: the position always, rotation and scale when
/// they differ from the stored values by more than 1e-12 (so a translation-only edit leaves
/// them bit for bit, also after earlier updates of the same drag).
fn write_transform(tx: &mut Tx, id: NodeId, new: &Transform) -> Result<(), EditError> {
    let now = tx
        .scene()
        .local_transform(id)
        .ok_or(EditError::UnknownNode(id))?;
    tx.set_param(id, POS, Value::Vec3(new.pos.to_array()))?;
    if (new.rot - now.rot).abs().max_element() > KEEP {
        tx.set_param(id, ROT, Value::Vec3(new.rot.to_array()))?;
    }
    if (new.scale - now.scale).abs().max_element() > KEEP {
        tx.set_param(id, SCALE, Value::Vec3(new.scale.to_array()))?;
    }
    Ok(())
}

/// The new transform for an axis moved by the object-space vector `c` (the centre of
/// something): position `+= O·c`, rotation and scale kept (`object.center_axis`).
pub fn shifted_axis(old: &Transform, c: DVec3) -> Transform {
    Transform {
        pos: old.pos + old.matrix().transform_vector3(c),
        ..*old
    }
}

/// `old` with its rotation replaced so the world rotation is `world_rot` under a parent whose
/// rotation part is `parent_rot`: `R = R_P⁻¹ · world_rot` (HPB).
pub fn with_world_rotation(old: &Transform, parent_rot: DMat3, world_rot: DMat3) -> Transform {
    Transform {
        rot: hpb_from_rotation(parent_rot.inverse() * world_rot),
        ..*old
    }
}
