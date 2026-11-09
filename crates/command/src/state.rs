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

/// A heads-up display item of a viewport pane (docs/specs/UI.md §23).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HudItemArg {
    Fps,
    Counts,
    Tool,
    Frame,
    Camera,
    SafeFrames,
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
#[derive(Debug, Clone, PartialEq)]
pub enum UiRequest {
    /// File > Open with a file dialog.
    Open,
    /// File > Save: to the current file, or asks for one.
    Save,
    /// File > Save As with a file dialog.
    SaveAs,
    /// File > Import with a file chooser and the options window (format id).
    ImportDialog(String),
    /// File > Export with a file chooser and the options window.
    ExportDialog { format: String, selected: bool },
    /// Starts the inline rename of the active object.
    RenameActive,
    /// Opens the command palette (docs/specs/UI.md §18).
    CommandPalette,
    /// Switches to a built-in workspace (docs/specs/UI.md §21).
    Workspace(String),
    /// Saves the current arrangement as a named layout.
    LayoutSave(String),
    /// Applies a saved layout.
    LayoutLoad(String),
    /// Deletes a saved layout.
    LayoutDelete(String),
    /// Switches the theme by name (docs/specs/UI.md §24.3).
    Theme(String),
    /// Sets the UI scale factor.
    Scale(f64),
}

/// Viewport display of motion paths and onion skin (docs/specs/ANIM.md section 2b.4). View
/// state: not saved with the document, not undoable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MotionDisplay {
    pub paths: bool,
    /// Custom path range; the project range when `None`.
    pub range: Option<(i64, i64)>,
    pub step: i64,
    pub onion: bool,
    pub before: u32,
    pub after: u32,
    pub onion_step: i64,
}

impl Default for MotionDisplay {
    fn default() -> Self {
        MotionDisplay {
            paths: false,
            range: None,
            step: 1,
            onion: false,
            before: 2,
            after: 2,
            onion_step: 1,
        }
    }
}

impl MotionDisplay {
    pub const MAX_STEP: i64 = 1000;
    pub const MAX_ONION: u32 = 10;
    pub const MAX_ONION_STEP: i64 = 100;

    /// The display after `view.motion_paths`. With no field at all `paths` is toggled.
    pub fn with_paths(
        &self,
        enabled: Option<bool>,
        range: Option<(i64, i64)>,
        reset_range: bool,
        step: Option<i64>,
    ) -> Result<MotionDisplay, String> {
        let mut out = self.clone();
        if let Some((a, b)) = range {
            if reset_range {
                return Err("`start`/`end` and `reset_range` exclude each other".into());
            }
            if a > b {
                return Err("`start` must not be after `end`".into());
            }
            out.range = Some((a, b));
        }
        if reset_range {
            out.range = None;
        }
        if let Some(s) = step {
            if !(1..=Self::MAX_STEP).contains(&s) {
                return Err(format!("`step` must be 1 to {}", Self::MAX_STEP));
            }
            out.step = s;
        }
        let nothing = enabled.is_none() && range.is_none() && !reset_range && step.is_none();
        out.paths = match enabled {
            Some(e) => e,
            None if nothing => !self.paths,
            None => self.paths,
        };
        Ok(out)
    }

    /// The display after `view.onion_skin`. With no field at all `onion` is toggled.
    pub fn with_onion(
        &self,
        enabled: Option<bool>,
        before: Option<u32>,
        after: Option<u32>,
        step: Option<i64>,
    ) -> Result<MotionDisplay, String> {
        let mut out = self.clone();
        for (name, v) in [("before", before), ("after", after)] {
            if v.is_some_and(|v| v > Self::MAX_ONION) {
                return Err(format!("`{name}` must be 0 to {}", Self::MAX_ONION));
            }
        }
        if let Some(s) = step
            && !(1..=Self::MAX_ONION_STEP).contains(&s)
        {
            return Err(format!("`step` must be 1 to {}", Self::MAX_ONION_STEP));
        }
        out.before = before.unwrap_or(self.before);
        out.after = after.unwrap_or(self.after);
        out.onion_step = step.unwrap_or(self.onion_step);
        let nothing = enabled.is_none() && before.is_none() && after.is_none() && step.is_none();
        out.onion = match enabled {
            Some(e) => e,
            None if nothing => !self.onion,
            None => self.onion,
        };
        Ok(out)
    }
}
