//! Evaluate live field-backed vertex map tags (FIELDS.md, brief 062).

use forge_core::tags::{VERTEX_MAP, restriction_weights_with, vertex_map_weights};
use forge_core::{EvalCtx, NodeId, Scene, Value};
use glam::DVec3;

use crate::FieldList;

/// Sample a vertex map at its host-local input points, or read its frozen buffer.
/// `None` means the tag is missing, disabled, or cannot be evaluated.
pub fn evaluate_vertex_map(
    scene: &Scene,
    tag: NodeId,
    points: &[DVec3],
    ctx: &EvalCtx,
) -> Option<Vec<f64>> {
    let node = scene.node(tag)?;
    if node.type_id != VERTEX_MAP || matches!(scene.param(tag, "enabled"), Some(Value::Bool(false)))
    {
        return None;
    }
    if !uses_fields(scene, tag) {
        let values = vertex_map_weights(scene, tag)?;
        return Some(
            (0..points.len())
                .map(|i| values.get(i).copied().unwrap_or(0.0))
                .collect(),
        );
    }
    let world = scene.world_matrix(node.host?)?;
    let list = FieldList::from_param(scene, tag, "fields").ok()?;
    if list.is_empty() {
        return Some(vec![0.0; points.len()]);
    }
    Some(
        points
            .iter()
            .map(|&p| {
                list.sample(world.transform_point3(p), ctx)
                    .value
                    .clamp(0.0, 1.0)
            })
            .collect(),
    )
}

fn uses_fields(scene: &Scene, tag: NodeId) -> bool {
    matches!(scene.param(tag, "use_fields"), Some(Value::Bool(true)))
}

/// Point restriction evaluated in the same context as the deformer.
pub fn restriction_weights(
    scene: &Scene,
    deformer: NodeId,
    target: NodeId,
    points: &[DVec3],
    ctx: &EvalCtx,
) -> Option<Vec<f64>> {
    restriction_weights_with(scene, deformer, target, points.len(), |tag| {
        if uses_fields(scene, tag) {
            evaluate_vertex_map(scene, tag, points, ctx)
        } else {
            None
        }
    })
}
