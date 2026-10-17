//! Coordinate panel model: local/world values and scale/size (docs/specs/MANIPULATE.md §4).

use forge_core::transform::{Transform, hpb_from_rotation, rotation_part};
use forge_core::{Document, EditError, NodeId, Scene};
use glam::{DMat4, DVec3};

use crate::attributes::Gesture;
use crate::manipulate::write_transform;

/// `Local` corresponds to the reference coordinate manager's "Object (Rel)".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Space {
    #[default]
    Local,
    World,
}

/// What the third row shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Third {
    #[default]
    Scale,
    Size,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Row {
    Position,
    Rotation,
    Third(Third),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CoordView {
    pub pos: DVec3,
    /// `(h, p, b)` radians.
    pub rot: DVec3,
    /// `None` for Size when the node has no mesh.
    pub third: Option<DVec3>,
}

fn column_lengths(m: &DMat4) -> DVec3 {
    DVec3::new(
        m.x_axis.truncate().length(),
        m.y_axis.truncate().length(),
        m.z_axis.truncate().length(),
    )
}

/// Bounding-box size of a generator node's local mesh.
fn mesh_size(scene: &Scene, id: NodeId) -> Option<DVec3> {
    let n = scene.node(id)?;
    let mesh = forge_generators::build(&n.type_id, &n.params).ok()?;
    let first = *mesh.points.first()?;
    let (lo, hi) = mesh
        .points
        .iter()
        .fold((first, first), |(lo, hi), p| (lo.min(*p), hi.max(*p)));
    Some(hi - lo)
}

pub fn coord_view(scene: &Scene, id: NodeId, space: Space, third: Third) -> Option<CoordView> {
    let local = scene.local_transform(id)?;
    let world = scene.world_matrix(id)?;
    let (pos, rot, scale) = match space {
        Space::Local => (local.pos, local.rot, local.scale),
        Space::World => (
            world.w_axis.truncate(),
            hpb_from_rotation(rotation_part(&world)),
            column_lengths(&world),
        ),
    };
    let third = match third {
        Third::Scale => Some(scale),
        Third::Size => mesh_size(scene, id).map(|s| s * scale),
    };
    Some(CoordView { pos, rot, third })
}

pub fn set_coord(
    doc: &mut Document,
    id: NodeId,
    space: Space,
    row: Row,
    value: DVec3,
    gesture: Gesture,
) -> Result<(), EditError> {
    let scene = doc.scene();
    let start = scene
        .local_transform(id)
        .ok_or(EditError::UnknownNode(id))?;
    let parent = scene
        .parent_world_matrix(id)
        .ok_or(EditError::UnknownNode(id))?;
    let parent_scale = column_lengths(&parent);
    // Per-axis divide that keeps the current value where the divisor is zero.
    let ratio = |num: DVec3, den: DVec3, keep: DVec3| {
        DVec3::from_array(std::array::from_fn(|k| {
            if den[k].abs() > 1e-12 {
                num[k] / den[k]
            } else {
                keep[k]
            }
        }))
    };
    let next = match (space, row) {
        (Space::Local, Row::Position) => Transform {
            pos: value,
            ..start
        },
        (Space::Local, Row::Rotation) => Transform {
            rot: value,
            ..start
        },
        (Space::Local, Row::Third(Third::Scale)) => Transform {
            scale: value,
            ..start
        },
        (Space::World, Row::Position) => Transform {
            pos: parent.inverse().transform_point3(value),
            ..start
        },
        (Space::World, Row::Rotation) => {
            let wanted = Transform {
                rot: value,
                ..Transform::IDENTITY
            }
            .rotation();
            Transform {
                rot: hpb_from_rotation(rotation_part(&parent).inverse() * wanted),
                ..start
            }
        }
        (Space::World, Row::Third(Third::Scale)) => Transform {
            scale: ratio(value, parent_scale, start.scale),
            ..start
        },
        (_, Row::Third(Third::Size)) => {
            let size = mesh_size(scene, id).unwrap_or(DVec3::ZERO);
            let scale = ratio(value, size, start.scale);
            let scale = if space == Space::World {
                ratio(scale, parent_scale, start.scale)
            } else {
                scale
            };
            Transform { scale, ..start }
        }
    };
    let edit = |tx: &mut forge_core::Tx| write_transform(tx, id, &next);
    match gesture {
        Gesture::Commit => doc.transact("Set coordinates", edit),
        Gesture::Drag(n) => {
            doc.transact_merged("Set coordinates", &format!("coords:{id}:{n}"), edit)
        }
    }
}
