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
                    ("custom", "Custom"),
                ],
            )
            .group(STEP_GROUP),
        )
        .param(ParamDesc::vec3("curve_start", "Curve Start", [0.0, 0.0, 0.0]).group(STEP_GROUP))
        .param(
            ParamDesc::vec3(
                "curve_start_handle",
                "Curve Start Handle",
                [0.25, 0.25, 0.0],
            )
            .group(STEP_GROUP),
        )
        .param(ParamDesc::vec3("curve_end", "Curve End", [1.0, 1.0, 0.0]).group(STEP_GROUP))
        .param(
            ParamDesc::vec3("curve_end_handle", "Curve End Handle", [-0.25, -0.25, 0.0])
                .group(STEP_GROUP),
        )
        .param(
            ParamDesc::int("step_gap", "Step Gap", 0)
                .range(0, GAP_MAX)
                .group(STEP_GROUP),
        )
}

/// Bezier control point of a Custom curve knot: `position + HANDLE_SCALE * handle`.
const HANDLE_SCALE: f64 = 4.0 / 3.0;

/// The shape of the weight along the clone list.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StepCurve {
    Linear,
    EaseIn,
    EaseOut,
    EaseInOut,
    /// Two Bezier knots `(position, handle)` as the app's spline editor stores them, in the
    /// curve's own units (x: position along the list, y: the weight); the Bezier control point is
    /// `position + 4/3 * handle` (`cloner/step-spline`, spec E19).
    Custom {
        start: [f64; 2],
        start_handle: [f64; 2],
        end: [f64; 2],
        end_handle: [f64; 2],
    },
}

impl StepCurve {
    /// The curve at `t` in 0..1 (spec E19).
    pub fn at(self, t: f64) -> f64 {
        match self {
            StepCurve::Linear => t,
            StepCurve::EaseIn => t * t,
            StepCurve::EaseOut => 1.0 - (1.0 - t) * (1.0 - t),
            StepCurve::EaseInOut => t * t * (3.0 - 2.0 * t),
            StepCurve::Custom {
                start,
                start_handle,
                end,
                end_handle,
            } => {
                let c = |a: [f64; 2], h: [f64; 2], i: usize| a[i] + h[i] * HANDLE_SCALE;
                let at = |i: usize, u: f64| {
                    let w = 1.0 - u;
                    w * w * w * start[i]
                        + 3.0 * w * w * u * c(start, start_handle, i)
                        + 3.0 * w * u * u * c(end, end_handle, i)
                        + u * u * u * end[i]
                };
                // x is monotonic for sane handles: solve x(u) = t by bisection
                let (mut lo, mut hi) = (0.0, 1.0);
                for _ in 0..60 {
                    let mid = 0.5 * (lo + hi);
                    if at(0, mid) < t {
                        lo = mid;
                    } else {
                        hi = mid;
                    }
                }
                at(1, 0.5 * (lo + hi))
            }
        }
    }
}

/// The Step effector's settings.
#[derive(Debug, Clone, PartialEq)]
pub struct Step {
    pub base: Plain,
    pub curve: StepCurve,
    /// Step Gap: clones in runs of `gap + 1` share the weight of the run's first clone.
    pub gap: usize,
}

impl Default for Step {
    /// The node defaults.
    fn default() -> Self {
        from_values(&BTreeMap::new(), DMat4::IDENTITY).expect("enabled by default")
    }
}

fn from_values(values: &BTreeMap<String, forge_core::Value>, world: DMat4) -> Option<Step> {
    let ty = step_type();
    let base = settings_from_params(&ty, values, world)?;
    let p = Params { ty: &ty, values };
    let xy = |id: &str| {
        let v = p.vec3(id);
        [v[0], v[1]]
    };
    Some(Step {
        gap: p.int("step_gap").max(0) as usize,
        base,
        curve: match p.option("curve").as_str() {
            "ease_in" => StepCurve::EaseIn,
            "ease_out" => StepCurve::EaseOut,
            "ease_in_out" => StepCurve::EaseInOut,
            "custom" => StepCurve::Custom {
                start: xy("curve_start"),
                start_handle: xy("curve_start_handle"),
                end: xy("curve_end"),
                end_handle: xy("curve_end_handle"),
            },
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
            let run = self.gap + 1;
            let t = if last == 0 {
                0.0
            } else {
                (c.position / run * run) as f64 / last as f64
            };
            Weights::uniform(self.base.amount(c.value * self.curve.at(t)))
        })
    }
}
