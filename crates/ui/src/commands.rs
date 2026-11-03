//! Editor actions are registry commands (docs/specs/COMMANDS.md, UI.md §5). This module runs
//! them against the editor state and implements the command context for it.

use forge_command::{
    CameraSet, CommandContext, GizmoModeArg, GizmoSpaceArg, LayoutArg, LightingArg, ModeArg,
    NodeSelection, PaneArg, SelectExtra, SelectOp, ShadingMode, Timeline, UiRequest, ViewArg,
};
use forge_core::{Category, Document, NodeId, Registry};
use forge_viewport::gizmo::{GizmoMode, Space as GizmoSpace};
use forge_viewport::render::Shading;
use serde_json::Value;

use crate::panes::{layout_of, pane_of};
use crate::state::EditorState;

/// Runs the command `id` with JSON `args` through the state's registry. The error is the
/// message for the status bar.
pub fn execute(s: &mut EditorState, id: &str, args: Value) -> Result<Value, String> {
    let registry = s.app.commands.clone();
    registry.run(s, id, args).map_err(|e| e.to_string())
}

/// RPC requests run with the editor's own context, so view commands act on the window
/// (docs/specs/RPC.md §5).
impl forge_rpc::Host for EditorState {
    fn app(&self) -> &forge_app::App {
        &self.app
    }

    fn run_command(&mut self, id: &str, args: Value) -> Result<Value, forge_command::CommandError> {
        let registry = self.app.commands.clone();
        let result = registry.run(self, id, args);
        // Dialogs and inline rename belong to the window, not to a remote caller.
        if !self.ui_requests.is_empty() {
            self.ui_requests.clear();
            return Err(forge_command::CommandError::Failed(
                "needs the editor window".into(),
            ));
        }
        result
    }

    fn screenshot(&mut self, width: u32, height: u32) -> Result<Vec<u8>, String> {
        crate::screenshot::render_png(self, width, height)
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
        ShadingMode::Quick => Shading::Quick,
        ShadingMode::Constant => Shading::Constant,
        ShadingMode::HiddenLine => Shading::HiddenLine,
        ShadingMode::BoundingBox => Shading::BoundingBox,
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
        Shading::Quick => ShadingMode::Quick,
        Shading::Constant => ShadingMode::Constant,
        Shading::HiddenLine => ShadingMode::HiddenLine,
        Shading::BoundingBox => ShadingMode::BoundingBox,
    }
}

impl CommandContext for EditorState {
    fn doc(&self) -> &Document {
        self.app.doc()
    }

    fn doc_mut(&mut self) -> &mut Document {
        self.app.doc_mut()
    }

    fn selection(&self) -> &NodeSelection {
        self.app.selection()
    }

    fn selection_mut(&mut self) -> &mut NodeSelection {
        self.app.selection_mut()
    }

    fn edit_mode(&self) -> ModeArg {
        self.app.edit_mode()
    }

    fn set_edit_mode(&mut self, mode: ModeArg) {
        self.app.set_edit_mode(mode);
    }

    fn component_settings(&self) -> &forge_modeling::ComponentSettings {
        self.app.component_settings()
    }

    fn component_settings_mut(&mut self) -> &mut forge_modeling::ComponentSettings {
        self.app.component_settings_mut()
    }

    fn transform_settings(&self) -> &forge_modeling::snap::TransformSettings {
        self.app.transform_settings()
    }

    fn transform_settings_mut(&mut self) -> &mut forge_modeling::snap::TransformSettings {
        self.app.transform_settings_mut()
    }

    fn timeline(&self) -> &Timeline {
        self.app.timeline()
    }

    fn timeline_mut(&mut self) -> &mut Timeline {
        self.app.timeline_mut()
    }

    fn autokey(&self) -> bool {
        self.app.autokey()
    }

    fn set_autokey(&mut self, on: bool) {
        self.app.set_autokey(on);
    }

    fn set_status(&mut self, text: String) {
        self.app.set_status(text);
    }

    fn select_components(&mut self, op: SelectOp) -> Result<(), String> {
        self.app.select_components(op)
    }

    fn select_extra(&mut self, cmd: SelectExtra) -> Result<(), String> {
        self.app.select_extra(cmd)
    }

    fn record_keys(&mut self) -> Result<usize, String> {
        self.app.record_keys()
    }

    fn delete_keys(&mut self) -> Result<usize, String> {
        self.app.delete_keys()
    }

    fn key_frame_near(&self, forward: bool) -> Option<i64> {
        self.app.key_frame_near(forward)
    }

