//! Revision-keyed evaluation cache (docs/specs/CORE.md §5).

use std::collections::HashMap;
use std::sync::Arc;

use crate::{NodeId, Scene};

/// Caches one computed value per node, valid while the node's deep revision is unchanged.
#[derive(Debug)]
pub struct EvalCache<T> {
    entries: HashMap<NodeId, (u64, Arc<T>)>,
}

impl<T> Default for EvalCache<T> {
    fn default() -> Self {
        EvalCache {
            entries: HashMap::new(),
        }
    }
}

impl<T> EvalCache<T> {
    pub fn new() -> Self {
        Self::default()
    }

    /// Cached value for `id`, recomputed with `compute` if the node changed.
    /// `None` if the node does not exist.
    pub fn get_or_compute(
        &mut self,
        scene: &Scene,
        id: NodeId,
        compute: impl FnOnce(&Scene, NodeId) -> T,
    ) -> Option<Arc<T>> {
        let rev = scene.deep_revision(id)?;
        if let Some((cached_rev, value)) = self.entries.get(&id)
            && *cached_rev == rev
        {
            return Some(Arc::clone(value));
        }
        let value = Arc::new(compute(scene, id));
        self.entries.insert(id, (rev, Arc::clone(&value)));
        Some(value)
    }

    /// Drops entries of nodes that no longer exist.
    pub fn purge(&mut self, scene: &Scene) {
        self.entries.retain(|id, _| scene.contains(*id));
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}
