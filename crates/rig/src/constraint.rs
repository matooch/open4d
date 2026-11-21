//! The constraint tag: PSR, Aim (with up vector), Parent and Clamp (docs/specs/RIG.md §11).

use forge_core::transform::{Transform, hpb_from_rotation, rotation_part};
use forge_core::{Category, NodeId, NodeType, ParamDesc, Scene, Unit, Value};
use glam::{DMat3, DMat4, DQuat, DVec3};

use crate::tags::priority_params;

/// Type id of the constraint tag.
pub const CONSTRAINT_TAG: &str = "forge.constraint_tag";

const AXES: [(&str, &str); 6] = [
    ("x", "X+"),
    ("-x", "X-"),
    ("y", "Y+"),
    ("-y", "Y-"),
    ("z", "Z+"),
    ("-z", "Z-"),
];

pub(crate) fn constraint_tag_type() -> NodeType {
    let mut ty = NodeType::new(CONSTRAINT_TAG, "Constraint", Category::Tag)
        .param(ParamDesc::bool("enabled", "Enabled", true))
        .param(ParamDesc::enumeration(
            "mode",
            "Mode",
            "psr",
            &[
                ("psr", "PSR"),
                ("aim", "Aim"),
                ("parent", "Parent"),
                ("clamp", "Clamp"),
            ],
        ))
        .param(ParamDesc::link_rows(
            "targets",
            "Targets",
            &[Category::Object],
            &[("normal", "Normal")],
        ))
        .param(ParamDesc::bool("position", "Position", true))
        .param(ParamDesc::bool("rotation", "Rotation", false))
        .param(ParamDesc::bool("scale", "Scale", false))
        .param(ParamDesc::bool("maintain_offset", "Maintain offset", false))
        .param(ParamDesc::vec3("offset_pos", "Offset position", [0.0; 3]).hidden())
        .param(ParamDesc::vec3("offset_rot", "Offset rotation", [0.0; 3]).hidden())
        .param(ParamDesc::vec3("offset_scale", "Offset scale", [1.0; 3]).hidden())
        .param(ParamDesc::enumeration("aim_axis", "Aim axis", "z", &AXES))
        .param(ParamDesc::link(
            "up_target",
            "Up target",
            &[Category::Object],
        ))
        .param(ParamDesc::enumeration("up_axis", "Up axis", "y", &AXES))
        .param(ParamDesc::float("clamp_min", "Minimum distance", 0.0).unit(Unit::Distance))
        .param(ParamDesc::float("clamp_max", "Maximum distance", 0.0).unit(Unit::Distance))
        .param(
            ParamDesc::float("strength", "Strength", 1.0)
                .range(0.0, 1.0)
                .unit(Unit::Percent),
        );
    for p in priority_params() {
        ty = ty.param(p);
    }
    ty
}

/// A world transform split into position, rotation and scale.
#[derive(Debug, Clone, Copy)]
struct Psr {
    pos: DVec3,
    rot: DQuat,
    scale: DVec3,
}

impl Psr {
    fn of(m: &DMat4) -> Psr {
        let m3 = DMat3::from_mat4(*m);
        Psr {
            pos: m.w_axis.truncate(),
            rot: DQuat::from_mat3(&rotation_part(m)).normalize(),
            scale: DVec3::new(m3.x_axis.length(), m3.y_axis.length(), m3.z_axis.length()),
        }
    }

    fn matrix(&self) -> DMat4 {
        DMat4::from_scale_rotation_translation(self.scale, self.rot, self.pos)
    }

    fn mix(&self, to: &Psr, w: f64) -> Psr {
        Psr {
            pos: self.pos.lerp(to.pos, w),
            rot: self.rot.slerp(to.rot, w).normalize(),
            scale: self.scale.lerp(to.scale, w),
        }
    }
}

fn enum_of<'a>(scene: &'a Scene, tag: NodeId, key: &str, default: &'a str) -> &'a str {
    match scene.param(tag, key) {
        Some(Value::Enum(s)) => s,
        _ => default,
    }
}

fn flag(scene: &Scene, tag: NodeId, key: &str, default: bool) -> bool {
    match scene.param(tag, key) {
        Some(Value::Bool(b)) => *b,
        _ => default,
    }
}

fn vec3(scene: &Scene, tag: NodeId, key: &str, default: DVec3) -> DVec3 {
    match scene.param(tag, key) {
        Some(Value::Vec3(v)) => DVec3::from_array(*v),
        _ => default,
    }
}

