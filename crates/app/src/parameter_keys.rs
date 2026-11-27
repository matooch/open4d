//! Descriptor-derived row keying for the Attributes panel (ANIM.md §2.5).

use forge_command::{
    CommandContext, CommandError, CommandRegistry, CommandSpec, param_value_from_json,
};
use forge_core::{Interp, Key, NodeId, ParamDesc, ParamKind, Value, parse_track_target};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value as Json, json};

/// The ordered track keys represented by an editable parameter row.
pub fn tracks(desc: &ParamDesc) -> Vec<String> {
    let axes: &[&str] = match desc.kind {
        ParamKind::Vec3 { .. } => &["x", "y", "z"],
        ParamKind::Color => &["r", "g", "b"],
        _ => &[],
    };
    let paths = if axes.is_empty() {
        vec![desc.id.clone()]
    } else {
        axes.iter()
            .map(|axis| format!("{}.{axis}", desc.id))
            .collect()
    };
    paths
        .into_iter()
        .filter(|key| parse_track_target(key, |id| (id == desc.id).then_some(desc)).is_ok())
        .collect()
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct SetArgs {
    node: u64,
    /// Parameter id, without a vector or colour component suffix.
    param: String,
    frame: i64,
    /// Full parameter value in display units, as for `param.set`.
    value: Json,
}

pub fn register(r: &mut CommandRegistry) -> Result<(), CommandError> {
    r.register_fn(CommandSpec {
        id: "key.set_parameter",
        label: "Set Parameter Key",
        undoable: true,
        sample: || json!({"node": 1, "param": "transform.pos", "frame": 10, "value": [0, 10, 0]}),
    }, set)
}

fn channels(desc: &ParamDesc, value: Value) -> Option<(Vec<f64>, Interp)> {
    let (values, interp) = match value {
        Value::Float(v) => (vec![v], Interp::Spline),
        Value::Int(v) if v.unsigned_abs() <= (1 << 53) => (vec![v as f64], Interp::Spline),
        Value::Vec3(v) | Value::Color(v) => (v.to_vec(), Interp::Spline),
        Value::Bool(v) => (vec![f64::from(u8::from(v))], Interp::Step),
        Value::Enum(v) => {
            let ParamKind::Enum { options } = &desc.kind else {
                return None;
            };
            let i = options.iter().position(|o| o.id == v)?;
            (vec![i as f64], Interp::Step)
        }
        Value::Link(v) if v.is_none_or(|id| id.0 <= (1 << 53)) => {
            (vec![v.map_or(0.0, |id| id.0 as f64)], Interp::Step)
        }
        _ => return None,
    };
    Some((values, interp))
}

fn set(ctx: &mut dyn CommandContext, args: SetArgs) -> Result<Json, CommandError> {
    let node = NodeId(args.node);
    let desc = ctx
        .doc()
        .scene()
        .param_desc(node, &args.param)
        .ok_or_else(|| CommandError::failed("unknown parameter"))?;
    let value = param_value_from_json(desc, &args.value)?;
    let count = key_parameter(ctx.doc_mut(), node, &args.param, args.frame, value)?;
    Ok(json!({"keys": count}))
}

/// Keys the whole parameter `param` of `node` with `value` at `frame` (all three components
/// of a vector or colour), one undo step. Returns how many keys were written or changed.
/// The command `key.set_parameter` and the draft commits of the editor call this.
pub fn key_parameter(
    doc: &mut forge_core::Document,
    node: NodeId,
    param: &str,
    frame: i64,
    value: Value,
) -> Result<usize, CommandError> {
    let scene = doc.scene();
    let desc = scene
        .param_desc(node, param)
        .ok_or_else(|| CommandError::failed("unknown parameter"))?;
    let tracks = tracks(desc);
    let value = desc.validate(value).map_err(CommandError::failed)?;
    if let (ParamKind::Link { accepts }, Value::Link(Some(target))) = (&desc.kind, &value) {
        let valid = if param == forge_core::layers::LAYER {
            scene.layers().contains(*target)
        } else {
            node != *target
                && scene
                    .node_type(*target)
                    .is_some_and(|ty| accepts.is_empty() || accepts.contains(&ty.category))
        };
        if !valid {
            return Err(CommandError::failed("invalid key link target"));
        }
    }
    let (values, interp) =
        channels(desc, value).ok_or_else(|| CommandError::failed("value cannot be keyed"))?;
    if tracks.len() != values.len() {
        return Err(CommandError::failed("parameter cannot be keyed"));
    }
    let fps = f64::from(scene.settings().fps);
    let time = frame as f64 / fps;
    let count = doc.transact("Set Parameter Key", |tx| {
        let mut count = 0;
        for (track, value) in tracks.iter().zip(values) {
            let existing = tx
                .scene()
                .node(node)
                .and_then(|n| n.tracks.get(track))
                .and_then(|t| {
                    t.keys
                        .iter()
                        .find(|k| (k.time * fps - frame as f64).abs() < 1e-6)
                })
                .copied();
            if existing.is_some_and(|key| key.value == value) {
                continue;
            }
            let mut key = existing.unwrap_or(Key {
                time,
                value,
                interp,
            });
            key.value = value;
            tx.set_key(node, track, key)?;
            count += 1;
        }
        Ok(count)
    })?;
    Ok(count)
}
