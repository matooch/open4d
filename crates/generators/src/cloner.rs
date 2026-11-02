//! The cloner generator: evaluates its children once as prototypes, makes the clone data
//! and expands the visible clones to geometry in clone order
//! (clone-toolset spec C3).

use std::collections::BTreeMap;

use forge_clone_engine::{CLONER, CloneError, apply_effectors, generate, settings_from_params};
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

pub(crate) fn evaluate_cloner(
    scene: &Scene,
    id: NodeId,
    values: &BTreeMap<String, Value>,
    ctx: &EvalCtx,
) -> Result<Option<PolyMesh>, BuildError> {
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
    let mut out = PolyMesh::default();
    for i in 0..clones.len() {
        if !clones.visible[i] {
            continue;
        }
        let proto = &protos[clones.child[i] as usize];
        let base = out.points.len() as u32;
        let m = clones.matrices[i];
        out.points
            .extend(proto.points.iter().map(|p| m.transform_point3(*p)));
        out.polys.extend(
            proto
                .polys
                .iter()
                .map(|f| f.iter().map(|&v| v + base).collect::<Vec<u32>>()),
        );
    }
    Ok(Some(out))
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
