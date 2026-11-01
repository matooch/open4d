//! Editor actions are registry commands (docs/specs/COMMANDS.md, UI.md §5). This module runs
//! them against the editor state and implements the command context for it.

use forge_command::{CommandContext, ModeArg, NodeSelection, SelectOp, ShadingMode, Timeline};
use forge_core::{Category, Document, NodeId, Registry};
use forge_viewport::render::Shading;
use serde_json::Value;

use crate::animate::{delete_keys, key_frames, next_key, prev_key, record_keys};
use crate::components::{EditMode, SelectCmd, run_select};
use crate::state::EditorState;

/// Runs the command `id` with JSON `args` through the state's registry. The error is the
/// message for the status bar.
pub fn execute(s: &mut EditorState, id: &str, args: Value) -> Result<Value, String> {
    let registry = s.commands.clone();
    registry.run(s, id, args).map_err(|e| e.to_string())
}

impl From<ModeArg> for EditMode {
    fn from(m: ModeArg) -> EditMode {
        match m {
            ModeArg::Model => EditMode::Model,
            ModeArg::Points => EditMode::Points,
            ModeArg::Edges => EditMode::Edges,
            ModeArg::Polys => EditMode::Polys,
        }
    }
}

impl From<EditMode> for ModeArg {
    fn from(m: EditMode) -> ModeArg {
        match m {
            EditMode::Model => ModeArg::Model,
            EditMode::Points => ModeArg::Points,
            EditMode::Edges => ModeArg::Edges,
            EditMode::Polys => ModeArg::Polys,
        }
    }
}

impl From<SelectOp> for SelectCmd {
    fn from(o: SelectOp) -> SelectCmd {
        match o {
            SelectOp::All => SelectCmd::All,
            SelectOp::None => SelectCmd::None,
            SelectOp::Invert => SelectCmd::Invert,
            SelectOp::Grow => SelectCmd::Grow,
            SelectOp::Shrink => SelectCmd::Shrink,
        }
    }
}

/// The viewport shading for a command argument.
pub fn shading_of(m: ShadingMode) -> Shading {
    match m {
        ShadingMode::Smooth => Shading::Smooth,
        ShadingMode::Flat => Shading::Flat,
        ShadingMode::Wire => Shading::Wire,
        ShadingMode::SmoothWire => Shading::SmoothWire,
        ShadingMode::FlatWire => Shading::FlatWire,
    }
}

/// The command argument for a viewport shading.
pub fn shading_mode_of(m: Shading) -> ShadingMode {
    match m {
        Shading::Smooth => ShadingMode::Smooth,
        Shading::Flat => ShadingMode::Flat,
        Shading::Wire => ShadingMode::Wire,
        Shading::SmoothWire => ShadingMode::SmoothWire,
        Shading::FlatWire => ShadingMode::FlatWire,
    }
}

impl EditorState {
    fn selected_ids(&self) -> Vec<NodeId> {
        self.selection.nodes.iter().copied().collect()
    }
}

impl CommandContext for EditorState {
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

    fn set_status(&mut self, text: String) {
        self.status = text;
    }

    fn select_components(&mut self, op: SelectOp) -> Result<(), String> {
        run_select(self, op.into())
    }

    fn record_keys(&mut self) -> Result<usize, String> {
        let ids = self.selected_ids();
        let ctx = self.anim_ctx();
        record_keys(&mut self.doc, &ids, &ctx)
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
        if full {
            // File > New: everything but the dock layout and the command set starts over.
            let dock = self.dock.clone();
            let commands = self.commands.clone();
            *self = EditorState::new(doc.scene().registry().clone());
            self.dock = dock;
            self.commands = commands;
        }
        self.doc = doc;
        self.eval.clear();
        self.timeline = Timeline::default();
        self.sync_timeline();
        self.selection.clear();
        self.mode = EditMode::Model;
        self.expanded.clear();
        self.status.clear();
        self.file_path = path;
        self.saved_revision = self.doc.scene().revision();
    }

    fn file_path(&self) -> Option<String> {
        self.file_path.clone()
    }

    fn mark_saved(&mut self, path: String) {
        self.file_path = Some(path);
        self.saved_revision = self.doc.scene().revision();
    }

    fn set_shading(&mut self, shading: ShadingMode) {
        self.shading = shading_of(shading);
    }

    fn frame_all(&mut self) {
        // No geometry yet (Phase 5): reset to the default view.
        self.camera = EditorState::default_camera();
    }

    fn toggle_expanded(&mut self, node: NodeId) {
        if !self.expanded.remove(&node) {
            self.expanded.insert(node);
        }
    }
}

/// Placeable categories in `Category` order, each with `(type id, label)` sorted by label.
pub fn create_menu(registry: &Registry) -> Vec<(Category, Vec<(String, String)>)> {
    let mut out: Vec<(Category, Vec<(String, String)>)> = Vec::new();
    for t in registry.types() {
        if !(t.category.is_hierarchy() || t.category == Category::Tag) || !t.in_create_menu {
            continue;
        }
        let item = (t.id.clone(), t.label.clone());
        match out.iter_mut().find(|(c, _)| *c == t.category) {
            Some((_, items)) => items.push(item),
            None => out.push((t.category, vec![item])),
        }
    }
    out.sort_by_key(|(c, _)| *c);
    for (_, items) in &mut out {
        items.sort_by(|a, b| a.1.cmp(&b.1));
    }
    out
}

pub fn category_label(c: Category) -> &'static str {
    match c {
        Category::Object => "Object",
        Category::Generator => "Generator",
        Category::Deformer => "Deformer",
        Category::Effector => "Effector",
        Category::Field => "Field",
        Category::Tag => "Tag",
        Category::Material => "Material",
        Category::Shader => "Shader",
        Category::RenderEngine => "Render engine",
    }
}
