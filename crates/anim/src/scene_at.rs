//! Animated scenes (docs/specs/ANIM.md §1.5).

use forge_core::{NodeId, Scene, Target, Value, parse_target};

use crate::value_at;

/// A copy of `scene` whose animated parameters hold their values at `time` (seconds).
/// Vec3 components without a track keep the stored value. Tracks that no longer address
/// a valid parameter are ignored.
pub fn scene_at(scene: &Scene, time: f64) -> Scene {
    scene.with_params(overrides(scene, time)).0
}

fn overrides(scene: &Scene, time: f64) -> Vec<(NodeId, String, Value)> {
    let mut out: Vec<(NodeId, String, Value)> = Vec::new();
    for id in all_nodes(scene) {
        let Some(node) = scene.node(id) else { continue };
        let Some(ty) = scene.node_type(id) else {
            continue;
        };
        for (key, track) in &node.tracks {
            let Ok(target) = parse_target(key, |p| ty.find_param(p)) else {
                continue;
            };
            let v = value_at(track, time);
            match target {
                Target::Float(p) => out.push((id, p, Value::Float(v))),
                Target::Component(p, axis) => {
                    // Start from the value an earlier component of this node produced.
                    let base = out
                        .iter()
                        .rev()
                        .find(|(i, q, _)| *i == id && *q == p)
                        .map(|(_, _, v)| v.clone())
                        .or_else(|| scene.param(id, &p).cloned());
                    if let Some(Value::Vec3(mut c)) = base {
                        c[axis] = v;
                        out.push((id, p, Value::Vec3(c)));
                    }
                }
            }
        }
    }
    out
}

/// Every node, tags included.
fn all_nodes(scene: &Scene) -> Vec<NodeId> {
    let mut ids = Vec::new();
    let mut stack: Vec<NodeId> = scene.roots().iter().rev().copied().collect();
    while let Some(id) = stack.pop() {
        ids.push(id);
        if let Some(n) = scene.node(id) {
            stack.extend(n.children.iter().rev());
            stack.extend(n.tags.iter().rev());
        }
    }
    ids
}
