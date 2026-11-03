//! The cloner generator: evaluates its children once as prototypes, makes the clone data
//! and expands the visible clones to geometry in clone order
//! (clone-toolset spec C3).

use std::collections::BTreeMap;

use forge_clone_engine::{
    CLONER, CloneData, CloneError, apply_effectors, generate, settings_from_params,
};
use forge_core::{EvalCtx, NodeId, Scene, Value};
use forge_geometry::PolyMesh;

use crate::{BuildError, evaluate, evaluate_geometry};

pub(crate) const TYPE: &str = CLONER;

fn map_error(e: CloneError) -> BuildError {
    match e {
        CloneError::Unsupported(what) => BuildError::Unsupported(what),
        CloneError::TooMany => BuildError::Unsupported("more than 1000000 clones"),
    }
}

/// The prototypes of a cloner: its children with geometry, in child order (C3.1).
pub fn prototypes_of(scene: &Scene, id: NodeId) -> Vec<NodeId> {
    let Some(node) = scene.node(id) else {
        return Vec::new();
    };
    node.children
        .iter()
        .copied()
        .filter(|c| {
            matches!(
                evaluate_geometry(scene, *c, &EvalCtx::default()),
                Ok(Some(_)) | Err(_)
            )
        })
        .collect()
}

/// The prototypes (local meshes, child transform baked in unless Reset Coordinates) and the
/// clone data with the children assigned; `None` when there is no prototype geometry.
fn build_instances(
    scene: &Scene,
    id: NodeId,
    values: &BTreeMap<String, Value>,
    ctx: &EvalCtx,
) -> Result<Option<ClonerInstances>, BuildError> {
    let settings = settings_from_params(values);
    let mut clones = generate(&settings, ctx).map_err(map_error)?;
    apply_effectors(scene, id, &mut clones, ctx).map_err(map_error)?;
    let mut protos = Vec::new();
    for child in prototypes_of(scene, id) {
        let Some(mut mesh) = evaluate(scene, child, ctx)? else {
            continue;
        };
        if !settings.reset_coordinates {
            let m = scene.local_transform(child).expect("child exists").matrix();
            for p in &mut mesh.points {
                *p = m.transform_point3(*p);
            }
        }
        protos.push(mesh);
    }
    if protos.is_empty() {
        return Ok(None);
    }
    clones
        .assign_children(protos.len(), settings.clones_mode)
        .map_err(map_error)?;
    Ok(Some(ClonerInstances {
        prototypes: protos,
        clones,
    }))
}

/// What a cloner shows: one mesh per prototype and the clone data (matrices in cloner space,
/// colours, visibility, prototype index). The expansion to geometry (C3) and the viewport's GPU
/// instancing (VIEWPORT.md §10.1) both read this.
#[derive(Debug, Clone)]
pub struct ClonerInstances {
    pub prototypes: Vec<PolyMesh>,
    pub clones: CloneData,
}

/// The cloner `id` as prototypes plus clone data, for display without expanding geometry.
/// `Ok(None)` means "expand instead": not a cloner, no prototype geometry, or enabled
/// deformer children (they deform the combined mesh, which instancing cannot show).
pub fn cloner_instances(
    scene: &Scene,
    id: NodeId,
    ctx: &EvalCtx,
) -> Result<Option<ClonerInstances>, BuildError> {
    let Some(node) = scene.node(id).filter(|n| n.type_id == TYPE) else {
        return Ok(None);
    };
    if !crate::deformer_children(scene, id).is_empty()
        || !forge_core::layers::generators_on(scene, id)
    {
        return Ok(None);
    }
    build_instances(scene, id, &node.params, ctx)
}

pub(crate) fn evaluate_cloner(
    scene: &Scene,
    id: NodeId,
    values: &BTreeMap<String, Value>,
    ctx: &EvalCtx,
) -> Result<Option<PolyMesh>, BuildError> {
    let Some(ClonerInstances {
        prototypes: protos,
        clones,
    }) = build_instances(scene, id, values, ctx)?
    else {
        return Ok(None);
    };
    let total: usize = (0..clones.len())
        .filter(|i| clones.visible[*i])
        .map(|i| protos[clones.child[i] as usize].points.len())
        .sum();
    Ok(Some(expand(&protos, &clones, total >= PARALLEL_POINTS)))
}

/// Output points from which the expansion runs on the rayon pool.
pub const PARALLEL_POINTS: usize = 4096;

/// The visible clones as one mesh, in clone order. `parallel` only changes who computes
/// each clone's block; the arithmetic per point is the same, so the result is bit-identical
/// (CORE.md section 5b).
pub fn expand(protos: &[PolyMesh], clones: &CloneData, parallel: bool) -> PolyMesh {
    use rayon::prelude::*;
    let mut visible = Vec::new();
    let mut base = 0u32;
    for i in 0..clones.len() {
        if clones.visible[i] {
            visible.push((i, base));
            base += protos[clones.child[i] as usize].points.len() as u32;
        }
    }
    let block = |&(i, base): &(usize, u32)| {
        let proto = &protos[clones.child[i] as usize];
        let m = clones.matrices[i];
        let points: Vec<_> = proto
            .points
            .iter()
            .map(|p| m.transform_point3(*p))
            .collect();
        let polys: Vec<Vec<u32>> = proto
            .polys
            .iter()
            .map(|f| f.iter().map(|&v| v + base).collect())
            .collect();
        (points, polys)
    };
    let blocks: Vec<(Vec<_>, Vec<Vec<u32>>)> = if parallel {
        visible.par_iter().map(block).collect()
    } else {
        visible.iter().map(block).collect()
    };
    let mut out = PolyMesh::default();
    out.points.reserve(base as usize);
    for (points, polys) in blocks {
        out.points.extend(points);
        out.polys.extend(polys);
    }
    out
}

/// The clone data of the cloner `id` (for consumers such as the viewport and effectors).
pub fn clone_data(
    scene: &Scene,
    id: NodeId,
    ctx: &EvalCtx,
) -> Result<Option<forge_clone_engine::CloneData>, BuildError> {
    let Some(node) = scene.node(id).filter(|n| n.type_id == TYPE) else {
        return Ok(None);
    };
    let settings = settings_from_params(&node.params);
    let mut clones = generate(&settings, ctx).map_err(map_error)?;
    apply_effectors(scene, id, &mut clones, ctx).map_err(map_error)?;
    clones
        .assign_children(prototypes_of(scene, id).len(), settings.clones_mode)
        .map_err(map_error)?;
    Ok(Some(clones))
}
