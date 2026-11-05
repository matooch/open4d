//! mesh.extrude, mesh.extrude_inner: modeling tools as commands (docs/specs/MODELING.md §0.1).

use std::sync::Arc;

use forge_core::{NodeId, Value};
use forge_generators::{
    POLYGON, SELECTION_EDGES, SELECTION_POINTS, SELECTION_POLYS, from_poly_mesh, to_poly_mesh,
};
use forge_geometry::selection::{IndexSet, Selection};
use forge_geometry::{Edge, PolyMesh};
use forge_modeling::{Settings, ToolDesc, array_descs, cutting_descs, tool_descs, tool_fn};
use serde_json::{Map, Value as Json, json};

use crate::context::CommandContext;
use crate::error::CommandError;
use crate::registry::{CommandHandler, CommandRegistry};

struct ToolCommand {
    /// The registry id (`desc.command`).
    command: &'static str,
    desc: ToolDesc,
    apply: forge_modeling::ToolFn,
}

pub(super) fn register(r: &mut CommandRegistry) -> Result<(), CommandError> {
    for desc in tool_descs()
        .into_iter()
        .chain(cutting_descs())
        .chain(array_descs())
    {
        let apply: forge_modeling::ToolFn = tool_fn(desc.id)
            .ok_or_else(|| CommandError::Failed(format!("tool {} has no function", desc.id)))?;
        r.register(Box::new(ToolCommand {
            command: desc.command,
            desc,
            apply,
        }))?;
    }
    Ok(())
}

/// Edge ids from the `edges` argument: an id, or a point pair `[a, b]` of an existing edge.
fn edge_ids(mesh: &PolyMesh, items: &[Json]) -> Result<IndexSet, String> {
    let ids = mesh.edge_ids();
    let index = |v: &Json| v.as_u64().and_then(|i| u32::try_from(i).ok());
    let mut out = IndexSet::new();
    for item in items {
        let id = match item {
            Json::Array(pair) if pair.len() == 2 => {
                let (Some(a), Some(b)) = (index(&pair[0]), index(&pair[1])) else {
                    return Err("bad edge pair".into());
                };
                *ids.get(&Edge::new(a, b))
                    .ok_or_else(|| format!("points {a} and {b} do not share an edge"))?
            }
            other => index(other).ok_or("bad edge id")?,
        };
        out.insert(id);
    }
    Ok(out)
}

/// Center Axis (MODELING.md §4.11): the position moves by the object-space `c` (through the
/// object's rotation and scale); direct children move back by `c` so nothing moves in the world.
fn center_object(
    tx: &mut forge_core::Tx,
    node: NodeId,
    c: glam::DVec3,
) -> Result<(), forge_core::EditError> {
    use forge_core::transform::POS;
    let scene = tx.scene();
    let t = scene
        .local_transform(node)
        .ok_or(forge_core::EditError::UnknownNode(node))?;
    let pos = t.pos + t.matrix().transform_vector3(c);
    let children: Vec<(NodeId, glam::DVec3)> = scene
        .node(node)
        .map(|n| n.children.clone())
        .unwrap_or_default()
        .into_iter()
        .filter_map(|ch| Some((ch, scene.local_transform(ch)?.pos - c)))
        .collect();
    tx.set_param(node, POS, Value::Vec3(pos.to_array()))?;
    for (ch, p) in children {
        tx.set_param(ch, POS, Value::Vec3(p.to_array()))?;
    }
    Ok(())
}

pub(super) fn setting_schema(p: &forge_core::ParamDesc) -> Json {
    match &p.default {
        Value::Bool(b) => json!({"type": "boolean", "default": b}),
        Value::Int(i) => json!({"type": "integer", "default": i}),
        Value::Float(f) => json!({"type": "number", "default": f}),
        Value::Enum(e) => {
            let options: Vec<String> = match &p.kind {
                forge_core::ParamKind::Enum { options } => {
                    options.iter().map(|o| o.id.clone()).collect()
                }
                _ => Vec::new(),
            };
            json!({"type": "string", "enum": options, "default": e})
        }
        Value::Vec3(v) => json!({
            "type": "array",
            "items": {"type": "number"},
            "minItems": 3,
            "maxItems": 3,
            "default": v
        }),
        _ => json!({}),
    }
}

