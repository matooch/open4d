//! Long jobs run in slices on the document's thread (docs/specs/RPC.md §10.3).

use std::collections::VecDeque;
use std::path::PathBuf;

use forge_app::{App, AppError};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::protocol::RpcError;

pub const MAX_ACTIVE_JOBS: usize = 8;
const KEEP_FINISHED: usize = 32;

/// What one slice of a job did.
pub enum Step {
    Progress { fraction: f64, message: String },
    Done(Value),
    Failed(String),
}

/// A job: `step` does a small amount of work and says how far it got.
pub trait LongJob: Send {
    fn step(&mut self, app: &App) -> Step;
}

/// Builds the job of `kind`; checks `args` and anything that can fail up front.
pub fn make_job(app: &App, kind: &str, args: Value) -> Result<Box<dyn LongJob>, RpcError> {
    match kind {
        "export.obj_sequence" => Ok(Box::new(ObjSequence::new(app, args)?)),
        other => Err(RpcError::invalid_params(format!(
            "unknown job kind {other} (known: export.obj_sequence)"
        ))),
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ObjSequenceArgs {
    dir: String,
    start: Option<i64>,
    end: Option<i64>,
    step: Option<i64>,
    prefix: Option<String>,
}

/// `export.obj_sequence`: one OBJ file per frame.
struct ObjSequence {
    dir: PathBuf,
    prefix: String,
    frames: Vec<i64>,
    next: usize,
}

impl ObjSequence {
    fn new(app: &App, args: Value) -> Result<ObjSequence, RpcError> {
        let a: ObjSequenceArgs = serde_json::from_value(args)
            .map_err(|e| RpcError::invalid_params(format!("job.start export.obj_sequence: {e}")))?;
        let settings = app.settings();
        let start = a.start.unwrap_or(settings.start);
        let end = a.end.unwrap_or(settings.end);
        let step = a.step.unwrap_or(1);
        if step < 1 {
            return Err(RpcError::invalid_params("step must be at least 1"));
        }
        if start > end {
            return Err(RpcError::invalid_params("start must not exceed end"));
        }
        let prefix = a.prefix.unwrap_or_else(|| "frame_".to_string());
        if prefix.contains(['/', '\\']) {
            return Err(RpcError::invalid_params(
                "prefix must not contain a path separator",
            ));
        }
        let dir = PathBuf::from(a.dir);
        std::fs::create_dir_all(&dir)
            .map_err(|e| RpcError::failed("io", format!("cannot create {}: {e}", dir.display())))?;
        let mut frames = Vec::new();
        let mut f = start;
        while f <= end {
            frames.push(f);
            f = match f.checked_add(step) {
                Some(n) => n,
                None => break,
            };
        }
        Ok(ObjSequence {
            dir,
            prefix,
            frames,
            next: 0,
        })
    }
}

impl LongJob for ObjSequence {
    fn step(&mut self, app: &App) -> Step {
        let frame = self.frames[self.next];
        let path = self.dir.join(format!("{}{frame:04}.obj", self.prefix));
        let written = app.export_obj_at(&path.to_string_lossy(), None, frame);
        if let Err(e) = written {
            return Step::Failed(match e {
                AppError::NothingToExport => "nothing to export".to_string(),
                other => other.to_string(),
            });
        }
        self.next += 1;
        if self.next == self.frames.len() {
            return Step::Done(json!({"files": self.next, "dir": self.dir.to_string_lossy()}));
        }
        Step::Progress {
            fraction: self.next as f64 / self.frames.len() as f64,
            message: format!("frame {frame} ({} of {})", self.next, self.frames.len()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    Done,
    Cancelled,
    Failed,
}

impl State {
    pub fn name(self) -> &'static str {
        match self {
            State::Done => "done",
            State::Cancelled => "cancelled",
            State::Failed => "failed",
        }
    }
}

pub struct Active {
    pub id: u64,
    pub kind: String,
    /// The connection that started it (gets the notifications).
    pub conn: u64,
    pub job: Box<dyn LongJob>,
    pub cancel: bool,
    pub fraction: f64,
    pub message: String,
}

struct Ended {
    id: u64,
    kind: String,
    state: State,
    fraction: f64,
    message: String,
}

#[derive(Default)]
pub struct Table {
    next: u64,
    pub active: Vec<Active>,
    ended: VecDeque<Ended>,
}

impl Table {
    pub fn start(&mut self, kind: &str, conn: u64, job: Box<dyn LongJob>) -> Result<u64, RpcError> {
        if self.active.len() >= MAX_ACTIVE_JOBS {
            return Err(RpcError::failed(
                "job",
                format!("{MAX_ACTIVE_JOBS} jobs are already running"),
            ));
        }
        self.next += 1;
        self.active.push(Active {
            id: self.next,
            kind: kind.to_string(),
            conn,
            job,
            cancel: false,
            fraction: 0.0,
            message: String::new(),
        });
        Ok(self.next)
    }

    pub fn cancel(&mut self, id: u64) -> Result<Value, RpcError> {
        if let Some(a) = self.active.iter_mut().find(|a| a.id == id) {
            a.cancel = true;
            return Ok(json!({"id": id, "cancelled": true}));
        }
        match self.ended.iter().find(|e| e.id == id) {
            Some(e) => Ok(json!({"id": id, "cancelled": false, "state": e.state.name()})),
            None => Err(RpcError::failed("job", format!("unknown job {id}"))),
        }
    }

    pub fn list(&self) -> Value {
        let mut jobs: Vec<(u64, Value)> = self
            .ended
            .iter()
            .map(|e| {
                (
                    e.id,
                    json!({"id": e.id, "kind": e.kind, "state": e.state.name(),
                        "fraction": e.fraction, "message": e.message}),
                )
            })
            .chain(self.active.iter().map(|a| {
                (
                    a.id,
                    json!({"id": a.id, "kind": a.kind, "state": "running",
                        "fraction": a.fraction, "message": a.message}),
                )
            }))
            .collect();
        jobs.sort_by_key(|(id, _)| *id);
        json!({"jobs": jobs.into_iter().map(|(_, j)| j).collect::<Vec<_>>()})
    }

    /// Moves a finished job out of `active`.
    pub fn end(&mut self, a: Active, state: State) {
        self.ended.push_back(Ended {
            id: a.id,
            kind: a.kind,
            state,
            fraction: a.fraction,
            message: a.message,
        });
        while self.ended.len() > KEEP_FINISHED {
            self.ended.pop_front();
        }
    }
}
