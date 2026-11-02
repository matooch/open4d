//! Command failures (docs/specs/COMMANDS.md §1).

/// Why a command did not run to completion. `Display` is the bare message.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CommandError {
    #[error("unknown command {0}")]
    Unknown(String),
    /// The arguments do not fit the command's schema or are out of range.
    #[error("{0}")]
    InvalidArgs(String),
    /// The command ran and refused; nothing changed.
    #[error("{0}")]
    Failed(String),
}

impl CommandError {
    pub fn failed(e: impl std::fmt::Display) -> CommandError {
        CommandError::Failed(e.to_string())
    }
}

impl From<forge_core::EditError> for CommandError {
    fn from(e: forge_core::EditError) -> CommandError {
        CommandError::Failed(e.to_string())
    }
}
