//! Edits, transactions and undo/redo (docs/specs/CORE.md §4).

use std::collections::BTreeMap;
use std::sync::Arc;

use crate::scene::{Slot, SlotOwner, Subtree};
use crate::{
    Category, Key, NodeId, ParamError, ParamKind, Registry, Scene, Track, TrackError, Value,
    parse_target,
};

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum EditError {
    #[error("unknown node type {0:?}")]
    UnknownType(String),
    #[error("node {0} does not exist")]
    UnknownNode(NodeId),
    #[error("unknown parameter {0:?}")]
    UnknownParam(String),
    #[error("{0:?} nodes cannot be placed in a scene yet")]
    NotPlaceable(String),
    #[error("wrong category: {0}")]
    WrongCategory(String),
    #[error("index out of range")]
    IndexOutOfRange,
    #[error("a node cannot be moved under itself or its descendants")]
    Cycle,
    #[error("invalid link: {0}")]
    InvalidLink(String),
    #[error(transparent)]
    Param(#[from] ParamError),
    #[error("invalid track: {0}")]
    Track(#[from] TrackError),
}

/// One primitive, invertible change.
#[derive(Debug, Clone, PartialEq)]
enum Edit {
    Insert {
        subtree: Subtree,
        slot: Slot,
    },
    Remove {
        id: NodeId,
    },
    SetParam {
        id: NodeId,
        param: String,
        value: Value,
    },
    Rename {
        id: NodeId,
        name: String,
    },
    Move {
        id: NodeId,
        slot: Slot,
    },
    SetTrack {
        id: NodeId,
        key: String,
        track: Option<Track>,
    },
}

/// Applies `edit` (already validated) and returns its inverse.
fn apply(scene: &mut Scene, edit: Edit) -> Edit {
    match edit {
        Edit::Insert { subtree, slot } => Edit::Remove {
            id: scene.insert_subtree(subtree, slot),
        },
        Edit::Remove { id } => {
            let (subtree, slot) = scene.remove_subtree(id);
            Edit::Insert { subtree, slot }
        }
        Edit::SetParam { id, param, value } => {
            let n = scene.node_mut(id).expect("validated node");
            let old = n
                .params
                .insert(param.clone(), value)
                .expect("validated param");
            scene.stamp(id);
            Edit::SetParam {
                id,
                param,
                value: old,
            }
        }
        Edit::Rename { id, name } => {
            let n = scene.node_mut(id).expect("validated node");
            let old = std::mem::replace(&mut n.name, name);
            scene.stamp(id);
            Edit::Rename { id, name: old }
        }
        Edit::Move { id, slot } => Edit::Move {
            id,
            slot: scene.move_to(id, slot),
        },
        Edit::SetTrack { id, key, track } => {
            let old = scene.replace_track(id, &key, track);
            Edit::SetTrack {
                id,
                key,
                track: old,
            }
        }
    }
}

/// Edits applied in order; applying them in reverse order toggles the action.
#[derive(Debug, Clone)]
struct Transaction {
    label: String,
    edits: Vec<Edit>,
}

#[derive(Debug, Clone, Default)]
pub struct History {
    undo: Vec<Transaction>,
    redo: Vec<Transaction>,
    /// Merge key of the last history action, while its merge window is open.
    merge_key: Option<String>,
}

impl History {
    /// Oldest first.
    pub fn undo_labels(&self) -> Vec<&str> {
        self.undo.iter().map(|t| t.label.as_str()).collect()
    }

    /// Oldest first; the last entry is redone next.
    pub fn redo_labels(&self) -> Vec<&str> {
        self.redo.iter().map(|t| t.label.as_str()).collect()
    }
}

/// An open transaction. Every operation validates fully before changing anything.
pub struct Tx<'a> {
    scene: &'a mut Scene,
    inverses: Vec<Edit>,
}

impl Tx<'_> {
    pub fn scene(&self) -> &Scene {
        self.scene
    }

    fn push(&mut self, edit: Edit) {
        let inverse = apply(self.scene, edit);
        self.inverses.push(inverse);
    }

    fn require(&self, id: NodeId) -> Result<&crate::Node, EditError> {
        self.scene.node(id).ok_or(EditError::UnknownNode(id))
    }

    fn require_hierarchy(&self, id: NodeId) -> Result<(), EditError> {
        if self.require(id)?.is_tag() {
            return Err(EditError::WrongCategory(format!("node {id} is a tag")));
        }
        Ok(())
    }

    fn type_of(&self, type_id: &str) -> Result<Arc<crate::NodeType>, EditError> {
        self.scene
            .registry()
            .get(type_id)
            .cloned()
            .ok_or_else(|| EditError::UnknownType(type_id.to_string()))
    }

    fn index_in(
        &self,
        owner: SlotOwner,
        index: Option<usize>,
        len_adjust: usize,
    ) -> Result<usize, EditError> {
        let len = self.scene.slot_list(owner).map_or(0, Vec::len) - len_adjust;
        match index {
            None => Ok(len),
            Some(i) if i <= len => Ok(i),
            Some(_) => Err(EditError::IndexOutOfRange),
        }
    }

    fn new_node(&mut self, ty: &crate::NodeType) -> crate::Node {
        let params: BTreeMap<String, Value> = ty
            .params
            .iter()
            .map(|p| (p.id.clone(), p.default.clone()))
            .collect();
        crate::Node {
            id: self.scene.alloc_id(),
            type_id: ty.id.clone(),
            name: ty.label.clone(),
            params,
            parent: None,
            children: Vec::new(),
            tags: Vec::new(),
            host: None,
            tracks: BTreeMap::new(),
            rev: 0,
        }
    }

    /// Creates a hierarchy node under `parent` (None = root) at `index` (None = end).
    pub fn create(
        &mut self,
        type_id: &str,
        parent: Option<NodeId>,
        index: Option<usize>,
    ) -> Result<NodeId, EditError> {
        let ty = self.type_of(type_id)?;
        if ty.category == Category::Tag {
            return Err(EditError::WrongCategory(format!(
                "{type_id} is a tag; use attach_tag"
            )));
        }
        if !ty.category.is_hierarchy() {
            return Err(EditError::NotPlaceable(type_id.to_string()));
        }
        if let Some(p) = parent {
            self.require_hierarchy(p)?;
        }
        let index = self.index_in(SlotOwner::Children(parent), index, 0)?;
        let node = self.new_node(&ty);
        let id = node.id;
        self.push(Edit::Insert {
            subtree: Subtree { nodes: vec![node] },
            slot: Slot::Child { parent, index },
        });
        Ok(id)
    }

    /// Creates a tag on `host` at `index` (None = end).
    pub fn attach_tag(
        &mut self,
        type_id: &str,
        host: NodeId,
        index: Option<usize>,
    ) -> Result<NodeId, EditError> {
        let ty = self.type_of(type_id)?;
        if ty.category != Category::Tag {
            return Err(EditError::WrongCategory(format!("{type_id} is not a tag")));
        }
        self.require_hierarchy(host)?;
        let index = self.index_in(SlotOwner::Tags(host), index, 0)?;
        let node = self.new_node(&ty);
        let id = node.id;
        self.push(Edit::Insert {
            subtree: Subtree { nodes: vec![node] },
            slot: Slot::Tag { host, index },
        });
        Ok(id)
    }

    /// Removes a node with its subtree and all their tags.
    pub fn delete(&mut self, id: NodeId) -> Result<(), EditError> {
        self.require(id)?;
        self.push(Edit::Remove { id });
        Ok(())
    }

    pub fn set_param(&mut self, id: NodeId, param: &str, value: Value) -> Result<(), EditError> {
        let node = self.require(id)?;
        let ty = self
            .scene
            .registry()
            .get(&node.type_id)
            .ok_or_else(|| EditError::UnknownType(node.type_id.clone()))?;
        let desc = ty
            .find_param(param)
            .ok_or_else(|| EditError::UnknownParam(param.to_string()))?;
        let value = desc.validate(value)?;
        if let (ParamKind::Link { accepts }, Value::Link(Some(target))) = (&desc.kind, &value) {
            let target = *target;
            if target == id {
                return Err(EditError::InvalidLink(
                    "a node cannot link to itself".into(),
                ));
            }
            if !self.scene.contains(target) {
                return Err(EditError::InvalidLink(format!(
                    "node {target} does not exist"
                )));
            }
            let category = self.scene.node_type(target).map(|t| t.category);
            if !accepts.is_empty() && !category.is_some_and(|c| accepts.contains(&c)) {
                return Err(EditError::InvalidLink(format!(
                    "node {target} has category {category:?}, expected one of {accepts:?}"
                )));
            }
        }
        self.push(Edit::SetParam {
            id,
            param: param.to_string(),
            value,
        });
        Ok(())
    }

    /// Sets (`Some`) or removes (`None`) the track `key` of a node (docs/specs/ANIM.md §1.2).
    /// The key must address an animatable Float parameter or a component of an animatable
    /// Vec3 parameter of the node's type, and the track must validate.
    pub fn set_track(
        &mut self,
        id: NodeId,
        key: &str,
        track: Option<Track>,
    ) -> Result<(), EditError> {
        let node = self.require(id)?;
        if let Some(t) = &track {
            let ty = self
                .scene
                .registry()
                .get(&node.type_id)
                .ok_or_else(|| EditError::UnknownType(node.type_id.clone()))?;
            parse_target(key, |p| ty.find_param(p))?;
            t.validate()?;
        }
        self.push(Edit::SetTrack {
            id,
            key: key.to_string(),
            track,
        });
        Ok(())
    }

    /// Adds `key` to a track (creating it), replacing a key at exactly the same time.
    pub fn set_key(&mut self, id: NodeId, track_key: &str, key: Key) -> Result<(), EditError> {
        let mut track = self
            .require(id)?
            .tracks
            .get(track_key)
            .cloned()
            .unwrap_or_default();
        track.set_key(key);
        self.set_track(id, track_key, Some(track))
    }

    /// Removes the key at exactly `time`; the track disappears with its last key. Missing
    /// track or key: no change.
    pub fn remove_key(&mut self, id: NodeId, track_key: &str, time: f64) -> Result<(), EditError> {
        let Some(mut track) = self.require(id)?.tracks.get(track_key).cloned() else {
            return Ok(());
        };
        if !track.remove_key(time) {
            return Ok(());
        }
        let track = (!track.keys.is_empty()).then_some(track);
        self.set_track(id, track_key, track)
    }

    pub fn rename(&mut self, id: NodeId, name: &str) -> Result<(), EditError> {
        self.require(id)?;
        self.push(Edit::Rename {
            id,
            name: name.to_string(),
        });
        Ok(())
    }

    /// Moves a hierarchy node under `parent` (None = root). `index` counts positions
    /// after the node has been removed from its current list.
    pub fn move_node(
        &mut self,
        id: NodeId,
        parent: Option<NodeId>,
        index: Option<usize>,
    ) -> Result<(), EditError> {
        self.require_hierarchy(id)?;
        if let Some(p) = parent {
            self.require_hierarchy(p)?;
            if p == id || self.scene.descendants(id).contains(&p) {
                return Err(EditError::Cycle);
            }
        }
        let same_list = self.scene.node(id).expect("checked").parent == parent;
        let index = self.index_in(SlotOwner::Children(parent), index, usize::from(same_list))?;
        self.push(Edit::Move {
            id,
            slot: Slot::Child { parent, index },
        });
        Ok(())
    }
}

