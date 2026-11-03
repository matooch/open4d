//! snap.set, snap.toggle, snap.reset and snap.query: snapping settings and a headless
//! world-space snap (docs/specs/MANIPULATE.md §6).

use forge_core::{EvalCtx, NodeId};
use forge_modeling::snap::{SnapSettings, SnapSource, snap_point};
use glam::DVec3;
use serde_json::{Map, Value as Json, json};

use super::cutting::{check_keys, number, vec3, vec3_schema};
use crate::context::CommandContext;
use crate::error::CommandError;
use crate::registry::{CommandHandler, CommandRegistry};

pub(super) fn register(r: &mut CommandRegistry) -> Result<(), CommandError> {
    r.register(Box::new(Set))?;
    r.register(Box::new(Toggle))?;
    r.register(Box::new(Reset))?;
    r.register(Box::new(Query))
}

struct Set;
struct Toggle;
struct Reset;
struct Query;

fn args_object<'a>(id: &str, args: &'a Json) -> Result<&'a Map<String, Json>, CommandError> {
    args.as_object()
        .ok_or_else(|| CommandError::InvalidArgs(format!("{id}: arguments must be an object")))
}

fn targets_schema() -> Json {
    let mut props = Map::new();
    for k in forge_modeling::snap::SnapKind::ALL {
        props.insert(k.id().into(), json!({"type": "boolean"}));
    }
    json!({"type": "object", "additionalProperties": false, "properties": props})
}

fn settings_schema() -> Json {
    json!({"type": "object", "additionalProperties": false, "properties": {
        "enabled": {"type": "boolean"},
        "mode": {"enum": ["3d", "2d"]},
        "radius": {"type": "number", "minimum": 1, "maximum": 100},
        "visible_only": {"type": "boolean"},
        "grid_spacing": {"type": "number", "minimum": 0, "maximum": 1e9},
        "targets": targets_schema(),
        "workplane": {"type": "object", "additionalProperties": false, "properties": {
            "origin": vec3_schema("A point of the workplane."),
            "normal": vec3_schema("The workplane normal (not zero).")}},
        "quantize": {"type": "object", "additionalProperties": false, "properties": {
            "enabled": {"type": "boolean"},
            "move": {"type": "number", "exclusiveMinimum": 0},
            "angle": {"type": "number", "exclusiveMinimum": 0},
            "percent": {"type": "number", "exclusiveMinimum": 0}}}}})
}

impl CommandHandler for Set {
    fn id(&self) -> &str {
        "snap.set"
    }

    fn label(&self) -> &str {
        "Snap Settings"
    }

    fn args_schema(&self) -> Json {
        settings_schema()
    }

    fn run(&self, ctx: &mut dyn CommandContext, args: Json) -> Result<Json, CommandError> {
        let args = if args.is_null() { json!({}) } else { args };
        let obj = args_object(self.id(), &args)?;
        let snap = &mut ctx.transform_settings_mut().snap;
        snap.apply_json(obj)
            .map_err(|e| CommandError::InvalidArgs(format!("snap.set: {e}")))?;
        Ok(snap.to_json())
    }

    fn undoable(&self) -> bool {
        false
    }

    fn sample_args(&self) -> Json {
        json!({"enabled": true, "targets": {"vertex": true, "grid_point": true}, "radius": 12})
    }
}

impl CommandHandler for Toggle {
    fn id(&self) -> &str {
        "snap.toggle"
    }

    fn label(&self) -> &str {
        "Snapping"
    }

    fn args_schema(&self) -> Json {
        json!({"type": "object", "additionalProperties": false,
               "properties": {"enabled": {"type": "boolean"}}})
    }

    fn run(&self, ctx: &mut dyn CommandContext, args: Json) -> Result<Json, CommandError> {
        let args = if args.is_null() { json!({}) } else { args };
        let obj = args_object(self.id(), &args)?;
        check_keys(self.id(), obj, &["enabled"])?;
        let snap = &mut ctx.transform_settings_mut().snap;
        snap.enabled = match obj.get("enabled") {
            None | Some(Json::Null) => !snap.enabled,
            Some(Json::Bool(b)) => *b,
            Some(_) => {
                return Err(CommandError::InvalidArgs(
                    "snap.toggle: enabled must be true or false".into(),
                ));
            }
        };
        Ok(json!({"enabled": snap.enabled}))
    }

    fn undoable(&self) -> bool {
        false
    }
}

impl CommandHandler for Reset {
    fn id(&self) -> &str {
        "snap.reset"
    }

    fn label(&self) -> &str {
        "Reset Snapping"
    }

