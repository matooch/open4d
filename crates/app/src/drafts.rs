//! The draft layer: edits of animated parameters that wait for the key button
//! (docs/specs/UI.md §12 "Keyframe drafts", owner note N10).
//!
//! A draft is a value for one `(node, param)` shown in place of the animated value in the
//! viewport, the gizmo, the Attributes and Coordinates panels. It is never part of the
//! document: it is not saved, not undoable, not rendered and not exported. Anything that
//! changes what the draft was made against discards it (the scope below).

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::sync::Arc;

use forge_core::{EditError, NodeId, Scene, Value};

use crate::parameter_keys;

/// What a set of drafts was made against; a different scope discards them. The frame
/// (owner note N10 leaves this open, parity-pending G5), the scene revision (undo, redo,
/// any document edit, load) and Auto-Key. A selection change keeps them: the reference keeps
/// an unkeyed edit when another object is selected and the first is reselected (G4 item 2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scope {
    pub revision: u64,
    /// The node count: adding or removing a root node does not raise the revision.
    pub nodes: usize,
    pub frame: i64,
    pub autokey: bool,
}

/// One draft: the node, the parameter id (without a component suffix) and the whole value.
pub type Draft = (NodeId, String, Value);

#[derive(Default)]
pub struct DraftLayer {
    inner: RefCell<Inner>,
}

#[derive(Default)]
struct Inner {
    scope: Option<Scope>,
    values: BTreeMap<(NodeId, String), Value>,
    generation: u64,
    /// The document scene with the drafts applied, for the scope and generation it was
    /// made for.
    shown: Option<(u64, u64, Arc<Scene>)>,
}

impl Inner {
    fn enter(&mut self, scope: Scope) {
        if self.scope.as_ref() != Some(&scope) {
            if !self.values.is_empty() {
                self.generation += 1;
            }
            self.values.clear();
            self.scope = Some(scope);
        }
    }
}

impl DraftLayer {
    /// The drafts valid in `scope`, in node and parameter order.
    pub fn all(&self, scope: Scope) -> Vec<Draft> {
        let mut inner = self.inner.borrow_mut();
        inner.enter(scope);
        inner
            .values
            .iter()
            .map(|((node, param), value)| (*node, param.clone(), value.clone()))
            .collect()
    }

    pub fn get(&self, scope: Scope, node: NodeId, param: &str) -> Option<Value> {
        let mut inner = self.inner.borrow_mut();
        inner.enter(scope);
        inner.values.get(&(node, param.to_string())).cloned()
    }

    pub fn set(&self, scope: Scope, node: NodeId, param: &str, value: Value) {
        let mut inner = self.inner.borrow_mut();
        inner.enter(scope);
        if inner
            .values
            .insert((node, param.to_string()), value.clone())
            != Some(value)
        {
            inner.generation += 1;
        }
    }

    pub fn remove(&self, scope: Scope, node: NodeId, param: &str) {
        let mut inner = self.inner.borrow_mut();
        inner.enter(scope);
        if inner.values.remove(&(node, param.to_string())).is_some() {
            inner.generation += 1;
        }
    }

    pub fn clear(&self) {
        let mut inner = self.inner.borrow_mut();
        if !inner.values.is_empty() {
            inner.generation += 1;
        }
        inner.values.clear();
        inner.scope = None;
    }

    /// `doc` with the drafts of `scope` applied and the tracks of their parameters dropped
    /// (so animation and tags see the drafted value), or `None` without drafts. Cached
    /// until a draft or the document changes.
    pub fn shown(&self, scope: Scope, doc: &Scene) -> Option<Arc<Scene>> {
        let mut inner = self.inner.borrow_mut();
        inner.enter(scope);
        if inner.values.is_empty() {
            inner.shown = None;
            return None;
        }
        let key = (inner.generation, doc.revision());
        if let Some((generation, revision, scene)) = &inner.shown
            && (*generation, *revision) == key
        {
            return Some(scene.clone());
        }
        let drafts = inner.values.iter().map(|((node, param), value)| {
            let tracks = doc
                .param_desc(*node, param)
                .map(parameter_keys::tracks)
                .unwrap_or_default();
            (*node, param.clone(), value.clone(), tracks)
        });
        let scene = Arc::new(doc.with_drafts(drafts));
        inner.shown = Some((key.0, key.1, scene.clone()));
        Some(scene)
    }
}

/// True when `node` has a track on `param` (any component of a vector or colour).
pub fn has_tracks(scene: &Scene, node: NodeId, param: &str) -> bool {
    let (Some(desc), Some(n)) = (scene.param_desc(node, param), scene.node(node)) else {
        return false;
    };
    parameter_keys::tracks(desc)
        .iter()
        .any(|track| n.tracks.contains_key(track))
}

/// Validates `value` for the parameter like `set_param` does (range clamp, type).
pub fn validated(
    scene: &Scene,
    node: NodeId,
    param: &str,
    value: Value,
) -> Result<Value, EditError> {
    let desc = scene
        .param_desc(node, param)
        .ok_or_else(|| EditError::UnknownParam(param.to_string()))?;
    Ok(desc.validate(value)?)
}
