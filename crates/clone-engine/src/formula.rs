//! The Formula effector (clone-toolset spec E20): the Plain effector with the weight of
//! each clone computed by a formula (forge-expr, degrees mode).

use std::collections::BTreeMap;

use forge_core::{NodeId, NodeType, ParamDesc, Scene};
use forge_expr::Expr;
use forge_fields::FieldList;
use glam::{DMat4, DQuat, EulerRot};

use crate::effector::{Effector, EffectorCtx};
use crate::node::Params;
use crate::plain::{CommonDefaults, Plain, Weights, common_type, frame, settings_from_params};
use crate::{CloneData, CloneError};

/// Type id of the Formula effector.
pub const FORMULA: &str = "forge.effector.formula";

/// The default formula text (O14).
pub const DEFAULT_FORMULA: &str = "sin(((id / count) + t) * f * 360.0)";

const BIG: f64 = 1e20;
const GROUP: &str = "Formula";

/// The variables of the formula, in the order of `variables` below.
const VARIABLES: [&str; 17] = [
    "px", "py", "pz", "rx", "ry", "rz", "sx", "sy", "sz", "u", "v", "w", "id", "count", "falloff",
    "t", "f",
];

const DEFAULTS: CommonDefaults = CommonDefaults {
    minimum: 0.0,
    position: true,
    p: [50.0, 0.0, 0.0],
    rotation: false,
    r: [0.0; 3],
    scale: true,
    color_mode: "off",
    uniform_scale: true,
    scale_value: 1.0,
    s: [0.0; 3],
};

/// The Formula effector's node type: the Plain parameters with the Formula defaults, plus
/// the formula, `t - Project Time` and `f - Frequency`.
pub fn formula_type() -> NodeType {
    common_type(FORMULA, "Formula", &DEFAULTS)
        .param(ParamDesc::string("formula", "Formula", DEFAULT_FORMULA).group(GROUP))
        .param(
            ParamDesc::float("t_project_time", "t - Project Time", 1.0)
                .range(-BIG, BIG)
                .group(GROUP),
        )
        .param(
            ParamDesc::float("f_frequency", "f - Frequency", 1.0)
                .range(-BIG, BIG)
                .group(GROUP),
        )
}

/// The Formula effector's settings.
#[derive(Debug, Clone, PartialEq)]
pub struct Formula {
    pub base: Plain,
    pub formula: String,
    /// Multiplier of the project time in seconds.
    pub t: f64,
    pub f: f64,
}

impl Default for Formula {
    /// The node defaults.
    fn default() -> Self {
        from_values(&BTreeMap::new(), DMat4::IDENTITY).expect("enabled by default")
    }
}

fn from_values(values: &BTreeMap<String, forge_core::Value>, world: DMat4) -> Option<Formula> {
    let ty = formula_type();
    let base = settings_from_params(&ty, values, world)?;
    let p = Params { ty: &ty, values };
    let formula = match values.get("formula") {
        Some(forge_core::Value::String(s)) => s.clone(),
        _ => DEFAULT_FORMULA.to_string(),
    };
    Some(Formula {
        base,
        formula,
        t: p.float("t_project_time"),
        f: p.float("f_frequency"),
    })
}

/// The Formula effector of a node; `None` if the node is disabled.
pub(crate) fn from_node(scene: &Scene, id: NodeId) -> Option<Formula> {
    from_values(&scene.node(id)?.params, scene.world_matrix(id)?)
}

impl Effector for Formula {
    fn apply(
        &self,
        data: &mut CloneData,
        falloff: &FieldList,
        ctx: &EffectorCtx,
    ) -> Result<(), CloneError> {
        let expr = Expr::parse_degrees(&self.formula, &VARIABLES)
            .map_err(|_| CloneError::Unsupported("a formula that does not parse"))?;
        let count = data.len() as f64;
        let time = ctx.eval.time * self.t;
        // The clone data before the effector: the closure cannot borrow `data`.
        let before = data.clone();
        self.base.apply_weighted(data, falloff, ctx, &mut |c| {
            let m = before.matrices[c.position];
            let f = frame(&m);
            let (ry, rx, rz) = DQuat::from_mat3(&f).to_euler(EulerRot::YXZ);
            let at = m.w_axis;
            let uvw = before.uvw[c.position];
            let vars = [
                at.x,
                at.y,
                at.z,
                -ry,
                -rx,
                -rz,
                m.x_axis.truncate().length(),
                m.y_axis.truncate().length(),
                m.z_axis.truncate().length(),
                uvw.x,
                uvw.y,
                uvw.z,
                c.position as f64,
                count,
                c.value,
                time,
                self.f,
            ];
            let r = expr.eval(&vars);
            let r = if r.is_finite() { r } else { 0.0 };
            Weights::uniform(self.base.amount(c.value * r))
        })
    }
}
