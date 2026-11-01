//! mesh.bridge, mesh.close_hole, mesh.stitch and mesh.pen: the topology tools as commands with
//! explicit indices and numbers (docs/specs/MODELING.md §6.6).

use std::sync::Arc;

use forge_core::{NodeId, Value};
use forge_generators::{SELECTION_EDGES, SELECTION_POINTS, SELECTION_POLYS, from_poly_mesh};
use forge_modeling::{
    BridgeEnd, BuildOutput, MAX_SEGMENTS, PenCorner, StitchTo, ToolError, bridge, close_hole, pen,
    stitch, stitch_edges,
};
use serde_json::{Map, Value as Json, json};

use super::cutting::{check_keys, edge_schema, edge_value, node_schema, target, vec3};
use crate::context::CommandContext;
use crate::error::CommandError;
use crate::registry::{CommandHandler, CommandRegistry};

pub(super) fn register(r: &mut CommandRegistry) -> Result<(), CommandError> {
    r.register(Box::new(Bridge))?;
    r.register(Box::new(CloseHole))?;
    r.register(Box::new(Stitch))?;
    r.register(Box::new(Pen))
}

struct Bridge;
struct CloseHole;
struct Stitch;
struct Pen;

fn args_object<'a>(id: &str, args: &'a Json) -> Result<&'a Map<String, Json>, CommandError> {
    args.as_object()
        .ok_or_else(|| CommandError::InvalidArgs(format!("{id}: arguments must be an object")))
}

fn index(id: &str, v: &Json, what: &str) -> Result<u32, CommandError> {
    v.as_u64()
        .and_then(|i| u32::try_from(i).ok())
        .ok_or_else(|| CommandError::InvalidArgs(format!("{id}: {what} must be an index")))
}

fn given(obj: &Map<String, Json>, key: &str) -> bool {
    !matches!(obj.get(key), None | Some(Json::Null))
}

fn failed(label: &str, e: ToolError) -> CommandError {
    CommandError::Failed(format!("{label}: {e}"))
}

/// Writes the mesh and selects the new polygons (or, without any, the new points) in one undo
/// step; the edge selection is cleared.
fn commit(
    ctx: &mut dyn CommandContext,
    label: &str,
    node: NodeId,
    out: &BuildOutput,
) -> Result<(), CommandError> {
    let data = Arc::new(from_poly_mesh(&out.mesh));
    let (polys, points) = (out.new_polys.clone(), out.new_points.clone());
    let points = if polys.is_empty() { points } else { Vec::new() };
    ctx.doc_mut().transact(label, |tx| {
        tx.set_param(node, "mesh", Value::Mesh(data))?;
        tx.set_param(node, SELECTION_POLYS, Value::IndexList(polys))?;
        tx.set_param(node, SELECTION_POINTS, Value::IndexList(points))?;
        tx.set_param(node, SELECTION_EDGES, Value::IndexList(Vec::new()))?;
        Ok(())
    })?;
    Ok(())
}

fn summary(node: NodeId, out: &BuildOutput) -> Json {
    json!({"node": node.0, "points": out.mesh.points.len(), "polys": out.mesh.polys.len(),
           "new_polys": out.new_polys, "new_points": out.new_points})
}

impl CommandHandler for Bridge {
    fn id(&self) -> &str {
        "mesh.bridge"
    }

    fn label(&self) -> &str {
        "Bridge"
    }

    fn args_schema(&self) -> Json {
        json!({"type": "object", "additionalProperties": false,
               "properties": {
                   "node": node_schema(),
                   "edges": {"description": "Two border edges (id or point pair); each stands for its whole border loop. Give edges or polys.",
                             "type": ["array", "null"], "items": edge_schema(),
                             "minItems": 2, "maxItems": 2},
                   "polys": {"description": "Two polygons of the same size: they are removed and their outlines bridged. Give edges or polys.",
                             "type": ["array", "null"],
                             "items": {"type": "integer", "minimum": 0},
                             "minItems": 2, "maxItems": 2},
                   "segments": {"description": "Rows of quads between the loops (default 1).",
                                "type": ["integer", "null"], "minimum": 1, "maximum": MAX_SEGMENTS}}})
    }

