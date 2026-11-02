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

/// Whether a converted selection needs all parts of an element or any part.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ConvertMode {
    /// All parts selected (observed, EDITABLE.md §2.8).
    #[default]
    Strict,
    /// Any part selected.
    Tolerant,
}

/// Selection commands that need arguments or are not plain set operations (EDITABLE.md §2.8).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectExtra {
    /// Whole connected pieces of the current mode's set.
    Connected,
    /// The current set converted to `to`; the source stays selected.
    Convert { to: ModeArg, mode: ConvertMode },
    /// Edge loops, or polygon strips crossing edge pair `across`.
    Loop { across: u8 },
    /// Edge rings, or the strips across the other pair.
    Ring { across: u8 },
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
    Quick,
    Constant,
    HiddenLine,
    BoundingBox,
}

/// One of the four viewport panes (mirrors the viewport's `Pane`; docs/specs/VIEWPORT.md §9).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PaneArg {
    Perspective,
    Top,
    Front,
    Right,
}

/// `view.layout`: one viewport or the 2x2 four-view grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum LayoutArg {
    Single,
    Four,
}

/// `view.lighting`: the lights the viewport shades with (docs/specs/VIEWPORT.md §8.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum LightingArg {
    Headlight,
    Rig,
}

/// The viewport gizmo's mode (mirrors the viewport's `GizmoMode`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum GizmoModeArg {
    Move,
    Rotate,
    Scale,
}

/// The frame the gizmo's axes follow (mirrors the viewport's `Space`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum GizmoSpaceArg {
    Object,
    World,
}

/// A named camera view (`view.set_view`; mirrors the viewport's `StandardView`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ViewArg {
    Perspective,
    Front,
    Back,
    Left,
    Right,
    Top,
    Bottom,
}

/// Projection kind of `view.set_camera`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ProjectionArg {
    Perspective,
    Orthographic,
}

/// The fields `view.set_camera` may change; absent fields are kept.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CameraSet {
    /// Pivot point in world coordinates.
    pub target: Option<[f64; 3]>,
    /// Pivot to eye distance, greater than 0.
    pub distance: Option<f64>,
    /// Display-space yaw in degrees.
    pub yaw: Option<f64>,
    /// Display-space pitch in degrees (clamped just short of straight up/down).
    pub pitch: Option<f64>,
    pub projection: Option<ProjectionArg>,
    /// Vertical field of view in degrees, between 1 and 170 (perspective).
    pub fov: Option<f64>,
    /// Visible world height, greater than 0 (orthographic).
    pub height: Option<f64>,
}

/// Something only the editor window can do (file dialogs, inline rename). A command asks
/// for it through [`crate::CommandContext::request_ui`]; headless contexts refuse.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiRequest {
    /// File > Open with a file dialog.
    Open,
    /// File > Save: to the current file, or asks for one.
    Save,
    /// File > Save As with a file dialog.
    SaveAs,
    /// Starts the inline rename of the active object.
    RenameActive,
}
