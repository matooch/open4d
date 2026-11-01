//! mesh.loop_cut and mesh.knife: the interactive cutting tools as commands with explicit
//! numeric arguments (docs/specs/MODELING.md §5.4).

use std::sync::Arc;

use forge_core::{NodeId, Value};
use forge_generators::{
    POLYGON, SELECTION_EDGES, SELECTION_POINTS, SELECTION_POLYS, from_poly_mesh, to_poly_mesh,
};
use forge_geometry::selection::IndexSet;
use forge_geometry::{Edge, PolyMesh};
use forge_modeling::{
    CutOutput, KnifeLine, Plane, View, even_offsets, knife_line, knife_plane, loop_cut,
};
use glam::DVec3;
use serde_json::{Map, Value as Json, json};

use crate::context::CommandContext;
use crate::error::CommandError;
use crate::registry::{CommandHandler, CommandRegistry};

pub(super) fn register(r: &mut CommandRegistry) -> Result<(), CommandError> {
    r.register(Box::new(LoopCut))?;
    r.register(Box::new(Knife))
}

struct LoopCut;
struct Knife;

fn vec3_schema(what: &str) -> Json {
    json!({"description": what, "type": "array", "items": {"type": "number"},
           "minItems": 3, "maxItems": 3})
}

fn edge_schema() -> Json {
    json!({"description": "Edge id of the mesh, or a point pair [a, b] of an existing edge.",
           "anyOf": [
               {"type": "integer", "minimum": 0},
               {"type": "array", "items": {"type": "integer", "minimum": 0},
                "minItems": 2, "maxItems": 2}]})
}

fn node_schema() -> Json {
    json!({"description": "Polygon object; the active node when absent.",
           "type": ["integer", "null"]})
}

/// Checks the keys of `obj` against `allowed`.
fn check_keys(id: &str, obj: &Map<String, Json>, allowed: &[&str]) -> Result<(), CommandError> {
    match obj.keys().find(|k| !allowed.contains(&k.as_str())) {
        Some(k) => Err(CommandError::InvalidArgs(format!("{id}: unknown key {k}"))),
        None => Ok(()),
    }
}

fn number(id: &str, v: &Json, what: &str) -> Result<f64, CommandError> {
    v.as_f64()
        .filter(|f| f.is_finite())
        .ok_or_else(|| CommandError::InvalidArgs(format!("{id}: {what} must be a number")))
}

fn vec3(id: &str, obj: &Map<String, Json>, key: &str) -> Result<Option<DVec3>, CommandError> {
    match obj.get(key) {
        None | Some(Json::Null) => Ok(None),
        Some(Json::Array(a)) if a.len() == 3 => {
            let mut v = [0.0; 3];
            for (slot, item) in v.iter_mut().zip(a) {
                *slot = number(id, item, key)?;
            }
            Ok(Some(DVec3::from_array(v)))
        }
        Some(_) => Err(CommandError::InvalidArgs(format!(
            "{id}: {key} must be three numbers"
        ))),
    }
}

fn required(id: &str, v: Option<DVec3>, key: &str) -> Result<DVec3, CommandError> {
    v.ok_or_else(|| CommandError::InvalidArgs(format!("{id}: {key} is required")))
}

/// The polygon object and its mesh.
fn target(
    ctx: &dyn CommandContext,
    id: &str,
    obj: &Map<String, Json>,
) -> Result<(NodeId, PolyMesh), CommandError> {
    let node = match obj.get("node") {
        Some(Json::Number(n)) => NodeId(
            n.as_u64()
                .ok_or_else(|| CommandError::InvalidArgs(format!("{id}: bad node id")))?,
        ),
        None | Some(Json::Null) => ctx
            .selection()
            .active
            .ok_or_else(|| CommandError::Failed(format!("{id}: select a polygon object")))?,
        Some(_) => return Err(CommandError::InvalidArgs(format!("{id}: bad node id"))),
    };
    let scene = ctx.doc().scene();
    let is_polygon = scene.node(node).is_some_and(|n| n.type_id == POLYGON);
    match scene.param(node, "mesh") {
        Some(Value::Mesh(m)) if is_polygon => Ok((node, to_poly_mesh(m))),
        _ => Err(CommandError::Failed(format!(
            "{id}: not a polygon object (use Make Editable first)"
        ))),
    }
}