fn axis_vector(id: &str) -> DVec3 {
    match id {
        "x" => DVec3::X,
        "-x" => DVec3::NEG_X,
        "y" => DVec3::Y,
        "-y" => DVec3::NEG_Y,
        "-z" => DVec3::NEG_Z,
        _ => DVec3::Z,
    }
}

/// The weighted mean of the enabled target rows (RIG.md §11 "Target").
fn blended_target(scene: &Scene, tag: NodeId) -> Option<Psr> {
    let Some(Value::LinkRows(rows)) = scene.param(tag, "targets") else {
        return None;
    };
    let mut sum = 0.0;
    let mut acc = Psr {
        pos: DVec3::ZERO,
        rot: DQuat::from_xyzw(0.0, 0.0, 0.0, 0.0),
        scale: DVec3::ZERO,
    };
    let mut first: Option<DQuat> = None;
    for row in rows.iter().filter(|r| r.enabled && r.strength > 0.0) {
        let Some(m) = scene.world_matrix(row.target) else {
            continue;
        };
        let p = Psr::of(&m);
        let mut q = p.rot;
        match first {
            None => first = Some(q),
            Some(f) if f.dot(q) < 0.0 => q = -q,
            Some(_) => {}
        }
        let w = row.strength;
        sum += w;
        acc.pos += p.pos * w;
        acc.scale += p.scale * w;
        acc.rot = DQuat::from_xyzw(
            acc.rot.x + q.x * w,
            acc.rot.y + q.y * w,
            acc.rot.z + q.z * w,
            acc.rot.w + q.w * w,
        );
    }
    (sum > 0.0).then(|| Psr {
        pos: acc.pos / sum,
        rot: acc.rot.normalize(),
        scale: acc.scale / sum,
    })
}

fn offset_transform(scene: &Scene, tag: NodeId) -> Transform {
    Transform {
        pos: vec3(scene, tag, "offset_pos", DVec3::ZERO),
        rot: vec3(scene, tag, "offset_rot", DVec3::ZERO),
        scale: vec3(scene, tag, "offset_scale", DVec3::ONE),
    }
}

/// The `offset_*` writes that record the object's offset to its targets in the scene's
/// current state (PSR: world delta, relative rotation and scale ratio; Parent: the object's
/// transform in the target's space). Empty without a valid target.
pub fn capture_offset(scene: &Scene, tag: NodeId) -> Vec<(NodeId, String, Value)> {
    let (Some(host), Some(t)) = (
        scene.node(tag).and_then(|n| n.host),
        blended_target(scene, tag),
    ) else {
        return Vec::new();
    };
    let Some(m0) = scene.world_matrix(host) else {
        return Vec::new();
    };
    let own = Psr::of(&m0);
    let (pos, rot, scale) = if enum_of(scene, tag, "mode", "psr") == "parent" {
        let rel = t.matrix().inverse() * m0;
        let r = Psr::of(&rel);
        (r.pos, DMat3::from_quat(r.rot), r.scale)
    } else {
        let safe = |a: f64, b: f64| if b.abs() < 1e-12 { 1.0 } else { a / b };
        (
            own.pos - t.pos,
            DMat3::from_quat(t.rot.inverse() * own.rot),
            DVec3::new(
                safe(own.scale.x, t.scale.x),
                safe(own.scale.y, t.scale.y),
                safe(own.scale.z, t.scale.z),
            ),
        )
    };
    vec![
        (tag, "offset_pos".to_string(), Value::Vec3(pos.to_array())),
        (
            tag,
            "offset_rot".to_string(),
            Value::Vec3(hpb_from_rotation(rot).to_array()),
        ),
        (
            tag,
            "offset_scale".to_string(),
            Value::Vec3(scale.to_array()),
        ),
    ]
}

/// Minimal rotation of `own` that turns its local `axis` to `d`.
fn swing(own: DQuat, axis: DVec3, d: DVec3) -> DQuat {
    DQuat::from_rotation_arc(own * axis, d) * own
}

