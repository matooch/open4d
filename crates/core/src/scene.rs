//! Scene graph: node arena, hierarchy, tags, revisions (docs/specs/CORE.md §3, §5).
//!
//! Public API is read-only; mutation goes through [`crate::Document`] so every change
//! is undoable.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};

use glam::DMat4;

use crate::transform::Transform;
use crate::{EditError, NodeType, Registry, Track, Value};

/// Highest revision handed out by [`Scene::with_params`] in this process.
static LAST_UNIQUE_REV: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct NodeId(pub u64);

impl std::fmt::Display for NodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Node {
    pub id: NodeId,
    pub type_id: String,
    pub name: String,
    pub params: BTreeMap<String, Value>,
    /// Hierarchy parent; `None` for roots and tags.
    pub parent: Option<NodeId>,
    pub children: Vec<NodeId>,
    pub tags: Vec<NodeId>,
    /// Set for tag nodes only.
    pub host: Option<NodeId>,
    /// Keyframe tracks by track key (`"<param>"` or `"<param>.<x|y|z>"`).
    pub tracks: BTreeMap<String, Track>,
    /// Revision of the last edit that touched this node.
    pub rev: u64,
}

impl Node {
    pub fn is_tag(&self) -> bool {
        self.host.is_some()
    }
}

/// Where a node sits: in a child list (or the root list) or a host's tag list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Slot {
    Child {
        parent: Option<NodeId>,
        index: usize,
    },
    Tag {
        host: NodeId,
        index: usize,
    },
}

/// A removed node with everything below it; `nodes[0]` is the root.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Subtree {
    pub nodes: Vec<Node>,
}

/// Document-wide animation settings (docs/specs/ANIM.md §2.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SceneSettings {
    /// Frames per second, `1..=240`.
    pub fps: u32,
    /// First and last frame of the project range, `start <= end`.
    pub start: i64,
    pub end: i64,
}

impl Default for SceneSettings {
    fn default() -> Self {
        SceneSettings {
            fps: 30,
            start: 0,
            end: 90,
        }
    }
}

impl SceneSettings {
    pub const MAX_FPS: u32 = 240;

