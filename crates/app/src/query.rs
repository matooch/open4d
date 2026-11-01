//! JSON views of a scene for the CLI and the RPC server (docs/specs/CLI.md `info`,
//! docs/specs/RPC.md §4): parameters in user units, tags, animated tracks, children.

use forge_core::{NodeId, Scene, Unit, Value};
use serde_json::{Map, json};

type Json = serde_json::Value;

/// A stored value in the units the commands use (degrees, percent).
pub fn display_value(value: &Value, unit: Unit) -> Json {
    let scale = |v: f64| match unit {
        Unit::Angle => v.to_degrees(),
        Unit::Percent => v * 100.0,
        _ => v,
    };
    match value {
        Value::Bool(b) => json!(b),
        Value::Int(i) => json!(i),
        Value::Float(f) => json!(scale(*f)),
        Value::Vec3(v) => json!(v.map(scale)),
        Value::Color(c) => json!(c),
        Value::Enum(s) | Value::String(s) => json!(s),
        Value::Link(l) => json!(l.map(|n| n.0)),
        Value::Mesh(m) => json!({"points": m.points.len(), "polys": m.polys.len()}),
        Value::IndexList(i) => json!(i),
    }
}

pub fn node_json(scene: &Scene, id: NodeId) -> Json {
    let Some(node) = scene.node(id) else {
        return Json::Null;
    };
    let descs = scene
        .node_type(id)
        .map(|t| t.params.as_slice())
        .unwrap_or(&[]);
    let mut params = Map::new();
    for (name, value) in &node.params {
        let unit = descs
            .iter()
            .find(|d| &d.id == name)
            .map(|d| d.ui.unit)
            .unwrap_or_default();
        params.insert(name.clone(), display_value(value, unit));
    }
    let mut out = Map::new();
    out.insert("id".into(), json!(id.0));
    out.insert("type".into(), json!(node.type_id));
    out.insert("name".into(), json!(node.name));
    out.insert("params".into(), Json::Object(params));
    if !node.tags.is_empty() {
        let tags: Vec<Json> = node.tags.iter().map(|t| node_json(scene, *t)).collect();
        out.insert("tags".into(), json!(tags));
    }
    if !node.tracks.is_empty() {
        out.insert(
            "animated".into(),
            json!(node.tracks.keys().collect::<Vec<_>>()),
        );
    }
    if !node.is_tag() {
        let children: Vec<Json> = node.children.iter().map(|c| node_json(scene, *c)).collect();
        out.insert("children".into(), json!(children));
    }
    Json::Object(out)
}
