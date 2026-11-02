//! mesh.transform, edit.soft_selection and edit.symmetry: component transforms with soft
//! selection and symmetry, headless (docs/specs/EDITABLE.md §5.5).

use std::sync::Arc;

use forge_core::{MeshData, NodeId, Value};
use forge_generators::{POLYGON, SELECTION_EDGES, SELECTION_POINTS, SELECTION_POLYS, to_poly_mesh};
use forge_geometry::PolyMesh;
use forge_geometry::selection::{IndexSet, edges_to_points, polys_to_points};
use forge_modeling::{ComponentPlan, ComponentSettings};
use glam::{DMat4, DQuat, DVec3};
use serde_json::{Map, Value as Json, json};

use super::cutting::{check_keys, node_schema, number, vec3, vec3_schema};
use crate::context::CommandContext;
use crate::error::CommandError;
use crate::registry::{CommandHandler, CommandRegistry};
use crate::state::ModeArg;

pub(super) fn register(r: &mut CommandRegistry) -> Result<(), CommandError> {
    r.register(Box::new(Transform))?;
    r.register(Box::new(SoftCmd))?;
    r.register(Box::new(SymmetryCmd))
}

struct Transform;
struct SoftCmd;
struct SymmetryCmd;

fn soft_schema() -> Json {
    json!({"type": "object", "additionalProperties": false, "properties": {
        "enabled": {"type": "boolean"},
        "radius": {"type": "number", "minimum": 0.001, "maximum": 1e9},
        "falloff": {"enum": ["linear", "dome", "bell", "spline"]},
        "distance": {"enum": ["volume", "surface"]},
        "curve": {"type": "array", "items": {"type": "number", "minimum": 0, "maximum": 1},
                  "minItems": 2, "maxItems": 64}}})
}

fn symmetry_schema() -> Json {
    json!({"type": "object", "additionalProperties": false, "properties": {
        "enabled": {"type": "boolean"},
        "axis": {"enum": ["x", "y", "z"]},
        "offset": {"type": "number"},
        "tolerance": {"type": "number", "minimum": 0, "maximum": 1e6}}})
}

fn object_args<'a>(id: &str, args: &'a Json) -> Result<&'a Map<String, Json>, CommandError> {
    args.as_object()
        .ok_or_else(|| CommandError::InvalidArgs(format!("{id}: arguments must be an object")))
}

fn invalid(id: &str, e: String) -> CommandError {
    CommandError::InvalidArgs(format!("{id}: {e}"))
}

impl CommandHandler for SoftCmd {
    fn id(&self) -> &str {
        "edit.soft_selection"
    }

    fn label(&self) -> &str {
        "Soft Selection"
    }

    fn args_schema(&self) -> Json {
        soft_schema()
    }

    fn run(&self, ctx: &mut dyn CommandContext, args: Json) -> Result<Json, CommandError> {
        let id = self.id();
        let args = if args.is_null() { json!({}) } else { args };
        let obj = object_args(id, &args)?;
        let soft = &mut ctx.component_settings_mut().soft;
        soft.apply_json(obj).map_err(|e| invalid(id, e))?;
        Ok(soft.to_json())
    }

    fn undoable(&self) -> bool {
        false
    }

    fn sample_args(&self) -> Json {
        json!({"enabled": true, "radius": 100.0, "falloff": "bell"})
    }
}

impl CommandHandler for SymmetryCmd {
    fn id(&self) -> &str {
        "edit.symmetry"
    }

    fn label(&self) -> &str {
        "Symmetry"
    }

    fn args_schema(&self) -> Json {
        symmetry_schema()
    }

    fn run(&self, ctx: &mut dyn CommandContext, args: Json) -> Result<Json, CommandError> {
        let id = self.id();
        let args = if args.is_null() { json!({}) } else { args };
        let obj = object_args(id, &args)?;
        let sym = &mut ctx.component_settings_mut().symmetry;
        sym.apply_json(obj).map_err(|e| invalid(id, e))?;
        Ok(sym.to_json())
    }

    fn undoable(&self) -> bool {
        false
    }

    fn sample_args(&self) -> Json {
        json!({"enabled": true, "axis": "x"})
    }
}

