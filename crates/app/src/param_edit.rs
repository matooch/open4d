//! `param.edit`: the attribute manager's edit command (docs/specs/COMMANDS.md §6, UI.md §12).
//!
//! Like `param.set` (display units, one undo step) but animation-aware (autokey and rule 5,
//! ANIM.md §2.4), applicable to several objects at once, and able to merge a continuous drag
//! into one undo step.

use forge_command::{
    CommandContext, CommandError, CommandRegistry, CommandSpec, display_to_stored,
    param_value_from_json,
};
use forge_core::{NodeId, ParamKind, Value as Param};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::animate::{AnimCtx, write_param};

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct ParamEditArgs {
    /// Parameter id.
    param: String,
    /// As for `param.set` (degrees, percent as shown). With `component`, one number.
    value: serde_json::Value,
    /// Objects to edit; absent means the current selection. Objects whose type lacks the
    /// parameter (or has it with another kind) are skipped.
    nodes: Option<Vec<u64>>,
    /// Vec3 parameters only: set just this component (0, 1, 2) and keep the others of each
    /// object.
    component: Option<u8>,
    /// Edits with the same gesture number merge into one undo step (a scrub or colour drag).
    gesture: Option<u64>,
}

/// Registers `param.edit`.
pub fn register(r: &mut CommandRegistry) -> Result<(), CommandError> {
    r.register_fn(
        CommandSpec {
            id: "param.edit",
            label: "Edit Parameter",
            undoable: true,
            sample: || json!({"param": "size", "value": 2.5, "nodes": [1], "gesture": 1}),
        },
        edit,
    )
}

fn edit(ctx: &mut dyn CommandContext, a: ParamEditArgs) -> Result<Value, CommandError> {
    let ids: Vec<NodeId> = match &a.nodes {
        Some(n) => n.iter().map(|i| NodeId(*i)).collect(),
        None => ctx.selection().nodes.iter().copied().collect(),
    };
    let shown = ctx.evaluated_scene();
    let mut reference: Option<(ParamKind, String)> = None;
    let mut targets: Vec<(NodeId, Param)> = Vec::new();
    for id in &ids {
        let scene = ctx.doc().scene();
        let Some(desc) = scene
            .node_type(*id)
            .and_then(|t| t.find_param(&a.param))
            .cloned()
        else {
            continue;
        };
        match &reference {
            None => reference = Some((desc.kind.clone(), desc.label.clone())),
            Some((kind, _)) if *kind != desc.kind => continue,
            Some(_) => {}
        }
        let value = match a.component {
            Some(c) => {
                let ParamKind::Vec3 { .. } = desc.kind else {
                    return Err(CommandError::InvalidArgs(format!(
                        "{}: component needs a vec3 parameter",
                        a.param
                    )));
                };
                let c = usize::from(c);
                let x = a
                    .value
                    .as_f64()
                    .filter(|x| x.is_finite() && c < 3)
                    .ok_or_else(|| {
                        CommandError::InvalidArgs("expected a number and component 0..2".into())
                    })?;
                let mut v = match shown.param(*id, &a.param).or(scene.param(*id, &a.param)) {
                    Some(Param::Vec3(v)) => *v,
                    _ => match desc.default {
                        Param::Vec3(v) => v,
                        _ => [0.0; 3],
                    },
                };
                v[c] = display_to_stored(desc.ui.unit, x);
                Param::Vec3(v)
            }
            None => param_value_from_json(&desc, &a.value)?,
        };
        targets.push((*id, value));
    }
    let Some((_, label)) = reference else {
        return Err(CommandError::Failed(format!(
            "no object has parameter {}",
            a.param
        )));
    };
    let anim = AnimCtx {
        frame: ctx.timeline().current,
        fps: ctx.doc().scene().settings().fps,
        autokey: ctx.autokey(),
    };
    let label = format!("Set {label}");
    let n = targets.len();
    let param = a.param.clone();
    let run = |tx: &mut forge_core::Tx| {
        for (id, v) in &targets {
            write_param(tx, *id, &param, v.clone(), Some(&anim))?;
        }
        Ok(())
    };
    match a.gesture {
        Some(g) => {
            let key = format!(
                "drag:{}:{}:{g}",
                targets
                    .iter()
                    .map(|(i, _)| i.to_string())
                    .collect::<Vec<_>>()
                    .join(","),
                a.param
            );
            ctx.doc_mut().transact_merged(&label, &key, run)?;
        }
        None => ctx.doc_mut().transact(&label, run)?,
    }
    Ok(json!({"changed": n}))
}
