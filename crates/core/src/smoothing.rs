//! The smoothing tag (docs/specs/SMOOTHING.md §2; observed in docs/parity/smoothing.md).

use std::f64::consts::PI;

use crate::{Category, NodeId, NodeType, ParamDesc, Scene, Unit, Value};

pub const TAG: &str = "forge.smoothing";

pub(crate) fn node_type() -> NodeType {
    NodeType::new(TAG, "Smoothing", Category::Tag)
        .param(ParamDesc::bool("angle_limit", "Angle limit", true))
        .param(
            ParamDesc::float("angle", "Smoothing angle", 40f64.to_radians())
                .range(0.0, PI)
                .unit(Unit::Angle),
        )
        .param(ParamDesc::bool("edge_breaks", "Edge breaks", true))
}

/// The node's smoothing angle from its first smoothing tag: the angle when the
/// angle limit is on, π when off; `None` (faceted) without a tag.
pub fn smoothing_angle(scene: &Scene, id: NodeId) -> Option<f64> {
    let tag = scene
        .node(id)?
        .tags
        .iter()
        .filter_map(|t| scene.node(*t))
        .find(|t| t.type_id == TAG)?;
    match (tag.params.get("angle_limit"), tag.params.get("angle")) {
        (Some(Value::Bool(false)), _) => Some(PI),
        (_, Some(Value::Float(a))) => Some(*a),
        _ => Some(40f64.to_radians()),
    }
}