    fn autosave_now(&mut self, wait: bool) -> Result<String, String> {
        let path = self.app.file_path.clone();
        let out = self
            .autosave
            .queue_now(self.app.doc.scene(), path.as_deref())?;
        if wait && let Some(e) = self.autosave.flush().into_iter().next() {
            return Err(e);
        }
        Ok(out)
    }

    fn reset_document(&mut self, doc: Document, path: Option<String>, full: bool) {
        self.autosave
            .document_replaced(self.app.file_path.as_deref());
        self.app.reset(doc, path, full);
        self.expanded.clear();
        self.tool = None;
        if full {
            // File > New: everything but the dock layout starts over.
            self.reset_views();
        }
    }

    fn file_path(&self) -> Option<String> {
        self.app.file_path()
    }

    fn evaluated_scene(&self) -> std::sync::Arc<forge_core::Scene> {
        self.app.evaluated()
    }

    fn mark_saved(&mut self, path: String) {
        self.autosave
            .document_saved(self.app.file_path.as_deref(), &path);
        self.app.mark_saved(path);
    }

    fn set_shading(&mut self, shading: ShadingMode, pane: Option<PaneArg>) {
        let shading = shading_of(shading);
        self.with_pane(pane.map(pane_of), |s| s.shading = shading);
    }

    fn frame_all(&mut self) {
        self.frame_all_geometry();
    }

    fn frame_selected(&mut self) {
        self.frame_selected_nodes();
    }

    fn set_view(&mut self, view: ViewArg, pane: Option<PaneArg>) {
        self.with_pane(pane.map(pane_of), |s| s.show_view(view));
    }

    fn set_layout(&mut self, layout: Option<LayoutArg>) {
        self.set_view_layout(layout.map(layout_of));
    }

    fn maximize(&mut self, pane: Option<PaneArg>) {
        self.toggle_maximize(pane.map(pane_of));
    }

    fn set_lighting(&mut self, lighting: LightingArg) {
        self.views.lighting = lighting;
    }

    fn set_shadows(&mut self, on: bool) {
        self.views.shadows = on;
    }

    fn set_antialias(&mut self, on: bool) {
        self.views.antialias = on;
    }

    fn set_clone_display(&mut self, indices: Option<bool>, axes: Option<bool>) {
        if let Some(on) = indices {
            self.views.clone_indices = on;
        }
        if let Some(on) = axes {
            self.views.clone_axes = on;
        }
    }

    fn set_camera(&mut self, set: CameraSet) {
        self.apply_camera_set(set);
    }

    fn camera_info(&self) -> Value {
        self.camera_json()
    }

    fn set_gizmo_mode(&mut self, mode: GizmoModeArg) {
        self.gizmo_mode = match mode {
            GizmoModeArg::Move => GizmoMode::Translate,
            GizmoModeArg::Rotate => GizmoMode::Rotate,
            GizmoModeArg::Scale => GizmoMode::Scale,
        };
    }

    fn set_gizmo_space(&mut self, space: Option<GizmoSpaceArg>) {
        self.gizmo_space = match space {
            Some(GizmoSpaceArg::Object) => GizmoSpace::Object,
            Some(GizmoSpaceArg::World) => GizmoSpace::World,
            None => match self.gizmo_space {
                GizmoSpace::Object => GizmoSpace::World,
                GizmoSpace::World => GizmoSpace::Object,
            },
        };
    }

    fn request_ui(&mut self, request: UiRequest) -> Result<(), String> {
        self.ui_requests.push(request);
        Ok(())
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

/// One entry of the Create > Clone toolset submenu: the command that creates it.
#[derive(Debug, Clone, PartialEq)]
pub struct ToolsetEntry {
    pub label: String,
    pub command: &'static str,
    pub args: Value,
}

/// The Create > Clone toolset submenu (docs/specs/UI.md §16.2): the cloner, Cloner from
/// Selection, then every registered effector and field type, taken from the registry.
pub fn clone_toolset_menu(registry: &Registry) -> Vec<ToolsetEntry> {
    let create = |id: &str, label: &str| ToolsetEntry {
        label: label.to_string(),
        command: "object.create",
        args: serde_json::json!({"type": id}),
    };
    let mut out = Vec::new();
    if let Some(t) = registry.get(forge_clone_engine::CLONER) {
        out.push(create(&t.id, &t.label));
        out.push(ToolsetEntry {
            label: "Cloner from Selection".into(),
            command: "object.create_cloner",
            args: serde_json::json!({}),
        });
    }
    for category in [Category::Effector, Category::Field] {
        let mut types: Vec<_> = registry
            .types()
            .filter(|t| t.category == category && t.in_create_menu)
            .collect();
        types.sort_by(|a, b| a.label.cmp(&b.label));
        out.extend(types.into_iter().map(|t| create(&t.id, &t.label)));
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
