//! forge-script: sandboxed Rhai scripting over the command registry.
//! Spec: docs/specs/SCRIPTING.md.
//!
//! A script calls registry commands through the host's [`CommandContext`] (there is no
//! second API), reads the scene through query handles, and runs as one undo step. Limits on
//! operations, time, commands and recursion stop a runaway script with an error.

pub mod api;
mod bind;
mod convert;
mod host;
mod maths;

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};

use forge_command::{CommandContext, CommandRegistry};
use rhai::{Dynamic, Engine, Scope};
use serde_json::Value as Json;

pub use host::BLOCKED_COMMANDS;
use host::Host;

/// Undo label of a script run.
pub const UNDO_LABEL: &str = "Run Script";

/// The sandbox limits (docs/specs/SCRIPTING.md §5).
#[derive(Debug, Clone)]
pub struct ScriptOptions {
    /// Rhai operations before the script is stopped.
    pub max_operations: u64,
    /// Commands one run may execute.
    pub max_commands: usize,
    /// Wall-clock limit, checked every 1024 operations.
    pub timeout: Duration,
    /// Function call depth (recursion).
    pub max_call_levels: usize,
}

impl Default for ScriptOptions {
    fn default() -> Self {
        ScriptOptions {
            max_operations: 5_000_000,
            max_commands: 10_000,
            timeout: Duration::from_secs(10),
            max_call_levels: 48,
        }
    }
}

/// What a successful run produced.
#[derive(Debug, Clone, PartialEq)]
pub struct ScriptReport {
    /// `print`/`debug` lines.
    pub output: Vec<String>,
    /// The script's last expression as JSON (`null` for `()`).
    pub value: Json,
    /// Commands run.
    pub commands: usize,
    /// Rhai operations used.
    pub operations: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureKind {
    Syntax,
    Runtime,
    /// A sandbox limit was hit.
    Limit,
}

/// A failed run: nothing it did to the document remains.
#[derive(Debug, Clone, PartialEq)]
pub struct ScriptFailure {
    pub kind: FailureKind,
    pub message: String,
    pub line: Option<usize>,
    /// What was printed before the failure.
    pub output: Vec<String>,
}

impl std::fmt::Display for ScriptFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.line {
            Some(line) => write!(f, "line {line}: {}", self.message),
            None => f.write_str(&self.message),
        }
    }
}

impl std::error::Error for ScriptFailure {}

const MAX_OUTPUT_LINES: usize = 10_000;

/// Runs `source` on `ctx`, calling commands from `commands`.
///
/// One undo step on success (`Run Script`); on failure every edit is rolled back.
pub fn run_script(
    ctx: &mut dyn CommandContext,
    commands: &Arc<CommandRegistry>,
    source: &str,
    opts: &ScriptOptions,
) -> Result<ScriptReport, ScriptFailure> {
    run_entry(ctx, commands, source, Entry::Script, opts)
}

/// Like [`run_script`], but runs only the function definitions of `source` and calls
/// `function(arg)` (`arg` converted from JSON). Top-level statements of `source` do not
/// run. Used by plugin commands (docs/specs/PLUGINS.md).
pub fn run_function(
    ctx: &mut dyn CommandContext,
    commands: &Arc<CommandRegistry>,
    source: &str,
    function: &str,
    arg: &Json,
    opts: &ScriptOptions,
) -> Result<ScriptReport, ScriptFailure> {
    run_entry(ctx, commands, source, Entry::Call { function, arg }, opts)
}

enum Entry<'a> {
    Script,
    Call { function: &'a str, arg: &'a Json },
}

