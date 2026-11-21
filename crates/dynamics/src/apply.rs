//! Writing simulated poses back into a scene (docs/specs/DYNAMICS.md §5.1).

use std::collections::BTreeMap;

use forge_core::{EvalCtx, NodeId, Scene, Value, transform};
use forge_generators::clone_data;
use glam::{DMat4, DVec3};

use crate::sim::BodyKey;

/// Parameter overrides that move every simulated object to its pose: position and rotation
/// of the local transform (the parent's world matrix is divided out), scale untouched. The
/// clones of a cloner go into its `dynamics_pos` / `dynamics_rot` lists (§6). Apply with
/// `Scene::apply_params`.
pub fn overrides(scene: &Scene, poses: &BTreeMap<BodyKey, DMat4>) -> Vec<(NodeId, String, Value)> {
    let mut out = Vec::new();
    let mut cloners: BTreeMap<NodeId, Vec<(u32, DMat4)>> = BTreeMap::new();
    for (key, world) in poses {
        let id = match key {
            BodyKey::Node(id) => id,
            BodyKey::Clone { cloner, index } => {
                cloners.entry(*cloner).or_default().push((*index, *world));
                continue;
            }
        };
        let Some(parent) = scene.parent_world_matrix(*id) else {
            continue;
        };
        let local = parent.inverse() * *world;
        let pos: DVec3 = local.w_axis.truncate();
        let rot = transform::hpb_from_rotation(transform::rotation_part(&local));
        out.push((*id, transform::POS.to_string(), Value::Vec3(pos.to_array())));
        out.push((*id, transform::ROT.to_string(), Value::Vec3(rot.to_array())));
    }
    for (cloner, clones) in cloners {
        out.extend(clone_overrides(scene, cloner, &clones));
    }
    out
}

/// The `dynamics_pos` / `dynamics_rot` lists of a cloner: the simulated clones at their pose
/// (cloner space), every other clone where the cloner generates it.
fn clone_overrides(
    scene: &Scene,
    cloner: NodeId,
    simulated: &[(u32, DMat4)],
) -> Vec<(NodeId, String, Value)> {
    let ctx = EvalCtx {
        time: 0.0,
        fps: f64::from(scene.settings().fps),
        seed: 0,
    };
    let Ok(Some(data)) = clone_data(scene, cloner, &ctx) else {
        return Vec::new();
    };
    let Some(world) = scene.world_matrix(cloner) else {
        return Vec::new();
    };
    let inverse = world.inverse();
    let split = |m: &DMat4| {
        let pos: DVec3 = m.w_axis.truncate();
        let rot = transform::hpb_from_rotation(transform::rotation_part(m));
        (pos.to_array(), rot.to_array())
    };
    let mut pos = Vec::with_capacity(data.len());
    let mut rot = Vec::with_capacity(data.len());
    for m in &data.matrices {
        let (p, r) = split(m);
        pos.push(p);
        rot.push(r);
    }
    for (index, pose) in simulated {
        let i = *index as usize;
        if i < pos.len() {
            (pos[i], rot[i]) = split(&(inverse * *pose));
        }
    }
    vec![
        (cloner, "dynamics_pos".to_string(), Value::Vec3List(pos)),
        (cloner, "dynamics_rot".to_string(), Value::Vec3List(rot)),
    ]
}

/// The `soft_points` overrides of soft bodies: their simulated particles (world space, cm) in
/// the object's own space, on the object's rigid body tag. Apply with `Scene::apply_params`.
pub fn soft_overrides(
    scene: &Scene,
    soft: &BTreeMap<BodyKey, Vec<DVec3>>,
) -> Vec<(NodeId, String, Value)> {
    let mut out = Vec::new();
    for (key, points) in soft {
        let BodyKey::Node(id) = key else { continue };
        let (Some(tag), Some(world)) =
            (crate::build::body_tag(scene, *id), scene.world_matrix(*id))
        else {
            continue;
        };
        let inverse = world.inverse();
        let local = points
            .iter()
            .map(|p| inverse.transform_point3(*p).to_array())
            .collect();
        out.push((tag, "soft_points".to_string(), Value::Vec3List(local)));
    }
    out
}
