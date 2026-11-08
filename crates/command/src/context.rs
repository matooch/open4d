//! What a command may touch (docs/specs/COMMANDS.md §3). No egui, no GPU.

use std::sync::Arc;

use forge_core::{Document, NodeId, Scene};
use forge_modeling::ComponentSettings;
use forge_modeling::snap::TransformSettings;
use serde_json::Value;

use crate::browser::Browser;
use crate::state::{
    CameraSet, GizmoModeArg, GizmoSpaceArg, HudItemArg, LayoutArg, LightingArg, ModeArg,
    MotionDisplay, NodeSelection, PaneArg, SelectExtra, SelectOp, ShadingMode, UiRequest, ViewArg,
};
use crate::timeline::Timeline;

pub trait CommandContext {
    fn doc(&self) -> &Document;
    fn doc_mut(&mut self) -> &mut Document;
    fn selection(&self) -> &NodeSelection;
    fn selection_mut(&mut self) -> &mut NodeSelection;
    fn edit_mode(&self) -> ModeArg;
    fn set_edit_mode(&mut self, mode: ModeArg);
    /// Soft selection and symmetry for component transforms (EDITABLE.md §5).
    fn component_settings(&self) -> &ComponentSettings;
    fn component_settings_mut(&mut self) -> &mut ComponentSettings;
    /// Snapping and axis mode (MANIPULATE.md §6, §7.4).
    fn transform_settings(&self) -> &TransformSettings;
    fn transform_settings_mut(&mut self) -> &mut TransformSettings;
    fn timeline(&self) -> &Timeline;
    fn timeline_mut(&mut self) -> &mut Timeline;
    fn autokey(&self) -> bool;
    fn set_autokey(&mut self, on: bool);
    /// Message for the status bar (empty clears it).
    fn set_status(&mut self, text: String);

    /// Select All / None / Invert / Grow / Shrink on the current mode's set (EDITABLE.md §2.5).
    ///
    /// `nodes` names the polygon objects to work on; the component targets when `None`
    /// (EDITABLE.md §6.3).
    fn select_components(&mut self, op: SelectOp, nodes: Option<&[NodeId]>) -> Result<(), String>;
    /// Connected, Convert, Loop and Ring on the current mode's set (EDITABLE.md §2.8).
    fn select_extra(&mut self, _cmd: SelectExtra, _nodes: Option<&[NodeId]>) -> Result<(), String> {
        Err("no components headless".into())
    }
    /// Keys the selected objects' transform at the current frame; returns the key count.
    fn record_keys(&mut self) -> Result<usize, String>;
    /// Removes the selected objects' keys on the current frame; returns the key count.
    fn delete_keys(&mut self) -> Result<usize, String>;
    /// The frame of the previous or next key of the selected objects, if any.
    fn key_frame_near(&self, forward: bool) -> Option<i64>;

    /// Queues an autosave of the document now (PLATFORM.md §7.8); `wait` flushes the write.
    /// Returns the autosave file's path.
    fn autosave_now(&mut self, _wait: bool) -> Result<String, String> {
        Err("needs the editor window".into())
    }

    /// Replaces the document (new or opened scene). `full` also resets view state.
    fn reset_document(&mut self, doc: Document, path: Option<String>, full: bool);
    /// The file the scene was last saved to or loaded from.
    fn file_path(&self) -> Option<String>;
    /// Records that the scene was saved to `path` (clears the dirty state).
    fn mark_saved(&mut self, path: String);

    /// The preset library and recent files (UI.md §26); `None` in contexts without them.
    fn browser(&self) -> Option<&Browser> {
        None
    }
    fn browser_mut(&mut self) -> Option<&mut Browser> {
        None
    }

    /// The scene at the current frame (animation applied). Contexts without an animation
    /// evaluator return the stored scene.
    fn evaluated_scene(&self) -> Arc<Scene> {
        Arc::new(self.doc().scene().clone())
    }

    // View-only state; headless contexts ignore it.
    /// Sets a pane's shading (`None`: the active pane).
    fn set_shading(&mut self, _shading: ShadingMode, _pane: Option<PaneArg>) {}
    fn frame_all(&mut self) {}
    /// Frames the selected nodes and their descendants (all when none has geometry).
    fn frame_selected(&mut self) {}
    /// Points a pane's camera along a named view (`None`: the active pane).
    fn set_view(&mut self, _view: ViewArg, _pane: Option<PaneArg>) {}
    /// Sets the viewport layout; `None` toggles single / four.
    fn set_layout(&mut self, _layout: Option<LayoutArg>) {}
    /// Toggles a pane's maximised state in the four-view layout (`None`: the active pane).
    fn maximize(&mut self, _pane: Option<PaneArg>) {}
    fn set_lighting(&mut self, _lighting: LightingArg) {}
    fn set_shadows(&mut self, _on: bool) {}
    fn set_antialias(&mut self, _on: bool) {}
    /// Shows or hides a HUD item in a pane (`None`: the active pane), in all panes, or
    /// toggles it (`enabled` `None`).
    fn set_hud(
        &mut self,
        _item: HudItemArg,
        _pane: Option<PaneArg>,
        _all_panes: bool,
        _enabled: Option<bool>,
    ) {
    }
    /// Switches the clone index and axes overlay (`None` keeps a setting).
    fn set_clone_display(&mut self, _indices: Option<bool>, _axes: Option<bool>) {}
    /// The motion path and onion skin display (default for headless contexts).
    fn motion_display(&self) -> MotionDisplay {
        MotionDisplay::default()
    }
    /// Stores an already validated motion display.
    fn set_motion_display(&mut self, _display: MotionDisplay) {}
    /// Applies validated camera fields.
    fn set_camera(&mut self, _set: CameraSet) {}
    /// The camera as JSON (`view.get_camera`); null without a viewport.
    fn camera_info(&self) -> Value {
        Value::Null
    }
    fn toggle_expanded(&mut self, _node: NodeId) {}
    fn set_gizmo_mode(&mut self, _mode: GizmoModeArg) {}
    /// Sets the gizmo space; `None` toggles between object and world.
    fn set_gizmo_space(&mut self, _space: Option<GizmoSpaceArg>) {}
    /// Asks the editor window for something only it can do.
    fn request_ui(&mut self, _request: UiRequest) -> Result<(), String> {
        Err("needs the editor window".into())
    }

    /// Drops selected nodes that no longer exist (after undo, redo, delete).
    fn retain_selection(&mut self) {
        let mut sel = self.selection().clone();
        sel.retain_existing(self.doc().scene());
        *self.selection_mut() = sel;
    }

    /// Pulls the current frame back into the project range (after undo, load, range edits).
    fn sync_timeline(&mut self) {
        let settings = self.doc().scene().settings();
        self.timeline_mut().clamp_to(&settings);
    }
}
