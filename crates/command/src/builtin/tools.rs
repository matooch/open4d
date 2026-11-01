//! mesh.extrude, mesh.extrude_inner: modeling tools as commands (docs/specs/MODELING.md §0.1).

use std::sync::Arc;

use forge_core::{NodeId, Value};
use forge_generators::{
    POLYGON, SELECTION_EDGES, SELECTION_POINTS, SELECTION_POLYS, from_poly_mesh, to_poly_mesh,
};
use forge_geometry::selection::IndexSet;
use forge_modeling::{Settings, ToolDesc, extrude, extrude_inner, tool_descs};
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
    for desc in tool_descs() {
        let apply: forge_modeling::ToolFn = match desc.id {
            "extrude" => extrude,
            _ => extrude_inner,
        };
        r.register(Box::new(ToolCommand {
            command: desc.command,
            desc,
            apply,
        }))?;
    }
    Ok(())
}

fn setting_schema(p: &forge_core::ParamDesc) -> Json {
    match &p.default {
        Value::Bool(b) => json!({"type": "boolean", "default": b}),
        Value::Int(i) => json!({"type": "integer", "default": i}),
        Value::Float(f) => json!({"type": "number", "default": f}),
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
        json!({
            "type": "object",
            "additionalProperties": false,
            "properties": {
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
            }
        })
    }

    fn run(&self, ctx: &mut dyn CommandContext, args: Json) -> Result<Json, CommandError> {
        let id = self.desc.command;
        let args = if args.is_null() { json!({}) } else { args };
        let obj = args.as_object().ok_or_else(|| {
            CommandError::InvalidArgs(format!("{id}: arguments must be an object"))
        })?;
        for key in obj.keys() {
            if !["node", "polys", "settings"].contains(&key.as_str()) {
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
        let given: Option<IndexSet> = match obj.get("polys") {
            None | Some(Json::Null) => None,
            Some(Json::Array(items)) => Some(
                items
                    .iter()
                    .map(|v| {
                        v.as_u64()
                            .and_then(|i| u32::try_from(i).ok())
                            .ok_or_else(|| {
                                CommandError::InvalidArgs(format!("{id}: bad polygon index"))
                            })
                    })
                    .collect::<Result<_, _>>()?,
            ),
            Some(_) => return Err(CommandError::InvalidArgs(format!("{id}: bad polys"))),
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
        let chosen = given.unwrap_or_else(|| match scene.param(node, SELECTION_POLYS) {
            Some(Value::IndexList(l)) => l.iter().copied().collect(),
            _ => IndexSet::new(),
        });
        let (mesh, selection) =
            (self.apply)(&base, &chosen, &settings).map_err(|e| fail(e.to_string()))?;
        let selected: Vec<u32> = selection.polys.iter().collect();
        let (points, polys) = (mesh.points.len(), mesh.polys.len());
        let data = Arc::new(from_poly_mesh(&mesh));
        let sel = selected.clone();
        ctx.doc_mut().transact(self.desc.label, |tx| {
            tx.set_param(node, "mesh", Value::Mesh(data))?;
            tx.set_param(node, SELECTION_POLYS, Value::IndexList(sel))?;
            tx.set_param(node, SELECTION_POINTS, Value::IndexList(Vec::new()))?;
            tx.set_param(node, SELECTION_EDGES, Value::IndexList(Vec::new()))?;
            Ok(())
        })?;
        Ok(json!({"node": node.0, "points": points, "polys": polys, "selected": selected}))
    }

    fn undoable(&self) -> bool {
        true
    }

    fn sample_args(&self) -> Json {
        json!({"polys": [0], "settings": {"offset": 10.0}})
    }
}
