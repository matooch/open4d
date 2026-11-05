//! Formula field: a one-line formula of the sample (docs/specs/FIELDS.md F7), compiled with
//! `forge-expr` in degrees mode.

use forge_core::{EvalCtx, NodeType, ParamDesc, Unit};
use forge_expr::Expr;
use glam::{DMat4, DVec3};

use crate::node::{BIG, FIELD, Params, field_type};
use crate::remap::Remap;
use crate::{Field, FieldError, FieldSample};

pub const FORMULA: &str = "forge.field.formula";

/// The text a new Formula field starts with (O23 `formula-defaults`).
pub const DEFAULT_FORMULA: &str = "0.5 + sin( ( subfields + ( id / count ) + d ) * f * 360 ) * 0.5";

/// The variables a formula may use, in the order [`FormulaField`] binds them.
pub const VARIABLES: [&str; 9] = ["px", "py", "pz", "id", "count", "subfields", "d", "t", "f"];

pub fn formula_type() -> NodeType {
    let mut ty = field_type(
        FORMULA,
        "Formula Field",
        vec![
            ParamDesc::string("formula", "Formula", DEFAULT_FORMULA).group(FIELD),
            ParamDesc::float("time_delta", "t (Time Delta)", 1.0)
                .range(-BIG, BIG)
                .unit(Unit::Percent)
                .group(FIELD),
            ParamDesc::float("frequency", "f (Frequency)", 1.0)
                .range(-BIG, BIG)
                .group(FIELD),
        ],
        0.0,
        false,
        false,
    );
    // The observed Formula field has both clamps off (`formula-defaults`, O23).
    for p in &mut ty.params {
        if p.id == "clamp_min" || p.id == "clamp_max" {
            p.default = forge_core::Value::Bool(false);
        }
    }
    ty
}

/// A formula evaluated per sample.
pub struct FormulaField {
    expr: Expr,
    remap: Remap,
    color: [f64; 3],
    world_to_local: DMat4,
    time_delta: f64,
    frequency: f64,
}

impl FormulaField {
    /// The value of sample number `id` of `count` at `p_world`.
    fn value(&self, p_world: DVec3, ctx: &EvalCtx, id: usize, count: usize) -> FieldSample {
        let p = self.world_to_local.transform_point3(p_world);
        let vars = [
            p.x,
            p.y,
            p.z,
            id as f64,
            count as f64,
            0.0, // subfields: no sub-field list yet
            0.0, // d: parity-pending
            ctx.time * self.time_delta,
            self.frequency,
        ];
        let v = self.expr.eval(&vars);
        let t = if v.is_finite() {
            v.clamp(0.0, 1.0)
        } else {
            0.0
        };
        FieldSample {
            color: self.color,
            ..FieldSample::from_value(self.remap.apply(t))
        }
    }
}

impl Field for FormulaField {
    fn sample(&self, p_world: DVec3, ctx: &EvalCtx) -> FieldSample {
        self.value(p_world, ctx, 0, 1)
    }

    fn sample_many(&self, points: &[DVec3], ctx: &EvalCtx) -> Vec<FieldSample> {
        points
            .iter()
            .enumerate()
            .map(|(i, p)| self.value(*p, ctx, i, points.len()))
            .collect()
    }
}

/// True if the formula text names the variable `t`.
pub(crate) fn mentions_time(text: &str) -> bool {
    text.split(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
        .any(|w| w.eq_ignore_ascii_case("t"))
}

pub(crate) fn field(p: &Params<'_>, world: DMat4) -> Result<FormulaField, FieldError> {
    let remap = p.remap();
    remap.check()?;
    let text = match p.get("formula") {
        forge_core::Value::String(s) => s,
        other => unreachable!("formula is not a string: {other:?}"),
    };
    let expr =
        Expr::parse_degrees(&text, &VARIABLES).map_err(|e| FieldError::Formula(e.to_string()))?;
    Ok(FormulaField {
        expr,
        remap,
        color: p.color("color"),
        world_to_local: world.inverse(),
        time_delta: p.float("time_delta"),
        frequency: p.float("frequency"),
    })
}
