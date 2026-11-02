//! The handler trait and the registry (docs/specs/COMMANDS.md §1, §2).

use schemars::JsonSchema;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};

use crate::context::CommandContext;
use crate::error::CommandError;

pub trait CommandHandler: Send + Sync {
    /// Dotted lowercase id, e.g. `object.create`.
    fn id(&self) -> &str;
    /// Menu text.
    fn label(&self) -> &str;
    /// JSON Schema of the arguments object.
    fn args_schema(&self) -> Value;
    fn run(&self, ctx: &mut dyn CommandContext, args: Value) -> Result<Value, CommandError>;
    /// True when a run may edit the document (one undo step).
    fn undoable(&self) -> bool;
    /// A valid example argument object.
    fn sample_args(&self) -> Value {
        json!({})
    }
}

/// Plugins add commands the same way they add node types.
pub trait CommandPlugin {
    fn register_commands(&self, _registry: &mut CommandRegistry) -> Result<(), CommandError> {
        Ok(())
    }
}

/// `domain.verb`: lowercase ASCII segments of `a-z 0-9 _`, at least two.
pub fn valid_command_id(id: &str) -> bool {
    let mut segments = 0;
    for seg in id.split('.') {
        segments += 1;
        if seg.is_empty()
            || !seg
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
        {
            return false;
        }
    }
    segments >= 2
}

#[derive(Default)]
pub struct CommandRegistry {
    handlers: Vec<Box<dyn CommandHandler>>,
}

impl CommandRegistry {
    pub fn new() -> CommandRegistry {
        CommandRegistry::default()
    }

    pub fn register(&mut self, handler: Box<dyn CommandHandler>) -> Result<(), CommandError> {
        let id = handler.id().to_string();
        if !valid_command_id(&id) {
            return Err(CommandError::InvalidArgs(format!(
                "malformed command id {id:?}"
            )));
        }
        if self.get(&id).is_some() {
            return Err(CommandError::InvalidArgs(format!(
                "duplicate command id {id}"
            )));
        }
        self.handlers.push(handler);
        Ok(())
    }

    /// Registers a command whose arguments are the serde struct `A`.
    pub fn register_fn<A>(
        &mut self,
        spec: CommandSpec,
        run: fn(&mut dyn CommandContext, A) -> Result<Value, CommandError>,
    ) -> Result<(), CommandError>
    where
        A: DeserializeOwned + JsonSchema + 'static,
    {
        self.register(Box::new(FnCommand { spec, run }))
    }

    pub fn get(&self, id: &str) -> Option<&dyn CommandHandler> {
        self.handlers
            .iter()
            .find(|h| h.id() == id)
            .map(|h| h.as_ref())
    }

    /// Handlers in registration order.
    pub fn iter(&self) -> impl Iterator<Item = &dyn CommandHandler> {
        self.handlers.iter().map(|h| h.as_ref())
    }

    pub fn len(&self) -> usize {
        self.handlers.len()
    }

    pub fn is_empty(&self) -> bool {
        self.handlers.is_empty()
    }

    pub fn run(
        &self,
        ctx: &mut dyn CommandContext,
        id: &str,
        args: Value,
    ) -> Result<Value, CommandError> {
        crate::history::record(id, &args);
        let handler = self
            .get(id)
            .ok_or_else(|| CommandError::Unknown(id.to_string()))?;
        tracing::debug!(command = id, "run");
        let result = handler.run(ctx, args);
        if let Err(e) = &result {
            tracing::warn!(command = id, error = %e, "command failed");
        }
        result
    }
}

/// Static description of a typed command.
pub struct CommandSpec {
    pub id: &'static str,
    pub label: &'static str,
    pub undoable: bool,
    /// A valid argument object for tests and documentation.
    pub sample: fn() -> Value,
}

struct FnCommand<A> {
    spec: CommandSpec,
    run: fn(&mut dyn CommandContext, A) -> Result<Value, CommandError>,
}

impl<A: DeserializeOwned + JsonSchema> CommandHandler for FnCommand<A> {
    fn id(&self) -> &str {
        self.spec.id
    }

    fn label(&self) -> &str {
        self.spec.label
    }

    fn args_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(A)).unwrap_or(Value::Null)
    }

    fn run(&self, ctx: &mut dyn CommandContext, args: Value) -> Result<Value, CommandError> {
        // Absent arguments mean "no arguments".
        let args = if args.is_null() { json!({}) } else { args };
        let typed: A = serde_json::from_value(args)
            .map_err(|e| CommandError::InvalidArgs(format!("{}: {e}", self.spec.id)))?;
        (self.run)(ctx, typed)
    }

    fn undoable(&self) -> bool {
        self.spec.undoable
    }

    fn sample_args(&self) -> Value {
        (self.spec.sample)()
    }
}

/// Arguments of a command that takes none.
#[derive(Debug, Clone, Copy, Default, serde::Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct NoArgs {}