/// A scene plus its undo history; the only way to mutate a scene.
#[derive(Debug, Clone)]
pub struct Document {
    scene: Scene,
    history: History,
}

impl Document {
    pub fn new(registry: Arc<Registry>) -> Document {
        Document::from_scene(Scene::new(registry))
    }

    pub fn from_scene(scene: Scene) -> Document {
        Document {
            scene,
            history: History::default(),
        }
    }

    pub fn scene(&self) -> &Scene {
        &self.scene
    }

    pub fn history(&self) -> &History {
        &self.history
    }

    /// Runs `f` as one undo step. On error every edit is reverted and nothing is recorded.
    pub fn transact<T>(
        &mut self,
        label: &str,
        f: impl FnOnce(&mut Tx) -> Result<T, EditError>,
    ) -> Result<T, EditError> {
        self.run(label, None, f)
    }

    /// Like [`Document::transact`], but merges into the previous step if that step was
    /// recorded with the same `key` and nothing else happened since.
    pub fn transact_merged<T>(
        &mut self,
        label: &str,
        key: &str,
        f: impl FnOnce(&mut Tx) -> Result<T, EditError>,
    ) -> Result<T, EditError> {
        self.run(label, Some(key), f)
    }

    fn run<T>(
        &mut self,
        label: &str,
        key: Option<&str>,
        f: impl FnOnce(&mut Tx) -> Result<T, EditError>,
    ) -> Result<T, EditError> {
        let mut tx = Tx {
            scene: &mut self.scene,
            inverses: Vec::new(),
        };
        let result = f(&mut tx);
        let mut edits = std::mem::take(&mut tx.inverses);
        match result {
            Err(e) => {
                for edit in edits.into_iter().rev() {
                    apply(&mut self.scene, edit);
                }
                Err(e)
            }
            Ok(value) => {
                if edits.is_empty() {
                    return Ok(value);
                }
                self.history.redo.clear();
                let merge = key.is_some() && self.history.merge_key.as_deref() == key;
                match self.history.undo.last_mut() {
                    Some(last) if merge => last.edits.append(&mut edits),
                    _ => self.history.undo.push(Transaction {
                        label: label.to_string(),
                        edits,
                    }),
                }
                self.history.merge_key = key.map(str::to_string);
                Ok(value)
            }
        }
    }

    fn toggle(scene: &mut Scene, t: Transaction) -> Transaction {
        let edits = t.edits.into_iter().rev().map(|e| apply(scene, e)).collect();
        Transaction {
            label: t.label,
            edits,
        }
    }

    /// Returns false when there is nothing to undo.
    pub fn undo(&mut self) -> bool {
        self.history.merge_key = None;
        let Some(t) = self.history.undo.pop() else {
            return false;
        };
        let t = Self::toggle(&mut self.scene, t);
        self.history.redo.push(t);
        true
    }

    /// Returns false when there is nothing to redo.
    pub fn redo(&mut self) -> bool {
        self.history.merge_key = None;
        let Some(t) = self.history.redo.pop() else {
            return false;
        };
        let t = Self::toggle(&mut self.scene, t);
        self.history.undo.push(t);
        true
    }
}