/// An edge id from an id or a point pair.
fn edge_arg(id: &str, mesh: &PolyMesh, obj: &Map<String, Json>) -> Result<u32, CommandError> {
    let index = |v: &Json| v.as_u64().and_then(|i| u32::try_from(i).ok());
    let bad = || CommandError::InvalidArgs(format!("{id}: edge must be an id or [a, b]"));
    match obj.get("edge") {
        Some(Json::Array(p)) if p.len() == 2 => {
            let (a, b) = (index(&p[0]).ok_or_else(bad)?, index(&p[1]).ok_or_else(bad)?);
            mesh.edge_ids()
                .get(&Edge::new(a, b))
                .copied()
                .ok_or_else(|| {
                    CommandError::Failed(format!("{id}: points {a} and {b} share no edge"))
                })
        }
        Some(v) => index(v).ok_or_else(bad),
        None => Err(CommandError::InvalidArgs(format!("{id}: edge is required"))),
    }
}

/// Writes the cut mesh and selects the created edges, in one undo step.
fn commit(
    ctx: &mut dyn CommandContext,
    label: &str,
    node: NodeId,
    out: &CutOutput,
) -> Result<Json, CommandError> {
    let cut_edges = out.new_edge_ids();
    let data = Arc::new(from_poly_mesh(&out.mesh));
    let sel_edges = cut_edges.clone();
    ctx.doc_mut().transact(label, |tx| {
        tx.set_param(node, "mesh", Value::Mesh(data))?;
        tx.set_param(node, SELECTION_POLYS, Value::IndexList(Vec::new()))?;
        tx.set_param(node, SELECTION_POINTS, Value::IndexList(Vec::new()))?;
        tx.set_param(node, SELECTION_EDGES, Value::IndexList(sel_edges))?;
        Ok(())
    })?;
    Ok(json!({"node": node.0, "points": out.mesh.points.len(),
              "polys": out.mesh.polys.len(), "cut_edges": cut_edges}))
}

impl CommandHandler for LoopCut {
    fn id(&self) -> &str {
        "mesh.loop_cut"
    }

    fn label(&self) -> &str {
        "Loop Cut"
    }

    fn args_schema(&self) -> Json {
        json!({"type": "object", "additionalProperties": false,
               "required": ["edge"],
               "properties": {
                   "node": node_schema(),
                   "edge": edge_schema(),
                   "offsets": {"description": "Cut positions as fractions (0..1, exclusive) along the edge, which runs from its lower to its higher point.",
                               "type": ["array", "null"],
                               "items": {"type": "number", "exclusiveMinimum": 0, "exclusiveMaximum": 1}},
                   "count": {"description": "Evenly spaced cuts instead of offsets (default 1 when neither is given).",
                             "type": ["integer", "null"], "minimum": 1, "maximum": 64}}})
    }

    fn run(&self, ctx: &mut dyn CommandContext, args: Json) -> Result<Json, CommandError> {
        let id = "mesh.loop_cut";
        let args = if args.is_null() { json!({}) } else { args };
        let obj = args.as_object().ok_or_else(|| {
            CommandError::InvalidArgs(format!("{id}: arguments must be an object"))
        })?;
        check_keys(id, obj, &["node", "edge", "offsets", "count"])?;
        let (node, mesh) = target(ctx, id, obj)?;
        let edge = edge_arg(id, &mesh, obj)?;
        let given = |k: &str| !matches!(obj.get(k), None | Some(Json::Null));
        let offsets: Vec<f64> = match (given("offsets"), given("count")) {
            (true, true) => {
                return Err(CommandError::InvalidArgs(format!(
                    "{id}: give offsets or count, not both"
                )));
            }
            (true, false) => match &obj["offsets"] {
                Json::Array(a) => a
                    .iter()
                    .map(|v| number(id, v, "offsets"))
                    .collect::<Result<_, _>>()?,
                _ => return Err(CommandError::InvalidArgs(format!("{id}: bad offsets"))),
            },
            (false, c) => {
                let n = if c {
                    obj["count"].as_u64().filter(|n| (1..=64).contains(n))
                } else {
                    Some(1)
                };
                even_offsets(
                    n.ok_or_else(|| CommandError::InvalidArgs(format!("{id}: count is 1..64")))?
                        as usize,
                )
            }
        };
        let out = loop_cut(&mesh, edge, &offsets)
            .map_err(|e| CommandError::Failed(format!("Loop Cut: {e}")))?;
        commit(ctx, "Loop Cut", node, &out)
    }

    fn undoable(&self) -> bool {
        true
    }

    fn sample_args(&self) -> Json {
        json!({"edge": 0, "offsets": [0.5]})
    }
}

impl CommandHandler for Knife {
    fn id(&self) -> &str {
        "mesh.knife"
    }

