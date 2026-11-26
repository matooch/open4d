//! Parameter reading with the descriptor default as fallback.

use forge_core::{NodeId, Scene, Value};

fn value(scene: &Scene, id: NodeId, key: &str) -> Option<Value> {
    scene.param(id, key).cloned().or_else(|| {
        scene
            .node_type(id)
            .and_then(|t| t.find_param(key).map(|d| d.default.clone()))
    })
}

pub(crate) fn float(scene: &Scene, id: NodeId, key: &str) -> f64 {
    match value(scene, id, key) {
        Some(Value::Float(v)) => v,
        Some(Value::Int(i)) => i as f64,
        _ => 0.0,
    }
}

pub(crate) fn int(scene: &Scene, id: NodeId, key: &str) -> i64 {
    match value(scene, id, key) {
        Some(Value::Int(i)) => i,
        Some(Value::Float(v)) => v.round() as i64,
        _ => 0,
    }
}

pub(crate) fn boolean(scene: &Scene, id: NodeId, key: &str) -> bool {
    matches!(value(scene, id, key), Some(Value::Bool(true)))
}

pub(crate) fn choice(scene: &Scene, id: NodeId, key: &str) -> String {
    match value(scene, id, key) {
        Some(Value::Enum(s)) => s,
        _ => String::new(),
    }
}

pub(crate) fn links(scene: &Scene, id: NodeId, key: &str) -> Vec<NodeId> {
    match value(scene, id, key) {
        Some(Value::LinkList(l)) => l,
        _ => Vec::new(),
    }
}