fn aim(scene: &Scene, tag: NodeId, own: &Psr, t: &Psr) -> DQuat {
    let d = (t.pos - own.pos).normalize_or_zero();
    if d == DVec3::ZERO {
        return own.rot;
    }
    let axis = axis_vector(enum_of(scene, tag, "aim_axis", "z"));
    let fallback = swing(own.rot, axis, d);
    let up_pos = match scene.param(tag, "up_target") {
        Some(Value::Link(Some(u))) => scene.world_matrix(*u).map(|m| m.w_axis.truncate()),
        _ => None,
    };
    let Some(up_pos) = up_pos else {
        return fallback;
    };
    let up_dir = up_pos - own.pos;
    let up_world = up_dir - d * up_dir.dot(d);
    let up_axis = axis_vector(enum_of(scene, tag, "up_axis", "y"));
    let up_local = up_axis - axis * up_axis.dot(axis);
    if up_world.length() < 1e-9 || up_local.length() < 1e-9 {
        return fallback;
    }
    let (uw, ul) = (up_world.normalize(), up_local.normalize());
    let local = DMat3::from_cols(axis, ul, axis.cross(ul));
    let world = DMat3::from_cols(d, uw, d.cross(uw));
    DQuat::from_mat3(&(world * local.transpose())).normalize()
}

/// The transform writes of one constraint tag (empty when it does nothing).
pub fn evaluate_tag(scene: &Scene, tag: NodeId) -> Vec<(NodeId, String, Value)> {
    let Some(host) = scene.node(tag).and_then(|n| n.host) else {
        return Vec::new();
    };
    let (Some(t), Some(m0), Some(parent)) = (
        blended_target(scene, tag),
        scene.world_matrix(host),
        scene.parent_world_matrix(host),
    ) else {
        return Vec::new();
    };
    let own = Psr::of(&m0);
    let (p_on, r_on, s_on) = (
        flag(scene, tag, "position", true),
        flag(scene, tag, "rotation", false),
        flag(scene, tag, "scale", false),
    );
    let mode = enum_of(scene, tag, "mode", "psr");
    let mut new = own;
    let (mut write_p, mut write_r, mut write_s) = (false, false, false);
    match mode {
        "psr" => {
            let off =
                flag(scene, tag, "maintain_offset", false).then(|| offset_transform(scene, tag));
            if p_on {
                new.pos = t.pos + off.map_or(DVec3::ZERO, |o| o.pos);
            }
            if r_on {
                let o = off.map_or(DQuat::IDENTITY, |o| {
                    DQuat::from_mat3(&o.rotation()).normalize()
                });
                new.rot = (t.rot * o).normalize();
            }
            if s_on {
                new.scale = t.scale * off.map_or(DVec3::ONE, |o| o.scale);
            }
            (write_p, write_r, write_s) = (p_on, r_on, s_on);
        }
        "parent" => {
            let o = offset_transform(scene, tag);
            let r = Psr::of(&(t.matrix() * o.matrix()));
            if p_on {
                new.pos = r.pos;
            }
            if r_on {
                new.rot = r.rot;
            }
            if s_on {
                new.scale = r.scale;
            }
            (write_p, write_r, write_s) = (p_on, r_on, s_on);
        }
        "aim" => {
            new.rot = aim(scene, tag, &own, &t);
            write_r = true;
        }
        _ => {
            let min = float_or(scene, tag, "clamp_min", 0.0);
            let max = float_or(scene, tag, "clamp_max", 0.0);
            let delta = own.pos - t.pos;
            let dist = delta.length();
            let dir = if dist > 1e-12 { delta / dist } else { DVec3::X };
            let want = if min > 0.0 && dist < min {
                min
            } else if max > 0.0 && dist > max {
                max
            } else {
                dist
            };
            new.pos = t.pos + dir * want;
            write_p = true;
        }
    }
    let strength = float_or(scene, tag, "strength", 1.0).clamp(0.0, 1.0);
    let new = own.mix(&new, strength);
    let local = local_parts(parent.inverse() * new.matrix());
    let mut out = Vec::new();
    if write_p {
        out.push((
            host,
            "transform.pos".to_string(),
            Value::Vec3(local.0.to_array()),
        ));
    }
    if write_r {
        out.push((
            host,
            "transform.rot".to_string(),
            Value::Vec3(local.1.to_array()),
        ));
    }
    if write_s {
        out.push((
            host,
            "transform.scale".to_string(),
            Value::Vec3(local.2.to_array()),
        ));
    }
    out
}

fn float_or(scene: &Scene, tag: NodeId, key: &str, default: f64) -> f64 {
    match scene.param(tag, key) {
        Some(Value::Float(v)) => *v,
        _ => default,
    }
}

/// `(pos, hpb, scale)` of a local matrix.
fn local_parts(m: DMat4) -> (DVec3, DVec3, DVec3) {
    let p = Psr::of(&m);
    (p.pos, hpb_from_rotation(DMat3::from_quat(p.rot)), p.scale)
}
