//! The Delay effector (clone-toolset spec E23): clones follow their input with a delay.
//! The smoothing is a pure function of the clone data of the earlier frames, which the
//! caller (`apply_effectors`) replays; the effector keeps no state.

use forge_core::{Category, NodeId, NodeType, ParamDesc, Scene, Unit, Value};
use forge_fields::FieldList;
use glam::{DMat3, DMat4, DQuat, DVec3};

use crate::effector::{Effector, EffectorCtx};
use crate::node::Params;
use crate::plain::frame;
use crate::{CloneData, CloneError};

/// Type id of the Delay effector.
pub const DELAY: &str = "forge.effector.delay";

/// Number of earlier frames the replay looks at (spec E23).
pub const WINDOW: i64 = 240;

const EFF: &str = "Effector";
const PAR: &str = "Parameter";

/// The Delay effector's node type.
pub fn delay_type() -> NodeType {
    NodeType::new(DELAY, "Delay", Category::Effector)
        .param(ParamDesc::bool("enabled", "Enabled", true).group(EFF))
        .param(
            ParamDesc::float("strength", "Strength", 0.5)
                .range(0.0, 1.0)
                .unit(Unit::Percent)
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
                "mode",
                "Mode",
                "even",
                &[("even", "Even"), ("blend", "Blend"), ("spring", "Spring")],
            )
            .group(PAR),
        )
        .param(ParamDesc::bool("position", "Position", true).group(PAR))
        .param(ParamDesc::bool("rotation", "Rotation", true).group(PAR))
        .param(ParamDesc::bool("scale", "Scale", true).group(PAR))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DelayMode {
    Even,
    Blend,
    Spring,
}

/// The Delay effector's settings (fractions for percent values).
#[derive(Debug, Clone, PartialEq)]
pub struct Delay {
    pub strength: f64,
    pub mode: DelayMode,
    pub position: bool,
    pub rotation: bool,
    pub scale: bool,
    /// Why the settings cannot run, if any.
    pub unsupported: Option<&'static str>,
}

impl Default for Delay {
    /// The node defaults.
    fn default() -> Self {
        Delay {
            strength: 0.5,
            mode: DelayMode::Even,
            position: true,
            rotation: true,
            scale: true,
            unsupported: None,
        }
    }
}

/// The Delay effector of a node; `None` if the node is disabled.
pub(crate) fn from_node(scene: &Scene, id: NodeId) -> Option<Delay> {
    let node = scene.node(id)?;
    let ty = delay_type();
    let p = Params {
        ty: &ty,
        values: &node.params,
    };
    if !p.bool("enabled") {
        return None;
    }
    Some(Delay {
        strength: p.float("strength"),
        mode: match p.option("mode").as_str() {
            "blend" => DelayMode::Blend,
            "spring" => DelayMode::Spring,
            _ => DelayMode::Even,
        },
        position: p.bool("position"),
        rotation: p.bool("rotation"),
        scale: p.bool("scale"),
        unsupported: (p.option("deformation") != "off").then_some("effector deformation modes"),
    })
}

/// True if the value of `id` is a Delay effector that is on.
pub(crate) fn is_delay(scene: &Scene, id: NodeId) -> bool {
    scene.node(id).is_some_and(|n| n.type_id == DELAY)
        && matches!(scene.param(id, "enabled"), None | Some(Value::Bool(true)))
}

/// Position, rotation and axis lengths of a clone.
#[derive(Debug, Clone, Copy)]
struct Pose {
    pos: DVec3,
    rot: DQuat,
    scale: DVec3,
}

impl Pose {
    fn of(m: &DMat4) -> Pose {
        Pose {
            pos: m.w_axis.truncate(),
            rot: DQuat::from_mat3(&frame(m)).normalize(),
            scale: DVec3::new(
                m.x_axis.truncate().length(),
                m.y_axis.truncate().length(),
                m.z_axis.truncate().length(),
            ),
        }
    }

