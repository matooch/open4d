//! The engine trait, progress events and cancel (RENDER.md R4).

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::output::RenderOutput;
use crate::settings::RenderSettings;
use crate::snapshot::Snapshot;

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum RenderError {
    #[error("render cancelled")]
    Cancelled,
    #[error("bad render settings: {0}")]
    Settings(String),
    #[error("bad snapshot: {0}")]
    Snapshot(String),
    #[error("unsupported: {0}")]
    Unsupported(String),
    #[error("output: {0}")]
    Output(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum RenderEvent {
    Started {
        width: u32,
        height: u32,
        tiles: usize,
    },
    /// Finished pixels of a tile: `rgba` is `width * height * 4` floats, row-major.
    Tile {
        x: u32,
        y: u32,
        width: u32,
        height: u32,
        rgba: Vec<f32>,
    },
    Progress {
        done: usize,
        total: usize,
    },
    Finished,
}

type Callback = Arc<dyn Fn(&RenderEvent) + Send + Sync>;

/// Cancel flag and event sink of one render. Clones share both.
#[derive(Clone, Default)]
pub struct Control {
    cancel: Arc<AtomicBool>,
    on_event: Option<Callback>,
}

impl Control {
    pub fn new() -> Control {
        Control::default()
    }

    pub fn with_events(f: impl Fn(&RenderEvent) + Send + Sync + 'static) -> Control {
        Control {
            cancel: Arc::new(AtomicBool::new(false)),
            on_event: Some(Arc::new(f)),
        }
    }

    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::Relaxed);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancel.load(Ordering::Relaxed)
    }

    pub fn emit(&self, e: RenderEvent) {
        self.emit_with(|| e);
    }

    /// Construct the event only if there is a consumer for it.
    pub fn emit_with(&self, event: impl FnOnce() -> RenderEvent) {
        if let Some(f) = &self.on_event {
            f(&event());
        }
    }
}

pub trait RenderEngine: Send + Sync {
    fn id(&self) -> &str;
    fn render(
        &self,
        snapshot: &Snapshot,
        settings: &RenderSettings,
        control: &Control,
    ) -> Result<RenderOutput, RenderError>;
}
