//! What a command may touch (docs/specs/COMMANDS.md §3). No egui, no GPU.

use forge_core::{Document, NodeId};

use crate::state::{ModeArg, NodeSelection, SelectOp, ShadingMode};
use crate::timeline::Timeline;

pub trait CommandContext {
    fn doc(&self) -> &Document;
    fn doc_mut(&mut self) -> &mut Document;
    fn selection(&self) -> &NodeSelection;
    fn selection_mut(&mut self) -> &mut NodeSelection;
    fn edit_mode(&self) -> ModeArg;
    fn set_edit_mode(&mut self, mode: ModeArg);
    fn timeline(&self) -> &Timeline;
    fn timeline_mut(&mut self) -> &mut Timeline;
    fn autokey(&self) -> bool;
    fn set_autokey(&mut self, on: bool);
    /// Message for the status bar (empty clears it).
    fn set_status(&mut self, text: String);

    /// Select All / None / Invert / Grow / Shrink on the current mode's set (EDITABLE.md §2.5).
    fn select_components(&mut self, op: SelectOp) -> Result<(), String>;
    /// Keys the selected objects' transform at the current frame; returns the key count.
    fn record_keys(&mut self) -> Result<usize, String>;
    /// Removes the selected objects' keys on the current frame; returns the key count.
    fn delete_keys(&mut self) -> Result<usize, String>;
    /// The frame of the previous or next key of the selected objects, if any.
    fn key_frame_near(&self, forward: bool) -> Option<i64>;

    /// Replaces the document (new or opened scene). `full` also resets view state.
    fn reset_document(&mut self, doc: Document, path: Option<String>, full: bool);
    /// The file the scene was last saved to or loaded from.
    fn file_path(&self) -> Option<String>;
    /// Records that the scene was saved to `path` (clears the dirty state).
    fn mark_saved(&mut self, path: String);

    // View-only state; headless contexts ignore it.
    fn set_shading(&mut self, _shading: ShadingMode) {}
    fn frame_all(&mut self) {}
    fn toggle_expanded(&mut self, _node: NodeId) {}

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
