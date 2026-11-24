//! UV editor commands; topology and coordinates live on the first UVW tag (UV.md U7).

use std::collections::BTreeSet;

use forge_command::{CommandContext, CommandError, CommandRegistry, CommandSpec};
use forge_core::{EditError, NodeId, Scene, Tx, Value};
use forge_generators::{POLYGON, to_poly_mesh};
use forge_geometry::{CornerUvs, PolyMesh, uv_edit};
use forge_material::{Projection, TagSpace, UVW_TAG, generate_uvw};
use glam::{DMat4, DVec2};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value as Json, json};

pub struct UvMesh {
    pub mesh: PolyMesh,
    pub uv: CornerUvs,
    pub seams: BTreeSet<u32>,
    tag: Option<NodeId>,
    stored: bool,
}

/// Reads stored corner UVs, or a cubic display fallback for a mesh without UV data.
pub fn read(scene: &Scene, object: NodeId) -> Result<UvMesh, String> {
    let node = scene
        .node(object)
        .filter(|n| n.type_id == POLYGON)
        .ok_or_else(|| "select a polygon object".to_string())?;
    let Some(Value::Mesh(mesh)) = node.params.get("mesh") else {
        return Err("object has no polygon mesh".into());
    };
    let mesh = to_poly_mesh(mesh);
    mesh.validate().map_err(|e| e.to_string())?;
    let tag = node
        .tags
        .iter()
        .copied()
        .find(|&id| scene.node(id).is_some_and(|n| n.type_id == UVW_TAG));
    let seams = match tag.and_then(|id| scene.param(id, "seams")) {
        Some(Value::IndexList(v)) => v.iter().copied().collect(),
        _ => BTreeSet::new(),
    };
    let uv = tag.and_then(|id| scene.param(id, "uvw")).and_then(|v| {
        let Value::FloatList(uv) = v else { return None };
        let (pairs, remainder) = uv.as_chunks::<2>();
        if !remainder.is_empty() || pairs.len() != mesh.polys.iter().map(Vec::len).sum::<usize>() {
            return None;
        }
        let mut pairs = pairs.iter().copied();
        Some(CornerUvs {
            corners: mesh
                .polys
                .iter()
                .map(|p| pairs.by_ref().take(p.len()).collect())
                .collect(),
        })
    });
    let stored = uv.is_some();
    let uv = match uv {
        Some(uv) => uv,
        None => generate_uvw(
            &mesh,
            &TagSpace::default(),
            Projection::Cubic,
            DMat4::IDENTITY,
        )
        .map_err(|e| e.to_string())?,
    };
    Ok(UvMesh {
        mesh,
        uv,
        seams,
        tag,
        stored,
    })
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct ObjectArgs {
    object: Option<u64>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct PackArgs {
    object: Option<u64>,
    /// Padding on each side of an island, in UV units (default 0.01).
    padding: Option<f64>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct SeamArgs {
    object: Option<u64>,
    edge: u32,
    marked: bool,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct TransformArgs {
    object: Option<u64>,
    /// Flattened corner indices (polygon order, then corner order).
    corners: Vec<usize>,
    offset: Option<[f64; 2]>,
    scale: Option<[f64; 2]>,
    rotation_deg: Option<f64>,
    /// Centre of scale/rotation (default: arithmetic centre of selected corners).
    pivot: Option<[f64; 2]>,
}

pub fn register(reg: &mut CommandRegistry) -> Result<(), CommandError> {
    let spec = |id, label, sample| CommandSpec {
        id,
        label,
        undoable: true,
        sample,
    };
    reg.register_fn(spec("uv.relax", "Relax UVs", || json!({})), relax)?;
    reg.register_fn(spec("uv.pack", "Pack UV Islands", || json!({})), pack)?;
    reg.register_fn(
        spec("uv.seam", "Set UV Seam", || json!({"edge":0,"marked":true})),
        seam,
    )?;
    reg.register_fn(
        spec(
            "uv.transform",
            "Transform UVs",
            || json!({"corners":[0],"offset":[0.1,0.0]}),
        ),
        transform,
    )
}

fn target(ctx: &dyn CommandContext, id: Option<u64>) -> Result<(NodeId, UvMesh), CommandError> {
    let id = id
        .map(NodeId)
        .or(ctx.selection().active)
        .ok_or_else(|| CommandError::failed("select a polygon object"))?;
    Ok((
        id,
        read(ctx.doc().scene(), id).map_err(CommandError::failed)?,
    ))
}

fn tag(tx: &mut Tx, current: Option<NodeId>, id: NodeId) -> Result<NodeId, EditError> {
    current.map_or_else(|| tx.attach_tag(UVW_TAG, id, None), Ok)
}

fn write(
    ctx: &mut dyn CommandContext,
    id: NodeId,
    source: &UvMesh,
    uv: CornerUvs,
    label: &str,
) -> Result<Json, CommandError> {
    let coords: Vec<_> = uv.corners.into_iter().flatten().flatten().collect();
    if coords.iter().any(|v| !v.is_finite() || v.abs() > 1e12) {
        return Err(CommandError::failed("UV coordinate is out of range"));
    }
    let count = coords.len() / 2;
    let value = Value::FloatList(coords);
    if let Some(tag) = source.tag
        && ctx.doc().scene().param(tag, "uvw") == Some(&value)
    {
        return Ok(json!({"tag":tag.0,"corners":count}));
    }
    let tag = ctx.doc_mut().transact(label, |tx| {
        let tag = tag(tx, source.tag, id)?;
        tx.set_param(tag, "uvw", value)?;
        Ok(tag)
    })?;
    Ok(json!({"tag":tag.0,"corners":count}))
}

fn relax(ctx: &mut dyn CommandContext, a: ObjectArgs) -> Result<Json, CommandError> {
    let (id, source) = target(ctx, a.object)?;
    let uv = uv_edit::relax(
        &source.mesh,
        source.stored.then_some(&source.uv),
        &source.seams,
    )
    .map_err(|e| CommandError::failed(e.to_string()))?;
    write(ctx, id, &source, uv, "Relax UVs")
}

fn pack(ctx: &mut dyn CommandContext, a: PackArgs) -> Result<Json, CommandError> {
    let (id, source) = target(ctx, a.object)?;
    let uv = uv_edit::pack(
        &source.mesh,
        &source.uv,
        &source.seams,
        a.padding.unwrap_or(0.01),
    )
    .map_err(|e| CommandError::failed(e.to_string()))?;
    write(ctx, id, &source, uv, "Pack UV Islands")
}

fn seam(ctx: &mut dyn CommandContext, a: SeamArgs) -> Result<Json, CommandError> {
    let (id, mut source) = target(ctx, a.object)?;
    if a.edge as usize >= source.mesh.edges().len() {
        return Err(CommandError::failed("edge out of range"));
    }
    let changed = if a.marked {
        source.seams.insert(a.edge)
    } else {
        source.seams.remove(&a.edge)
    };
    if changed {
        let value = Value::IndexList(source.seams.into_iter().collect());
        let tag = ctx.doc_mut().transact("Set UV Seam", |tx| {
            let tag = tag(tx, source.tag, id)?;
            tx.set_param(tag, "seams", value)?;
            Ok(tag)
        })?;
        source.tag = Some(tag);
    }
    Ok(json!({"tag":source.tag.map(|id| id.0), "changed":changed}))
}

fn transform(ctx: &mut dyn CommandContext, a: TransformArgs) -> Result<Json, CommandError> {
    let (id, source) = target(ctx, a.object)?;
    let chosen: BTreeSet<_> = a.corners.into_iter().collect();
    let n = source.uv.corners.iter().map(Vec::len).sum::<usize>();
    if chosen.is_empty() || chosen.last().is_some_and(|i| *i >= n) {
        return Err(CommandError::failed("select valid UV corners"));
    }
    let centre = a.pivot.map(DVec2::from).unwrap_or_else(|| {
        source
            .uv
            .corners
            .iter()
            .flatten()
            .enumerate()
            .filter(|(i, _)| chosen.contains(i))
            .map(|(_, p)| DVec2::from(*p))
            .sum::<DVec2>()
            / chosen.len() as f64
    });
    let offset = DVec2::from(a.offset.unwrap_or([0.0; 2]));
    let scale = DVec2::from(a.scale.unwrap_or([1.0; 2]));
    let rotation = glam::DMat2::from_angle(a.rotation_deg.unwrap_or(0.0).to_radians());
    let mut uv = source.uv.clone();
    for (i, p) in uv.corners.iter_mut().flatten().enumerate() {
        if chosen.contains(&i) {
            *p = (centre + offset + rotation * ((DVec2::from(*p) - centre) * scale)).to_array();
        }
    }
    write(ctx, id, &source, uv, "Transform UVs")
}
