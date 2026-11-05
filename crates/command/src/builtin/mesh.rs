//! mesh.make_editable, mesh.current_state_to_object.

use forge_core::{EditError, NodeId, Tx};
use forge_generators::EditableError;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};

use super::{spec, target_nodes};
use crate::context::CommandContext;
use crate::error::CommandError;
use crate::registry::CommandRegistry;

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct NodesArgs {
    /// Node ids; the current selection when absent.
    nodes: Option<Vec<u64>>,
}

type Convert = fn(&mut Tx, NodeId) -> Result<NodeId, EditableError>;

pub(super) fn register(r: &mut CommandRegistry) -> Result<(), CommandError> {
    r.register_fn(
        spec(
            "mesh.make_editable",
            "Make Editable",
            true,
            || json!({"nodes": [1]}),
        ),
        |c, a: NodesArgs| {
            convert(
                c,
                a.nodes,
                "Make Editable",
                forge_generators::make_editable,
                true,
            )
        },
    )?;
    r.register_fn(
        spec(
            "mesh.current_state_to_object",
            "Current State to Object",
            true,
            || json!({}),
        ),
        |c, a: NodesArgs| {
            convert(
                c,
                a.nodes,
                "Current State to Object",
                forge_generators::current_state_to_object,
                false,
            )
        },
    )?;
    Ok(())
}

/// Runs `convert` on every chosen node as one undo step. Nodes without a mesh are
/// skipped with a status message. With `select_new` the selection becomes the new objects
/// (Make Editable); without it the originals stay selected (Current State to Object,
/// EDITABLE.md §1.4).
fn convert(
    ctx: &mut dyn CommandContext,
    nodes: Option<Vec<u64>>,
    label: &str,
    convert: Convert,
    select_new: bool,
) -> Result<Value, CommandError> {
    let ids = target_nodes(ctx, nodes);
    if ids.is_empty() {
        return Err(CommandError::Failed(format!(
            "{label}: select an object first"
        )));
    }
    let mut made = Vec::new();
    let mut skipped = Vec::new();
    // A build failure is not an `EditError`: it is parked here and aborts the step with a
    // placeholder error, so the transaction reverts every edit made so far.
    let mut failure = None;
    let result = ctx.doc_mut().transact(label, |tx| {
        for id in ids {
            // consumed children of an earlier chosen node are gone already
            if !tx.scene().contains(id) {
                continue;
            }
            match convert(tx, id) {
                Ok(new) => made.push(new),
                Err(EditableError::NoMesh(name)) => skipped.push(name),
                Err(EditableError::Edit(e)) => return Err(e),
                Err(e @ EditableError::Build(_)) => {
                    failure = Some(e.to_string());
                    return Err(EditError::UnknownNode(id));
                }
            }
        }
        Ok(())
    });
    if let Some(msg) = failure {
        return Err(CommandError::Failed(format!("{label}: {msg}")));
    }
    result?;
    ctx.set_status(
        skipped
            .iter()
            .map(|name| format!("{label}: {name} has no mesh"))
            .collect::<Vec<_>>()
            .join("; "),
    );
    if let Some((first, rest)) = made.split_first().filter(|_| select_new) {
        let sel = ctx.selection_mut();
        sel.click(*first);
        for id in rest {
            sel.toggle(*id);
        }
    }
    ctx.retain_selection();
    Ok(json!({
        "made": made.iter().map(|n| n.0).collect::<Vec<_>>(),
        "skipped": skipped,
    }))
}
