//! Command registry: every user action as a named, typed, undoable command
//! (docs/specs/COMMANDS.md).

mod builtin;
mod context;
mod error;
mod registry;
pub mod schema;
mod state;
mod timeline;

pub use builtin::{display_to_stored, param_value_from_json, register_editor_commands};
pub use context::CommandContext;
pub use error::CommandError;
pub use registry::{
    CommandHandler, CommandPlugin, CommandRegistry, CommandSpec, NoArgs, valid_command_id,
};
pub use state::{
    CameraSet, ConvertMode, GizmoModeArg, GizmoSpaceArg, LayoutArg, LightingArg, ModeArg,
    NodeSelection, PaneArg, ProjectionArg, SelectExtra, SelectOp, ShadingMode, UiRequest, ViewArg,
};
pub use timeline::Timeline;