    fn label(&self) -> &str {
        "Knife"
    }

    fn args_schema(&self) -> Json {
        json!({"type": "object", "additionalProperties": false,
               "required": ["mode"],
               "properties": {
                   "node": node_schema(),
                   "mode": {"type": "string", "enum": ["line", "plane", "loop"]},
                   "polys": {"description": "Polygons to cut (plane and line modes); all when absent.",
                             "type": ["array", "null"],
                             "items": {"type": "integer", "minimum": 0}},
                   "point": vec3_schema("plane mode: a point of the plane (object space)"),
                   "normal": vec3_schema("plane mode: the plane normal"),
                   "from": vec3_schema("line mode: start of the drawn segment (object space)"),
                   "to": vec3_schema("line mode: end of the drawn segment"),
                   "eye": vec3_schema("line mode: the viewer's position (perspective); or direction"),
                   "direction": vec3_schema("line mode: the view direction (parallel view); or eye"),
                   "visible_only": {"description": "line mode: skip polygons facing away (default true).",
                                    "type": "boolean"},
                   "edge": edge_schema(),
                   "offset": {"description": "loop mode: position along the edge (default 0.5).",
                              "type": "number", "exclusiveMinimum": 0, "exclusiveMaximum": 1}}})
    }

    fn run(&self, ctx: &mut dyn CommandContext, args: Json) -> Result<Json, CommandError> {
        let id = "mesh.knife";
        let obj = args.as_object().ok_or_else(|| {
            CommandError::InvalidArgs(format!("{id}: arguments must be an object"))
        })?;
        check_keys(
            id,
            obj,
            &[
                "node",
                "mode",
                "polys",
                "point",
                "normal",
                "from",
                "to",
                "eye",
                "direction",
                "visible_only",
                "edge",
                "offset",
            ],
        )?;
        let (node, mesh) = target(ctx, id, obj)?;
        let polys: IndexSet = match obj.get("polys") {
            None | Some(Json::Null) => IndexSet::new(),
            Some(Json::Array(a)) => a
                .iter()
                .map(|v| {
                    v.as_u64()
                        .and_then(|i| u32::try_from(i).ok())
                        .ok_or_else(|| CommandError::InvalidArgs(format!("{id}: bad polys")))
                })
                .collect::<Result<_, _>>()?,
            Some(_) => return Err(CommandError::InvalidArgs(format!("{id}: bad polys"))),
        };
        let fail = |e: forge_modeling::ToolError| CommandError::Failed(format!("Knife: {e}"));
        let out = match obj.get("mode").and_then(Json::as_str) {
            Some("plane") => {
                let plane = Plane {
                    point: required(id, vec3(id, obj, "point")?, "point")?,
                    normal: required(id, vec3(id, obj, "normal")?, "normal")?,
                };
                knife_plane(&mesh, &polys, plane).map_err(fail)?
            }
            Some("line") => {
                let view = match (vec3(id, obj, "eye")?, vec3(id, obj, "direction")?) {
                    (Some(e), None) => View::Eye(e),
                    (None, Some(d)) => View::Direction(d),
                    _ => {
                        return Err(CommandError::InvalidArgs(format!(
                            "{id}: line mode needs exactly one of eye and direction"
                        )));
                    }
                };
                let line = KnifeLine {
                    from: required(id, vec3(id, obj, "from")?, "from")?,
                    to: required(id, vec3(id, obj, "to")?, "to")?,
                    view,
                    visible_only: obj.get("visible_only").map_or(Ok(true), |v| {
                        v.as_bool().ok_or_else(|| {
                            CommandError::InvalidArgs(format!("{id}: visible_only is a boolean"))
                        })
                    })?,
                };
                knife_line(&mesh, &polys, &line).map_err(fail)?
            }
            Some("loop") => {
                let edge = edge_arg(id, &mesh, obj)?;
                let offset = match obj.get("offset") {
                    None | Some(Json::Null) => 0.5,
                    Some(v) => number(id, v, "offset")?,
                };
                loop_cut(&mesh, edge, &[offset]).map_err(fail)?
            }
            _ => {
                return Err(CommandError::InvalidArgs(format!(
                    "{id}: mode must be line, plane or loop"
                )));
            }
        };
        commit(ctx, "Knife", node, &out)
    }

    fn undoable(&self) -> bool {
        true
    }

    fn sample_args(&self) -> Json {
        json!({"mode": "plane", "point": [0.0, 0.0, 0.0], "normal": [1.0, 0.0, 0.0]})
    }
}
