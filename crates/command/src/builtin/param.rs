//! param.set

use forge_core::{NodeId, ParamDesc, ParamKind, Unit, Value as Param};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};

use super::spec;
use crate::error::CommandError;
use crate::registry::CommandRegistry;

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct ParamSetArgs {
    node: u64,
    /// Parameter id.
    param: String,
    /// Bool, number, `[x, y, z]`, string, node id or null, or an array of indices.
    /// Angles are degrees and percentages are as shown (50 is 50 %).
    value: serde_json::Value,
}

pub(super) fn register(r: &mut CommandRegistry) -> Result<(), CommandError> {
    r.register_fn(
        spec(
            "param.set",
            "Set Parameter",
            true,
            || json!({"node": 1, "param": "size", "value": 2.5}),
        ),
        |ctx, a: ParamSetArgs| {
            let id = NodeId(a.node);
            let scene = ctx.doc().scene();
            let node = scene
                .node(id)
                .ok_or_else(|| CommandError::Failed(format!("unknown node {id}")))?;
            let ty = scene
                .registry()
                .get(&node.type_id)
                .ok_or_else(|| CommandError::Failed(format!("unknown type {}", node.type_id)))?;
            let desc = ty
                .find_param(&a.param)
                .ok_or_else(|| CommandError::Failed(format!("unknown parameter {}", a.param)))?;
            let value = value_from_json(desc, &a.value)?;
            ctx.doc_mut()
                .transact("Set Parameter", |tx| tx.set_param(id, &a.param, value))?;
            Ok(Value::Null)
        },
    )
}

/// Display units (degrees, percent) to stored units (radians, fractions).
pub fn stored(unit: Unit, v: f64) -> f64 {
    match unit {
        Unit::Angle => v.to_radians(),
        Unit::Percent => v / 100.0,
        _ => v,
    }
}

fn number(v: &Value) -> Result<f64, CommandError> {
    v.as_f64()
        .filter(|f| f.is_finite())
        .ok_or_else(|| CommandError::InvalidArgs(format!("expected a number, got {v}")))
}

fn triple(v: &Value) -> Result<[f64; 3], CommandError> {
    match v.as_array().map(Vec::as_slice) {
        Some([x, y, z]) => Ok([number(x)?, number(y)?, number(z)?]),
        _ => Err(CommandError::InvalidArgs(format!(
            "expected [x, y, z], got {v}"
        ))),
    }
}

/// Converts JSON by the parameter kind (docs/specs/COMMANDS.md §6, `param.set`).
pub fn value_from_json(desc: &ParamDesc, v: &Value) -> Result<Param, CommandError> {
    let unit = desc.ui.unit;
    let wrong = |what: &str| CommandError::InvalidArgs(format!("{}: expected {what}", desc.id));
    Ok(match &desc.kind {
        ParamKind::Bool => Param::Bool(v.as_bool().ok_or_else(|| wrong("a boolean"))?),
        ParamKind::Int { .. } => Param::Int(v.as_i64().ok_or_else(|| wrong("an integer"))?),
        ParamKind::Float { .. } => Param::Float(stored(unit, number(v)?)),
        ParamKind::Vec3 { .. } => Param::Vec3(triple(v)?.map(|c| stored(unit, c))),
        ParamKind::Color => Param::Color(triple(v)?),
        ParamKind::Enum { .. } => Param::Enum(v.as_str().ok_or_else(|| wrong("a string"))?.into()),
        ParamKind::String => Param::String(v.as_str().ok_or_else(|| wrong("a string"))?.into()),
        ParamKind::Link { .. } => match v {
            Value::Null => Param::Link(None),
            _ => Param::Link(Some(NodeId(
                v.as_u64().ok_or_else(|| wrong("a node id or null"))?,
            ))),
        },
        ParamKind::LinkList { .. } => {
            let items = v.as_array().ok_or_else(|| wrong("an array of node ids"))?;
            Param::LinkList(
                items
                    .iter()
                    .map(|i| {
                        i.as_u64()
                            .map(NodeId)
                            .ok_or_else(|| wrong("an array of node ids"))
                    })
                    .collect::<Result<_, _>>()?,
            )
        }
        ParamKind::IndexList => {
            let items = v.as_array().ok_or_else(|| wrong("an array"))?;
            Param::IndexList(
                items
                    .iter()
                    .map(|i| {
                        i.as_u64()
                            .and_then(|n| u32::try_from(n).ok())
                            .ok_or_else(|| wrong("an array of indices"))
                    })
                    .collect::<Result<_, _>>()?,
            )
        }
        ParamKind::Mesh => {
            return Err(CommandError::Failed(format!(
                "{}: mesh parameters are not supported",
                desc.id
            )));
        }
    })
}