    fn run(&self, ctx: &mut dyn CommandContext, args: Json) -> Result<Json, CommandError> {
        let id = "mesh.bridge";
        let args = if args.is_null() { json!({}) } else { args };
        let obj = args_object(id, &args)?;
        check_keys(id, obj, &["node", "edges", "polys", "segments"])?;
        let (node, mesh) = target(ctx, id, obj)?;
        let two = |key: &str| -> Result<[&Json; 2], CommandError> {
            match obj.get(key) {
                Some(Json::Array(a)) if a.len() == 2 => Ok([&a[0], &a[1]]),
                _ => Err(CommandError::InvalidArgs(format!(
                    "{id}: {key} must have two entries"
                ))),
            }
        };
        let ends = match (given(obj, "edges"), given(obj, "polys")) {
            (true, false) => {
                let [a, b] = two("edges")?;
                [
                    BridgeEnd::Edge(edge_value(id, &mesh, a)?),
                    BridgeEnd::Edge(edge_value(id, &mesh, b)?),
                ]
            }
            (false, true) => {
                let [a, b] = two("polys")?;
                [
                    BridgeEnd::Poly(index(id, a, "polys")?),
                    BridgeEnd::Poly(index(id, b, "polys")?),
                ]
            }
            _ => {
                return Err(CommandError::InvalidArgs(format!(
                    "{id}: give exactly one of edges and polys"
                )));
            }
        };
        let segments = match obj.get("segments") {
            None | Some(Json::Null) => 1,
            Some(v) => v
                .as_u64()
                .filter(|n| (1..=MAX_SEGMENTS as u64).contains(n))
                .ok_or_else(|| CommandError::InvalidArgs(format!("{id}: segments is 1..64")))?
                as usize,
        };
        let out = bridge(&mesh, ends[0], ends[1], segments).map_err(|e| failed("Bridge", e))?;
        commit(ctx, "Bridge", node, &out)?;
        Ok(summary(node, &out))
    }

    fn undoable(&self) -> bool {
        true
    }

    fn sample_args(&self) -> Json {
        json!({"polys": [0, 1], "segments": 1})
    }
}

impl CommandHandler for CloseHole {
    fn id(&self) -> &str {
        "mesh.close_hole"
    }

    fn label(&self) -> &str {
        "Close Polygon Hole"
    }

    fn args_schema(&self) -> Json {
        json!({"type": "object", "additionalProperties": false,
               "required": ["edge"],
               "properties": {"node": node_schema(), "edge": edge_schema()}})
    }

    fn run(&self, ctx: &mut dyn CommandContext, args: Json) -> Result<Json, CommandError> {
        let id = "mesh.close_hole";
        let obj = args_object(id, &args)?;
        check_keys(id, obj, &["node", "edge"])?;
        let (node, mesh) = target(ctx, id, obj)?;
        let edge = match obj.get("edge") {
            Some(v) => edge_value(id, &mesh, v)?,
            None => {
                return Err(CommandError::InvalidArgs(format!("{id}: edge is required")));
            }
        };
        let out = close_hole(&mesh, edge).map_err(|e| failed("Close Polygon Hole", e))?;
        commit(ctx, "Close Polygon Hole", node, &out)?;
        Ok(summary(node, &out))
    }

    fn undoable(&self) -> bool {
        true
    }

    fn sample_args(&self) -> Json {
        json!({"edge": 0})
    }
}

impl CommandHandler for Stitch {
    fn id(&self) -> &str {
        "mesh.stitch"
    }

    fn label(&self) -> &str {
        "Stitch and Sew"
    }

    fn args_schema(&self) -> Json {
        json!({"type": "object", "additionalProperties": false,
               "properties": {
                   "node": node_schema(),
                   "points": {"description": "Point pairs [a, b] to merge (into the lowest index of each set). Give points or edges.",
                              "type": ["array", "null"],
                              "items": {"type": "array", "items": {"type": "integer", "minimum": 0},
                                        "minItems": 2, "maxItems": 2}},
                   "edges": {"description": "Pairs of border edges [e1, e2] (id or point pair) to sew together. Give points or edges.",
                             "type": ["array", "null"],
                             "items": {"type": "array", "items": edge_schema(),
                                       "minItems": 2, "maxItems": 2}},
                   "position": {"description": "Where merged points go (default middle).",
                                "type": ["string", "null"], "enum": ["middle", "first", null]}}})
    }

