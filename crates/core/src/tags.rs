//! Selection, vertex map and restriction tags (docs/specs/EDITABLE.md §7, brief 027).
//!
//! The point and edge selection tags and the vertex map tag store data on an object; the
//! restriction tag, put on a deformer, names one of them and limits the deformer to it.
//! The polygon selection tag (`forge.polygon_selection`) is registered by forge-generators
//! (it predates this module) and is read here by its type id.

use crate::{Category, NodeId, NodeType, ParamDesc, Scene, Value};

/// A stored set of point indices.
pub const POINT_SELECTION: &str = "forge.point_selection";
/// A stored set of edge ids (as numbered by `PolyMesh::edges()`).
pub const EDGE_SELECTION: &str = "forge.edge_selection";
/// A stored set of polygon indices (registered by forge-generators).
pub const POLYGON_SELECTION: &str = "forge.polygon_selection";
/// One weight per point index.
pub const VERTEX_MAP: &str = "forge.vertex_map";
/// Limits the deformer it sits on to a stored selection or a vertex map of the deformed object.
pub const RESTRICTION: &str = "forge.restriction";

/// Shared row labels for field lists, including those hosted on built-in tags.
pub const FIELD_BLEND_OPTIONS: [(&str, &str); 9] = [
    ("normal", "Normal"),
    ("max", "Max"),
    ("min", "Min"),
    ("add", "Add"),
    ("subtract", "Subtract"),
    ("multiply", "Multiply"),
    ("overlay", "Overlay"),
    ("screen", "Screen"),
    ("clip", "Clip"),
];

/// The element kind a selection tag stores.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectionKind {
    Points,
    Edges,
    Polys,
}

impl SelectionKind {
    /// The tag type that stores this kind.
    pub fn tag_type(self) -> &'static str {
        match self {
            SelectionKind::Points => POINT_SELECTION,
            SelectionKind::Edges => EDGE_SELECTION,
            SelectionKind::Polys => POLYGON_SELECTION,
        }
    }

    /// The kind a tag type stores; `None` for other types.
    pub fn of_tag_type(type_id: &str) -> Option<SelectionKind> {
        match type_id {
            POINT_SELECTION => Some(SelectionKind::Points),
            EDGE_SELECTION => Some(SelectionKind::Edges),
            POLYGON_SELECTION => Some(SelectionKind::Polys),
            _ => None,
        }
    }
}

pub(crate) fn point_selection_type() -> NodeType {
    NodeType::new(POINT_SELECTION, "Point Selection", Category::Tag)
        .param(ParamDesc::index_list("indices", "Points"))
}

pub(crate) fn edge_selection_type() -> NodeType {
    NodeType::new(EDGE_SELECTION, "Edge Selection", Category::Tag)
        .param(ParamDesc::index_list("indices", "Edges"))
}

pub(crate) fn vertex_map_type() -> NodeType {
    NodeType::new(VERTEX_MAP, "Vertex Map", Category::Tag)
        .param(ParamDesc::bool("enabled", "Enable", true))
        .param(ParamDesc::color("color", "Color", [1.0, 1.0, 1.0]))
        .param(ParamDesc::bool("use_fields", "Use Fields", false))
        .param(ParamDesc::link_rows(
            "fields",
            "Fields",
            &[Category::Field],
            &FIELD_BLEND_OPTIONS,
        ))
        .param(ParamDesc::float_list("weights", "Weights").range(0.0, 1.0))
}

pub(crate) fn restriction_type() -> NodeType {
    NodeType::new(RESTRICTION, "Restriction", Category::Tag)
        .param(ParamDesc::bool("enabled", "Enable", true))
        .param(ParamDesc::string(
            "selection",
            "Point Selection / Vertex Map",
            "",
        ))
        .param(ParamDesc::bool("invert", "Invert", false))
}

