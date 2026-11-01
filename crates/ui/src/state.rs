//! Editor state and node selection (docs/specs/UI.md §1).

use std::collections::BTreeSet;
use std::sync::Arc;

use forge_command::CommandRegistry;
use forge_core::{Document, NodeId, Registry, Scene, SceneSettings};
use forge_viewport::camera::{Camera, Handedness};
use forge_viewport::render::Shading;

use crate::animate::{AnimCtx, Evaluated};
use crate::components::EditMode;
use crate::dock::Dock;
use crate::timeline::Timeline;

pub use forge_command::NodeSelection;

pub struct EditorState {
    pub doc: Document,
    pub selection: NodeSelection,
    pub expanded: BTreeSet<NodeId>,
    pub dock: Dock,
    pub camera: Camera,
    pub shading: Shading,
    /// Current frame and playback; fps and range are scene settings.
    pub timeline: Timeline,
    /// Autokey: edits of animatable parameters write keys (docs/specs/ANIM.md §2.4).
    pub autokey: bool,
    /// The scene at the current frame, cached.
    pub eval: Evaluated,
    /// Last message for the status bar (errors from failed edits).
    pub status: String,
    /// What viewport clicks select (docs/specs/EDITABLE.md §2.2). Not saved, not undoable.
    pub mode: EditMode,
    /// The command registry every action goes through (docs/specs/COMMANDS.md).
    pub commands: Arc<CommandRegistry>,
    /// The file the scene was last saved to or loaded from.
    pub file_path: Option<String>,
    /// Scene revision at the last save or load (the dirty marker).
    pub saved_revision: u64,
}

impl EditorState {
    /// Left-handed view matching the reference default directions; the exact default
    /// pose is not shown by the reference app (docs/specs/SMOOTHING.md §4).
    pub fn default_camera() -> Camera {
        Camera {
            yaw: 0.68,
            pitch: 0.52,
            distance: 1000.0,
            handedness: Handedness::Left,
            ..Camera::default()
        }
    }

    /// fps and range of the document.
    pub fn settings(&self) -> SceneSettings {
        self.doc.scene().settings()
    }

    /// Time context for animation edits.
    pub fn anim_ctx(&self) -> AnimCtx {
        AnimCtx {
            frame: self.timeline.current,
            fps: self.settings().fps,
            autokey: self.autokey,
        }
    }

    /// The scene at the current frame: what the viewport and the panels show.
    pub fn evaluated(&self) -> Arc<Scene> {
        self.eval.get(self.doc.scene(), self.timeline.current)
    }

    /// Keeps the current frame inside the project range (after undo, load, range edits).
    pub fn sync_timeline(&mut self) {
        let settings = self.settings();
        self.timeline.clamp_to(&settings);
    }

    pub fn new(registry: Arc<Registry>) -> EditorState {
        let camera = Self::default_camera();
        let doc = Document::new(registry);
        let mut commands = CommandRegistry::new();
        forge_command::register_editor_commands(&mut commands)
            .expect("built-in command ids are unique");
        EditorState {
            saved_revision: doc.scene().revision(),
            doc,
            selection: NodeSelection::default(),
            expanded: BTreeSet::new(),
            dock: Dock::default(),
            camera,
            shading: Shading::SmoothWire,
            timeline: Timeline::default(),
            autokey: false,
            eval: Evaluated::default(),
            status: String::new(),
            mode: EditMode::default(),
            commands: Arc::new(commands),
            file_path: None,
        }
    }
}
