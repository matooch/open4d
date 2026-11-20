//! The skin deformer (docs/specs/RIG.md §3): linear blend skinning with a dual
//! quaternion option.

use forge_core::{Category, NodeId, NodeType, ParamDesc, Scene, Value};
use glam::{DMat3, DMat4, DQuat, DVec3};

use crate::weights::{WeightMap, find_weight_tag, read_bind_pose, read_weights, tag_joints};

/// Type id of the skin deformer.
pub const SKIN: &str = "forge.skin";

pub(crate) fn skin_type() -> NodeType {
    NodeType::new(SKIN, "Skin", Category::Deformer)
        .param(ParamDesc::enumeration(
            "type",
            "Type",
            "linear",
            &[("linear", "Linear"), ("dual_quaternion", "Dual Quaternion")],
        ))
        .param(ParamDesc::int("max_influences", "Max influences", 0).range(0i64, 32i64))
        .param(ParamDesc::bool("normalise", "Normalise", true))
        .param(ParamDesc::bool("enabled", "Enabled", true))
}

/// How the per-joint transforms are blended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkinType {
    Linear,
    DualQuaternion,
}

/// Skinning settings independent of the scene.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SkinSettings {
    pub kind: SkinType,
    /// 0 = unlimited.
    pub max_influences: usize,
    pub normalise: bool,
}

impl Default for SkinSettings {
    fn default() -> Self {
        SkinSettings {
            kind: SkinType::Linear,
            max_influences: 0,
            normalise: true,
        }
    }
}

/// The weights of one point after the influence limit and normalisation (RIG.md §3).
pub fn effective_weights(row: &[(u32, f64)], s: &SkinSettings) -> Vec<(u32, f64)> {
    let mut row: Vec<(u32, f64)> = row.to_vec();
    if s.max_influences > 0 && row.len() > s.max_influences {
        // largest weights first; ties keep the lower slot (the sort is stable)
        row.sort_by(|a, b| b.1.total_cmp(&a.1));
        row.truncate(s.max_influences);
        row.sort_by_key(|(slot, _)| *slot);
    }
    if s.normalise {
        let sum: f64 = row.iter().map(|(_, w)| *w).sum();
        if sum != 0.0 {
            for (_, w) in row.iter_mut() {
                *w /= sum;
            }
        }
    }
    row
}

/// Skins `points` in place: `p' = Σ w_j · K_j · p`. `k[j]` is `K_j`.
pub fn skin_points(points: &mut [DVec3], weights: &WeightMap, k: &[DMat4], s: &SkinSettings) {
    let dual: Vec<DualQuat> = if s.kind == SkinType::DualQuaternion {
        k.iter().map(DualQuat::from_matrix).collect()
    } else {
        Vec::new()
    };
    for (i, p) in points.iter_mut().enumerate() {
        let row = effective_weights(weights.get(i as u32), s);
        let row: Vec<(u32, f64)> = row
            .into_iter()
            .filter(|(slot, _)| (*slot as usize) < k.len())
            .collect();
        if row.is_empty() || row.iter().map(|(_, w)| *w).sum::<f64>() == 0.0 {
            continue;
        }
        *p = match s.kind {
            SkinType::Linear => row.iter().fold(DVec3::ZERO, |acc, (slot, w)| {
                acc + k[*slot as usize].transform_point3(*p) * *w
            }),
            SkinType::DualQuaternion => blend_dual(&dual, &row).transform_point(*p),
        };
    }
}

/// A unit dual quaternion `r + ε d`, a rigid transform.
#[derive(Debug, Clone, Copy)]
struct DualQuat {
    r: DQuat,
    d: DQuat,
}

fn quat_scale(q: DQuat, s: f64) -> DQuat {
    DQuat::from_xyzw(q.x * s, q.y * s, q.z * s, q.w * s)
}

fn quat_add(a: DQuat, b: DQuat) -> DQuat {
    DQuat::from_xyzw(a.x + b.x, a.y + b.y, a.z + b.z, a.w + b.w)
}