/// The element indices a selection tag stores (`None` for other nodes).
pub fn selection_indices(scene: &Scene, tag: NodeId) -> Option<(SelectionKind, &[u32])> {
    let n = scene.node(tag)?;
    let kind = SelectionKind::of_tag_type(&n.type_id)?;
    match n.params.get("indices") {
        Some(Value::IndexList(l)) => Some((kind, l.as_slice())),
        _ => Some((kind, &[])),
    }
}

/// The weights of a vertex map tag (`None` for other nodes).
pub fn vertex_map_weights(scene: &Scene, tag: NodeId) -> Option<&[f64]> {
    let n = scene.node(tag)?;
    if n.type_id != VERTEX_MAP {
        return None;
    }
    match n.params.get("weights") {
        Some(Value::FloatList(l)) => Some(l.as_slice()),
        _ => Some(&[]),
    }
}

/// The weight of every point of `host`'s mesh for the point selection or vertex map tag named
/// `name` on `host`: 1 or 0 for a selection, the stored weight for a vertex map (0 past the
/// end of the list). The first tag with that name wins; a disabled vertex map and every other
/// tag type are skipped. `None` when no such tag exists.
pub fn named_point_weights(
    scene: &Scene,
    host: NodeId,
    name: &str,
    point_count: usize,
) -> Option<Vec<f64>> {
    named_point_weights_with(scene, host, name, point_count, |_| None)
}

fn named_point_weights_with(
    scene: &Scene,
    host: NodeId,
    name: &str,
    point_count: usize,
    mut resolve_live: impl FnMut(NodeId) -> Option<Vec<f64>>,
) -> Option<Vec<f64>> {
    for t in &scene.node(host)?.tags {
        let n = scene.node(*t)?;
        if n.name != name {
            continue;
        }
        match n.type_id.as_str() {
            POINT_SELECTION => {
                let mut w = vec![0.0; point_count];
                if let Some(Value::IndexList(l)) = n.params.get("indices") {
                    for i in l.iter().filter(|i| (**i as usize) < point_count) {
                        w[*i as usize] = 1.0;
                    }
                }
                return Some(w);
            }
            VERTEX_MAP if !matches!(n.params.get("enabled"), Some(Value::Bool(false))) => {
                if let Some(live) = resolve_live(*t) {
                    return Some(live);
                }
                let stored = vertex_map_weights(scene, *t).unwrap_or(&[]);
                return Some(
                    (0..point_count)
                        .map(|i| stored.get(i).copied().unwrap_or(0.0))
                        .collect(),
                );
            }
            _ => {}
        }
    }
    None
}

/// The per-point weights a deformer's restriction tag imposes on `target`'s points: the first
/// enabled restriction tag of `deformer` decides (EDITABLE.md §7.4); `None` when there is no
/// such tag, its name is empty, or the name matches no point selection or vertex map of
/// `target` (the deformer then acts on every point).
pub fn restriction_weights(
    scene: &Scene,
    deformer: NodeId,
    target: NodeId,
    point_count: usize,
) -> Option<Vec<f64>> {
    restriction_weights_with(scene, deformer, target, point_count, |_| None)
}

/// Resolve a live vertex map if the consumer has the points and field evaluator;
/// otherwise use the stored restriction data.
pub fn restriction_weights_with(
    scene: &Scene,
    deformer: NodeId,
    target: NodeId,
    point_count: usize,
    resolve_live: impl FnMut(NodeId) -> Option<Vec<f64>>,
) -> Option<Vec<f64>> {
    let tag = scene
        .node(deformer)?
        .tags
        .iter()
        .filter_map(|t| scene.node(*t))
        .find(|n| {
            n.type_id == RESTRICTION && !matches!(n.params.get("enabled"), Some(Value::Bool(false)))
        })?;
    let Some(Value::String(name)) = tag.params.get("selection") else {
        return None;
    };
    if name.is_empty() {
        return None;
    }
    let mut w = named_point_weights_with(scene, target, name, point_count, resolve_live)?;
    if matches!(tag.params.get("invert"), Some(Value::Bool(true))) {
        for x in &mut w {
            *x = 1.0 - *x;
        }
    }
    Some(w)
}