    fn matrix(&self) -> DMat4 {
        let r = DMat3::from_quat(self.rot);
        DMat4::from_cols(
            (r.x_axis * self.scale.x).extend(0.0),
            (r.y_axis * self.scale.y).extend(0.0),
            (r.z_axis * self.scale.z).extend(0.0),
            self.pos.extend(1.0),
        )
    }
}

/// The rotation vector (axis times angle, shortest way) of `a · b⁻¹`.
fn rotation_vector(a: DQuat, b: DQuat) -> DVec3 {
    let mut q = (a * b.inverse()).normalize();
    if q.w < 0.0 {
        q = -q;
    }
    let (axis, angle) = q.to_axis_angle();
    axis * angle
}

/// The velocities of the spring mode (position, rotation vector, scale).
#[derive(Debug, Clone, Copy, Default)]
struct Velocity {
    pos: DVec3,
    rot: DVec3,
    scale: DVec3,
}

impl Delay {
    /// Runs the delay over `history` (the input clone data of the frames before this one,
    /// oldest first) and the current input `data`, which becomes the output.
    pub fn smooth(
        &self,
        history: &[CloneData],
        data: &mut CloneData,
        falloff: &FieldList,
        ctx: &EffectorCtx,
    ) -> Result<(), CloneError> {
        if let Some(what) = self.unsupported {
            return Err(CloneError::Unsupported(what));
        }
        // Only the newest run of frames with the current clone count counts.
        let n = data.len();
        let usable = history.iter().rev().take_while(|h| h.len() == n).count();
        let history = &history[history.len() - usable..];
        if history.is_empty() || n == 0 {
            return Ok(());
        }
        for i in 0..n {
            let at = data.matrices[i].w_axis.truncate();
            let value = falloff
                .sample(ctx.cloner_world.transform_point3(at), ctx.eval)
                .value;
            let s = (self.strength * value).clamp(0.0, 1.0);
            let mut inputs = history
                .iter()
                .map(|h| Pose::of(&h.matrices[i]))
                .chain(std::iter::once(Pose::of(&data.matrices[i])));
            let mut previous = inputs.next().expect("history is not empty");
            let mut out = previous;
            let mut velocity = Velocity::default();
            for x in inputs {
                out = self.step(out, previous, x, s, &mut velocity);
                previous = x;
            }
            data.matrices[i] = out.matrix();
        }
        Ok(())
    }

    /// One frame: the output `y` follows the input `x` (`previous` is the last frame's
    /// input), `s` the delay of the clone.
    fn step(&self, y: Pose, previous: Pose, x: Pose, s: f64, v: &mut Velocity) -> Pose {
        let k = 1.0 - s;
        let mut out = x;
        match self.mode {
            DelayMode::Even => {
                out.pos = y.pos + (x.pos - y.pos) * k;
                out.rot = y.rot.slerp(x.rot, k);
                out.scale = y.scale + (x.scale - y.scale) * k;
            }
            DelayMode::Blend => {
                out.pos = previous.pos.lerp(x.pos, k);
                out.rot = previous.rot.slerp(x.rot, k);
                out.scale = previous.scale.lerp(x.scale, k);
            }
            DelayMode::Spring => {
                v.pos = v.pos * s + (x.pos - y.pos) * k;
                v.rot = v.rot * s + rotation_vector(x.rot, y.rot) * k;
                v.scale = v.scale * s + (x.scale - y.scale) * k;
                out.pos = y.pos + v.pos;
                out.rot = (DQuat::from_scaled_axis(v.rot) * y.rot).normalize();
                out.scale = y.scale + v.scale;
            }
        }
        if !self.position {
            out.pos = x.pos;
        }
        if !self.rotation {
            out.rot = x.rot;
        }
        if !self.scale {
            out.scale = x.scale;
        }
        out
    }
}

impl Effector for Delay {
    /// Without earlier frames the output is the input (frame 0 of the observation).
    fn apply(
        &self,
        data: &mut CloneData,
        falloff: &FieldList,
        ctx: &EffectorCtx,
    ) -> Result<(), CloneError> {
        self.smooth(&[], data, falloff, ctx)
    }
}
