//! The random-command soak (docs/specs/PLATFORM.md §9.5): an in-process `App` driven through
//! the RPC `dispatch` with random schema-valid commands, checking undo/redo consistency and
//! save/load round trips at checkpoints.

use std::time::{Duration, Instant};

use forge_app::App;
use forge_core::file;
use forge_rpc::dispatch;
use serde_json::{Value, json};

use crate::rng::XorShift64;
use crate::schema_gen::{GenCtx, corrupt, generate};
use crate::targets::FS_COMMANDS;

/// How long and how hard to soak.
#[derive(Debug, Clone)]
pub struct SoakConfig {
    pub seed: u64,
    /// Stop after this many steps (`None`: no step limit).
    pub max_steps: Option<u64>,
    /// Stop after this much time (`None`: no time limit).
    pub max_time: Option<Duration>,
    /// Steps between invariant checkpoints.
    pub checkpoint_every: u64,
}

/// What a run did.
#[derive(Debug, Clone, Default)]
pub struct SoakStats {
    pub steps: u64,
    pub ok: u64,
    pub failed: u64,
    pub checkpoints: u64,
    pub commands_used: usize,
}

/// Scene size above which a step deletes everything.
const MAX_SOAK_NODES: usize = 80;

struct Command {
    id: String,
    undoable: bool,
    schema: Value,
}

fn bytes_of(app: &App) -> Vec<u8> {
    file::save_with(app.doc.scene(), &file::SaveOptions::default())
}

fn node_ids(app: &App) -> Vec<u64> {
    let scene = app.doc.scene();
    let mut out = Vec::new();
    let mut stack: Vec<_> = scene.roots().to_vec();
    while let Some(id) = stack.pop() {
        out.push(id.0);
        if let Some(n) = scene.node(id) {
            stack.extend(n.children.iter().copied());
        }
    }
    out.sort_unstable();
    out
}

fn list_commands(app: &mut App) -> Vec<Command> {
    let listed = dispatch(app, "command.list", json!({})).expect("command.list");
    let mut out: Vec<Command> = listed["commands"]
        .as_array()
        .expect("commands array")
        .iter()
        .map(|c| Command {
            id: c["id"].as_str().unwrap_or_default().to_string(),
            undoable: c["undoable"].as_bool().unwrap_or(true),
            schema: c["args_schema"].clone(),
        })
        .filter(|c| !FS_COMMANDS.contains(&c.id.as_str()))
        .collect();
    out.sort_by(|a, b| a.id.cmp(&b.id));
    out
}

fn undo_all(app: &mut App) -> usize {
    let mut n = 0;
    while app.doc.undo() {
        n += 1;
    }
    n
}

fn redo_n(app: &mut App, n: usize) -> usize {
    (0..n).take_while(|_| app.doc.redo()).count()
}

