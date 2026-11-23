//! Host-local clone selection and initial weights (clone-toolset spec O5).

use forge_core::{Category, NodeId, NodeType, ParamDesc, Scene, Value};

use crate::CloneData;

pub const CLONE_SELECTION: &str = "forge.clone_selection";
pub const CLONE_WEIGHT: &str = "forge.clone_weight";

pub(crate) fn selection_type() -> NodeType {
    NodeType::new(CLONE_SELECTION, "Clone Selection", Category::Tag)
        .param(ParamDesc::index_list("indices", "Indices"))
}

pub(crate) fn weight_type() -> NodeType {
    NodeType::new(CLONE_WEIGHT, "Clone Weight", Category::Tag)
        .param(ParamDesc::float_list("weights", "Weights").range(0.0, 1.0))
}

pub(crate) fn apply_weights(scene: &Scene, host: NodeId, data: &mut CloneData) {
    let Some(node) = scene.node(host) else { return };
    let tag = node
        .tags
        .iter()
        .filter_map(|id| scene.node(*id))
        .find(|tag| tag.type_id == CLONE_WEIGHT);
    if let Some(Value::FloatList(weights)) = tag.and_then(|tag| tag.params.get("weights")) {
        for (dst, src) in data.weights.iter_mut().zip(weights) {
            *dst = *src;
        }
    }
}

pub(crate) fn selection(scene: &Scene, host: NodeId, effector: NodeId) -> Option<&[u32]> {
    let Value::String(name) = scene.param(effector, "selection")? else {
        return None;
    };
    if name.is_empty() {
        return None;
    }
    let tag = scene
        .node(host)?
        .tags
        .iter()
        .filter_map(|id| scene.node(*id))
        .find(|tag| tag.type_id == CLONE_SELECTION && tag.name == *name)?;
    let Value::IndexList(indices) = tag.params.get("indices")? else {
        return None;
    };
    Some(indices)
}
