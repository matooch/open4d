//! The Plain effector (clone-toolset spec E2-E8).

use std::collections::BTreeMap;

use forge_core::{Category, NodeId, NodeType, ParamDesc, Scene, Unit};
use forge_fields::FieldList;
use glam::{DMat3, DMat4, DVec3};

use crate::effector::{Effector, EffectorCtx};
use crate::node::Params;
use crate::{CloneData, CloneError, hpb};

/// Type id of the Plain effector.
pub const PLAIN: &str = "forge.effector.plain";

const BIG: f64 = 1e20;
const EFF: &str = "Effector";
const PAR: &str = "Parameter";
const COL: &str = "Color";
const WEIGHT: &str = "Weight";

/// The defaults that differ between the effectors sharing the Plain parameter set
/// (spec E2, E11).
#[derive(Debug, Clone, Copy)]
pub(crate) struct CommonDefaults {
    pub minimum: f64,
    pub p: [f64; 3],
    pub color_mode: &'static str,
    pub uniform_scale: bool,
    pub scale_value: f64,
    pub s: [f64; 3],
}

impl CommonDefaults {
    const PLAIN: CommonDefaults = CommonDefaults {
        minimum: 0.0,
        p: [0.0, 100.0, 0.0],
        color_mode: "fields_color",
        uniform_scale: false,
        scale_value: 0.0,
        s: [0.0; 3],
    };
}

/// The Plain effector's node type.
pub fn plain_type() -> NodeType {
    common_type(PLAIN, "Plain", &CommonDefaults::PLAIN)
}

