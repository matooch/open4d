//! Object state flags: editor and render visibility and the enable flag
//! (docs/specs/TRANSFORM.md §2, UI.md §13). Stored as the `object.*` params every
//! hierarchy type has; resolved down the hierarchy here.

use crate::{NodeId, ParamDesc, Scene, Value};

pub const EDITOR_VIS: &str = "object.editor_vis";
pub const RENDER_VIS: &str = "object.render_vis";
pub const ENABLED: &str = "object.enabled";
/// Param ids with this prefix are reserved for the object state.
pub const PREFIX: &str = "object.";

const GROUP: &str = "Object";

/// A visibility setting: follow the parent, force on, force off.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Vis {
    Default,
    On,
    Off,
}

impl Vis {
    pub fn id(self) -> &'static str {
        match self {
            Vis::Default => "default",
            Vis::On => "on",
            Vis::Off => "off",
        }
    }

    pub fn from_id(id: &str) -> Option<Vis> {
        match id {
            "default" => Some(Vis::Default),
            "on" => Some(Vis::On),
            "off" => Some(Vis::Off),
            _ => None,
        }
    }

    /// The dot click cycle: Default -> Off -> On -> Default.
    pub fn next(self) -> Vis {
        match self {
            Vis::Default => Vis::Off,
            Vis::Off => Vis::On,
            Vis::On => Vis::Default,
        }
    }
}

/// The three params every hierarchy type gets (after the transform params).
pub(crate) fn params() -> [ParamDesc; 3] {
    let opts = [("default", "Default"), ("on", "On"), ("off", "Off")];
    [
        ParamDesc::enumeration(EDITOR_VIS, "Visible in Editor", "default", &opts)
            .group(GROUP)
            .hidden(),
        ParamDesc::enumeration(RENDER_VIS, "Visible in Renderer", "default", &opts)
            .group(GROUP)
            .hidden(),
        ParamDesc::bool(ENABLED, "Active", true)
            .group(GROUP)
            .hidden(),
    ]
}

fn vis_param(scene: &Scene, id: NodeId, param: &str) -> Vis {
    match scene.param(id, param) {
        Some(Value::Enum(v)) => Vis::from_id(v).unwrap_or(Vis::Default),
        _ => Vis::Default,
    }
}

fn flag(scene: &Scene, id: NodeId) -> bool {
    !matches!(scene.param(id, ENABLED), Some(Value::Bool(false)))
}

/// A node's own settings: `(editor, render, enabled)`.
pub fn own(scene: &Scene, id: NodeId) -> (Vis, Vis, bool) {
    (
        vis_param(scene, id, EDITOR_VIS),
        vis_param(scene, id, RENDER_VIS),
        flag(scene, id),
    )
}

/// The node and its ancestors, nearest first.
fn chain(scene: &Scene, id: NodeId) -> impl Iterator<Item = NodeId> + '_ {
    std::iter::successors(Some(id), move |i| scene.node(*i).and_then(|n| n.parent))
}

/// True iff the node and every ancestor are enabled.
pub fn enabled(scene: &Scene, id: NodeId) -> bool {
    chain(scene, id).all(|i| flag(scene, i))
}

fn resolve(scene: &Scene, id: NodeId, param: &str) -> bool {
    enabled(scene, id)
        && chain(scene, id)
            .map(|i| vis_param(scene, i, param))
            .find(|v| *v != Vis::Default)
            .is_none_or(|v| v == Vis::On)
}

/// Whether the viewport draws the node: enabled, and the nearest `on`/`off` setting up the
/// hierarchy is `on` (or none is set).
pub fn editor_visible(scene: &Scene, id: NodeId) -> bool {
    resolve(scene, id, EDITOR_VIS)
}

/// Whether a render includes the node (same rule with the render setting).
pub fn render_visible(scene: &Scene, id: NodeId) -> bool {
    resolve(scene, id, RENDER_VIS)
}
