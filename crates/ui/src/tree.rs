//! Object tree rows (docs/specs/UI.md §3).

use std::collections::BTreeSet;

use forge_core::{NodeId, Scene};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreeRow {
    pub id: NodeId,
    pub depth: usize,
    pub name: String,
    pub type_label: String,
    pub has_children: bool,
    pub expanded: bool,
    /// `(tag id, tag type label)`, in order.
    pub tags: Vec<(NodeId, String)>,
    pub missing_type: bool,
}

/// Label of a node's type, and whether the type is missing from the registry.
pub fn type_label(scene: &Scene, id: NodeId) -> (String, bool) {
    match (scene.node_type(id), scene.node(id)) {
        (Some(t), _) => (t.label.clone(), false),
        (None, Some(n)) => (n.type_id.clone(), true),
        (None, None) => (String::new(), true),
    }
}

/// Visible rows in depth-first order; children of collapsed nodes are skipped.
pub fn tree_rows(scene: &Scene, expanded: &BTreeSet<NodeId>) -> Vec<TreeRow> {
    let mut out = Vec::new();
    let mut stack: Vec<(NodeId, usize)> = scene.roots().iter().rev().map(|r| (*r, 0)).collect();
    while let Some((id, depth)) = stack.pop() {
        let Some(n) = scene.node(id) else { continue };
        let open = expanded.contains(&id);
        let (label, missing) = type_label(scene, id);
        out.push(TreeRow {
            id,
            depth,
            name: n.name.clone(),
            type_label: label,
            has_children: !n.children.is_empty(),
            expanded: open,
            tags: n
                .tags
                .iter()
                .map(|t| (*t, type_label(scene, *t).0))
                .collect(),
            missing_type: missing,
        });
        if open {
            stack.extend(n.children.iter().rev().map(|c| (*c, depth + 1)));
        }
    }
    out
}
