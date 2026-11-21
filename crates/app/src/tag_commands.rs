//! `select.store`, `select.restore`, `vertexmap.*` and `restriction.set`: stored selections,
//! vertex maps and restriction tags as commands (docs/specs/EDITABLE.md section 7).

use forge_command::{CommandContext, CommandError, CommandRegistry, CommandSpec, ModeArg};
use forge_core::tags::{self, SelectionKind};
use forge_core::{NodeId, Value};
use forge_generators::{POLYGON, SELECTION_EDGES, SELECTION_POINTS, SELECTION_POLYS};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value as Json, json};

#[derive(Deserialize, JsonSchema, Clone, Copy, PartialEq)]
#[serde(rename_all = "snake_case")]
enum KindArg {
    Points,
    Edges,
    Polys,
}

impl From<KindArg> for SelectionKind {
    fn from(k: KindArg) -> SelectionKind {
        match k {
            KindArg::Points => SelectionKind::Points,
            KindArg::Edges => SelectionKind::Edges,
            KindArg::Polys => SelectionKind::Polys,
        }
    }
}

#[derive(Deserialize, JsonSchema, Clone, Copy, Default, PartialEq)]
#[serde(rename_all = "snake_case")]
enum OpArg {
    #[default]
    Replace,
    Add,
    Subtract,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct StoreArgs {
    /// Default: the current component mode.
    kind: Option<KindArg>,
    /// A polygon object (default: the active node).
    object: Option<u64>,
    /// Tag name (default: the tag type's label).
    name: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct RestoreArgs {
    /// A point, edge or polygon selection tag.
    tag: u64,
    op: Option<OpArg>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct MapCreateArgs {
    object: Option<u64>,
    name: Option<String>,
    /// Weight of every point not given by `weights` (default 0).
    value: Option<f64>,
    weights: Option<Vec<f64>>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct MapSetArgs {
    /// A vertex map tag.
    tag: u64,
    /// The whole weight list.
    weights: Option<Vec<f64>>,
    /// Points that take `value`.
    points: Option<Vec<u32>>,
    value: Option<f64>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct RestrictArgs {
    /// A deformer (default: the active node).
    deformer: Option<u64>,
    /// Name of a point selection or vertex map tag of the deformed object.
    selection: String,
    invert: Option<bool>,
}

/// Registers the commands.
pub fn register(r: &mut CommandRegistry) -> Result<(), CommandError> {
    r.register_fn(
        CommandSpec {
            id: "select.store",
            label: "Set Selection",
            undoable: true,
            sample: || json!({"kind": "points"}),
        },
        store,
    )?;
    r.register_fn(
        CommandSpec {
            id: "select.restore",
            label: "Restore Selection",
            undoable: true,
            sample: || json!({"tag": 1}),
        },
        restore,
    )?;
    r.register_fn(
        CommandSpec {
            id: "vertexmap.create",
            label: "Create Vertex Map",
            undoable: true,
            sample: || json!({"value": 0.0}),
        },
        map_create,
    )?;
    r.register_fn(
        CommandSpec {
            id: "vertexmap.set",
            label: "Set Vertex Map Weights",
            undoable: true,
            sample: || json!({"tag": 1, "points": [0], "value": 1.0}),
        },
        map_set,
    )?;
    r.register_fn(
        CommandSpec {
            id: "restriction.set",
            label: "Restrict Deformer",
            undoable: true,
            sample: || json!({"selection": "Vertex Map"}),
        },
        restrict,
    )
}

fn node_arg(ctx: &dyn CommandContext, id: Option<u64>, what: &str) -> Result<NodeId, CommandError> {
    let n = match id {
        Some(i) => NodeId(i),
        None => ctx
            .selection()
            .active
            .ok_or_else(|| CommandError::failed(format!("select {what}")))?,
    };
    if ctx.doc().scene().contains(n) {
        Ok(n)
    } else {
        Err(CommandError::failed(format!("unknown node {}", n.0)))
    }
}

fn polygon_object(ctx: &dyn CommandContext, object: Option<u64>) -> Result<NodeId, CommandError> {
    let n = node_arg(ctx, object, "a polygon object")?;
    match ctx.doc().scene().node(n) {
        Some(node) if node.type_id == POLYGON => Ok(n),
        _ => Err(CommandError::failed("select a polygon object")),
    }
}

fn set_param(kind: SelectionKind) -> &'static str {
    match kind {
        SelectionKind::Points => SELECTION_POINTS,
        SelectionKind::Edges => SELECTION_EDGES,
        SelectionKind::Polys => SELECTION_POLYS,
    }
}

fn stored(ctx: &dyn CommandContext, node: NodeId, kind: SelectionKind) -> Vec<u32> {
    match ctx.doc().scene().param(node, set_param(kind)) {
        Some(Value::IndexList(l)) => l.clone(),
        _ => Vec::new(),
    }
}

fn point_count(ctx: &dyn CommandContext, node: NodeId) -> usize {
    match ctx.doc().scene().param(node, "mesh") {
        Some(Value::Mesh(m)) => m.points.len(),
        _ => 0,
    }
}

fn store(ctx: &mut dyn CommandContext, a: StoreArgs) -> Result<Json, CommandError> {
    let node = polygon_object(ctx, a.object)?;
    let kind = match a.kind {
        Some(k) => k.into(),
        None => match ctx.edit_mode() {
            ModeArg::Points => SelectionKind::Points,
            ModeArg::Edges => SelectionKind::Edges,
            ModeArg::Polys => SelectionKind::Polys,
            ModeArg::Model => {
                return Err(CommandError::failed(
                    "select.store needs a component mode or a kind",
                ));
            }
        },
    };
    let indices = stored(ctx, node, kind);
    let tag = ctx.doc_mut().transact("Store Selection", |tx| {
        let tag = tx.attach_tag(kind.tag_type(), node, None)?;
        if let Some(name) = &a.name {
            tx.rename(tag, name)?;
        }
        tx.set_param(tag, "indices", Value::IndexList(indices))?;
        Ok(tag)
    })?;
    Ok(json!({"tag": tag.0}))
}

fn restore(ctx: &mut dyn CommandContext, a: RestoreArgs) -> Result<Json, CommandError> {
    let tag = NodeId(a.tag);
    let (kind, indices) = tags::selection_indices(ctx.doc().scene(), tag)
        .map(|(k, l)| (k, l.to_vec()))
        .ok_or_else(|| CommandError::failed(format!("node {} is not a selection tag", a.tag)))?;
    let host = ctx
        .doc()
        .scene()
        .node(tag)
        .and_then(|n| n.host)
        .ok_or_else(|| CommandError::failed("the tag has no host"))?;
    let host = polygon_object(ctx, Some(host.0))?;
    let current = stored(ctx, host, kind);
    let next: Vec<u32> = match a.op.unwrap_or_default() {
        OpArg::Replace => indices,
        OpArg::Add => {
            let mut v = current.clone();
            v.extend(indices);
            v
        }
        OpArg::Subtract => current
            .iter()
            .copied()
            .filter(|i| !indices.contains(i))
            .collect(),
    };
    let mut next = next;
    next.sort_unstable();
    next.dedup();
    let count = next.len();
    if next != current {
        ctx.doc_mut().transact("Select", |tx| {
            tx.set_param(host, set_param(kind), Value::IndexList(next))
        })?;
    }
    ctx.set_edit_mode(match kind {
        SelectionKind::Points => ModeArg::Points,
        SelectionKind::Edges => ModeArg::Edges,
        SelectionKind::Polys => ModeArg::Polys,
    });
    let kind_name = match kind {
        SelectionKind::Points => "points",
        SelectionKind::Edges => "edges",
        SelectionKind::Polys => "polys",
    };
    Ok(json!({"kind": kind_name, "count": count}))
}

fn weight(v: f64) -> Result<f64, CommandError> {
    if v.is_finite() {
        Ok(v.clamp(0.0, 1.0))
    } else {
        Err(CommandError::InvalidArgs("weights must be finite".into()))
    }
}

fn map_create(ctx: &mut dyn CommandContext, a: MapCreateArgs) -> Result<Json, CommandError> {
    let node = polygon_object(ctx, a.object)?;
    let n = point_count(ctx, node);
    let fill = weight(a.value.unwrap_or(0.0))?;
    let mut w = a.weights.unwrap_or_default();
    if w.len() > n {
        return Err(CommandError::InvalidArgs(format!(
            "{} weights for {n} points",
            w.len()
        )));
    }
    for x in &mut w {
        *x = weight(*x)?;
    }
    w.resize(n, fill);
    let tag = ctx.doc_mut().transact("Create Vertex Map", |tx| {
        let tag = tx.attach_tag(tags::VERTEX_MAP, node, None)?;
        if let Some(name) = &a.name {
            tx.rename(tag, name)?;
        }
        tx.set_param(tag, "weights", Value::FloatList(w))?;
        Ok(tag)
    })?;
    Ok(json!({"tag": tag.0}))
}

fn map_set(ctx: &mut dyn CommandContext, a: MapSetArgs) -> Result<Json, CommandError> {
    let tag = NodeId(a.tag);
    let mut w = tags::vertex_map_weights(ctx.doc().scene(), tag)
        .map(<[f64]>::to_vec)
        .ok_or_else(|| CommandError::failed(format!("node {} is not a vertex map", a.tag)))?;
    match (a.weights, a.points) {
        (Some(_), Some(_)) | (None, None) => {
            return Err(CommandError::InvalidArgs(
                "give either weights or points with value".into(),
            ));
        }
        (Some(list), None) => {
            w = list.into_iter().map(weight).collect::<Result<_, _>>()?;
        }
        (None, Some(points)) => {
            let v = weight(a.value.unwrap_or(1.0))?;
            for p in points {
                let p = p as usize;
                if p >= w.len() {
                    w.resize(p + 1, 0.0);
                }
                w[p] = v;
            }
        }
    }
    let count = w.len();
    ctx.doc_mut().transact("Set Vertex Map Weights", |tx| {
        tx.set_param(tag, "weights", Value::FloatList(w))
    })?;
    Ok(json!({"count": count}))
}

fn restrict(ctx: &mut dyn CommandContext, a: RestrictArgs) -> Result<Json, CommandError> {
    let deformer = node_arg(ctx, a.deformer, "a deformer")?;
    let is_deformer = ctx
        .doc()
        .scene()
        .node_type(deformer)
        .is_some_and(|t| t.category == forge_core::Category::Deformer);
    if !is_deformer {
        return Err(CommandError::failed("select a deformer"));
    }
    let existing = ctx
        .doc()
        .scene()
        .node(deformer)
        .map(|n| n.tags.clone())
        .unwrap_or_default()
        .into_iter()
        .find(|t| {
            ctx.doc()
                .scene()
                .node(*t)
                .is_some_and(|n| n.type_id == tags::RESTRICTION)
        });
    let tag = ctx.doc_mut().transact("Restrict Deformer", |tx| {
        let tag = match existing {
            Some(t) => t,
            None => tx.attach_tag(tags::RESTRICTION, deformer, None)?,
        };
        tx.set_param(tag, "selection", Value::String(a.selection.clone()))?;
        if let Some(inv) = a.invert {
            tx.set_param(tag, "invert", Value::Bool(inv))?;
        }
        Ok(tag)
    })?;
    Ok(json!({"tag": tag.0}))
}