/// The editor settings with the `soft` and `symmetry` arguments of one call applied.
fn effective(
    id: &str,
    base: &ComponentSettings,
    obj: &Map<String, Json>,
) -> Result<ComponentSettings, CommandError> {
    let mut s = base.clone();
    match obj.get("soft") {
        None | Some(Json::Null) => {}
        Some(Json::Bool(b)) => s.soft.enabled = *b,
        Some(Json::Object(o)) => {
            s.soft.enabled = true;
            s.soft.apply_json(o).map_err(|e| invalid(id, e))?;
        }
        Some(_) => return Err(invalid(id, "soft must be true, false or an object".into())),
    }
    match obj.get("symmetry") {
        None | Some(Json::Null) => {}
        Some(Json::Bool(b)) => s.symmetry.enabled = *b,
        Some(Json::Object(o)) => {
            s.symmetry.enabled = true;
            s.symmetry.apply_json(o).map_err(|e| invalid(id, e))?;
        }
        Some(_) => {
            return Err(invalid(
                id,
                "symmetry must be true, false or an object".into(),
            ));
        }
    }
    Ok(s)
}

fn stored(ctx: &dyn CommandContext, node: NodeId, key: &str) -> IndexSet {
    match ctx.doc().scene().param(node, key) {
        Some(Value::IndexList(l)) => l.iter().copied().collect(),
        _ => IndexSet::new(),
    }
}

/// The point ids to transform: the `points` argument, else the stored selection of the
/// current edit mode.
fn selected_points(
    ctx: &dyn CommandContext,
    id: &str,
    node: NodeId,
    mesh: &PolyMesh,
    obj: &Map<String, Json>,
) -> Result<Vec<u32>, CommandError> {
    let n = mesh.points.len() as u32;
    let list: Vec<u32> = match obj.get("points") {
        None | Some(Json::Null) => match ctx.edit_mode() {
            ModeArg::Model => {
                return Err(CommandError::Failed(format!(
                    "{id}: choose a component mode or give points"
                )));
            }
            ModeArg::Points => stored(ctx, node, SELECTION_POINTS).iter().collect(),
            ModeArg::Edges => edges_to_points(mesh, &stored(ctx, node, SELECTION_EDGES))
                .iter()
                .collect(),
            ModeArg::Polys => polys_to_points(mesh, &stored(ctx, node, SELECTION_POLYS))
                .iter()
                .collect(),
        },
        Some(Json::Array(a)) => a
            .iter()
            .map(|v| {
                v.as_u64()
                    .and_then(|i| u32::try_from(i).ok())
                    .ok_or_else(|| invalid(id, "points must be indices".into()))
            })
            .collect::<Result<_, _>>()?,
        Some(_) => return Err(invalid(id, "points must be a list of indices".into())),
    };
    let set: IndexSet = list.into_iter().filter(|i| *i < n).collect();
    if set.is_empty() {
        return Err(CommandError::Failed(format!(
            "{id}: no points to transform"
        )));
    }
    Ok(set.iter().collect())
}

fn scale_arg(id: &str, v: &Json) -> Result<DVec3, CommandError> {
    match v {
        Json::Number(_) => Ok(DVec3::splat(number(id, v, "scale")?)),
        Json::Array(_) => {
            let o: Map<String, Json> = Map::from_iter([("scale".to_string(), v.clone())]);
            vec3(id, &o, "scale")?.ok_or_else(|| invalid(id, "bad scale".into()))
        }
        _ => Err(invalid(id, "scale must be a number or [x, y, z]".into())),
    }
}

impl CommandHandler for Transform {
    fn id(&self) -> &str {
        "mesh.transform"
    }

    fn label(&self) -> &str {
        "Transform Components"
    }

    fn args_schema(&self) -> Json {
        json!({"type": "object", "additionalProperties": false, "properties": {
            "node": node_schema(),
            "points": {"description": "Point ids; the stored selection of the current mode (points, points of the edges or of the polygons) when absent.",
                       "type": ["array", "null"], "items": {"type": "integer", "minimum": 0}},
            "move": vec3_schema("Translation in object space."),
            "rotate": {"description": "Rotation about the pivot, angle in degrees.",
                       "type": "object", "additionalProperties": false, "required": ["axis", "angle"],
                       "properties": {"axis": vec3_schema("Rotation axis, object space."),
                                      "angle": {"type": "number"}}},
            "scale": {"description": "Scale about the pivot: one number or [x, y, z].",
                      "anyOf": [{"type": "number"}, vec3_schema("Scale factors.")]},
            "pivot": vec3_schema("Pivot, object space; the mean of the selected points when absent."),
            "soft": {"description": "false: off, true: on with the editor settings, object: on with these values over the editor settings. Absent: the editor setting.",
                     "anyOf": [{"type": "boolean"}, soft_schema()]},
            "symmetry": {"description": "As soft, for the symmetry plane.",
                         "anyOf": [{"type": "boolean"}, symmetry_schema()]}}})
    }