/// One object's result of a tool run, before anything is written.
struct Outcome {
    node: NodeId,
    mesh: PolyMesh,
    selection: Selection,
    /// The copy `mesh.split` spawns as a new object.
    split: Option<PolyMesh>,
    /// The shift of the axis `mesh.center_axis` makes (object space).
    moved: Option<glam::DVec3>,
}

fn chosen_is_empty(sel: &Selection) -> bool {
    sel.polys.is_empty() && sel.points.is_empty() && sel.edges.is_empty()
}

/// The polygon objects a mesh command works on: `node`, `nodes`, or the selected polygon
/// objects when the active node is one (EDITABLE.md §6.5). Each id is checked by the caller.
pub(super) fn polygon_targets(
    ctx: &dyn CommandContext,
    id: &str,
    label: &str,
    obj: &Map<String, Json>,
) -> Result<Vec<NodeId>, CommandError> {
    let bad = || CommandError::InvalidArgs(format!("{id}: bad node id"));
    let one = |v: &Json| v.as_u64().map(NodeId).ok_or_else(bad);
    let given = |key: &str| obj.get(key).filter(|v| !v.is_null());
    match (given("node"), given("nodes")) {
        (Some(_), Some(_)) => Err(CommandError::InvalidArgs(format!(
            "{id}: give node or nodes, not both"
        ))),
        (Some(n), None) => Ok(vec![one(n)?]),
        (None, Some(Json::Array(list))) => {
            let mut out: Vec<NodeId> = Vec::new();
            for v in list {
                let n = one(v)?;
                if !out.contains(&n) {
                    out.push(n);
                }
            }
            if out.is_empty() {
                return Err(CommandError::InvalidArgs(format!("{id}: nodes is empty")));
            }
            Ok(out)
        }
        (None, Some(_)) => Err(CommandError::InvalidArgs(format!(
            "{id}: nodes must be a list of ids"
        ))),
        (None, None) => {
            let active = ctx
                .selection()
                .active
                .ok_or_else(|| CommandError::Failed(format!("{label}: select a polygon object")))?;
            let scene = ctx.doc().scene();
            let is_polygon = |n: NodeId| scene.node(n).is_some_and(|x| x.type_id == POLYGON);
            if !is_polygon(active) {
                return Ok(vec![active]);
            }
            let mut out: std::collections::BTreeSet<NodeId> = ctx
                .selection()
                .nodes
                .iter()
                .copied()
                .filter(|n| is_polygon(*n))
                .collect();
            out.insert(active);
            Ok(out.into_iter().collect())
        }
    }
}

impl CommandHandler for ToolCommand {
    fn id(&self) -> &str {
        self.command
    }

    fn label(&self) -> &str {
        self.desc.label
    }

    fn args_schema(&self) -> Json {
        let settings: Map<String, Json> = self
            .desc
            .params
            .iter()
            .map(|p| (p.id.clone(), setting_schema(p)))
            .collect();
        let mut properties = json!({
            "node": {
                "description": "Polygon object; the active node when absent.",
                "type": ["integer", "null"]
            },
            "nodes": {
                "description": "Polygon objects, all edited in one undo step, each with its own \
                                stored selection. Absent (and no node): every selected polygon \
                                object. Not together with node; polys, edges and points need a \
                                single object.",
                "type": ["array", "null"],
                "items": {"type": "integer", "minimum": 0}
            },
            "polys": {
                "description": "Polygon indices; the stored polygon selection when absent.",
                "type": ["array", "null"],
                "items": {"type": "integer", "minimum": 0}
            },
            "settings": {
                "description": "Tool settings; absent keys keep the factory defaults.",
                "type": "object",
                "additionalProperties": false,
                "properties": settings
            }
        });
        if self.desc.components {
            properties["polys"]["description"] = json!(
                "Polygon indices (polygon mode). When none of polys, edges, points is given \
                 the stored selection is used."
            );
            properties["edges"] = json!({
                "description": "Edges (edge mode): edge ids of the mesh, or point pairs [a, b].",
                "type": ["array", "null"],
                "items": {"anyOf": [
                    {"type": "integer", "minimum": 0},
                    {"type": "array", "items": {"type": "integer", "minimum": 0},
                     "minItems": 2, "maxItems": 2}
                ]}
            });
            properties["points"] = json!({
                "description": "Point indices (point mode).",
                "type": ["array", "null"],
                "items": {"type": "integer", "minimum": 0}
            });
        }
        json!({
            "type": "object",
            "additionalProperties": false,
            "properties": properties
        })
    }

