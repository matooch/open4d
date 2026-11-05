//! The Target effector (clone-toolset spec E22): turns each clone so that its z axis points
//! at a target object or a neighbouring clone.

use std::collections::BTreeMap;

use forge_core::{Category, NodeId, NodeType, ParamDesc, Scene, Value};
use forge_fields::FieldList;
use glam::{DMat3, DMat4, DQuat, DVec3};

use crate::effector::{Effector, EffectorCtx};
use crate::node::Params;
use crate::plain::frame;
use crate::{CloneData, CloneError};

/// Type id of the Target effector.
pub const TARGET: &str = "forge.effector.target";

const BIG: f64 = 1e20;
const EFF: &str = "Effector";
const TGT: &str = "Target";

/// The Target effector's node type.
pub fn target_type() -> NodeType {
    NodeType::new(TARGET, "Target", Category::Effector)
        .param(ParamDesc::bool("enabled", "Enabled", true).group(EFF))
        .param(
            ParamDesc::float("strength", "Strength", 1.0)
                .range(-BIG, BIG)
                .unit(forge_core::Unit::Percent)
                .group(EFF),
        )
        .param(ParamDesc::string("selection", "Selection", "").group(EFF))
        .param(
            ParamDesc::enumeration(
                "deformation",
                "Deformation",
                "off",
                &[
                    ("off", "Off"),
                    ("object", "Object"),
                    ("point", "Point"),
                    ("polygon", "Polygon"),
                ],
            )
            .group(EFF),
        )
        .param(forge_fields::field_list_param("fields", "Fields").group(EFF))
        .param(
            ParamDesc::enumeration(
                "target_mode",
                "Target Mode",
                "object_target",
                &[
                    ("object_target", "Object Target"),
                    ("look_at_camera", "Look At Camera"),
                    ("next_node", "Next Node"),
                    ("previous_node", "Previous Node"),
                    ("field_direction", "Field Direction"),
                ],
            )
            .group(TGT),
        )
        .param(ParamDesc::link("target_object", "Target Object", &[Category::Object]).group(TGT))
        .param(ParamDesc::bool("use_pitch", "Use Pitch", true).group(TGT))
        .param(ParamDesc::bool("reverse_heading", "Reverse Heading", false).group(TGT))
        .param(
            ParamDesc::enumeration(
                "up_vector",
                "Up Vector",
                "none",
                &[
                    ("none", "None"),
                    ("object", "Object"),
                    ("x_plus", "X+"),
                    ("x_minus", "X-"),
                    ("y_plus", "Y+"),
                    ("y_minus", "Y-"),
                    ("z_plus", "Z+"),
                    ("z_minus", "Z-"),
                ],
            )
            .group(TGT),
        )
        .param(ParamDesc::bool("repel", "Repel", false).group(TGT))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetMode {
    ObjectTarget,
    LookAtCamera,
    NextNode,
    PreviousNode,
    FieldDirection,
}

/// The Target effector's settings (fractions for percent values).
#[derive(Debug, Clone, PartialEq)]
pub struct Target {
    pub strength: f64,
    pub mode: TargetMode,
    /// World position of the Target Object (`None`: no object, the effector does nothing).
    pub target: Option<DVec3>,
    pub use_pitch: bool,
    pub reverse_heading: bool,
    /// The up axis in cloner space; `None` for Up Vector None (the Y axis).
    pub up: Option<DVec3>,
    /// Why the settings cannot run, if any.
    pub unsupported: Option<&'static str>,
}

impl Default for Target {
    /// The node defaults, with no Target Object.
    fn default() -> Self {
        from_values(&BTreeMap::new(), None).expect("enabled by default")
    }
}

fn from_values(values: &BTreeMap<String, Value>, target: Option<DVec3>) -> Option<Target> {
    let ty = target_type();
    let p = Params { ty: &ty, values };
    if !p.bool("enabled") {
        return None;
    }
    let mut unsupported = None;
    let mode = match p.option("target_mode").as_str() {
        "look_at_camera" => {
            unsupported = Some("Target Mode Look At Camera");
            TargetMode::LookAtCamera
        }
        "next_node" => TargetMode::NextNode,
        "previous_node" => TargetMode::PreviousNode,
        "field_direction" => {
            unsupported = Some("Target Mode Field Direction");
            TargetMode::FieldDirection
        }
        _ => TargetMode::ObjectTarget,
    };
    let up = match p.option("up_vector").as_str() {
        "object" => {
            unsupported = Some("Target Up Vector Object");
            None
        }
        "x_plus" => Some(DVec3::X),
        "x_minus" => Some(DVec3::NEG_X),
        "y_plus" => Some(DVec3::Y),
        "y_minus" => Some(DVec3::NEG_Y),
        "z_plus" => Some(DVec3::Z),
        "z_minus" => Some(DVec3::NEG_Z),
        _ => None,
    };
    if p.bool("repel") {
        unsupported = Some("Target Repel");
    }
    if p.option("deformation") != "off" {
        unsupported = Some("effector deformation modes");
    }
    Some(Target {
        strength: p.float("strength"),
        mode,
        target,
        use_pitch: p.bool("use_pitch"),
        reverse_heading: p.bool("reverse_heading"),
        up,
        unsupported,
    })
}

/// The Target effector of a node; `None` if the node is disabled.
pub(crate) fn from_node(scene: &Scene, id: NodeId) -> Option<Target> {
    let node = scene.node(id)?;
    let target = match node.params.get("target_object") {
        Some(Value::Link(Some(t))) => scene.world_matrix(*t).map(|m| m.w_axis.truncate()),
        _ => None,
    };
    from_values(&node.params, target)
}

impl Target {
    /// The frame (columns x, y, z) that looks along `dir`, or `None` for no direction.
    fn aim(&self, dir: DVec3, fallback_x: DVec3) -> Option<DMat3> {
        let up = self.up.unwrap_or(DVec3::Y);
        let along = up * dir.dot(up);
        let across = dir - along;
        let mut d = dir;
        if !self.use_pitch {
            d = across;
        }
        if self.reverse_heading {
            d = (d - across) - across;
        }
        let z = d.try_normalize()?;
        let x = up.cross(z).try_normalize().unwrap_or(fallback_x);
        let y = z.cross(x);
        Some(DMat3::from_cols(x, y, z))
    }
}

impl Effector for Target {
    fn apply(
        &self,
        data: &mut CloneData,
        falloff: &FieldList,
        ctx: &EffectorCtx,
    ) -> Result<(), CloneError> {
        if let Some(what) = self.unsupported {
            return Err(CloneError::Unsupported(what));
        }
        let to_cloner = ctx.cloner_world.inverse();
        let positions: Vec<DVec3> = data.matrices.iter().map(|m| m.w_axis.truncate()).collect();
        let n = positions.len();
        for i in 0..n {
            let at = positions[i];
            let dir = match self.mode {
                TargetMode::ObjectTarget => match self.target {
                    Some(t) => to_cloner.transform_point3(t) - at,
                    None => continue,
                },
                TargetMode::NextNode if i + 1 < n => positions[i + 1] - at,
                TargetMode::NextNode if n > 1 => at - positions[i - 1],
                TargetMode::PreviousNode if i > 0 => positions[i - 1] - at,
                TargetMode::PreviousNode if n > 1 => at - positions[1],
                _ => continue,
            };
            let m = data.matrices[i];
            let current = frame(&m);
            let Some(aimed) = self.aim(dir, current.x_axis) else {
                continue;
            };
            let value = falloff
                .sample(ctx.cloner_world.transform_point3(at), ctx.eval)
                .value;
            let w = self.strength * value;
            let q = DQuat::from_mat3(&current).slerp(DQuat::from_mat3(&aimed), w);
            let r = DMat3::from_quat(q);
            let lens = [
                m.x_axis.truncate().length(),
                m.y_axis.truncate().length(),
                m.z_axis.truncate().length(),
            ];
            data.matrices[i] = DMat4::from_cols(
                (r.x_axis * lens[0]).extend(0.0),
                (r.y_axis * lens[1]).extend(0.0),
                (r.z_axis * lens[2]).extend(0.0),
                m.w_axis,
            );
        }
        Ok(())
    }
}
