//! `material.*`, `shader.set` and `uv.generate`: materials and shaders hosted in the document
//! (docs/specs/MATERIAL.md M6, UV.md U5).

use forge_command::{
    CommandContext, CommandError, CommandRegistry, CommandSpec, param_value_from_json,
};
use forge_core::{Category, NodeId, Value};
use forge_generators::{POLYGON, to_poly_mesh};
use forge_material::node_material::{FROM_STANDARD, GRAPH, NODE_MATERIAL, compile_graph};
use forge_material::{Camera, MATERIAL, Projection, TEXTURE_TAG, TagSpace, UVW_TAG, generate_uvw};
use glam::DVec3;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Map, Value as Json, json};

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct CreateArgs {
    /// Material type (default `forge.material.standard`).
    #[serde(rename = "type")]
    ty: Option<String>,
    name: Option<String>,
    params: Option<Map<String, Json>>,
    /// Position in the material list (default: the end).
    index: Option<usize>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct ConvertArgs {
    material: u64,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct DeleteArgs {
    materials: Vec<u64>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct GraphArgs {
    material: u64,
    /// Serialized forge-nodes material graph.
    graph: String,
    /// Merge repeated edits in one continuous canvas gesture.
    gesture: Option<u64>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct AssignArgs {
    material: u64,
    /// Objects (default: the selection).
    nodes: Option<Vec<u64>>,
    /// Projection of the new tags (a texture tag `projection` id, defaults to UVW mapping).
    projection: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct ShaderArgs {
    /// A `forge.shader.*` type; null removes the shader of the slot.
    #[serde(rename = "type")]
    ty: Option<String>,
    /// With `channel`: the material.
    material: Option<u64>,
    /// A slot: a channel name (`color`, `bump`, ...) or a slot param id.
    channel: Option<String>,
    /// Instead of a slot: a shader that takes the new one as an input.
    parent: Option<u64>,
    /// Parameters of the new shader, JSON by the shader type's units.
    params: Option<Map<String, Json>>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct CameraArg {
    position: [f64; 3],
    rotation_deg: [f64; 3],
    /// Film height over width (default 16 / 9).
    aspect: Option<f64>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct UvArgs {
    /// A polygon object (default: the active node).
    object: Option<u64>,
    /// A texture tag of the object (default: its first).
    tag: Option<u64>,
    camera: Option<CameraArg>,
}

/// Registers the commands.
pub fn register(r: &mut CommandRegistry) -> Result<(), CommandError> {
    let spec = |id, label, sample| CommandSpec {
        id,
        label,
        undoable: true,
        sample,
    };
    r.register_fn(
        spec("material.create", "Create Material", || json!({})),
        create,
    )?;
    r.register_fn(
        spec(
            "material.convert_to_node",
            "Convert to Node Material",
            || json!({"material": 1}),
        ),
        convert_to_node,
    )?;
    r.register_fn(
        spec("material.set_graph", "Edit Material Graph", || {
            json!({"material": 1, "graph": forge_material::node_material::default_graph().to_json()})
        }),
        set_graph,
    )?;
    r.register_fn(
        spec(
            "material.delete",
            "Delete Material",
            || json!({"materials": [1]}),
        ),
        delete,
    )?;
    r.register_fn(
        spec(
            "material.assign",
            "Assign Material",
            || json!({"material": 1}),
        ),
        assign,
    )?;
    r.register_fn(
        spec(
            "shader.set",
            "Set Shader",
            || json!({"type": "forge.shader.gradient", "material": 1, "channel": "color"}),
        ),
        shader_set,
    )?;
    r.register_fn(
        spec("uv.generate", "Generate UVW", || json!({})),
        uv_generate,
    )
}

fn require_material(ctx: &dyn CommandContext, id: u64) -> Result<NodeId, CommandError> {
    let n = NodeId(id);
    if ctx.doc().scene().materials().contains(&n) {
        Ok(n)
    } else {
        Err(CommandError::failed(format!("node {id} is not a material")))
    }
}

fn create(ctx: &mut dyn CommandContext, a: CreateArgs) -> Result<Json, CommandError> {
    let ty = a.ty.unwrap_or_else(|| MATERIAL.to_string());
    let mut values = Vec::new();
    if let Some(params) = a.params {
        let desc = ctx
            .doc()
            .scene()
            .registry()
            .get(&ty)
            .ok_or_else(|| CommandError::failed(format!("unknown material type {ty}")))?;
        for (key, input) in params {
            let param = desc
                .find_param(&key)
                .ok_or_else(|| CommandError::failed(format!("unknown material parameter {key}")))?;
            values.push((key, param_value_from_json(param, &input)?));
        }
    }
    let id = ctx.doc_mut().transact("Create Material", |tx| {
        let id = tx.create_material(&ty, a.index)?;
        if let Some(name) = &a.name {
            tx.rename(id, name)?;
        }
        for (key, value) in values {
            tx.set_param(id, &key, value)?;
        }
        Ok(id)
    })?;
    Ok(json!({"material": id.0}))
}

fn convert_to_node(ctx: &mut dyn CommandContext, a: ConvertArgs) -> Result<Json, CommandError> {
    let id = require_material(ctx, a.material)?;
    if !ctx
        .doc()
        .scene()
        .node(id)
        .is_some_and(|n| n.type_id == MATERIAL)
    {
        return Err(CommandError::failed(
            "select a Standard material to convert",
        ));
    }
    ctx.doc_mut().transact("Convert to Node Material", |tx| {
        tx.upgrade_material_type(id, NODE_MATERIAL)?;
        tx.set_param(id, FROM_STANDARD, Value::Bool(true))
    })?;
    Ok(json!({"material": id.0}))
}

fn set_graph(ctx: &mut dyn CommandContext, a: GraphArgs) -> Result<Json, CommandError> {
    let id = require_material(ctx, a.material)?;
    let scene = ctx.doc().scene();
    if !scene.node(id).is_some_and(|n| n.type_id == NODE_MATERIAL) {
        return Err(CommandError::failed("not a node material"));
    }
    let graph =
        forge_nodes::Graph::from_json(&a.graph).map_err(|e| CommandError::failed(e.to_string()))?;
    compile_graph(&graph).map_err(|e| CommandError::failed(e.to_string()))?;
    let value = Value::String(graph.to_json());
    if scene.param(id, GRAPH) != Some(&value) {
        let edit = |tx: &mut forge_core::Tx| tx.set_param(id, GRAPH, value);
        if let Some(gesture) = a.gesture {
            ctx.doc_mut().transact_merged(
                "Edit Material Graph",
                &format!("material.graph:{id}:{gesture}"),
                edit,
            )?;
        } else {
            ctx.doc_mut().transact("Edit Material Graph", edit)?;
        }
    }
    Ok(json!({"material": id.0}))
}

fn delete(ctx: &mut dyn CommandContext, a: DeleteArgs) -> Result<Json, CommandError> {
    let ids: Vec<NodeId> = a
        .materials
        .iter()
        .map(|m| require_material(ctx, *m))
        .collect::<Result<_, _>>()?;
    ctx.doc_mut().transact("Delete Material", |tx| {
        for id in &ids {
            tx.delete(*id)?;
        }
        Ok(())
    })?;
    Ok(json!({"deleted": ids.len()}))
}

fn assign(ctx: &mut dyn CommandContext, a: AssignArgs) -> Result<Json, CommandError> {
    let material = require_material(ctx, a.material)?;
    let nodes: Vec<NodeId> = match &a.nodes {
        Some(n) => n.iter().map(|i| NodeId(*i)).collect(),
        None => ctx.selection().nodes.iter().copied().collect(),
    };
    if nodes.is_empty() {
        return Err(CommandError::failed(
            "select objects to assign the material to",
        ));
    }
    let projection = Value::Enum(a.projection.unwrap_or_else(|| "uvw".to_owned()));
    let tags = ctx.doc_mut().transact("Assign Material", |tx| {
        let mut tags = Vec::new();
        for n in &nodes {
            let tag = tx.attach_tag(TEXTURE_TAG, *n, None)?;
            tx.set_param(tag, "material", Value::Link(Some(material)))?;
            tx.set_param(tag, "projection", projection.clone())?;
            tags.push(tag.0);
        }
        Ok(tags)
    })?;
    Ok(json!({"tags": tags}))
}

/// The slot param of a channel name (`color` -> `color.texture`) or a slot param id.
fn slot_param(
    ctx: &dyn CommandContext,
    material: NodeId,
    channel: &str,
) -> Result<String, CommandError> {
    let scene = ctx.doc().scene();
    let ty = scene
        .node_type(material)
        .ok_or_else(|| CommandError::failed("unknown material type"))?;
    for cand in [channel.to_string(), format!("{channel}.texture")] {
        if let Some(d) = ty.find_param(&cand)
            && matches!(&d.kind, forge_core::ParamKind::Link { accepts } if accepts == &[Category::Shader])
        {
            return Ok(cand);
        }
    }
    Err(CommandError::failed(format!(
        "{channel} is not a texture slot"
    )))
}

fn shader_set(ctx: &mut dyn CommandContext, a: ShaderArgs) -> Result<Json, CommandError> {
    enum Target {
        Slot(NodeId, String),
        Input(NodeId),
    }
    let target = match (a.material, &a.channel, a.parent) {
        (Some(m), Some(c), None) => {
            let m = require_material(ctx, m)?;
            let p = slot_param(ctx, m, c)?;
            Target::Slot(m, p)
        }
        (None, None, Some(p)) => Target::Input(NodeId(p)),
        _ => {
            return Err(CommandError::InvalidArgs(
                "give material and channel, or parent".into(),
            ));
        }
    };
    if a.ty.is_none() && matches!(target, Target::Input(_)) {
        return Err(CommandError::InvalidArgs(
            "a shader type is needed for parent".into(),
        ));
    }
    // convert the params before anything changes
    let mut values: Vec<(String, Value)> = Vec::new();
    if let Some(ty_id) = &a.ty {
        let registry = ctx.doc().scene().registry().clone();
        let ty = registry
            .get(ty_id)
            .ok_or_else(|| CommandError::failed(format!("unknown type {ty_id}")))?;
        for (k, v) in a.params.iter().flatten() {
            let desc = ty
                .find_param(k)
                .ok_or_else(|| CommandError::failed(format!("unknown parameter {k}")))?;
            values.push((k.clone(), param_value_from_json(desc, v)?));
        }
    }
    let shader = ctx.doc_mut().transact("Set Shader", |tx| {
        let (parent, slot) = match &target {
            Target::Slot(m, p) => {
                if let Some(Value::Link(Some(old))) = tx.scene().param(*m, p).cloned() {
                    tx.delete(old)?; // clears the slot link too
                }
                (*m, Some(p.clone()))
            }
            Target::Input(p) => (*p, None),
        };
        let Some(ty_id) = &a.ty else {
            return Ok(None);
        };
        let id = tx.create_shader(ty_id, parent, None)?;
        for (k, v) in &values {
            tx.set_param(id, k, v.clone())?;
        }
        if let Some(p) = slot {
            tx.set_param(parent, &p, Value::Link(Some(id)))?;
        }
        Ok(Some(id))
    })?;
    Ok(json!({"shader": shader.map(|s| s.0)}))
}

fn triple(v: [f64; 3], what: &str) -> Result<[f64; 3], CommandError> {
    if v.iter().all(|c| c.is_finite()) {
        Ok(v)
    } else {
        Err(CommandError::InvalidArgs(format!("{what} must be finite")))
    }
}

fn vec3(scene_value: Option<&Value>, default: [f64; 3]) -> [f64; 3] {
    match scene_value {
        Some(Value::Vec3(v)) => *v,
        _ => default,
    }
}

fn uv_generate(ctx: &mut dyn CommandContext, a: UvArgs) -> Result<Json, CommandError> {
    let object = match a.object {
        Some(o) => NodeId(o),
        None => ctx
            .selection()
            .active
            .ok_or_else(|| CommandError::failed("select a polygon object"))?,
    };
    let scene = ctx.doc().scene();
    let node = scene
        .node(object)
        .filter(|n| n.type_id == POLYGON)
        .ok_or_else(|| CommandError::failed("select a polygon object"))?;
    let tag = match a.tag {
        Some(t) => NodeId(t),
        None => node
            .tags
            .iter()
            .copied()
            .find(|t| scene.node(*t).is_some_and(|n| n.type_id == TEXTURE_TAG))
            .ok_or_else(|| CommandError::failed("the object has no texture tag"))?,
    };
    let tn = scene
        .node(tag)
        .filter(|n| n.type_id == TEXTURE_TAG && n.host == Some(object))
        .ok_or_else(|| CommandError::failed("not a texture tag of the object"))?;
    let camera = match &a.camera {
        Some(c) => Some(Camera {
            position: DVec3::from(triple(c.position, "camera position")?),
            hpb: triple(c.rotation_deg, "camera rotation")?.map(f64::to_radians),
            aspect: c.aspect.unwrap_or(16.0 / 9.0),
        }),
        None => None,
    };
    let Some(Value::Enum(proj_id)) = tn.params.get("projection") else {
        return Err(CommandError::failed("the tag has no projection"));
    };
    let projection =
        Projection::from_id(proj_id, camera).map_err(|e| CommandError::failed(e.to_string()))?;
    let space = TagSpace {
        position: DVec3::from(vec3(tn.params.get("position"), [0.0; 3])),
        rotation: vec3(tn.params.get("rotation"), [0.0; 3]),
        size: DVec3::from(vec3(tn.params.get("size"), [100.0; 3])),
    };
    let Some(Value::Mesh(mesh)) = node.params.get("mesh") else {
        return Err(CommandError::failed("the object has no mesh"));
    };
    let mesh = to_poly_mesh(mesh);
    let world = scene
        .world_matrix(object)
        .ok_or_else(|| CommandError::failed("the object has no transform"))?;
    let uvs = generate_uvw(&mesh, &space, projection, world)
        .map_err(|e| CommandError::failed(e.to_string()))?;
    // an existing UVW tag keeps the polygons the tag's selection leaves out
    let existing = node
        .tags
        .iter()
        .copied()
        .find(|t| scene.node(*t).is_some_and(|n| n.type_id == UVW_TAG));
    let old: Vec<f64> = match existing.and_then(|t| scene.param(t, "uvw")) {
        Some(Value::FloatList(l)) => l.clone(),
        _ => Vec::new(),
    };
    let only: Option<Vec<u32>> = match tn.params.get("selection") {
        Some(Value::String(name)) if !name.is_empty() => node
            .tags
            .iter()
            .filter_map(|t| scene.node(*t))
            .find(|n| n.type_id == forge_core::tags::POLYGON_SELECTION && n.name == *name)
            .and_then(|n| match n.params.get("indices") {
                Some(Value::IndexList(l)) => Some(l.clone()),
                _ => None,
            }),
        _ => None,
    };
    let mut flat = Vec::new();
    let mut at = 0;
    for (p, corners) in uvs.corners.iter().enumerate() {
        let keep = only.as_ref().is_some_and(|s| !s.contains(&(p as u32)));
        for c in corners {
            if keep {
                flat.push(old.get(at).copied().unwrap_or(0.0));
                flat.push(old.get(at + 1).copied().unwrap_or(0.0));
            } else {
                flat.extend_from_slice(c);
            }
            at += 2;
        }
    }
    let count = at / 2;
    if let Some(t) = existing
        && old == flat
    {
        return Ok(json!({"tag": t.0, "corners": count}));
    }
    let uvw_tag = ctx.doc_mut().transact("Generate UVW", |tx| {
        let t = match existing {
            Some(t) => t,
            None => tx.attach_tag(UVW_TAG, object, None)?,
        };
        tx.set_param(t, "uvw", Value::FloatList(flat))?;
        Ok(t)
    })?;
    Ok(json!({"tag": uvw_tag.0, "corners": count}))
}
