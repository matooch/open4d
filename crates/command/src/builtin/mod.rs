//! The editor's built-in commands (docs/specs/COMMANDS.md §6).

use forge_core::NodeId;
use serde_json::{Value, json};

use crate::context::CommandContext;
use crate::error::CommandError;
use crate::registry::{CommandRegistry, CommandSpec, NoArgs};

mod arrange;
mod axis;
mod building;
mod components;
mod cutting;
mod file;
mod layer;
mod mesh;
mod object;
mod param;
mod snap;
pub use param::{stored as display_to_stored, value_from_json as param_value_from_json};
mod timeline;
pub use timeline::timeline_state;
mod tools;

/// Registers every built-in command, in a fixed order.
pub fn register_editor_commands(r: &mut CommandRegistry) -> Result<(), CommandError> {
    object::register(r)?;
    layer::register(r)?;
    mesh::register(r)?;
    tools::register(r)?;
    arrange::register(r)?;
    cutting::register(r)?;
    building::register(r)?;
    components::register(r)?;
    axis::register(r)?;
    snap::register(r)?;
    param::register(r)?;
    timeline::register(r)?;
    file::register(r)?;
    Ok(())
}

pub(crate) fn spec(
    id: &'static str,
    label: &'static str,
    undoable: bool,
    sample: fn() -> Value,
) -> CommandSpec {
    CommandSpec {
        id,
        label,
        undoable,
        sample,
    }
}

pub(crate) fn no_args() -> Value {
    json!({})
}

/// Registers a command without arguments.
pub(crate) fn register_plain(
    r: &mut CommandRegistry,
    id: &'static str,
    label: &'static str,
    undoable: bool,
    run: fn(&mut dyn CommandContext) -> Result<Value, CommandError>,
) -> Result<(), CommandError> {
    // A non-capturing closure cannot carry `run`, so each plain command goes through a
    // small adapter type.
    r.register(Box::new(Plain {
        spec: spec(id, label, undoable, no_args),
        run,
    }))
}

struct Plain {
    spec: CommandSpec,
    run: fn(&mut dyn CommandContext) -> Result<Value, CommandError>,
}

impl crate::registry::CommandHandler for Plain {
    fn id(&self) -> &str {
        self.spec.id
    }

    fn label(&self) -> &str {
        self.spec.label
    }

    fn args_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(NoArgs)).unwrap_or(Value::Null)
    }

    fn run(&self, ctx: &mut dyn CommandContext, args: Value) -> Result<Value, CommandError> {
        if !(args.is_null() || args.as_object().is_some_and(|o| o.is_empty())) {
            return Err(CommandError::InvalidArgs(format!(
                "{} takes no arguments",
                self.spec.id
            )));
        }
        (self.run)(ctx)
    }

    fn undoable(&self) -> bool {
        self.spec.undoable
    }
}

/// The nodes a command acts on: the explicit ids, or the current selection.
pub(crate) fn target_nodes(ctx: &dyn CommandContext, nodes: Option<Vec<u64>>) -> Vec<NodeId> {
    match nodes {
        Some(ids) => ids.into_iter().map(NodeId).collect(),
        None => ctx.selection().nodes.iter().copied().collect(),
    }
}
