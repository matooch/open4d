//! Bone display geometry (docs/specs/RIG.md §1.4): what the viewport draws for a joint.

use forge_core::{NodeId, Scene, Value};
use glam::DVec3;

use crate::joint::{bone_vector, first_child_joint, is_joint};

/// The bone of one joint in world space.
#[derive(Debug, Clone, PartialEq)]
pub struct Bone {
    /// The joint's world position.
    pub start: DVec3,
    /// The first child joint's position, or the end of the bone vector.
    pub end: DVec3,
    /// Octahedron points: start apex, four waist points, end apex.
    pub octahedron: [DVec3; 6],
}

/// The eight triangles of the octahedron points (indices into `Bone::octahedron`).
pub const OCTAHEDRON_FACES: [[u32; 3]; 8] = [
    [0, 1, 2],
    [0, 2, 3],
    [0, 3, 4],
    [0, 4, 1],
    [5, 2, 1],
    [5, 3, 2],
    [5, 4, 3],
    [5, 1, 4],
];

/// An octahedron around `a`..`b`: apexes at both ends, the waist at 0.1 of the length from
/// `a` with radius 0.1 of the length. `None` for a zero-length bone.
pub fn octahedron(a: DVec3, b: DVec3) -> Option<[DVec3; 6]> {
    let axis = b - a;
    let len = axis.length();
    if len < 1e-12 {
        return None;
    }
    let dir = axis / len;
    let helper = if dir.x.abs() < 0.9 {
        DVec3::X
    } else {
        DVec3::Y
    };
    let u = dir.cross(helper).normalize();
    let v = dir.cross(u);
    let c = a + dir * (0.1 * len);
    let r = 0.1 * len;
    Some([a, c + u * r, c + v * r, c - u * r, c - v * r, b])
}

/// The bone of `joint`, or `None` for a node that is not a joint, a joint whose display is
/// `none`, or a zero-length bone.
pub fn bone_display(scene: &Scene, joint: NodeId) -> Option<Bone> {
    if !is_joint(scene, joint) {
        return None;
    }
    if matches!(scene.param(joint, "display_object"), Some(Value::Enum(d)) if d == "none") {
        return None;
    }
    let world = scene.world_matrix(joint)?;
    let start = world.w_axis.truncate();
    let end = match first_child_joint(scene, joint).and_then(|c| scene.world_matrix(c)) {
        Some(c) => c.w_axis.truncate(),
        None => world.transform_point3(bone_vector(scene, joint)),
    };
    Some(Bone {
        start,
        end,
        octahedron: octahedron(start, end)?,
    })
}
