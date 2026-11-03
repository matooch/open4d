//! Layers: document level groups of objects with toggles (docs/specs/CORE.md §7).
//!
//! A layer is not a tree node. Objects link to at most one layer through the hidden
//! `object.layer` param; the functions here resolve the toggles for a node.

use serde::{Deserialize, Serialize};

use crate::{Category, NodeId, Scene, Value};

/// The hidden link param every hierarchy type has.
pub const LAYER: &str = "object.layer";

/// A layer id: taken from the scene's node id counter, so it never equals a node id.
pub type LayerId = NodeId;

/// Colours new layers cycle through (linear 0..1 RGB).
pub const PALETTE: [[f64; 3]; 8] = [
    [0.86, 0.33, 0.30],
    [0.90, 0.60, 0.22],
    [0.88, 0.80, 0.30],
    [0.40, 0.70, 0.38],
    [0.28, 0.68, 0.72],
    [0.30, 0.50, 0.85],
    [0.60, 0.42, 0.82],
    [0.82, 0.42, 0.66],
];

fn on() -> bool {
    true
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Layer {
    pub id: LayerId,
    pub name: String,
    pub color: [f64; 3],
    /// Only layers with solo on (and objects on them) are shown in the viewport, while any
    /// layer is solo.
    #[serde(default)]
    pub solo: bool,
    /// Viewport visibility.
    #[serde(default = "on")]
    pub view: bool,
    /// Render visibility.
    #[serde(default = "on")]
    pub render: bool,
    /// Whether the object manager lists the members.
    #[serde(default = "on")]
    pub manager: bool,
    /// Members cannot be picked or dragged in the viewport.
    #[serde(default)]
    pub locked: bool,
    /// Tracks of the members are evaluated.
    #[serde(default = "on")]
    pub animation: bool,
    /// Generator members produce geometry.
    #[serde(default = "on")]
    pub generators: bool,
    /// Deformer members deform.
    #[serde(default = "on")]
    pub deformers: bool,
    /// Expression tags on members run.
    #[serde(default = "on")]
    pub expressions: bool,
}

impl Layer {
    /// A layer with the defaults: everything on, not solo, not locked.
    pub fn new(id: LayerId, name: &str, color: [f64; 3]) -> Layer {
        Layer {
            id,
            name: name.to_string(),
            color,
            solo: false,
            view: true,
            render: true,
            manager: true,
            locked: false,
            animation: true,
            generators: true,
            deformers: true,
            expressions: true,
        }
    }
}

/// The layer table in creation order.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Layers {
    list: Vec<Layer>,
}

impl Layers {
    pub fn from_list(list: Vec<Layer>) -> Layers {
        Layers { list }
    }

    pub fn iter(&self) -> std::slice::Iter<'_, Layer> {
        self.list.iter()
    }

    pub fn as_slice(&self) -> &[Layer] {
        &self.list
    }

    pub fn len(&self) -> usize {
        self.list.len()
    }

    pub fn is_empty(&self) -> bool {
        self.list.is_empty()
    }

    pub fn get(&self, id: LayerId) -> Option<&Layer> {
        self.list.iter().find(|l| l.id == id)
    }

    pub fn contains(&self, id: LayerId) -> bool {
        self.get(id).is_some()
    }

    pub fn any_solo(&self) -> bool {
        self.list.iter().any(|l| l.solo)
    }

    /// Replaces the layer with the same id; false when there is none.
    pub(crate) fn update(&mut self, layer: Layer) -> bool {
        match self.list.iter_mut().find(|l| l.id == layer.id) {
            Some(slot) => {
                *slot = layer;
                true
            }
            None => false,
        }
    }

    pub(crate) fn push(&mut self, layer: Layer) {
        self.list.push(layer);
    }

    pub(crate) fn remove(&mut self, id: LayerId) {
        self.list.retain(|l| l.id != id);
    }

    /// The colour a new layer gets.
    pub fn next_color(&self) -> [f64; 3] {
        PALETTE[self.list.len() % PALETTE.len()]
    }

    /// "Layer N" with the smallest N not used by a name yet.
    pub fn next_name(&self) -> String {
        (1..)
            .map(|n| format!("Layer {n}"))
            .find(|name| self.list.iter().all(|l| &l.name != name))
            .expect("an unused name exists")
    }
}

/// The layer id a node links to (a tag: its host's), if it is in the table.
pub fn layer_id(scene: &Scene, id: NodeId) -> Option<LayerId> {
    let node = scene.node(id)?;
    let owner = node.host.unwrap_or(id);
    match scene.param(owner, LAYER) {
        Some(Value::Link(Some(l))) if scene.layers().contains(*l) => Some(*l),
        _ => None,
    }
}

/// The node's own layer (a tag: its host's).
pub fn of(scene: &Scene, id: NodeId) -> Option<&Layer> {
    scene.layers().get(layer_id(scene, id)?)
}

/// The nodes that link to `layer`, in document order (tags excluded).
pub fn members(scene: &Scene, layer: LayerId) -> Vec<NodeId> {
    let mut out = Vec::new();
    let mut stack: Vec<NodeId> = scene.roots().iter().rev().copied().collect();
    while let Some(id) = stack.pop() {
        if matches!(scene.param(id, LAYER), Some(Value::Link(Some(l))) if *l == layer) {
            out.push(id);
        }
        if let Some(n) = scene.node(id) {
            stack.extend(n.children.iter().rev());
        }
    }
    out
}

/// Whether the viewport draws the node as far as layers decide (solo and view).
pub fn shown(scene: &Scene, id: NodeId) -> bool {
    let solo = scene.layers().any_solo();
    match of(scene, id) {
        Some(l) => l.view && (!solo || l.solo),
        None => !solo,
    }
}

/// Whether a render includes the node as far as layers decide.
pub fn render_shown(scene: &Scene, id: NodeId) -> bool {
    of(scene, id).is_none_or(|l| l.render)
}

/// The viewport cannot pick or drag the node.
pub fn locked(scene: &Scene, id: NodeId) -> bool {
    of(scene, id).is_some_and(|l| l.locked)
}

/// The object manager lists the node.
pub fn in_manager(scene: &Scene, id: NodeId) -> bool {
    of(scene, id).is_none_or(|l| l.manager)
}

/// The node's tracks are evaluated.
pub fn animation_on(scene: &Scene, id: NodeId) -> bool {
    of(scene, id).is_none_or(|l| l.animation)
}

/// A generator node produces geometry (nodes of other categories are never bypassed).
pub fn generators_on(scene: &Scene, id: NodeId) -> bool {
    of(scene, id).is_none_or(|l| l.generators)
        || !scene
            .node_type(id)
            .is_some_and(|t| t.category == Category::Generator)
}

/// The node, if it is a deformer, deforms.
pub fn deformers_on(scene: &Scene, id: NodeId) -> bool {
    of(scene, id).is_none_or(|l| l.deformers)
}

/// The expression tag (or any node) runs its expressions.
pub fn expressions_on(scene: &Scene, id: NodeId) -> bool {
    of(scene, id).is_none_or(|l| l.expressions)
}

/// Everything the layers decide about a node, as bits, for change detection (CORE.md §7.5).
pub fn effect(scene: &Scene, id: NodeId) -> u8 {
    u8::from(shown(scene, id))
        | u8::from(render_shown(scene, id)) << 1
        | u8::from(animation_on(scene, id)) << 2
        | u8::from(generators_on(scene, id)) << 3
        | u8::from(deformers_on(scene, id)) << 4
        | u8::from(expressions_on(scene, id)) << 5
}
