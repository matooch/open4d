//! forge-app: the headless application core. Spec: docs/specs/APP.md.
//!
//! Loads and saves scenes, runs commands, evaluates any frame and exports, with no window
//! and no GPU. The editor and every automation front end build on it.

pub mod animate;
pub mod query;
pub mod select;

use std::sync::Arc;

use forge_command::{
    CommandContext, CommandError, CommandRegistry, ModeArg, NodeSelection, SelectOp, Timeline,
    register_editor_commands,
};
use forge_core::{Document, EvalCtx, NodeId, Registry, RegistryError, Scene, SceneSettings};
use forge_generators::{Primitives, evaluate};
use forge_geometry::PolyMesh;
use forge_io::export_scene;
use forge_io::obj::{WriteOptions, write};
use forge_modifiers::Deformers;
use serde_json::Value;

use crate::animate::{AnimCtx, Evaluated, delete_keys, key_frames, next_key, prev_key};
use crate::select::{EditMode, run_select};

/// The built-in node types plus every plugin the editor installs.
pub fn default_registry() -> Result<Registry, RegistryError> {
    let mut registry = Registry::with_builtins();
    registry.install(&Primitives)?;
    registry.install(&Deformers)?;
    Ok(registry)
}

/// The built-in editor commands (docs/specs/COMMANDS.md §6).
pub fn default_command_registry() -> CommandRegistry {
    let mut commands = CommandRegistry::new();
    register_editor_commands(&mut commands).expect("built-in command ids are unique");
    commands
}

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum AppError {
    #[error("unknown node {0}")]
    UnknownNode(u64),
    #[error("node {0} has no geometry")]
    NoGeometry(u64),
    #[error("{0}")]
    Build(String),
    #[error("nothing to export")]
    NothingToExport,
    #[error("{0}")]
    Io(String),
    #[error(transparent)]
    Command(#[from] CommandError),
}

pub struct App {
    pub doc: Document,
    /// The command registry every action goes through (docs/specs/COMMANDS.md).
    pub commands: Arc<CommandRegistry>,
    pub selection: NodeSelection,
    /// What viewport clicks select (docs/specs/EDITABLE.md §2.2). Not saved, not undoable.
    pub mode: EditMode,
    /// Current frame and playback; fps and range are scene settings.
    pub timeline: Timeline,
    /// Autokey: edits of animatable parameters write keys (docs/specs/ANIM.md §2.4).
    pub autokey: bool,
    /// The scene at the current frame, cached.
    pub eval: Evaluated,
    /// Last message for the status bar (errors from failed edits).
    pub status: String,
    /// The file the scene was last saved to or loaded from.
    pub file_path: Option<String>,
    /// Scene revision at the last save or load (the dirty marker).
    pub saved_revision: u64,
}

impl App {
    pub fn new(registry: Arc<Registry>) -> App {
        App::with_commands(registry, Arc::new(default_command_registry()))
    }

    /// An empty scene with every plugin installed.
    pub fn new_default() -> App {
        let registry = default_registry().expect("built-in plugins register");
        App::new(Arc::new(registry))
    }

    pub fn with_commands(registry: Arc<Registry>, commands: Arc<CommandRegistry>) -> App {
        let doc = Document::new(registry);
        App {
            saved_revision: doc.scene().revision(),
            doc,
            commands,
            selection: NodeSelection::default(),
            mode: EditMode::default(),
            timeline: Timeline::default(),
            autokey: false,
            eval: Evaluated::default(),
            status: String::new(),
            file_path: None,
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

    /// The scene at the current frame.
    pub fn evaluated(&self) -> Arc<Scene> {
        self.eval.get(self.doc.scene(), self.timeline.current)
    }

    /// Keeps the current frame inside the project range (after undo, load, range edits).
    pub fn sync_timeline(&mut self) {
        let settings = self.settings();
        self.timeline.clamp_to(&settings);
    }

    /// Runs the command `id` with JSON `args` (one undo step if it edits).
    pub fn run(&mut self, id: &str, args: Value) -> Result<Value, CommandError> {
        let commands = self.commands.clone();
        commands.run(self, id, args)
    }

    /// Replaces the scene with the file at `path`.
    pub fn open(&mut self, path: &str) -> Result<Value, CommandError> {
        self.run("file.open", serde_json::json!({ "path": path }))
    }

    /// Writes the scene to `path`.
    pub fn save(&mut self, path: &str) -> Result<Value, CommandError> {
        self.run("file.save_as", serde_json::json!({ "path": path }))
    }

    /// Sets the current frame (clamped to the project range) and returns the scene there.
    pub fn frame(&mut self, n: i64) -> Arc<Scene> {
        let settings = self.settings();
        self.timeline.set_current(n, &settings);
        self.evaluated()
    }

    /// The node's mesh at `frame` (animation and deformers applied), in world space. The
    /// timeline does not move.
    pub fn eval_mesh(&self, node: NodeId, frame: i64) -> Result<PolyMesh, AppError> {
        let scene = self.eval.get(self.doc.scene(), frame);
        if !scene.contains(node) {
            return Err(AppError::UnknownNode(node.0));
        }
        let mut mesh = evaluate(
            &scene,
            node,
            &EvalCtx::at_frame(frame, f64::from(scene.settings().fps)),
        )
        .map_err(|e| AppError::Build(e.to_string()))?
        .ok_or(AppError::NoGeometry(node.0))?;
        let world = scene
            .world_matrix(node)
            .ok_or(AppError::UnknownNode(node.0))?;
        for p in &mut mesh.points {
            *p = world.transform_point3(*p);
        }
        Ok(mesh)
    }

    /// Writes the scene evaluated at the current frame as OBJ: all of it, or the listed
    /// nodes with their descendants. Returns the object count.
    pub fn export_obj(&self, path: &str, roots: Option<&[NodeId]>) -> Result<usize, AppError> {
        let scene = self.evaluated();
        let objects = export_scene(&scene, roots);
        if objects.is_empty() {
            return Err(AppError::NothingToExport);
        }
        std::fs::write(path, write(&objects, &WriteOptions::default()))
            .map_err(|e| AppError::Io(e.to_string()))?;
        Ok(objects.len())
    }

    /// Replaces the document. `full` also resets everything but the command set.
    pub fn reset(&mut self, doc: Document, path: Option<String>, full: bool) {
        if full {
            *self = App::with_commands(doc.scene().registry().clone(), self.commands.clone());
        }
        self.doc = doc;
        self.eval.clear();
        self.timeline = Timeline::default();
        self.sync_timeline();
        self.selection.clear();
        self.mode = EditMode::Model;
        self.status.clear();
        self.file_path = path;
        self.saved_revision = self.doc.scene().revision();
    }

    fn selected_ids(&self) -> Vec<NodeId> {
        self.selection.nodes.iter().copied().collect()
    }
}

impl CommandContext for App {
    fn doc(&self) -> &Document {
        &self.doc
    }

    fn doc_mut(&mut self) -> &mut Document {
        &mut self.doc
    }

    fn selection(&self) -> &NodeSelection {
        &self.selection
    }

    fn selection_mut(&mut self) -> &mut NodeSelection {
        &mut self.selection
    }

    fn edit_mode(&self) -> ModeArg {
        self.mode.into()
    }

    fn set_edit_mode(&mut self, mode: ModeArg) {
        self.mode = mode.into();
    }

    fn timeline(&self) -> &Timeline {
        &self.timeline
    }

    fn timeline_mut(&mut self) -> &mut Timeline {
        &mut self.timeline
    }

    fn autokey(&self) -> bool {
        self.autokey
    }

    fn set_autokey(&mut self, on: bool) {
        self.autokey = on;
    }

    fn evaluated_scene(&self) -> Arc<Scene> {
        self.evaluated()
    }

    fn set_status(&mut self, text: String) {
        self.status = text;
    }

    fn select_components(&mut self, op: SelectOp) -> Result<(), String> {
        run_select(self, op.into())
    }

    fn record_keys(&mut self) -> Result<usize, String> {
        let ids = self.selected_ids();
        let ctx = self.anim_ctx();
        animate::record_keys(&mut self.doc, &ids, &ctx)
    }

    fn delete_keys(&mut self) -> Result<usize, String> {
        let ids = self.selected_ids();
        let ctx = self.anim_ctx();
        delete_keys(&mut self.doc, &ids, &ctx)
    }

    fn key_frame_near(&self, forward: bool) -> Option<i64> {
        let frames = key_frames(self.doc.scene(), &self.selected_ids());
        let current = self.timeline.current;
        if forward {
            next_key(&frames, current)
        } else {
            prev_key(&frames, current)
        }
    }

    fn reset_document(&mut self, doc: Document, path: Option<String>, full: bool) {
        self.reset(doc, path, full);
    }

    fn file_path(&self) -> Option<String> {
        self.file_path.clone()
    }

    fn mark_saved(&mut self, path: String) {
        self.file_path = Some(path);
        self.saved_revision = self.doc.scene().revision();
    }
}