    fn run(&self, ctx: &mut dyn CommandContext, args: Json) -> Result<Json, CommandError> {
        let id = self.id();
        let args = if args.is_null() { json!({}) } else { args };
        let obj = object_args(id, &args)?;
        check_keys(
            id,
            obj,
            &[
                "node", "points", "move", "rotate", "scale", "pivot", "soft", "symmetry",
            ],
        )?;
        let node = match obj.get("node") {
            Some(Json::Number(n)) => NodeId(
                n.as_u64()
                    .ok_or_else(|| invalid(id, "bad node id".into()))?,
            ),
            None | Some(Json::Null) => ctx
                .selection()
                .active
                .ok_or_else(|| CommandError::Failed(format!("{id}: select a polygon object")))?,
            Some(_) => return Err(invalid(id, "bad node id".into())),
        };
        let scene = ctx.doc().scene();
        let data: Arc<MeshData> = match scene.param(node, "mesh") {
            Some(Value::Mesh(m)) if scene.node(node).is_some_and(|n| n.type_id == POLYGON) => {
                m.clone()
            }
            _ => {
                return Err(CommandError::Failed(format!(
                    "{id}: not a polygon object (use Make Editable first)"
                )));
            }
        };
        let mesh = to_poly_mesh(&data);
        let settings = effective(id, ctx.component_settings(), obj)?;
        let points = selected_points(ctx, id, node, &mesh, obj)?;

        let mv = vec3(id, obj, "move")?;
        let scale = obj.get("scale").filter(|v| !v.is_null());
        let scale = scale.map(|v| scale_arg(id, v)).transpose()?;
        let rotate = match obj.get("rotate") {
            None | Some(Json::Null) => None,
            Some(Json::Object(o)) => {
                check_keys(id, o, &["axis", "angle"])?;
                let axis = vec3(id, o, "axis")?
                    .ok_or_else(|| invalid(id, "rotate needs axis and angle".into()))?;
                let angle = o
                    .get("angle")
                    .ok_or_else(|| invalid(id, "rotate needs axis and angle".into()))?;
                if axis.length() < 1e-12 {
                    return Err(invalid(id, "the rotation axis is zero".into()));
                }
                Some((axis.normalize(), number(id, angle, "angle")?.to_radians()))
            }
            Some(_) => return Err(invalid(id, "rotate must be {axis, angle}".into())),
        };
        if mv.is_none() && scale.is_none() && rotate.is_none() {
            return Err(invalid(id, "give move, rotate or scale".into()));
        }
        let pivot = match vec3(id, obj, "pivot")? {
            Some(p) => p,
            None => {
                points
                    .iter()
                    .map(|i| mesh.points[*i as usize])
                    .sum::<DVec3>()
                    / points.len() as f64
            }
        };
        let mut m =
            DMat4::from_translation(mv.unwrap_or(DVec3::ZERO)) * DMat4::from_translation(pivot);
        if let Some((axis, angle)) = rotate {
            m *= DMat4::from_quat(DQuat::from_axis_angle(axis, angle));
        }
        if let Some(s) = scale {
            m *= DMat4::from_scale(s);
        }
        m *= DMat4::from_translation(-pivot);

        let plan = ComponentPlan::new(&mesh, &points, &settings);
        let moved_to = plan.apply(&mesh.points, m);
        let mut out = (*data).clone();
        let mut moved = 0;
        for (slot, p) in out.points.iter_mut().zip(&moved_to) {
            if p.to_array() != *slot {
                moved += 1;
            }
            *slot = p.to_array();
        }
        let label = match (mv.is_some(), rotate.is_some(), scale.is_some()) {
            (true, false, false) => "Move",
            (false, true, false) => "Rotate",
            (false, false, true) => "Scale",
            _ => "Transform",
        };
        let out = Arc::new(out);
        ctx.doc_mut()
            .transact(label, |tx| tx.set_param(node, "mesh", Value::Mesh(out)))?;
        Ok(
            json!({"node": node.0, "moved": moved, "soft": settings.soft.to_json(),
                  "symmetry": settings.symmetry.to_json()}),
        )
    }

    fn undoable(&self) -> bool {
        true
    }

    fn sample_args(&self) -> Json {
        json!({"points": [0], "move": [0.0, 10.0, 0.0],
               "soft": {"radius": 100.0, "falloff": "bell"}, "symmetry": {"axis": "x"}})
    }
}
