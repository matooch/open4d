//! Tag priorities and the rig stage of the frame (docs/specs/RIG.md §12).

use forge_core::{NodeId, ParamDesc, Scene, Value};

use crate::{constraint, ik};

/// The priority groups in run order.
pub const GROUPS: [(&str, &str); 5] = [
    ("initial", "Initial"),
    ("animation", "Animation"),
    ("expression", "Expression"),
    ("dynamics", "Dynamics"),
    ("generators", "Generators"),
];

/// The two params every ordered tag has.
pub(crate) fn priority_params() -> [ParamDesc; 2] {
    [
        ParamDesc::enumeration("priority_group", "Priority group", "expression", &GROUPS),
        ParamDesc::int("priority", "Priority", 0).range(-1000, 1000),
    ]
}

/// True for the IK and constraint tag types.
pub fn is_rig_tag(scene: &Scene, id: NodeId) -> bool {
    scene
        .node(id)
        .is_some_and(|n| n.type_id == ik::IK_TAG || n.type_id == constraint::CONSTRAINT_TAG)
}

/// `(group rank, number)` of a tag; tags without the params (expression tags) are
/// `(expression, 0)`.
pub fn priority_key(scene: &Scene, id: NodeId) -> (usize, i64) {
    let group = match scene.param(id, "priority_group") {
        Some(Value::Enum(g)) => GROUPS.iter().position(|(k, _)| k == g).unwrap_or(2),
        _ => 2,
    };
    let number = match scene.param(id, "priority") {
        Some(Value::Int(n)) => *n,
        _ => 0,
    };
    (group, number)
}

/// The tags of the given types in document order: an object, its tags, then its children.
pub fn document_tags(scene: &Scene, types: &[&str]) -> Vec<NodeId> {
    let mut out = Vec::new();
    let mut stack: Vec<NodeId> = scene.roots().iter().rev().copied().collect();
    while let Some(id) = stack.pop() {
        let Some(node) = scene.node(id) else { continue };
        out.extend(node.tags.iter().copied().filter(|t| {
            scene
                .node(*t)
                .is_some_and(|n| types.contains(&n.type_id.as_str()))
        }));
        stack.extend(node.children.iter().rev());
    }
    out
}

/// `ids` (in document order) sorted by priority; ties keep document order.
pub fn run_order(scene: &Scene, ids: &[NodeId]) -> Vec<NodeId> {
    let mut v: Vec<NodeId> = ids.to_vec();
    v.sort_by_key(|id| priority_key(scene, *id));
    v
}

/// The (node, param) writes of one rig tag, as overrides.
pub fn tag_writes(scene: &Scene, tag: NodeId) -> Vec<(NodeId, String, Value)> {
    if matches!(scene.param(tag, "enabled"), Some(Value::Bool(false)))
        || !forge_core::layers::expressions_on(scene, tag)
    {
        return Vec::new();
    }
    match scene.node(tag).map(|n| n.type_id.as_str()) {
        Some(ik::IK_TAG) => ik::solve_tag(scene, tag),
        Some(constraint::CONSTRAINT_TAG) => constraint::evaluate_tag(scene, tag),
        _ => Vec::new(),
    }
}

/// Applies one rig tag to `scene`; returns the pairs it wrote.
pub fn apply_tag(scene: &mut Scene, tag: NodeId) -> Vec<(NodeId, String)> {
    let writes = tag_writes(scene, tag);
    let touched = writes.iter().map(|(n, p, _)| (*n, p.clone())).collect();
    scene.apply_params(writes);
    touched
}

/// All rig tags of the scene in priority order (no expression tags).
pub fn apply_rig_tags(mut scene: Scene) -> Scene {
    let ids = document_tags(&scene, &[ik::IK_TAG, constraint::CONSTRAINT_TAG]);
    for id in run_order(&scene, &ids) {
        apply_tag(&mut scene, id);
    }
    scene
}