/// Runs the soak; panics with the seed, step and command on any violated invariant.
pub fn run(config: &SoakConfig) -> SoakStats {
    let mut app = App::new_default();
    let commands = list_commands(&mut app);
    assert!(!commands.is_empty(), "no commands to soak");
    let mut rng = XorShift64::new(config.seed);
    let start_bytes = bytes_of(&app);
    let started = Instant::now();
    let mut stats = SoakStats {
        commands_used: commands.len(),
        ..SoakStats::default()
    };
    // The text of the last few steps, printed when something panics.
    let mut recent: std::collections::VecDeque<String> = Default::default();

    loop {
        if config.max_steps.is_some_and(|m| stats.steps >= m)
            || config.max_time.is_some_and(|t| started.elapsed() >= t)
        {
            break;
        }
        stats.steps += 1;
        let nodes = node_ids(&app);
        // Keep the scene small: duplicating N nodes 60 times is legitimate but multiplies the
        // work of every later step, so past a size limit the step deletes the roots instead.
        let prune = nodes.len() > MAX_SOAK_NODES;
        let cmd = if prune {
            commands
                .iter()
                .find(|c| c.id == "object.delete")
                .expect("object.delete exists")
        } else {
            &commands[rng.below(commands.len())]
        };
        let mut args = if prune {
            let roots: Vec<u64> = app.doc.scene().roots().iter().map(|r| r.0).collect();
            json!({ "nodes": roots })
        } else {
            let mut cx = GenCtx {
                rng: &mut rng,
                nodes: &nodes,
            };
            generate(&mut cx, &cmd.schema, &cmd.schema, 0)
        };
        if !prune && rng.chance(8) {
            corrupt(&mut rng, &mut args);
        }
        let line = format!("{} {}", cmd.id, args);
        if recent.len() == 8 {
            recent.pop_front();
        }
        recent.push_back(line.clone());

        // History commands are not themselves undo steps but do change the scene.
        let history_cmd = cmd.id.starts_with("edit.undo")
            || cmd.id.starts_with("edit.redo")
            || cmd.id.starts_with("edit.history");
        let t0 = Instant::now();
        if std::env::var_os("FORGE_SOAK_TRACE").is_some() {
            eprintln!(
                "start {} {}",
                stats.steps,
                line.chars().take(300).collect::<String>()
            );
        }
        let before = (!cmd.undoable && !history_cmd).then(|| bytes_of(&app));
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            dispatch(&mut app, "command.run", json!({"id": cmd.id, "args": args}))
        }));
        let result = match outcome {
            Ok(r) => r,
            Err(p) => {
                let msg = p
                    .downcast_ref::<String>()
                    .cloned()
                    .or_else(|| p.downcast_ref::<&str>().map(|s| s.to_string()))
                    .unwrap_or_default();
                panic!(
                    "soak: panic at step {} (seed {}) in `{line}`: {msg}\nrecent steps:\n{}",
                    stats.steps,
                    config.seed,
                    recent.iter().cloned().collect::<Vec<_>>().join("\n")
                );
            }
        };
        if std::env::var_os("FORGE_SOAK_TRACE").is_some() {
            let shown: String = line.chars().take(200).collect();
            eprintln!(
                "{:>5} {:>6}ms {shown}",
                stats.steps,
                t0.elapsed().as_millis()
            );
        }
        if result.is_ok() {
            stats.ok += 1;
        } else {
            stats.failed += 1;
        }
        if let Some(before) = before
            && result.is_ok()
        {
            assert!(
                before == bytes_of(&app),
                "soak: non-undoable command changed the scene: step {} (seed {}) `{line}`",
                stats.steps,
                config.seed
            );
        }

        if stats.steps.is_multiple_of(config.checkpoint_every) {
            stats.checkpoints += 1;
            checkpoint(&mut app, &start_bytes, config.seed, stats.steps);
        }
    }
    checkpoint(&mut app, &start_bytes, config.seed, stats.steps);
    stats.checkpoints += 1;
    stats
}

/// Save/load round trip, then undo everything (must give the start bytes) and redo everything
/// (must give the bytes from before).
fn checkpoint(app: &mut App, start_bytes: &[u8], seed: u64, step: u64) {
    let here = bytes_of(app);
    let (loaded, _) =
        file::load_bytes(app.doc.scene().registry().clone(), &here).unwrap_or_else(|e| {
            panic!("soak: saved scene does not load (seed {seed}, step {step}): {e}")
        });
    let again = file::save_with(&loaded, &file::SaveOptions::default());
    assert!(
        again == here,
        "soak: save/load round trip changed the bytes (seed {seed}, step {step})"
    );

    let undone = undo_all(app);
    assert!(
        bytes_of(app) == start_bytes,
        "soak: undoing all {undone} steps did not restore the start scene (seed {seed}, step {step})"
    );
    // Steps that were already undone before the checkpoint stay on the redo stack, so redo
    // exactly the ones undone here.
    let redone = redo_n(app, undone);
    assert!(
        undone == redone,
        "soak: redo count differs: {undone} undone, {redone} redone (seed {seed}, step {step})"
    );
    assert!(
        bytes_of(app) == here,
        "soak: redoing all steps did not restore the scene (seed {seed}, step {step})"
    );
}