    fn args_schema(&self) -> Json {
        json!({"type": "object", "additionalProperties": false, "properties": {}})
    }

    fn run(&self, ctx: &mut dyn CommandContext, args: Json) -> Result<Json, CommandError> {
        if !(args.is_null() || args.as_object().is_some_and(|o| o.is_empty())) {
            return Err(CommandError::InvalidArgs(
                "snap.reset takes no arguments".into(),
            ));
        }
        let snap = &mut ctx.transform_settings_mut().snap;
        *snap = SnapSettings::default();
        Ok(snap.to_json())
    }

    fn undoable(&self) -> bool {
        false
    }
}

/// World-space meshes of the drawn nodes (the viewport's rule: no consumed inputs, editor
/// visible), as snap sources.
fn sources(ctx: &dyn CommandContext, only: Option<&[NodeId]>) -> Vec<(NodeId, SnapSource)> {
    let scene = ctx.doc().scene();
    let ectx = EvalCtx::default();
    let mut out = Vec::new();
    for root in scene.roots() {
        for id in std::iter::once(*root).chain(scene.descendants(*root)) {
            if only.is_some_and(|o| !o.contains(&id))
                || forge_generators::is_consumed(scene, id)
                || !forge_core::visibility::editor_visible(scene, id)
            {
                continue;
            }
            let (Ok(Some(mut mesh)), Some(world)) = (
                forge_generators::evaluate(scene, id, &ectx),
                scene.world_matrix(id),
            ) else {
                continue;
            };
            for p in &mut mesh.points {
                *p = world.transform_point3(*p);
            }
            out.push((
                id,
                SnapSource {
                    mesh,
                    origin: Some(world.w_axis.truncate()),
                    skip_points: Vec::new(),
                },
            ));
        }
    }
    out
}

impl CommandHandler for Query {
    fn id(&self) -> &str {
        "snap.query"
    }

    fn label(&self) -> &str {
        "Snap Query"
    }

    fn args_schema(&self) -> Json {
        json!({"type": "object", "additionalProperties": false, "required": ["point"],
               "properties": {
                   "point": vec3_schema("World point to snap."),
                   "radius": {"description": "World units (default 10).",
                              "type": "number", "exclusiveMinimum": 0},
                   "nodes": {"description": "Only snap to these nodes; the whole scene when absent.",
                             "type": ["array", "null"],
                             "items": {"type": "integer", "minimum": 0}}}})
    }

    fn run(&self, ctx: &mut dyn CommandContext, args: Json) -> Result<Json, CommandError> {
        let id = self.id();
        let obj = args_object(id, &args)?;
        check_keys(id, obj, &["point", "radius", "nodes"])?;
        let p = vec3(id, obj, "point")?
            .ok_or_else(|| CommandError::InvalidArgs(format!("{id}: point is required")))?;
        let radius = match obj.get("radius") {
            None | Some(Json::Null) => 10.0,
            Some(v) => number(id, v, "radius")?,
        };
        if radius <= 0.0 {
            return Err(CommandError::InvalidArgs(format!(
                "{id}: radius must be greater than 0"
            )));
        }
        let only: Option<Vec<NodeId>> = match obj.get("nodes") {
            None | Some(Json::Null) => None,
            Some(Json::Array(a)) => Some(
                a.iter()
                    .map(|v| {
                        v.as_u64().map(NodeId).ok_or_else(|| {
                            CommandError::InvalidArgs(format!("{id}: nodes must be ids"))
                        })
                    })
                    .collect::<Result<_, _>>()?,
            ),
            Some(_) => {
                return Err(CommandError::InvalidArgs(format!(
                    "{id}: nodes must be a list of ids"
                )));
            }
        };
        let (nodes, srcs): (Vec<NodeId>, Vec<SnapSource>) =
            sources(ctx, only.as_deref()).into_iter().unzip();
        let settings = ctx.transform_settings().snap.clone();
        let hit = snap_point(&srcs, p, radius, &settings);
        let v = |p: DVec3| json!([p.x, p.y, p.z]);
        Ok(match hit {
            None => json!({"hit": null}),
            Some(h) => json!({"hit": {
                "kind": h.kind.id(),
                "position": v(h.position),
                "raw": v(h.raw),
                "node": h.source.and_then(|s| nodes.get(s)).map(|n| n.0),
                "index": h.index,
                "distance": h.distance,
            }}),
        })
    }

    fn undoable(&self) -> bool {
        false
    }

    fn sample_args(&self) -> Json {
        json!({"point": [0.0, 0.0, 0.0], "radius": 25})
    }
}