impl DualQuat {
    /// The rigid part of an affine matrix: orthonormalised rotation columns and translation.
    fn from_matrix(m: &DMat4) -> DualQuat {
        let x = m.x_axis.truncate().normalize_or_zero();
        let y0 = m.y_axis.truncate();
        let y = (y0 - x * x.dot(y0)).normalize_or_zero();
        let z = x.cross(y);
        let r = DQuat::from_mat3(&DMat3::from_cols(x, y, z)).normalize();
        let t = m.w_axis.truncate();
        let tq = DQuat::from_xyzw(t.x, t.y, t.z, 0.0);
        DualQuat {
            r,
            d: quat_scale(tq * r, 0.5),
        }
    }

    fn transform_point(&self, p: DVec3) -> DVec3 {
        let r = self.r.normalize();
        let t = quat_scale(self.d * r.conjugate(), 2.0);
        r * p + DVec3::new(t.x, t.y, t.z)
    }
}

/// Blends dual quaternions with the sign of each flipped against the first (so the shorter
/// arc is taken), then normalises by the rotation part's length.
fn blend_dual(dual: &[DualQuat], row: &[(u32, f64)]) -> DualQuat {
    let first = dual[row[0].0 as usize].r;
    let mut r = DQuat::from_xyzw(0.0, 0.0, 0.0, 0.0);
    let mut d = r;
    for (slot, w) in row {
        let q = dual[*slot as usize];
        let sign = if q.r.dot(first) < 0.0 { -1.0 } else { 1.0 };
        r = quat_add(r, quat_scale(q.r, w * sign));
        d = quat_add(d, quat_scale(q.d, w * sign));
    }
    let n = r.length();
    if n == 0.0 {
        return DualQuat {
            r: DQuat::IDENTITY,
            d: DQuat::from_xyzw(0.0, 0.0, 0.0, 0.0),
        };
    }
    DualQuat {
        r: quat_scale(r, 1.0 / n),
        d: quat_scale(d, 1.0 / n),
    }
}

fn read_settings(scene: &Scene, deformer: NodeId) -> Option<SkinSettings> {
    let node = scene.node(deformer)?;
    if matches!(node.params.get("enabled"), Some(Value::Bool(false))) {
        return None;
    }
    let kind = match node.params.get("type") {
        Some(Value::Enum(t)) if t == "dual_quaternion" => SkinType::DualQuaternion,
        _ => SkinType::Linear,
    };
    let max_influences = match node.params.get("max_influences") {
        Some(Value::Int(n)) if *n > 0 => *n as usize,
        _ => 0,
    };
    let normalise = !matches!(node.params.get("normalise"), Some(Value::Bool(false)));
    Some(SkinSettings {
        kind,
        max_influences,
        normalise,
    })
}

/// The per-joint skinning matrices `K_j = inverse(O) · J_j · B_j⁻¹` in the object's local
/// space (`O` the object's world matrix, `J_j` the joint's, `B_j` the bound pose).
pub fn skinning_matrices(scene: &Scene, object: NodeId, tag: NodeId) -> Option<Vec<DMat4>> {
    let joints = tag_joints(scene, tag);
    let bind = read_bind_pose(scene, tag, joints.len());
    let o_inv = scene.world_matrix(object)?.inverse();
    Some(
        joints
            .iter()
            .zip(&bind)
            .map(|(j, b)| match scene.world_matrix(*j) {
                Some(jw) => o_inv * jw * b.inverse(),
                None => DMat4::IDENTITY,
            })
            .collect(),
    )
}

/// Applies the skin deformer `deformer` to `points` of `target` (object-local space). Does
/// nothing when the skin is disabled or the object has no weight tag.
pub fn apply_skin(scene: &Scene, deformer: NodeId, target: NodeId, points: &mut [DVec3]) {
    let Some(settings) = read_settings(scene, deformer) else {
        return;
    };
    let Some(tag) = find_weight_tag(scene, target) else {
        return;
    };
    let Some(k) = skinning_matrices(scene, target, tag) else {
        return;
    };
    if !k.iter().all(|m| m.is_finite()) {
        return;
    }
    let weights = read_weights(scene, tag);
    // The tag's Weight (O34, default 1) blends the whole skin with the bind pose.
    let strength = match scene.param(tag, "weight") {
        Some(Value::Float(w)) => *w,
        _ => 1.0,
    };
    if strength == 1.0 {
        skin_points(points, &weights, &k, &settings);
        return;
    }
    let before = points.to_vec();
    skin_points(points, &weights, &k, &settings);
    for (p, p0) in points.iter_mut().zip(before) {
        *p = p0 + (*p - p0) * strength;
    }
}