fn run_entry(
    ctx: &mut dyn CommandContext,
    commands: &Arc<CommandRegistry>,
    source: &str,
    entry: Entry<'_>,
    opts: &ScriptOptions,
) -> Result<ScriptReport, ScriptFailure> {
    let output: Rc<RefCell<Vec<String>>> = Rc::default();
    let operations: Rc<Cell<u64>> = Rc::default();
    let host = Rc::new(Host::new(commands.clone(), opts.max_commands));

    let mut engine = Engine::new();
    configure(&mut engine, opts, &output, &operations);
    bind::register(&mut engine, &host);

    let ast = engine
        .compile(source)
        .map_err(|e| convert::syntax_failure(&e))?;

    let depth = ctx.doc().undo_depth();
    let guard = host.lend(ctx);
    let mut scope = Scope::new();
    let result = match entry {
        Entry::Script => engine.eval_ast_with_scope::<Dynamic>(&mut scope, &ast),
        Entry::Call { function, arg } => engine.call_fn::<Dynamic>(
            &mut scope,
            &ast.clone_functions_only(),
            function,
            (convert::from_json(arg),),
        ),
    };
    drop(guard);

    match result {
        Ok(value) => {
            let value = convert::to_json(&value).unwrap_or(Json::Null);
            ctx.doc_mut().group_undo_since(depth, UNDO_LABEL);
            Ok(ScriptReport {
                output: output.take(),
                value,
                commands: host.commands_run(),
                operations: operations.get(),
            })
        }
        Err(e) => {
            ctx.doc_mut().rollback_to(depth);
            ctx.retain_selection();
            ctx.sync_timeline();
            let mut failure = convert::failure(&e, opts);
            failure.output = output.take();
            Err(failure)
        }
    }
}

/// Calls `function(arg)` from the function definitions of `source` in a sandbox that has
/// no scene and no commands: only the maths types and seeded random numbers. The result is
/// deterministic for equal inputs. Used by plugin generators (docs/specs/PLUGINS.md).
pub fn eval_function(
    source: &str,
    function: &str,
    arg: &Json,
    opts: &ScriptOptions,
) -> Result<ScriptReport, ScriptFailure> {
    let output: Rc<RefCell<Vec<String>>> = Rc::default();
    let operations: Rc<Cell<u64>> = Rc::default();
    let mut engine = Engine::new();
    configure(&mut engine, opts, &output, &operations);
    maths::register(&mut engine);
    let ast = engine
        .compile(source)
        .map_err(|e| convert::syntax_failure(&e))?;
    let mut scope = Scope::new();
    let result = engine.call_fn::<Dynamic>(
        &mut scope,
        &ast.clone_functions_only(),
        function,
        (convert::from_json(arg),),
    );
    match result {
        Ok(value) => Ok(ScriptReport {
            output: output.take(),
            value: convert::to_json(&value).unwrap_or(Json::Null),
            commands: 0,
            operations: operations.get(),
        }),
        Err(e) => {
            let mut failure = convert::failure(&e, opts);
            failure.output = output.take();
            Err(failure)
        }
    }
}

/// The functions `source` defines as `(name, parameter count)`; a syntax error fails.
pub fn script_functions(source: &str) -> Result<Vec<(String, usize)>, ScriptFailure> {
    let mut engine = Engine::new();
    engine.set_max_expr_depths(64, 32);
    let ast = engine
        .compile(source)
        .map_err(|e| convert::syntax_failure(&e))?;
    Ok(ast
        .iter_functions()
        .map(|f| (f.name.to_string(), f.params.len()))
        .collect())
}

fn configure(
    engine: &mut Engine,
    opts: &ScriptOptions,
    output: &Rc<RefCell<Vec<String>>>,
    operations: &Rc<Cell<u64>>,
) {
    engine.set_max_operations(opts.max_operations);
    engine.set_max_call_levels(opts.max_call_levels);
    engine.set_max_expr_depths(64, 32);
    engine.set_max_string_size(1 << 20);
    engine.set_max_array_size(100_000);
    engine.set_max_map_size(100_000);
    engine.set_strict_variables(true);
    engine.disable_symbol("eval");

    let started = Instant::now();
    let timeout = opts.timeout;
    let ops = operations.clone();
    engine.on_progress(move |n| {
        ops.set(n);
        if n % 1024 == 0 && started.elapsed() > timeout {
            return Some(convert::TIMEOUT.into());
        }
        None
    });

    let lines = output.clone();
    engine.on_print(move |s| push_line(&lines, s));
    let lines = output.clone();
    engine.on_debug(move |s, _, _| push_line(&lines, s));
}

fn push_line(lines: &Rc<RefCell<Vec<String>>>, text: &str) {
    let mut lines = lines.borrow_mut();
    if lines.len() < MAX_OUTPUT_LINES {
        lines.push(text.to_string());
    }
}

/// The command registry the default application uses, for hosts that have none.
pub fn default_commands() -> Arc<CommandRegistry> {
    Arc::new(forge_app::default_command_registry())
}
