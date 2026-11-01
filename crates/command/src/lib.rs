//! Command registry: every user action as a named, typed, undoable command
//! (docs/specs/COMMANDS.md).

mod builtin;
mod context;
mod error;
mod registry;
pub mod schema;
mod state;
mod timeline;

pub use builtin::register_editor_commands;
pub use context::CommandContext;
pub use error::CommandError;
pub use registry::{
    CommandHandler, CommandPlugin, CommandRegistry, CommandSpec, NoArgs, valid_command_id,
};
pub use state::{ModeArg, NodeSelection, SelectOp, ShadingMode};
pub use timeline::Timeline;
