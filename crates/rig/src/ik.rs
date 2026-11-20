//! The IK tag: analytic two-bone solver, FABRIK for longer chains, pole and twist, IK/FK
//! blend (docs/specs/RIG.md §10).

use forge_core::transform::{Transform, hpb_from_rotation, rotation_part};
use forge_core::{Category, NodeId, NodeType, ParamDesc, Scene, Unit, Value};
use glam::{DMat3, DQuat, DVec3};

use crate::joint::is_joint;
use crate::tags::priority_params;

/// Type id of the IK tag.
pub const IK_TAG: &str = "forge.ik_tag";

pub(crate) fn ik_tag_type() -> NodeType {
    let mut ty = NodeType::new(IK_TAG, "IK", Category::Tag)
        .param(ParamDesc::bool("enabled", "Enabled", true))
        .param(ParamDesc::link("end", "End", &[Category::Object]))
        .param(ParamDesc::link("goal", "Goal", &[Category::Object]))
        .param(ParamDesc::link("pole", "Pole", &[Category::Object]))
        .param(ParamDesc::float("twist", "Twist", 0.0).unit(Unit::Angle))
        .param(
            ParamDesc::float("strength", "Strength", 1.0)
                .range(0.0, 1.0)
                .unit(Unit::Percent),
        )
        .param(
            ParamDesc::float("ik_fk", "IK/FK", 0.0)
                .range(0.0, 1.0)
                .unit(Unit::Percent),
        )
        .param(ParamDesc::float("threshold", "Threshold", 0.01).range(1e-9, 1e6))
        .param(ParamDesc::int("max_iterations", "Max iterations", 10000).range(1, 1_000_000));
    for p in priority_params() {
        ty = ty.param(p);
    }
    ty
}

/// Bend plane normal for the chord `c` from `root` (RIG.md §10 rule 1).
pub fn hinge(c: DVec3, root: DVec3, root_z: DVec3, pole: Option<DVec3>, twist: f64) -> DVec3 {
    let mut h = None;
    if let Some(q) = pole {
        let q = q - root;
        let qp = q - c * q.dot(c);
        if qp.length() > 1e-9 {
            h = Some(qp.cross(c).normalize());
        }
    }
    let h = h.unwrap_or_else(|| {
        let z = root_z - c * root_z.dot(c);
        if z.length() > 1e-9 {
            z.normalize()
        } else {
            c.any_orthonormal_vector()
        }
    });
    DQuat::from_axis_angle(c, -twist) * h
}

/// Joint positions of the solved chain (first = `fk[0]`) and the hinge, or `None` for a
/// degenerate setup (goal on the root, fewer than two joints).
pub fn solve_positions(
    fk: &[DVec3],
    goal: DVec3,
    pole: Option<DVec3>,
    root_z: DVec3,
    twist: f64,
    threshold: f64,
    max_iterations: usize,
) -> Option<(Vec<DVec3>, DVec3)> {
    if fk.len() < 2 {
        return None;
    }
    let p0 = fk[0];
    let to = goal - p0;
    let dist = to.length();
    if dist < 1e-9 {
        return None;
    }
    let c = to / dist;
    let h = hinge(c, p0, root_z, pole, twist);
    let l: Vec<f64> = fk.windows(2).map(|w| (w[1] - w[0]).length()).collect();
    let total: f64 = l.iter().sum();
    let mut out = vec![p0];
    match l.len() {
        1 => out.push(p0 + c * l[0]),
        2 => {
            let d = dist.clamp((l[0] - l[1]).abs(), l[0] + l[1]);
            let cos = ((l[0] * l[0] + d * d - l[1] * l[1]) / (2.0 * l[0] * d)).clamp(-1.0, 1.0);
            let dir = DQuat::from_axis_angle(h, cos.acos()) * c;
            out.push(p0 + dir * l[0]);
            out.push(p0 + c * d);
        }
        _ if dist >= total => {
            let mut p = p0;
            for len in &l {
                p += c * *len;
                out.push(p);
            }
        }
        _ => out = fabrik(fk, &l, goal, h, c, threshold, max_iterations),
    }
    Some((out, h))
}

fn fabrik(
    fk: &[DVec3],
    l: &[f64],
    goal: DVec3,
    h: DVec3,
    c: DVec3,
    threshold: f64,
    max_iterations: usize,
) -> Vec<DVec3> {
    let p0 = fk[0];
    let n = fk.len();
    let mut p: Vec<DVec3> = fk.iter().map(|q| *q - h * (*q - p0).dot(h)).collect();
    let step = |from: DVec3, to: DVec3, len: f64| {
        let d = (to - from).normalize_or_zero();
        let d = if d == DVec3::ZERO { c } else { d };
        to - d * len
    };
    for _ in 0..max_iterations {
        if (p[n - 1] - goal).length() < threshold {
            break;
        }
        p[n - 1] = goal;
        for i in (0..n - 1).rev() {
            p[i] = step(p[i], p[i + 1], l[i]);
        }
        p[0] = p0;
        for i in 1..n {
            let d = (p[i] - p[i - 1]).normalize_or_zero();
            let d = if d == DVec3::ZERO { c } else { d };
            p[i] = p[i - 1] + d * l[i - 1];
        }
    }
    p
}

