//! Writing simulated poses back into a scene (docs/specs/DYNAMICS.md §5.1).

use std::collections::BTreeMap;

use forge_core::{NodeId, Scene, Value, transform};
use glam::{DMat4, DVec3};

use crate::sim::BodyKey;

/// Parameter overrides that move every simulated object to its pose: position and rotation
/// of the local transform (the parent's world matrix is divided out), scale untouched. Clone
/// bodies are not written here (they live in clone data, §6). Apply with
/// `Scene::apply_params`.
pub fn overrides(scene: &Scene, poses: &BTreeMap<BodyKey, DMat4>) -> Vec<(NodeId, String, Value)> {
    let mut out = Vec::new();
    for (key, world) in poses {
        let BodyKey::Node(id) = key else { continue };
        let Some(parent) = scene.parent_world_matrix(*id) else {
            continue;
        };
        let local = parent.inverse() * *world;
        let pos: DVec3 = local.w_axis.truncate();
        let rot = transform::hpb_from_rotation(transform::rotation_part(&local));
        out.push((*id, transform::POS.to_string(), Value::Vec3(pos.to_array())));
        out.push((*id, transform::ROT.to_string(), Value::Vec3(rot.to_array())));
    }
    out
}
