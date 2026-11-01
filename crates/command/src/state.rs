//! Editor state that commands read and write without any UI (docs/specs/COMMANDS.md §3).

use std::collections::BTreeSet;

use forge_core::{NodeId, Scene};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

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

/// What viewport clicks select (mirrors the editor's `EditMode`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ModeArg {
    #[default]
    Model,
    Points,
    Edges,
    Polys,
}

/// Select All / Deselect All / Invert / Grow / Shrink (EDITABLE.md §2.5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SelectOp {
    All,
    None,
    Invert,
    Grow,
    Shrink,
}

/// Viewport shading (mirrors the viewport's `Shading`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ShadingMode {
    Smooth,
    Flat,
    Wire,
    SmoothWire,
    FlatWire,
}