    fn run(&self, ctx: &mut dyn CommandContext, args: Json) -> Result<Json, CommandError> {
        let id = self.desc.command;
        let args = if args.is_null() { json!({}) } else { args };
        let obj = args.as_object().ok_or_else(|| {
            CommandError::InvalidArgs(format!("{id}: arguments must be an object"))
        })?;
        for key in obj.keys() {
            let component = self.desc.components && ["edges", "points"].contains(&key.as_str());
            if !(component || ["node", "nodes", "polys", "settings"].contains(&key.as_str())) {
                return Err(CommandError::InvalidArgs(format!(
                    "{id}: unknown key {key}"
                )));
            }
        }
        let settings = match obj.get("settings") {
            None | Some(Json::Null) => Settings::defaults(&self.desc),
            Some(Json::Object(o)) => Settings::from_json(&self.desc, o)
                .map_err(|e| CommandError::InvalidArgs(format!("{id}: {e}")))?,
            Some(_) => {
                return Err(CommandError::InvalidArgs(format!(
                    "{id}: settings must be an object"
                )));
            }
        };
        let nodes = polygon_targets(ctx, id, self.desc.label, obj)?;
        let multi = nodes.len() > 1;
        let index_list = |key: &str| -> Result<Option<IndexSet>, CommandError> {
            match obj.get(key) {
                None | Some(Json::Null) => Ok(None),
                Some(Json::Array(items)) => items
                    .iter()
                    .map(|v| {
                        v.as_u64()
                            .and_then(|i| u32::try_from(i).ok())
                            .ok_or_else(|| CommandError::InvalidArgs(format!("{id}: bad {key}")))
                    })
                    .collect::<Result<IndexSet, _>>()
                    .map(Some),
                Some(_) => Err(CommandError::InvalidArgs(format!("{id}: bad {key}"))),
            }
        };
        let given_polys = index_list("polys")?;
        let given_points = index_list("points")?;
        let given_edges = match obj.get("edges") {
            None | Some(Json::Null) => None,
            Some(Json::Array(items)) => Some(items.clone()),
            Some(_) => return Err(CommandError::InvalidArgs(format!("{id}: bad edges"))),
        };
        let any_given = given_polys.is_some() || given_points.is_some() || given_edges.is_some();
        if multi && any_given {
            return Err(CommandError::InvalidArgs(format!(
                "{id}: polys, edges and points name the components of one object; \
                 give a single node"
            )));
        }
        let fail_for = |node: NodeId, msg: String| {
            if multi {
                CommandError::Failed(format!("{}: node {}: {msg}", self.desc.label, node.0))
            } else {
                CommandError::Failed(format!("{}: {msg}", self.desc.label))
            }
        };
        let mut done: Vec<Outcome> = Vec::new();
        let mut skipped: Vec<(NodeId, CommandError)> = Vec::new();
        for node in nodes {
            let scene = ctx.doc().scene();
            let is_polygon = scene.node(node).is_some_and(|n| n.type_id == POLYGON);
            let Some(Value::Mesh(stored)) = scene.param(node, "mesh").filter(|_| is_polygon) else {
                return Err(fail_for(
                    node,
                    "not a polygon object (use Make Editable first)".into(),
                ));
            };
            let base = to_poly_mesh(stored);
            let stored = |key: &str| -> IndexSet {
                match scene.param(node, key) {
                    Some(Value::IndexList(l)) => l.iter().copied().collect(),
                    _ => IndexSet::new(),
                }
            };
            let chosen = if !self.desc.components {
                Selection {
                    polys: given_polys
                        .clone()
                        .unwrap_or_else(|| stored(SELECTION_POLYS)),
                    ..Selection::default()
                }
            } else if any_given {
                let edges = match &given_edges {
                    Some(items) => edge_ids(&base, items).map_err(|e| fail_for(node, e))?,
                    None => IndexSet::new(),
                };
                Selection {
                    polys: given_polys.clone().unwrap_or_default(),
                    points: given_points.clone().unwrap_or_default(),
                    edges,
                }
            } else {
                Selection {
                    polys: stored(SELECTION_POLYS),
                    points: stored(SELECTION_POINTS),
                    edges: stored(SELECTION_EDGES),
                }
            };
            let applied = (self.apply)(&base, &chosen, &settings).and_then(|(mesh, selection)| {
                // the two commands that do more than replace the mesh (MODELING.md §4)
                let split = match self.desc.id {
                    "split" => Some(forge_modeling::split_copy(&base, &chosen)?),
                    _ => None,
                };
                let moved = match self.desc.id {
                    "center_axis" => Some(forge_modeling::center_axis(&base, &settings)?.1),
                    _ => None,
                };
                Ok((mesh, selection, split, moved))
            });
            match applied {
                Ok((mesh, selection, split, moved)) => done.push(Outcome {
                    node,
                    mesh,
                    selection,
                    split,
                    moved,
                }),
                // an object without a selection the tool cannot use is left out when
                // other objects are edited (MODELING.md §0.1)
                Err(e) if multi && chosen_is_empty(&chosen) => {
                    skipped.push((node, fail_for(node, e.to_string())));
                }
                Err(e) => return Err(fail_for(node, e.to_string())),
            }
        }
        if done.is_empty() {
            return Err(skipped
                .into_iter()
                .next()
                .map(|(_, e)| e)
                .unwrap_or_else(|| {
                    CommandError::Failed(format!("{}: select a polygon object", self.desc.label))
                }));
        }
        let keeps_selection = self.desc.id == "slide";
        let mut created: Vec<NodeId> = Vec::new();
        ctx.doc_mut().transact(self.desc.label, |tx| {
            for o in &done {
                let node = o.node;
                tx.set_param(node, "mesh", Value::Mesh(Arc::new(from_poly_mesh(&o.mesh))))?;
                let polys: Vec<u32> = o.selection.polys.iter().collect();
                tx.set_param(node, SELECTION_POLYS, Value::IndexList(polys))?;
                // Slide moves points only: the stored edge and point selection stay valid
                let (keep_points, keep_edges) = if keeps_selection {
                    (
                        o.selection.points.iter().collect(),
                        o.selection.edges.iter().collect(),
                    )
                } else {
                    (Vec::new(), Vec::new())
                };
                tx.set_param(node, SELECTION_POINTS, Value::IndexList(keep_points))?;
                tx.set_param(node, SELECTION_EDGES, Value::IndexList(keep_edges))?;
                if let Some(copy) = &o.split {
                    created.push(forge_generators::spawn_split(tx, node, copy).map_err(
                        |e| match e {
                            forge_generators::EditableError::Edit(e) => e,
                            _ => forge_core::EditError::UnknownNode(node),
                        },
                    )?);
                }
                if let Some(c) = o.moved {
                    center_object(tx, node, c)?;
                }
            }
            Ok(())
        })?;
        if let Some((first, rest)) = created.split_first() {
            let sel = ctx.selection_mut();
            sel.click(*first);
            for n in rest {
                sel.toggle(*n);
            }
            ctx.retain_selection();
        }
        let entry = |o: &Outcome| {
            json!({"node": o.node.0, "points": o.mesh.points.len(), "polys": o.mesh.polys.len(),
                   "selected": o.selection.polys.iter().collect::<Vec<u32>>()})
        };
        let mut result = entry(&done[0]);
        result["nodes"] = Json::Array(done.iter().map(entry).collect());
        result["skipped"] = json!(skipped.iter().map(|(n, _)| n.0).collect::<Vec<_>>());
        if let Some(new) = created.first() {
            result["created"] = json!(new.0);
        }
        if let Some(c) = done[0].moved {
            result["center"] = json!([c.x, c.y, c.z]);
        }
        Ok(result)
    }

    fn undoable(&self) -> bool {
        true
    }

    fn sample_args(&self) -> Json {
        if self.desc.id == "slide" {
            json!({"edges": [0], "settings": {"amount": 0.5}})
        } else if self.desc.id == "array" {
            json!({"settings": {"count_x": 2, "offset": [200.0, 0.0, 0.0]}})
        } else if self.desc.params.iter().any(|p| p.id == "offset") {
            json!({"polys": [0], "settings": {"offset": 10.0}})
        } else {
            json!({"polys": [0]})
        }
    }
}