/// A node type with the parameters every transforming effector shares (spec E2).
pub(crate) fn common_type(id: &str, name: &str, d: &CommonDefaults) -> NodeType {
    let pct = |id: &str, label: &str, default: f64, group: &str| {
        ParamDesc::float(id, label, default)
            .range(-BIG, BIG)
            .unit(Unit::Percent)
            .group(group)
    };
    NodeType::new(id, name, Category::Effector)
        .param(ParamDesc::bool("enabled", "Enabled", true).group(EFF))
        .param(pct("strength", "Strength", 1.0, EFF))
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
        .param(pct("maximum", "Maximum", 1.0, EFF))
        .param(pct("minimum", "Minimum", d.minimum, EFF))
        .param(
            ParamDesc::enumeration(
                "transform_mode",
                "Transform Mode",
                "relative",
                &[
                    ("relative", "Relative"),
                    ("absolute", "Absolute"),
                    ("remap", "Remap"),
                ],
            )
            .group(PAR),
        )
        .param(
            ParamDesc::enumeration(
                "transform_space",
                "Transform Space",
                "node",
                &[
                    ("node", "Node"),
                    ("effector", "Effector"),
                    ("object", "Object"),
                ],
            )
            .group(PAR),
        )
        .param(ParamDesc::bool("position", "Position", true).group(PAR))
        .param(
            ParamDesc::vec3("p", "P", d.p)
                .range(-BIG, BIG)
                .unit(Unit::Distance)
                .group(PAR),
        )
        .param(ParamDesc::bool("rotation", "Rotation", false).group(PAR))
        .param(
            ParamDesc::vec3("r", "R", [0.0; 3])
                .range(-BIG, BIG)
                .unit(Unit::Angle)
                .group(PAR),
        )
        .param(ParamDesc::bool("scale", "Scale", false).group(PAR))
        .param(ParamDesc::bool("uniform_scale", "Uniform Scale", d.uniform_scale).group(PAR))
        .param(ParamDesc::bool("absolute_scale", "Absolute Scale", false).group(PAR))
        .param(ParamDesc::vec3("s", "S", d.s).range(-BIG, BIG).group(PAR))
        .param(
            ParamDesc::float("scale_value", "Scale", d.scale_value)
                .range(-BIG, BIG)
                .group(PAR),
        )
        .param(
            ParamDesc::enumeration(
                "color_mode",
                "Color Mode",
                d.color_mode,
                &[
                    ("off", "Off"),
                    ("effector_color", "Effector Color"),
                    ("custom_color", "Custom Color"),
                    ("fields_color", "Fields Color"),
                ],
            )
            .group(COL),
        )
        .param(ParamDesc::color("color", "Color", [1.0; 3]).group(COL))
        .param(ParamDesc::bool("use_alpha_strength", "Use Alpha/Strength", false).group(COL))
        .param(
            ParamDesc::enumeration(
                "blending_mode",
                "Blending Mode",
                "default",
                &[
                    ("default", "Default"),
                    ("add", "Add"),
                    ("subtract", "Subtract"),
                    ("multiply", "Multiply"),
                    ("divide", "Divide"),
                ],
            )
            .group(COL),
        )
        .param(pct("weight_transform", "Weight Transform", 0.0, WEIGHT))
        .param(pct("u_transform", "U Transform", 0.0, WEIGHT))
        .param(pct("v_transform", "V Transform", 0.0, WEIGHT))
        .param(pct("modify_clone", "Modify Clone", 0.0, WEIGHT))
        .param(ParamDesc::bool("visibility", "Visibility", false).group(WEIGHT))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransformMode {
    Relative,
    Absolute,
    Remap,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Space {
    Node,
    Effector,
    Object,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorMode {
    Off,
    EffectorColor,
    CustomColor,
    FieldsColor,
}

/// The Plain effector's settings (fractions for percent values, radians for angles).
#[derive(Debug, Clone, PartialEq)]
pub struct Plain {
    pub strength: f64,
    pub maximum: f64,
    pub minimum: f64,
    pub mode: TransformMode,
    pub space: Space,
    pub position: Option<DVec3>,
    pub rotation: Option<DVec3>,
    /// Per-axis scale value; `uniform_scale` already expanded.
    pub scale: Option<DVec3>,
    /// One value for the three axes (the Random effector draws one value then).
    pub uniform_scale: bool,
    pub absolute_scale: bool,
    pub color_mode: ColorMode,
    pub color: [f64; 3],
    pub use_alpha_strength: bool,
    pub weight: f64,
    pub u: f64,
    pub v: f64,
    pub visibility: bool,
    /// Why the settings cannot run (an option without derived rules), if any.
    pub unsupported: Option<&'static str>,
    /// World matrix of the effector node.
    pub world: DMat4,
}

impl Default for Plain {
    /// The node defaults: Position on with P (0, 100, 0), everything else off.
    fn default() -> Self {
        Plain {
            strength: 1.0,
            maximum: 1.0,
            minimum: 0.0,
            mode: TransformMode::Relative,
            space: Space::Node,
            position: Some(DVec3::new(0.0, 100.0, 0.0)),
            rotation: None,
            scale: None,
            uniform_scale: false,
            absolute_scale: false,
            color_mode: ColorMode::FieldsColor,
            color: [1.0; 3],
            use_alpha_strength: false,
            weight: 0.0,
            u: 0.0,
            v: 0.0,
            visibility: false,
            unsupported: None,
            world: DMat4::IDENTITY,
        }
    }
}

/// The Plain effector of a node; `None` if the node is disabled.
pub(crate) fn from_node(scene: &Scene, id: NodeId) -> Option<Plain> {
    let node = scene.node(id)?;
    let ty = plain_type();
    settings_from_params(&ty, &node.params, scene.world_matrix(id)?)
}

pub(crate) fn settings_from_params(
    ty: &NodeType,
    values: &BTreeMap<String, forge_core::Value>,
    world: DMat4,
) -> Option<Plain> {
    let p = Params { ty, values };
    if !p.bool("enabled") {
        return None;
    }
    let on = |flag: &str, value: DVec3| p.bool(flag).then_some(value);
    let uniform = p.bool("uniform_scale");
    let scale_value = if uniform {
        DVec3::splat(p.float("scale_value"))
    } else {
        p.vec3("s")
    };
    let mut unsupported = None;
    if p.option("deformation") != "off" {
        unsupported = Some("effector deformation modes");
    }
    if p.option("blending_mode") != "default" {
        unsupported = Some("effector colour blending modes other than Default");
    }
    let color_mode = match p.option("color_mode").as_str() {
        "off" => ColorMode::Off,
        "effector_color" => ColorMode::EffectorColor,
        "custom_color" => ColorMode::CustomColor,
        _ => ColorMode::FieldsColor,
    };
    if color_mode == ColorMode::EffectorColor {
        unsupported = Some("effector colour mode Effector Color");
    }
    Some(Plain {
        strength: p.float("strength"),
        maximum: p.float("maximum"),
        minimum: p.float("minimum"),
        mode: match p.option("transform_mode").as_str() {
            "absolute" => TransformMode::Absolute,
            "remap" => TransformMode::Remap,
            _ => TransformMode::Relative,
        },
        space: match p.option("transform_space").as_str() {
            "effector" => Space::Effector,
            "object" => Space::Object,
            _ => Space::Node,
        },
        position: on("position", p.vec3("p")),
        rotation: on("rotation", p.vec3("r")),
        scale: on("scale", scale_value),
        uniform_scale: uniform,
        absolute_scale: p.bool("absolute_scale"),
        color_mode,
        color: p.color("color"),
        use_alpha_strength: p.bool("use_alpha_strength"),
        weight: p.float("weight_transform"),
        u: p.float("u_transform"),
        v: p.float("v_transform"),
        visibility: p.bool("visibility"),
        unsupported,
        world,
    })
}

/// The matrix with its three axes normalised (the rotation of a possibly scaled frame).
fn frame(m: &DMat4) -> DMat3 {
    let unit = |v: glam::DVec4, fallback: DVec3| v.truncate().try_normalize().unwrap_or(fallback);
    DMat3::from_cols(
        unit(m.x_axis, DVec3::X),
        unit(m.y_axis, DVec3::Y),
        unit(m.z_axis, DVec3::Z),
    )
}

impl Plain {
    /// The effect weight of a clone: strength times the field value, remapped between
    /// Minimum and Maximum in Remap mode (spec E3).
    fn amount(&self, value: f64) -> f64 {
        let w = self.strength * value;
        match self.mode {
            TransformMode::Remap => self.minimum + w * (self.maximum - self.minimum),
            _ => w,
        }
    }
}

/// The effect weight of one clone per channel (spec E12): Plain uses the same weight for
/// all of them, the Random effector draws each.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Weights {
    pub pos: DVec3,
    pub rot: DVec3,
    pub scale: DVec3,
    /// Weight, U, V and Visibility.
    pub scalar: f64,
}

impl Weights {
    pub(crate) fn uniform(w: f64) -> Weights {
        Weights {
            pos: DVec3::splat(w),
            rot: DVec3::splat(w),
            scale: DVec3::splat(w),
            scalar: w,
        }
    }
}

/// What a per-clone weight source may look at.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ClonePoint {
    /// Position in the clone list.
    pub position: usize,
    /// The clone's index property.
    pub index: u32,
    /// World position of the clone.
    pub world: DVec3,
    pub uvw: DVec3,
    /// The falloff value at the clone.
    pub value: f64,
}

impl Effector for Plain {
    fn apply(
        &self,
        data: &mut CloneData,
        falloff: &FieldList,
        ctx: &EffectorCtx,
    ) -> Result<(), CloneError> {
        self.apply_weighted(data, falloff, ctx, &mut |c| {
            Weights::uniform(self.amount(c.value))
        })
    }
}

impl Plain {
    /// The shared body of the Plain and Random effectors: `weights` gives each clone's
    /// weights, everything else is the Plain rule (spec E3-E8, E12).
    pub(crate) fn apply_weighted(
        &self,
        data: &mut CloneData,
        falloff: &FieldList,
        ctx: &EffectorCtx,
        weights: &mut dyn FnMut(&ClonePoint) -> Weights,
    ) -> Result<(), CloneError> {
        if let Some(what) = self.unsupported {
            return Err(CloneError::Unsupported(what));
        }
        let object_to_world = ctx.cloner_world;
        let effector_frame = frame(&(object_to_world.inverse() * self.world));
        let absolute = self.absolute_scale || self.mode == TransformMode::Absolute;
        for i in 0..data.len() {
            let m = data.matrices[i];
            let at = m.w_axis.truncate();
            let world = object_to_world.transform_point3(at);
            let sample = falloff.sample(world, ctx.eval);
            let value = sample.value;
            let w = weights(&ClonePoint {
                position: i,
                index: data.index[i],
                world,
                uvw: data.uvw[i],
                value,
            });
            let node_frame = frame(&m);
            let space = match self.space {
                Space::Node => node_frame,
                Space::Effector => effector_frame,
                Space::Object => DMat3::IDENTITY,
            };
            let mut m3 = DMat3::from_cols(
                m.x_axis.truncate(),
                m.y_axis.truncate(),
                m.z_axis.truncate(),
            );
            // Rotation: the delta turns about the space's axes (E4).
            if let Some(r) = self.rotation {
                let d = DMat3::from_mat4(hpb(r * w.rot));
                m3 = space * d * space.transpose() * m3;
            }
            // Scale: along the clone's own axes (E5).
            if let Some(s) = self.scale {
                let lens = [m3.x_axis.length(), m3.y_axis.length(), m3.z_axis.length()];
                let mut cols = [m3.x_axis, m3.y_axis, m3.z_axis];
                for k in 0..3 {
                    if lens[k] > 1e-12 {
                        let target = if absolute {
                            lens[k] + (s[k] - lens[k]) * w.scale[k]
                        } else {
                            lens[k] + s[k] * w.scale[k]
                        };
                        cols[k] *= target / lens[k];
                    }
                }
                m3 = DMat3::from_cols(cols[0], cols[1], cols[2]);
            }
            // Position: the delta in the space's axes (E4).
            let mut pos = at;
            if let Some(p) = self.position {
                pos += space * (p * w.pos);
            }
            data.matrices[i] = DMat4::from_cols(
                m3.x_axis.extend(0.0),
                m3.y_axis.extend(0.0),
                m3.z_axis.extend(0.0),
                pos.extend(1.0),
            );
            // Colour (E6).
            let wc = (value
                * if self.use_alpha_strength {
                    self.strength
                } else {
                    1.0
                })
            .clamp(0.0, 1.0);
            let target = match self.color_mode {
                ColorMode::CustomColor => Some(self.color),
                ColorMode::FieldsColor if !falloff.is_empty() => Some(sample.color),
                _ => None,
            };
            if let Some(t) = target {
                let c = data.colors[i];
                data.colors[i] = std::array::from_fn(|k| c[k] + (t[k] - c[k]) * wc);
            }
            // Weight, U, V (E7).
            let w_s = w.scalar;
            let set = |old: f64, value: f64| {
                if self.mode == TransformMode::Absolute {
                    old + (value - old) * w_s
                } else {
                    old + value * w_s
                }
            };
            data.weights[i] = set(data.weights[i], self.weight);
            data.uvw[i].x = set(data.uvw[i].x, self.u);
            data.uvw[i].y = set(data.uvw[i].y, self.v);
            // Visibility (E7).
            if self.visibility && w_s < 0.5 {
                data.visible[i] = false;
            }
        }
        Ok(())
    }
}
