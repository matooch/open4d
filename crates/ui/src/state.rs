//! Editor state and node selection (docs/specs/UI.md §1).

use std::collections::BTreeSet;
use std::sync::Arc;

use forge_core::{Document, NodeId, Registry, Scene};
use forge_viewport::camera::Camera;
use forge_viewport::render::Shading;

use crate::dock::Dock;
use crate::timeline::Timeline;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NodeSelection {
    pub nodes: BTreeSet<NodeId>,
    pub active: Option<NodeId>,
    pub anchor: Option<NodeId>,
}

impl NodeSelection {
    pub fn clear(&mut self) {
        *self = NodeSelection::default();
    }

    /// Selects only `id`.
    pub fn click(&mut self, id: NodeId) {
        self.nodes = BTreeSet::from([id]);
        self.active = Some(id);
        self.anchor = Some(id);
    }

    /// Adds or removes `id` (Ctrl-click).
    pub fn toggle(&mut self, id: NodeId) {
        if self.nodes.remove(&id) {
            self.active = self.nodes.last().copied();
        } else {
            self.nodes.insert(id);
            self.active = Some(id);
        }
        self.anchor = Some(id);
    }

    /// Selects the visible rows from the anchor to `id` (Shift-click).
    pub fn range(&mut self, id: NodeId, rows: &[NodeId]) {
        let (Some(anchor), Some(to)) = (self.anchor, rows.iter().position(|r| *r == id)) else {
            return self.click(id);
        };
        let Some(from) = rows.iter().position(|r| *r == anchor) else {
            return self.click(id);
        };
        let (lo, hi) = (from.min(to), from.max(to));
        self.nodes = rows[lo..=hi].iter().copied().collect();
        self.active = Some(id);
    }

    /// Drops nodes that no longer exist.
    pub fn retain_existing(&mut self, scene: &Scene) {
        self.nodes.retain(|id| scene.contains(*id));
        let gone = |id: &mut Option<NodeId>| {
            if id.is_some_and(|i| !scene.contains(i)) {
                *id = None;
            }
        };
        gone(&mut self.active);
        gone(&mut self.anchor);
        if self.active.is_none() {
            self.active = self.nodes.last().copied();
        }
    }
}

pub struct EditorState {
    pub doc: Document,
    pub selection: NodeSelection,
    pub expanded: BTreeSet<NodeId>,
    pub dock: Dock,
    pub camera: Camera,
    pub shading: Shading,
    pub timeline: Timeline,
    /// Last message for the status bar (errors from failed edits).
    pub status: String,
}

impl EditorState {
    pub fn new(registry: Arc<Registry>) -> EditorState {
        let mut camera = Camera::default();
        camera.orbit(0.6, 0.45);
        EditorState {
            doc: Document::new(registry),
            selection: NodeSelection::default(),
            expanded: BTreeSet::new(),
            dock: Dock::default(),
            camera,
            shading: Shading::SmoothWire,
            timeline: Timeline::default(),
            status: String::new(),
        }
    }
}
