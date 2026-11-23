//! Descriptors and shared channel weighting for the remaining effectors.

use forge_core::{Category, NodeId, NodeType, ParamDesc, Scene, Unit, Value};
use forge_fields::FieldList;
use glam::{DMat4, DVec3};

use crate::plain::{self, ColorMode, CommonDefaults, Weights};
use crate::{CloneData, CloneError, EffectorCtx};

pub const SHADER: &str = "forge.effector.shader";
pub const SPLINE: &str = "forge.effector.spline";
pub const PUSH_APART: &str = "forge.effector.push_apart";
pub const INHERITANCE: &str = "forge.effector.inheritance";
pub const VOLUME: &str = "forge.effector.volume";
pub const GROUP: &str = "forge.effector.group";

pub(crate) fn types() -> [NodeType; 6] {
    let shape = &[Category::Object, Category::Generator];
    let boolean = ParamDesc::bool;
    let shared = |id, name| {
        NodeType::new(id, name, Category::Effector)
            .param(boolean("enabled", "Enabled", true))
            .param(pct("strength", "Strength", 1.0))
            .param(forge_fields::field_list_param("fields", "Fields"))
    };
    let channels = |n: NodeType| {
        n.param(boolean("position", "Position", true))
            .param(boolean("rotation", "Rotation", true))
            .param(boolean("scale", "Scale", true))
    };
    [
        plain::common_type(
            SHADER,
            "Shader",
            &CommonDefaults {
                position: false,
                scale: true,
                uniform_scale: true,
                scale_value: 0.5,
                color_mode: "effector_color",
                ..CommonDefaults::PLAIN
            },
        )
        .param(ParamDesc::link("shader", "Shader", &[Category::Shader]))
        .param(ParamDesc::float("offsetu", "Offset U", 0.0))
        .param(ParamDesc::float("offsetv", "Offset V", 0.0))
        .param(pct("lengthu", "Length U", 1.0))
        .param(pct("lengthv", "Length V", 1.0))
        .param(boolean("tile", "Tile", true))
        .param(boolean("invert", "Invert", false)),
        channels(shared(SPLINE, "Spline"))
            .param(ParamDesc::link("spline", "Spline", shape))
            .param(ParamDesc::link("rail", "Rail", shape))
            .param(ParamDesc::vec3("up_vector", "Up Vector", [0.0; 3]))
            .param(ParamDesc::enumeration(
                "mode",
                "Mode",
                "step",
                &[
                    ("step", "Step"),
                    ("falloff", "Falloff"),
                    ("relative", "Relative"),
                ],
            ))
            .param(pct("start", "Start", 0.0))
            .param(pct("end", "End", 1.0))
            .param(pct("offset", "Offset", 0.0))
            .param(boolean("clamp", "Clamp", false)),
        shared(PUSH_APART, "Push Apart")
            .param(ParamDesc::enumeration(
                "mode",
                "Mode",
                "push_apart",
                &[("push_apart", "Push Apart"), ("hide", "Hide")],
            ))
            .param(
                ParamDesc::float("radius", "Radius", 100.0)
                    .range(0.0, f64::MAX)
                    .unit(Unit::Distance),
            )
            .param(ParamDesc::int("iterations", "Iterations", 10).range(0, 1000)),
        channels(shared(INHERITANCE, "Inheritance"))
            .param(ParamDesc::link("object", "Object", shape))
            .param(boolean("morph_motion_object", "Morph Motion Object", false))
            .param(boolean("falloff_based", "Falloff Based", false)),
        plain::common_type(
            VOLUME,
            "Volume",
            &CommonDefaults {
                position: false,
                scale: true,
                uniform_scale: true,
                scale_value: -1.0,
                color_mode: "off",
                ..CommonDefaults::PLAIN
            },
        )
        .param(ParamDesc::link("volume_object", "Volume Object", shape)),
        NodeType::new(GROUP, "Group", Category::Effector)
            .param(boolean("enabled", "Enabled", true)),
    ]
}

fn pct(id: &str, label: &str, default: f64) -> ParamDesc {
    ParamDesc::float(id, label, default).unit(Unit::Percent)
}

/// Inputs to the per-clone sampling callback for Shader and Volume.
pub struct ChannelPoint {
    pub world: DVec3,
    pub uvw: DVec3,
}

/// Apply the standard effector transform channels with a shader/volume scalar.
pub fn apply_sampled(
    scene: &Scene,
    id: NodeId,
    data: &mut CloneData,
    ctx: &EffectorCtx,
    mut sample: impl FnMut(ChannelPoint) -> f64,
) -> Result<(), CloneError> {
    let (Some(n), Some(ty), Some(world)) =
        (scene.node(id), scene.node_type(id), scene.world_matrix(id))
    else {
        return Ok(());
    };
    let Some(mut plain) = plain::settings_from_params(ty, &n.params, world) else {
        return Ok(());
    };
    let shader_color = n.type_id == SHADER && plain.color_mode == ColorMode::EffectorColor;
    if shader_color {
        plain.color_mode = ColorMode::Off;
        if plain.unsupported == Some("effector colour mode Effector Color") {
            plain.unsupported = None;
        }
        if matches!(scene.param(id, "deformation"), Some(Value::Enum(s)) if s != "off")
            || matches!(scene.param(id, "blending_mode"), Some(Value::Enum(s)) if s != "default")
        {
            return Err(CloneError::Unsupported(
                "shader deformation/colour blending mode",
            ));
        }
    }
    let falloff = FieldList::from_param(scene, id, "fields")
        .map_err(|_| CloneError::Unsupported("effector field"))?;
    let mut colors = Vec::new();
    plain.apply_weighted(data, &falloff, ctx, &mut |c| {
        let v = sample(ChannelPoint {
            world: c.world,
            uvw: c.uvw,
        });
        let w = if v == 0.0 && n.type_id == VOLUME {
            0.0
        } else {
            (plain.minimum + (plain.maximum - plain.minimum) * v) * plain.strength * c.value
        };
        if shader_color {
            colors.push((
                c.position,
                v.clamp(0.0, 1.0),
                (plain.strength * c.value).clamp(0.0, 1.0),
            ));
        }
        Weights::uniform(w)
    })?;
    for (slot, g, w) in colors {
        data.colors[slot] = data.colors[slot].map(|c| c + (g - c) * w);
    }
    Ok(())
}

/// Common amount from the node's strength and world-space field sampling.
pub fn fields(scene: &Scene, id: NodeId) -> Result<FieldList, CloneError> {
    FieldList::from_param(scene, id, "fields")
        .map_err(|_| CloneError::Unsupported("effector field"))
}

/// Blend independently enabled TRS channels without discarding existing scale.
pub fn blend_pose(base: DMat4, target: DMat4, [p, r, s]: [bool; 3], weight: f64) -> DMat4 {
    if base.determinant().abs() < 1e-12 || target.determinant().abs() < 1e-12 || !r && !s {
        let mut out = base;
        if p {
            out.w_axis = base.w_axis.lerp(target.w_axis, weight);
        }
        return out;
    }
    let (mut scale, mut rotation, mut pos) = base.to_scale_rotation_translation();
    let (ts, tr, tp) = target.to_scale_rotation_translation();
    if p {
        pos = pos.lerp(tp, weight);
    }
    if r {
        rotation = rotation.slerp(tr, weight.clamp(0.0, 1.0));
    }
    if s {
        scale = scale.lerp(ts, weight);
    }
    DMat4::from_scale_rotation_translation(scale, rotation, pos)
}