    /// Why these settings are not valid, if they are not.
    pub fn check(&self) -> Result<(), String> {
        if !(1..=Self::MAX_FPS).contains(&self.fps) {
            return Err(format!(
                "fps must be 1..={}, got {}",
                Self::MAX_FPS,
                self.fps
            ));
        }
        if self.start > self.end {
            return Err(format!(
                "start frame {} is after end frame {}",
                self.start, self.end
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct Scene {
    registry: Arc<Registry>,
    nodes: BTreeMap<NodeId, Node>,
    roots: Vec<NodeId>,
    next_id: u64,
    revision: u64,
    settings: SceneSettings,
}

impl Scene {
    pub fn new(registry: Arc<Registry>) -> Scene {
        Scene {
            registry,
            nodes: BTreeMap::new(),
            roots: Vec::new(),
            next_id: 1,
            revision: 0,
            settings: SceneSettings::default(),
        }
    }

    /// Animation settings: fps and project range.
    pub fn settings(&self) -> SceneSettings {
        self.settings
    }

    pub fn registry(&self) -> &Arc<Registry> {
        &self.registry
    }

    pub fn node(&self, id: NodeId) -> Option<&Node> {
        self.nodes.get(&id)
    }

    /// The registered type of a node; `None` for missing nodes and unknown types.
    pub fn node_type(&self, id: NodeId) -> Option<&Arc<NodeType>> {
        self.registry.get(&self.node(id)?.type_id)
    }

    pub fn contains(&self, id: NodeId) -> bool {
        self.nodes.contains_key(&id)
    }

    pub fn roots(&self) -> &[NodeId] {
        &self.roots
    }

    pub fn param(&self, id: NodeId, param: &str) -> Option<&Value> {
        self.node(id)?.params.get(param)
    }

    /// Number of nodes, tags included.
    /// The scene's revision counter; any edit raises it.
    pub fn revision(&self) -> u64 {
        self.revision
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Hierarchy descendants of `id` in depth-first pre-order, excluding `id` and tags.
    pub fn descendants(&self, id: NodeId) -> Vec<NodeId> {
        let mut out = Vec::new();
        let mut stack: Vec<NodeId> = match self.node(id) {
            Some(n) => n.children.iter().rev().copied().collect(),
            None => return out,
        };
        while let Some(c) = stack.pop() {
            out.push(c);
            if let Some(n) = self.node(c) {
                stack.extend(n.children.iter().rev());
            }
        }
        out
    }

    /// The node's own transform; `None` for missing nodes and tags.
    pub fn local_transform(&self, id: NodeId) -> Option<Transform> {
        let n = self.node(id)?;
        (!n.is_tag()).then(|| Transform::from_params(&n.params))
    }

    /// World matrix of the parent (identity for roots); `None` for missing nodes and tags.
    pub fn parent_world_matrix(&self, id: NodeId) -> Option<DMat4> {
        let n = self.node(id)?;
        if n.is_tag() {
            return None;
        }
        match n.parent {
            None => Some(DMat4::IDENTITY),
            Some(p) => self.world_matrix(p),
        }
    }

    /// Parent chain composed root-first: `world(parent) · local(id)`.
    pub fn world_matrix(&self, id: NodeId) -> Option<DMat4> {
        let mut m = self.local_transform(id)?.matrix();
        let mut parent = self.node(id)?.parent;
        while let Some(p) = parent {
            m = self.local_transform(p)?.matrix() * m;
            parent = self.node(p)?.parent;
        }
        Some(m)
    }

    /// Max revision over the node, its subtree, its tags and its link targets.
    pub fn deep_revision(&self, id: NodeId) -> Option<u64> {
        self.contains(id)
            .then(|| self.deep_rev(id, &mut BTreeSet::new()))
    }

    fn deep_rev(&self, id: NodeId, visited: &mut BTreeSet<NodeId>) -> u64 {
        if !visited.insert(id) {
            return 0;
        }
        let Some(n) = self.node(id) else { return 0 };
        let links = n.params.values().filter_map(|v| match v {
            Value::Link(Some(t)) => Some(*t),
            _ => None,
        });
        let deps: Vec<NodeId> = n
            .children
            .iter()
            .chain(&n.tags)
            .copied()
            .chain(links)
            .collect();
        deps.into_iter()
            .map(|d| self.deep_rev(d, visited))
            .fold(n.rev, u64::max)
    }

    /// A copy of the scene with some parameter values replaced, for evaluating animation
    /// at a time (docs/specs/ANIM.md §1.5). Each override is validated like `set_param`
    /// (type, range clamp); one that fails is skipped and returned with its error. The
    /// copy is not an undoable edit. Every node whose value changes gets a revision no
    /// scene copy has used before, so one [`crate::EvalCache`] can serve copies made for
    /// different times.
    pub fn with_params(
        &self,
        overrides: impl IntoIterator<Item = (NodeId, String, Value)>,
    ) -> (Scene, Vec<(NodeId, String, EditError)>) {
        let mut scene = self.clone();
        let mut skipped = Vec::new();
        for (id, param, value) in overrides {
            match scene.check_override(id, &param, value) {
                Ok(value) => {
                    let n = scene.nodes.get_mut(&id).expect("checked node");
                    if n.params.get(&param) != Some(&value) {
                        n.params.insert(param, value);
                        scene.stamp_unique(id);
                    }
                }
                Err(e) => skipped.push((id, param, e)),
            }
        }
        (scene, skipped)
    }

    fn check_override(&self, id: NodeId, param: &str, value: Value) -> Result<Value, EditError> {
        let node = self.node(id).ok_or(EditError::UnknownNode(id))?;
        let ty = self
            .registry
            .get(&node.type_id)
            .ok_or_else(|| EditError::UnknownType(node.type_id.clone()))?;
        let desc = ty
            .find_param(param)
            .ok_or_else(|| EditError::UnknownParam(param.to_string()))?;
        Ok(desc.validate(value)?)
    }

    /// Stamps with a revision above this scene's and above every one handed out so far.
    fn stamp_unique(&mut self, id: NodeId) {
        let local = self.revision;
        let prev = LAST_UNIQUE_REV
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |g| {
                Some(g.max(local) + 1)
            })
            .expect("closure always returns Some");
        self.revision = prev.max(local) + 1;
        let rev = self.revision;
        if let Some(n) = self.nodes.get_mut(&id) {
            n.rev = rev;
        }
    }

    // --- crate-internal mutation primitives (used by Document and file) ---

    /// Replaces the settings (already validated), raises the revision, returns the old ones.
    pub(crate) fn replace_settings(&mut self, settings: SceneSettings) -> SceneSettings {
        self.revision += 1;
        std::mem::replace(&mut self.settings, settings)
    }

    /// Sets the settings of a freshly loaded scene without touching the revision.
    pub(crate) fn init_settings(&mut self, settings: SceneSettings) {
        self.settings = settings;
    }

    pub(crate) fn alloc_id(&mut self) -> NodeId {
        let id = NodeId(self.next_id);
        self.next_id += 1;
        id
    }

    pub(crate) fn set_next_id(&mut self, next: u64) {
        self.next_id = self.next_id.max(next);
    }

    pub(crate) fn stamp(&mut self, id: NodeId) {
        self.revision += 1;
        let rev = self.revision;
        if let Some(n) = self.nodes.get_mut(&id) {
            n.rev = rev;
        }
    }

    /// Replaces (or removes) a track and returns the old one.
    pub(crate) fn replace_track(
        &mut self,
        id: NodeId,
        key: &str,
        track: Option<Track>,
    ) -> Option<Track> {
        let n = self.nodes.get_mut(&id).expect("validated node");
        let old = match track {
            Some(t) => n.tracks.insert(key.to_string(), t),
            None => n.tracks.remove(key),
        };
        self.stamp(id);
        old
    }

    pub(crate) fn node_mut(&mut self, id: NodeId) -> Option<&mut Node> {
        self.nodes.get_mut(&id)
    }

    /// The ordered list a slot indexes into.
    pub(crate) fn slot_list(&self, slot_owner: SlotOwner) -> Option<&Vec<NodeId>> {
        match slot_owner {
            SlotOwner::Children(None) => Some(&self.roots),
            SlotOwner::Children(Some(p)) => self.node(p).map(|n| &n.children),
            SlotOwner::Tags(h) => self.node(h).map(|n| &n.tags),
        }
    }

    fn slot_list_mut(&mut self, slot_owner: SlotOwner) -> &mut Vec<NodeId> {
        match slot_owner {
            SlotOwner::Children(None) => &mut self.roots,
            SlotOwner::Children(Some(p)) => {
                &mut self.nodes.get_mut(&p).expect("slot parent").children
            }
            SlotOwner::Tags(h) => &mut self.nodes.get_mut(&h).expect("slot host").tags,
        }
    }

    /// Current slot of a node.
    pub(crate) fn slot_of(&self, id: NodeId) -> Slot {
        let n = &self.nodes[&id];
        let (owner, list) = match n.host {
            Some(h) => (SlotOwner::Tags(h), &self.nodes[&h].tags),
            None => (
                SlotOwner::Children(n.parent),
                self.slot_list(SlotOwner::Children(n.parent))
                    .expect("parent"),
            ),
        };
        let index = list
            .iter()
            .position(|c| *c == id)
            .expect("node in its slot list");
        owner.slot(index)
    }

    fn detach(&mut self, id: NodeId) -> Slot {
        let slot = self.slot_of(id);
        let (owner, index) = SlotOwner::of(slot);
        self.slot_list_mut(owner).remove(index);
        if let Some(stamp) = owner.node() {
            self.stamp(stamp);
        }
        slot
    }

    fn attach(&mut self, id: NodeId, slot: Slot) {
        let (owner, index) = SlotOwner::of(slot);
        self.slot_list_mut(owner).insert(index, id);
        let n = self.nodes.get_mut(&id).expect("attached node exists");
        match slot {
            Slot::Child { parent, .. } => (n.parent, n.host) = (parent, None),
            Slot::Tag { host, .. } => (n.parent, n.host) = (None, Some(host)),
        }
        if let Some(stamp) = owner.node() {
            self.stamp(stamp);
        }
    }

    pub(crate) fn insert_subtree(&mut self, subtree: Subtree, slot: Slot) -> NodeId {
        let root = subtree.nodes[0].id;
        let ids: Vec<NodeId> = subtree.nodes.iter().map(|n| n.id).collect();
        for n in subtree.nodes {
            let prev = self.nodes.insert(n.id, n);
            assert!(prev.is_none(), "insert_subtree: id already in use");
        }
        for id in ids {
            self.stamp(id);
        }
        self.attach(root, slot);
        root
    }

    pub(crate) fn remove_subtree(&mut self, id: NodeId) -> (Subtree, Slot) {
        let slot = self.detach(id);
        let mut nodes = Vec::new();
        let mut stack = vec![id];
        while let Some(cur) = stack.pop() {
            let n = self.nodes.remove(&cur).expect("subtree node exists");
            stack.extend(n.children.iter().chain(&n.tags).rev());
            nodes.push(n);
        }
        (Subtree { nodes }, slot)
    }

    pub(crate) fn move_to(&mut self, id: NodeId, slot: Slot) -> Slot {
        let old = self.detach(id);
        self.attach(id, slot);
        self.stamp(id);
        old
    }

    /// Inserts a loaded node verbatim (file loading); no stamping of others.
    pub(crate) fn insert_loaded(&mut self, mut node: Node) {
        self.revision += 1;
        node.rev = self.revision;
        if node.parent.is_none() && node.host.is_none() {
            self.roots.push(node.id);
        }
        self.nodes.insert(node.id, node);
    }
}

/// The owner of an ordered node list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SlotOwner {
    Children(Option<NodeId>),
    Tags(NodeId),
}

impl SlotOwner {
    pub(crate) fn of(slot: Slot) -> (SlotOwner, usize) {
        match slot {
            Slot::Child { parent, index } => (SlotOwner::Children(parent), index),
            Slot::Tag { host, index } => (SlotOwner::Tags(host), index),
        }
    }

    fn slot(self, index: usize) -> Slot {
        match self {
            SlotOwner::Children(parent) => Slot::Child { parent, index },
            SlotOwner::Tags(host) => Slot::Tag { host, index },
        }
    }

    fn node(self) -> Option<NodeId> {
        match self {
            SlotOwner::Children(p) => p,
            SlotOwner::Tags(h) => Some(h),
        }
    }
}
