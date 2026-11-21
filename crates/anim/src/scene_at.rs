//! Animated scenes (docs/specs/ANIM.md §1.5).

use std::sync::Arc;

use forge_core::{MeshData, NodeId, ParamKind, Scene, TrackTarget, Value, parse_track_target};

use crate::{int_at, points_at, step_at, value_at};

/// A copy of `scene` whose animated parameters hold their values at `time` (seconds).
/// Vec3 components without a track keep the stored value. Tracks that no longer address
/// a valid parameter are ignored.
pub fn scene_at(scene: &Scene, time: f64) -> Scene {
    scene
        .with_params(overrides(scene, &track_nodes(scene), time))
        .0
}

/// The nodes that own at least one track, in document order.
pub fn track_nodes(scene: &Scene) -> Vec<NodeId> {
    all_nodes(scene)
        .into_iter()
        .filter(|id| scene.node(*id).is_some_and(|n| !n.tracks.is_empty()))
        .collect()
}

/// The parameter values the tracks of `ids` give at `time` (seconds), in the order
/// [`scene_at`] applies them; `scene` holds the stored values. Applying them to a copy of
/// `scene` with `Scene::apply_params` is exactly `scene_at` when `ids` is [`track_nodes`].
pub fn overrides(scene: &Scene, ids: &[NodeId], time: f64) -> Vec<(NodeId, String, Value)> {
    let mut out: Vec<(NodeId, String, Value)> = Vec::new();
    for &id in ids {
        let Some(node) = scene.node(id) else { continue };
        let Some(ty) = scene.node_type(id) else {
            continue;
        };
        // a layer with animation off keeps the stored values (CORE.md §7.4)
        if !forge_core::layers::animation_on(scene, id) {
            continue;
        }
        for (key, track) in &node.tracks {
            let Ok(target) = parse_track_target(key, |p| ty.find_param(p)) else {
                continue;
            };
            // the value an earlier component or channel of this node produced, else the stored one
            let base = |out: &[(NodeId, String, Value)], p: &str| {
                out.iter()
                    .rev()
                    .find(|(i, q, _)| *i == id && q == p)
                    .map(|(_, _, v)| v.clone())
                    .or_else(|| scene.param(id, p).cloned())
            };
            match target {
                TrackTarget::Float(p) => out.push((id, p, Value::Float(value_at(track, time)))),
                TrackTarget::Int(p) => out.push((id, p, Value::Int(int_at(track, time, false)))),
                TrackTarget::Bool(p) => {
                    out.push((id, p, Value::Bool(step_at(track, time) >= 0.5)));
                }
                TrackTarget::Enum(p) => {
                    let index = step_at(track, time).max(0.0) as usize;
                    let id_of = ty.find_param(&p).and_then(|d| match &d.kind {
                        ParamKind::Enum { options } => options.get(index).map(|o| o.id.clone()),
                        _ => None,
                    });
                    if let Some(o) = id_of {
                        out.push((id, p, Value::Enum(o)));
                    }
                }
                TrackTarget::Link(p) => {
                    let n = step_at(track, time).max(0.0) as u64;
                    let link = (n != 0).then_some(NodeId(n));
                    out.push((id, p, Value::Link(link)));
                }
                TrackTarget::Component(p, axis) => {
                    let rot_tracks =
                        [0, 1, 2].map(|a| node.tracks.get(&format!("{p}.{}", ["x", "y", "z"][a])));
                    let quaternion = p == forge_core::transform::ROT
                        && rot_tracks.iter().flatten().any(|t| t.quaternion);
                    if quaternion {
                        // one orientation for the three components: written once, at the
                        // first of them
                        let first = rot_tracks.iter().position(|t| t.is_some());
                        if let (true, Some(Value::Vec3(c))) = (first == Some(axis), base(&out, &p))
                        {
                            let r = crate::quaternion_rotation_at(rot_tracks, c.into(), time);
                            out.push((id, p, Value::Vec3(r.into())));
                        }
                    } else if let Some(Value::Vec3(mut c)) = base(&out, &p) {
                        c[axis] = value_at(track, time);
                        out.push((id, p, Value::Vec3(c)));
                    }
                }
                TrackTarget::Channel(p, axis) => {
                    if let Some(Value::Color(mut c)) = base(&out, &p) {
                        c[axis] = value_at(track, time).max(0.0);
                        out.push((id, p, Value::Color(c)));
                    }
                }
                TrackTarget::Points(p) => {
                    if let (Some(Value::Mesh(m)), Some(points)) =
                        (base(&out, &p), points_at(track, time))
                        && points.len() == m.points.len()
                    {
                        let mesh = MeshData {
                            points,
                            polys: m.polys.clone(),
                        };
                        out.push((id, p, Value::Mesh(Arc::new(mesh))));
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