    fn run(&self, ctx: &mut dyn CommandContext, args: Json) -> Result<Json, CommandError> {
        let id = "mesh.stitch";
        let args = if args.is_null() { json!({}) } else { args };
        let obj = args_object(id, &args)?;
        check_keys(id, obj, &["node", "points", "edges", "position"])?;
        let (node, mesh) = target(ctx, id, obj)?;
        let to = match obj.get("position").and_then(Json::as_str) {
            None | Some("middle") => StitchTo::Middle,
            Some("first") => StitchTo::First,
            Some(_) => {
                return Err(CommandError::InvalidArgs(format!(
                    "{id}: position is middle or first"
                )));
            }
        };
        let list = |key: &str| -> Result<&Vec<Json>, CommandError> {
            match obj.get(key) {
                Some(Json::Array(a)) => Ok(a),
                _ => Err(CommandError::InvalidArgs(format!("{id}: bad {key}"))),
            }
        };
        let pair = |v: &Json| -> Result<[Json; 2], CommandError> {
            match v {
                Json::Array(p) if p.len() == 2 => Ok([p[0].clone(), p[1].clone()]),
                _ => Err(CommandError::InvalidArgs(format!(
                    "{id}: each pair has two entries"
                ))),
            }
        };
        let pairs: Vec<(u32, u32)> = match (given(obj, "points"), given(obj, "edges")) {
            (true, false) => list("points")?
                .iter()
                .map(|v| {
                    let [a, b] = pair(v)?;
                    Ok((index(id, &a, "points")?, index(id, &b, "points")?))
                })
                .collect::<Result<_, CommandError>>()?,
            (false, true) => {
                let edge_pairs: Vec<(u32, u32)> = list("edges")?
                    .iter()
                    .map(|v| {
                        let [a, b] = pair(v)?;
                        Ok((edge_value(id, &mesh, &a)?, edge_value(id, &mesh, &b)?))
                    })
                    .collect::<Result<_, CommandError>>()?;
                stitch_edges(&mesh, &edge_pairs).map_err(|e| failed("Stitch and Sew", e))?
            }
            _ => {
                return Err(CommandError::InvalidArgs(format!(
                    "{id}: give exactly one of points and edges"
                )));
            }
        };
        let out = stitch(&mesh, &pairs, to).map_err(|e| failed("Stitch and Sew", e))?;
        commit(ctx, "Stitch and Sew", node, &out)?;
        let mut result = summary(node, &out);
        result["merged"] = json!(out.new_points);
        Ok(result)
    }

    fn undoable(&self) -> bool {
        true
    }

    fn sample_args(&self) -> Json {
        json!({"points": [[0, 1]], "position": "middle"})
    }
}

impl CommandHandler for Pen {
    fn id(&self) -> &str {
        "mesh.pen"
    }

    fn label(&self) -> &str {
        "Polygon Pen"
    }

    fn args_schema(&self) -> Json {
        json!({"type": "object", "additionalProperties": false,
               "required": ["corners"],
               "properties": {
                   "node": node_schema(),
                   "corners": {"description": "One corner adds a point, three or more add a polygon. A corner is {point: index} (an existing point) or {pos: [x, y, z]} (a new point, object space).",
                               "type": "array", "minItems": 1,
                               "items": {"anyOf": [
                                   {"type": "object", "additionalProperties": false,
                                    "required": ["point"],
                                    "properties": {"point": {"type": "integer", "minimum": 0}}},
                                   {"type": "object", "additionalProperties": false,
                                    "required": ["pos"],
                                    "properties": {"pos": {"type": "array", "items": {"type": "number"},
                                                           "minItems": 3, "maxItems": 3}}}]}}}})
    }

    fn run(&self, ctx: &mut dyn CommandContext, args: Json) -> Result<Json, CommandError> {
        let id = "mesh.pen";
        let obj = args_object(id, &args)?;
        check_keys(id, obj, &["node", "corners"])?;
        let (node, mesh) = target(ctx, id, obj)?;
        let Some(Json::Array(list)) = obj.get("corners") else {
            return Err(CommandError::InvalidArgs(format!(
                "{id}: corners is required"
            )));
        };
        let mut corners = Vec::new();
        for c in list {
            let o = c.as_object().ok_or_else(|| {
                CommandError::InvalidArgs(format!("{id}: a corner is {{point}} or {{pos}}"))
            })?;
            check_keys(id, o, &["point", "pos"])?;
            corners.push(match (o.get("point"), o.get("pos")) {
                (Some(p), None) => PenCorner::Point(index(id, p, "point")?),
                (None, Some(_)) => {
                    let pos = vec3(id, o, "pos")?.ok_or_else(|| {
                        CommandError::InvalidArgs(format!("{id}: pos must be three numbers"))
                    })?;
                    PenCorner::New(pos)
                }
                _ => {
                    return Err(CommandError::InvalidArgs(format!(
                        "{id}: a corner has exactly one of point and pos"
                    )));
                }
            });
        }
        let out = pen(&mesh, &corners).map_err(|e| failed("Polygon Pen", e))?;
        commit(ctx, "Polygon Pen", node, &out)?;
        Ok(summary(node, &out))
    }

    fn undoable(&self) -> bool {
        true
    }

    fn sample_args(&self) -> Json {
        json!({"corners": [{"pos": [0.0, 0.0, 0.0]}, {"pos": [1.0, 0.0, 0.0]}, {"pos": [0.0, 1.0, 0.0]}]})
    }
}
