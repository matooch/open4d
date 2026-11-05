//! spline.create, spline.edit_points, spline.insert_point, spline.delete_points,
//! spline.set_closed and spline.set_type: editing the points, tangents, segments and
//! interpolation type of an editable spline (docs/specs/SPLINES.md §5).

use forge_core::{EditError, NodeId, Tx, Value};
use forge_generators::editable_spline::{SplineData, SplineType, TYPE, data_of};
use glam::DVec3;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value as Json, json};

use super::{spec, target_nodes};
use crate::context::CommandContext;
use crate::error::CommandError;
use crate::registry::CommandRegistry;

/// The interpolation types (SPLINES.md §1.1).
#[derive(Deserialize, JsonSchema, Clone, Copy, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
enum TypeArg {
    Linear,
    Cubic,
    Akima,
    Bspline,
    #[default]
    Bezier,
}

impl TypeArg {
    fn id(self) -> &'static str {
        match self {
            TypeArg::Linear => SplineType::Linear,
            TypeArg::Cubic => SplineType::Cubic,
            TypeArg::Akima => SplineType::Akima,
            TypeArg::Bspline => SplineType::BSpline,
            TypeArg::Bezier => SplineType::Bezier,
        }
        .id()
    }
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct CreateArgs {
    /// The points `[x, y, z]`, at least two.
    points: Vec<[f64; 3]>,
    /// Join the last point to the first (default false).
    closed: Option<bool>,
    /// Interpolation type (default `bezier`; all handles start at zero).
    #[serde(rename = "type")]
    kind: Option<TypeArg>,
    /// Object name (default "Spline").
    name: Option<String>,
    /// Parent node; the top level when absent.
    parent: Option<u64>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct PointEdit {
    /// Index into the spline's point list.
    index: usize,
    /// New position (local space).
    position: Option<[f64; 3]>,
    /// New left (incoming) handle, an offset from the point.
    left: Option<[f64; 3]>,
    /// New right (outgoing) handle, an offset from the point.
    right: Option<[f64; 3]>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct EditArgs {
    /// The editable spline.
    node: u64,
    /// At least one edit; each sets at least one of position, left, right.
    edits: Vec<PointEdit>,
    /// Edits with the same gesture number on the same spline merge into one undo step.
    gesture: Option<u64>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct InsertArgs {
    /// The editable spline.
    node: u64,
    /// Where the point ends up in the list, 0 ..= the number of points.
    index: usize,
    position: [f64; 3],
    /// Left handle (default zero).
    left: Option<[f64; 3]>,
    /// Right handle (default zero).
    right: Option<[f64; 3]>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct DeleteArgs {
    /// The editable spline.
    node: u64,
    /// Point indices to remove; a segment keeps at least two points.
    indices: Vec<usize>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct ClosedArgs {
    /// The editable spline.
    node: u64,
    /// Segment index (default 0).
    segment: Option<usize>,
    closed: bool,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct TypeArgs {
    /// Editable splines; the current selection when absent.
    nodes: Option<Vec<u64>>,
    #[serde(rename = "type")]
    kind: TypeArg,
}

pub(super) fn register(r: &mut CommandRegistry) -> Result<(), CommandError> {
    r.register_fn(
        spec(
            "spline.create",
            "Create Spline",
            true,
            || json!({"points": [[0, 0, 0], [100, 0, 0], [100, 100, 0]], "closed": false}),
        ),
        create,
    )?;
    r.register_fn(
        spec(
            "spline.edit_points",
            "Edit Spline Points",
            true,
            || json!({"node": 1, "edits": [{"index": 0, "position": [10, 0, 0]}]}),
        ),
        edit_points,
    )?;
    r.register_fn(
        spec(
            "spline.insert_point",
            "Insert Spline Point",
            true,
            || json!({"node": 1, "index": 1, "position": [50, 0, 0]}),
        ),
        insert_point,
    )?;
    r.register_fn(
        spec(
            "spline.delete_points",
            "Delete Spline Points",
            true,
            || json!({"node": 1, "indices": [1]}),
        ),
        delete_points,
    )?;
    r.register_fn(
        spec(
            "spline.set_closed",
            "Close Spline",
            true,
            || json!({"node": 1, "closed": true}),
        ),
        set_closed,
    )?;
    r.register_fn(
        spec(
            "spline.set_type",
            "Set Spline Type",
            true,
            || json!({"nodes": [1], "type": "akima"}),
        ),
        set_type,
    )?;
    Ok(())
}

fn vec(v: [f64; 3]) -> DVec3 {
    DVec3::from_array(v)
}

fn finite(v: &[f64; 3]) -> Result<(), CommandError> {
    if v.iter().all(|c| c.is_finite()) {
        Ok(())
    } else {
        Err(CommandError::InvalidArgs(format!("{v:?} is not finite")))
    }
}

/// The editable spline `node` and its data.
fn load(ctx: &dyn CommandContext, node: u64) -> Result<(NodeId, SplineData), CommandError> {
    let id = NodeId(node);
    let scene = ctx.doc().scene();
    let n = scene
        .node(id)
        .ok_or_else(|| CommandError::Failed(format!("no node {node}")))?;
    if n.type_id != TYPE {
        return Err(CommandError::Failed(format!(
            "{} is not an editable spline; make it editable first (mesh.make_editable)",
            n.name
        )));
    }
    let data = data_of(scene, id).map_err(|e| CommandError::Failed(e.to_string()))?;
    Ok((id, data))
}

fn store(tx: &mut Tx, id: NodeId, data: &SplineData) -> Result<(), EditError> {
    for (k, v) in data.to_values() {
        tx.set_param(id, &k, v)?;
    }
    Ok(())
}

/// Writes `data` as one undo step, merged with the previous one when `key` is given.
fn commit(
    ctx: &mut dyn CommandContext,
    label: &str,
    key: Option<String>,
    id: NodeId,
    data: &SplineData,
) -> Result<(), CommandError> {
    let write = |tx: &mut Tx| store(tx, id, data);
    match key {
        Some(k) => ctx.doc_mut().transact_merged(label, &k, write)?,
        None => ctx.doc_mut().transact(label, write)?,
    };
    Ok(())
}

fn create(ctx: &mut dyn CommandContext, a: CreateArgs) -> Result<Json, CommandError> {
    if a.points.len() < 2 {
        return Err(CommandError::InvalidArgs(
            "a spline needs at least two points".into(),
        ));
    }
    a.points.iter().try_for_each(finite)?;
    let data = SplineData::from_points(
        a.points.iter().copied().map(vec).collect(),
        a.closed.unwrap_or(false),
    );
    let kind = a.kind.unwrap_or_default();
    let id = ctx.doc_mut().transact("Create Spline", |tx| {
        let id = tx.create(TYPE, a.parent.map(NodeId), None)?;
        if let Some(name) = &a.name {
            tx.rename(id, name)?;
        }
        store(tx, id, &data)?;
        tx.set_param(id, "type", Value::Enum(kind.id().into()))?;
        Ok(id)
    })?;
    ctx.selection_mut().click(id);
    Ok(json!({"node": id.0}))
}

fn edit_points(ctx: &mut dyn CommandContext, a: EditArgs) -> Result<Json, CommandError> {
    if a.edits.is_empty() {
        return Err(CommandError::InvalidArgs("edits is empty".into()));
    }
    let (id, mut data) = load(ctx, a.node)?;
    for e in &a.edits {
        if e.index >= data.points.len() {
            return Err(CommandError::InvalidArgs(format!(
                "index {} is out of range ({} points)",
                e.index,
                data.points.len()
            )));
        }
        if e.position.is_none() && e.left.is_none() && e.right.is_none() {
            return Err(CommandError::InvalidArgs(format!(
                "the edit of point {} sets nothing",
                e.index
            )));
        }
        for v in [&e.position, &e.left, &e.right].into_iter().flatten() {
            finite(v)?;
        }
        if let Some(p) = e.position {
            data.points[e.index] = vec(p);
        }
        if let Some(l) = e.left {
            data.left[e.index] = vec(l);
        }
        if let Some(r) = e.right {
            data.right[e.index] = vec(r);
        }
    }
    let key = a.gesture.map(|g| format!("spline:{}:{g}", a.node));
    commit(ctx, "Edit Spline Points", key, id, &data)?;
    Ok(json!({"changed": a.edits.len()}))
}

fn insert_point(ctx: &mut dyn CommandContext, a: InsertArgs) -> Result<Json, CommandError> {
    let (id, mut data) = load(ctx, a.node)?;
    finite(&a.position)?;
    let handle = |h: Option<[f64; 3]>| h.map_or(Ok(DVec3::ZERO), |h| finite(&h).map(|()| vec(h)));
    data.insert(a.index, vec(a.position), handle(a.left)?, handle(a.right)?)
        .map_err(CommandError::InvalidArgs)?;
    commit(ctx, "Insert Spline Point", None, id, &data)?;
    Ok(json!({"index": a.index}))
}

fn delete_points(ctx: &mut dyn CommandContext, a: DeleteArgs) -> Result<Json, CommandError> {
    if a.indices.is_empty() {
        return Err(CommandError::InvalidArgs("indices is empty".into()));
    }
    let (id, mut data) = load(ctx, a.node)?;
    data.delete(&a.indices).map_err(CommandError::Failed)?;
    commit(ctx, "Delete Spline Points", None, id, &data)?;
    Ok(json!({"deleted": a.indices.len()}))
}

fn set_closed(ctx: &mut dyn CommandContext, a: ClosedArgs) -> Result<Json, CommandError> {
    let (id, mut data) = load(ctx, a.node)?;
    let seg = a.segment.unwrap_or(0);
    let count = data.segments.len();
    data.segments
        .get_mut(seg)
        .ok_or_else(|| {
            CommandError::InvalidArgs(format!("segment {seg} is out of range ({count} segments)"))
        })?
        .closed = a.closed;
    let label = if a.closed {
        "Close Spline"
    } else {
        "Open Spline"
    };
    commit(ctx, label, None, id, &data)?;
    Ok(Json::Null)
}

fn set_type(ctx: &mut dyn CommandContext, a: TypeArgs) -> Result<Json, CommandError> {
    let ids: Vec<NodeId> = target_nodes(ctx, a.nodes)
        .into_iter()
        .filter(|id| {
            ctx.doc()
                .scene()
                .node(*id)
                .is_some_and(|n| n.type_id == TYPE)
        })
        .collect();
    if ids.is_empty() {
        return Err(CommandError::Failed(
            "select an editable spline first (mesh.make_editable makes one)".into(),
        ));
    }
    ctx.doc_mut().transact("Set Spline Type", |tx| {
        ids.iter()
            .try_for_each(|id| tx.set_param(*id, "type", Value::Enum(a.kind.id().into())))
    })?;
    Ok(json!({"changed": ids.len()}))
}