/// The local bone axis `a` and up axis `u` of a joint whose first child sits at `v`
/// (RIG.md §10 rule 4).
fn bone_axes(v: DVec3) -> Option<(DVec3, DVec3)> {
    if v.length() < 1e-12 {
        return None;
    }
    let a = v.normalize();
    let ab = a.abs();
    let axis = if ab.y >= ab.x && ab.y >= ab.z {
        DVec3::Z
    } else if ab.z >= ab.x {
        DVec3::X
    } else {
        DVec3::Y
    };
    let u = axis - a * axis.dot(a);
    Some((a, u.normalize()))
}

fn frame(a: DVec3, u: DVec3, d: DVec3, h: DVec3) -> DMat3 {
    DMat3::from_cols(d, h, d.cross(h)) * DMat3::from_cols(a, u, a.cross(u)).transpose()
}

fn link(scene: &Scene, tag: NodeId, key: &str) -> Option<NodeId> {
    match scene.param(tag, key) {
        Some(Value::Link(Some(id))) => Some(*id),
        _ => None,
    }
}

fn float(scene: &Scene, tag: NodeId, key: &str, default: f64) -> f64 {
    match scene.param(tag, key) {
        Some(Value::Float(v)) => *v,
        _ => default,
    }
}

/// The joint chain from the tag's host down to `end`, root first.
pub fn chain_of(scene: &Scene, tag: NodeId) -> Option<Vec<NodeId>> {
    let host = scene.node(tag)?.host?;
    let end = link(scene, tag, "end")?;
    let mut chain = vec![end];
    let mut cur = end;
    while cur != host {
        cur = scene.node(cur)?.parent?;
        chain.push(cur);
    }
    chain.reverse();
    (chain.len() >= 2 && chain.iter().all(|j| is_joint(scene, *j))).then_some(chain)
}

/// The `transform.rot` writes that pose the chain of an IK tag (empty when the tag cannot
/// solve: no chain, no goal, goal on the root).
pub fn solve_tag(scene: &Scene, tag: NodeId) -> Vec<(NodeId, String, Value)> {
    let Some(chain) = chain_of(scene, tag) else {
        return Vec::new();
    };
    let Some(goal_node) = link(scene, tag, "goal") else {
        return Vec::new();
    };
    let Some(goal_m) = scene.world_matrix(goal_node) else {
        return Vec::new();
    };
    let pole = link(scene, tag, "pole")
        .and_then(|p| scene.world_matrix(p))
        .map(|m| m.w_axis.truncate());
    let worlds: Option<Vec<_>> = chain.iter().map(|j| scene.world_matrix(*j)).collect();
    let Some(worlds) = worlds else {
        return Vec::new();
    };
    let fk: Vec<DVec3> = worlds.iter().map(|m| m.w_axis.truncate()).collect();
    let root_z = rotation_part(&worlds[0]).z_axis;
    let iterations = match scene.param(tag, "max_iterations") {
        Some(Value::Int(n)) => (*n).max(1) as usize,
        _ => 10000,
    };
    let Some((pos, h)) = solve_positions(
        &fk,
        goal_m.w_axis.truncate(),
        pole,
        root_z,
        float(scene, tag, "twist", 0.0),
        float(scene, tag, "threshold", 0.01),
        iterations,
    ) else {
        return Vec::new();
    };
    let weight = (float(scene, tag, "strength", 1.0) * (1.0 - float(scene, tag, "ik_fk", 0.0)))
        .clamp(0.0, 1.0);
    let n = chain.len();
    let mut parent_rot = match scene.parent_world_matrix(chain[0]) {
        Some(m) => rotation_part(&m),
        None => return Vec::new(),
    };
    let mut out = Vec::new();
    for i in 0..n - 1 {
        let Some(child) = scene.local_transform(chain[i + 1]) else {
            return Vec::new();
        };
        let Some((a, u)) = bone_axes(child.pos) else {
            return Vec::new();
        };
        let d = (pos[i + 1] - pos[i]).normalize_or_zero();
        if d == DVec3::ZERO {
            return Vec::new();
        }
        let w = frame(a, u, d, h);
        let ik_local = parent_rot.transpose() * w;
        let fk_local =
            Transform::from_params(&scene.node(chain[i]).expect("chain node").params).rotation();
        let local = if weight >= 1.0 {
            ik_local
        } else {
            let q = DQuat::from_mat3(&fk_local)
                .slerp(DQuat::from_mat3(&ik_local), weight)
                .normalize();
            DMat3::from_quat(q)
        };
        out.push((
            chain[i],
            "transform.rot".to_string(),
            Value::Vec3(hpb_from_rotation(local).to_array()),
        ));
        parent_rot *= local;
    }
    out
}
