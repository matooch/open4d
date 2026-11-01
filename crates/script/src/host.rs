//! The bridge between Rhai callbacks (`'static` closures) and the host's borrowed context.

use std::cell::Cell;
use std::ptr::NonNull;
use std::sync::Arc;

use forge_command::{CommandContext, CommandRegistry};
use rhai::{Dynamic, EvalAltResult, Position};
use serde_json::Value as Json;

use crate::convert;

/// Commands a script may not run (docs/specs/SCRIPTING.md §3): they would break the single
/// undo step or reach the file system without an explicit export path.
pub const BLOCKED_COMMANDS: &[&str] = &[
    "edit.undo",
    "edit.redo",
    "file.new",
    "file.open",
    "file.save",
    "file.save_as",
    "file.import_obj",
];

fn blocked(id: &str) -> bool {
    BLOCKED_COMMANDS.contains(&id)
}

pub(crate) struct Host {
    registry: Arc<CommandRegistry>,
    ctx: Cell<Option<NonNull<dyn CommandContext>>>,
    commands: Cell<usize>,
    max_commands: usize,
}

/// Clears the lent context when the run ends (also on unwind).
pub(crate) struct Lend<'a>(&'a Host);

impl Drop for Lend<'_> {
    fn drop(&mut self) {
        self.0.ctx.set(None);
    }
}

type RhaiResult<T> = Result<T, Box<EvalAltResult>>;

pub(crate) fn error(message: impl Into<String>) -> Box<EvalAltResult> {
    EvalAltResult::ErrorRuntime(Dynamic::from(message.into()), Position::NONE).into()
}

impl Host {
    pub(crate) fn new(registry: Arc<CommandRegistry>, max_commands: usize) -> Host {
        Host {
            registry,
            ctx: Cell::new(None),
            commands: Cell::new(0),
            max_commands,
        }
    }

    /// Makes `ctx` reachable from script callbacks until the guard drops.
    pub(crate) fn lend<'a>(&'a self, ctx: &mut dyn CommandContext) -> Lend<'a> {
        let raw: *mut (dyn CommandContext + '_) = ctx;
        // SAFETY: only the trait-object lifetime bound is erased. The pointer is stored in
        // `self.ctx`, read only by `with` (called from callbacks that run inside
        // `eval_ast_with_scope`, while the caller's `&mut` borrow of `ctx` is held by
        // `run_script`), and cleared by the returned guard before that borrow ends. The
        // engine and this host never leave `run_script`.
        let raw: *mut (dyn CommandContext + 'static) = unsafe { std::mem::transmute(raw) };
        self.ctx.set(NonNull::new(raw));
        Lend(self)
    }

    /// Runs `f` on the lent context.
    pub(crate) fn with<R>(&self, f: impl FnOnce(&mut dyn CommandContext) -> R) -> R {
        let mut ptr = self.ctx.get().expect("context is lent while a script runs");
        // SAFETY: see `lend`. Callbacks run one at a time on the thread of `run_script` and
        // none keeps the reference, so no other reference to the context is live.
        f(unsafe { ptr.as_mut() })
    }

    pub(crate) fn commands_run(&self) -> usize {
        self.commands.get()
    }

    /// Runs one command through the registry; counts against the command limit.
    pub(crate) fn command(&self, id: &str, args: Json) -> RhaiResult<Json> {
        if blocked(id) {
            return Err(error(format!("command {id} is not available in scripts")));
        }
        let n = self.commands.get() + 1;
        if n > self.max_commands {
            return Err(EvalAltResult::ErrorTerminated(
                convert::COMMAND_LIMIT.into(),
                Position::NONE,
            )
            .into());
        }
        self.commands.set(n);
        self.with(|ctx| self.registry.run(ctx, id, args))
            .map_err(|e| error(format!("{id}: {e}")))
    }

    pub(crate) fn command_ids(&self) -> Vec<String> {
        self.registry.iter().map(|c| c.id().to_string()).collect()
    }
}
