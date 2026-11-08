//! The Step effector (clone-toolset spec E19): the Plain effector with a weight that
//! grows along the clone list through a curve.

use std::collections::BTreeMap;

use forge_core::{NodeId, NodeType, ParamDesc, Scene};
use forge_fields::FieldList;
use glam::DMat4;

use crate::effector::{Effector, EffectorCtx};
use crate::node::Params;
use crate::plain::{CommonDefaults, Plain, Weights, common_type, settings_from_params};
use crate::{CloneData, CloneError};

/// Type id of the Step effector.
pub const STEP: &str = "forge.effector.step";

const STEP_GROUP: &str = "Step";
const GAP_MAX: i64 = 2_147_483_647;

const DEFAULTS: CommonDefaults = CommonDefaults {
    minimum: 0.0,
    position: false,
    p: [0.0; 3],
    rotation: false,
    r: [0.0; 3],
    scale: true,
    color_mode: "off",
    uniform_scale: true,
    scale_value: 1.0,
    s: [0.0; 3],
};

/// The Step effector's node type: the Plain parameters with the Step defaults, plus the
/// curve and Step Gap.
pub fn step_type() -> NodeType {
    common_type(STEP, "Step", &DEFAULTS)
        .param(
            ParamDesc::enumeration(
                "curve",
                "Curve",
                "linear",
                &[
                    ("linear", "Linear"),
                    ("ease_in", "Ease In"),
                    ("ease_out", "Ease Out"),
                    ("ease_in_out", "Ease In Out"),
                ],
            )
            .group(STEP_GROUP),
        )
        .param(
            ParamDesc::int("step_gap", "Step Gap", 0)
                .range(0, GAP_MAX)
                .group(STEP_GROUP),
        )
}

/// The shape of the weight along the clone list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepCurve {
    Linear,
    EaseIn,
    EaseOut,
    EaseInOut,
}

impl StepCurve {
    /// The curve at `t` in 0..1 (spec E19).
    pub fn at(self, t: f64) -> f64 {
        match self {
            StepCurve::Linear => t,
            StepCurve::EaseIn => t * t,
            StepCurve::EaseOut => 1.0 - (1.0 - t) * (1.0 - t),
            StepCurve::EaseInOut => t * t * (3.0 - 2.0 * t),
        }
    }
}

/// The Step effector's settings.
#[derive(Debug, Clone, PartialEq)]
pub struct Step {
    pub base: Plain,
    pub curve: StepCurve,
}

impl Default for Step {
    /// The node defaults.
    fn default() -> Self {
        from_values(&BTreeMap::new(), DMat4::IDENTITY).expect("enabled by default")
    }
}

fn from_values(values: &BTreeMap<String, forge_core::Value>, world: DMat4) -> Option<Step> {
    let ty = step_type();
    let mut base = settings_from_params(&ty, values, world)?;
    let p = Params { ty: &ty, values };
    if p.int("step_gap") != 0 {
        base.unsupported = Some("Step Gap other than 0");
    }
    Some(Step {
        base,
        curve: match p.option("curve").as_str() {
            "ease_in" => StepCurve::EaseIn,
            "ease_out" => StepCurve::EaseOut,
            "ease_in_out" => StepCurve::EaseInOut,
            _ => StepCurve::Linear,
        },
    })
}

/// The Step effector of a node; `None` if the node is disabled.
pub(crate) fn from_node(scene: &Scene, id: NodeId) -> Option<Step> {
    from_values(&scene.node(id)?.params, scene.world_matrix(id)?)
}

impl Effector for Step {
    fn apply(
        &self,
        data: &mut CloneData,
        falloff: &FieldList,
        ctx: &EffectorCtx,
    ) -> Result<(), CloneError> {
        let last = data.len().saturating_sub(1);
        self.base.apply_weighted(data, falloff, ctx, &mut |c| {
            let t = if last == 0 {
                0.0
            } else {
                c.position as f64 / last as f64
            };
            Weights::uniform(self.base.amount(c.value * self.curve.at(t)))
        })
    }
}
