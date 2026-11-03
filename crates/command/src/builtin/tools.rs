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
            if !(component || ["node", "polys", "settings"].contains(&key.as_str())) {
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
        let node = match obj.get("node") {
            Some(Json::Number(n)) => NodeId(
                n.as_u64()
                    .ok_or_else(|| CommandError::InvalidArgs(format!("{id}: bad node id")))?,
            ),
            None | Some(Json::Null) => ctx.selection().active.ok_or_else(|| {
                CommandError::Failed(format!("{}: select a polygon object", self.desc.label))
            })?,
            Some(_) => return Err(CommandError::InvalidArgs(format!("{id}: bad node id"))),
        };
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
        let fail = |msg: String| CommandError::Failed(format!("{}: {msg}", self.desc.label));
        let scene = ctx.doc().scene();
        let is_polygon = scene.node(node).is_some_and(|n| n.type_id == POLYGON);
        let Some(Value::Mesh(stored)) = scene.param(node, "mesh").filter(|_| is_polygon) else {
            return Err(fail(
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
        let any_given = given_polys.is_some() || given_points.is_some() || given_edges.is_some();
        let chosen = if !self.desc.components {
            Selection {
                polys: given_polys.unwrap_or_else(|| stored(SELECTION_POLYS)),
                ..Selection::default()
            }
        } else if any_given {
            let edges = match given_edges {
                Some(items) => edge_ids(&base, &items).map_err(fail)?,
                None => IndexSet::new(),
            };
            Selection {
                polys: given_polys.unwrap_or_default(),
                points: given_points.unwrap_or_default(),
                edges,
            }
        } else {
            Selection {
                polys: stored(SELECTION_POLYS),
                points: stored(SELECTION_POINTS),
                edges: stored(SELECTION_EDGES),
            }
        };
        let (mesh, selection) =
            (self.apply)(&base, &chosen, &settings).map_err(|e| fail(e.to_string()))?;
        let selected: Vec<u32> = selection.polys.iter().collect();
        let (points, polys) = (mesh.points.len(), mesh.polys.len());
        let data = Arc::new(from_poly_mesh(&mesh));
        let sel = selected.clone();
        // the two commands that do more than replace the mesh (MODELING.md §4)
        let split = match self.desc.id {
            "split" => {
                Some(forge_modeling::split_copy(&base, &chosen).map_err(|e| fail(e.to_string()))?)
            }
            _ => None,
        };
        let moved = match self.desc.id {
            "center_axis" => Some(
                forge_modeling::center_axis(&base, &settings)
                    .map_err(|e| fail(e.to_string()))?
                    .1,
            ),
            _ => None,
        };
        let keeps_selection = self.desc.id == "slide";
        let kept_points: Vec<u32> = selection.points.iter().collect();
        let kept_edges: Vec<u32> = selection.edges.iter().collect();
        let mut created = None;
        ctx.doc_mut().transact(self.desc.label, |tx| {
            tx.set_param(node, "mesh", Value::Mesh(data))?;
            tx.set_param(node, SELECTION_POLYS, Value::IndexList(sel))?;
            // Slide moves points only: the stored edge and point selection stay valid
            let (keep_points, keep_edges) = if keeps_selection {
                (kept_points, kept_edges)
            } else {
                (Vec::new(), Vec::new())
            };
            tx.set_param(node, SELECTION_POINTS, Value::IndexList(keep_points))?;
            tx.set_param(node, SELECTION_EDGES, Value::IndexList(keep_edges))?;
            if let Some(copy) = &split {
                created = Some(forge_generators::spawn_split(tx, node, copy).map_err(
                    |e| match e {
                        forge_generators::EditableError::Edit(e) => e,
                        _ => forge_core::EditError::UnknownNode(node),
                    },
                )?);
            }
            if let Some(c) = moved {
                center_object(tx, node, c)?;
            }
            Ok(())
        })?;
        if let Some(new) = created {
            ctx.selection_mut().click(new);
            ctx.retain_selection();
        }
        let mut result =
            json!({"node": node.0, "points": points, "polys": polys, "selected": selected});
        if let Some(new) = created {
            result["created"] = json!(new.0);
        }
        if let Some(c) = moved {
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
            json!({"settings": {"copies": 2, "offset": [200.0, 0.0, 0.0]}})
        } else if self.desc.params.iter().any(|p| p.id == "offset") {
            json!({"polys": [0], "settings": {"offset": 10.0}})
        } else {
            json!({"polys": [0]})
        }
    }
}
