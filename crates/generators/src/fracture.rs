//! The Fracture generator: its input objects split into fragments that move as clones
//! (clone-toolset spec F1-F2).

use std::collections::BTreeMap;

use crate::cloner::effects::apply_effectors;
use forge_clone_engine::{
    FRACTURE, FractureMode, fracture_settings, fragment_clones, polygon_islands,
};
use forge_core::{EvalCtx, NodeId, Scene, Value};
use forge_geometry::PolyMesh;
use glam::DVec3;

use crate::{BuildError, evaluate};

pub(crate) const TYPE: &str = FRACTURE;

/// The input objects of a Fracture: its children with geometry, in child order.
pub fn inputs_of(scene: &Scene, id: NodeId) -> Vec<NodeId> {
    let Some(node) = scene.node(id) else {
        return Vec::new();
    };
    node.children
        .iter()
        .copied()
        .filter(|c| {
            matches!(
                crate::evaluate_geometry(scene, *c, &EvalCtx::default()),
                Ok(Some(_)) | Err(_)
            )
        })
        .collect()
}

/// One fragment: a mesh in its own pivot frame and the pivot.
struct Fragment {
    mesh: PolyMesh,
    pivot: DVec3,
}

/// The part of `mesh` made by the polygons `which`: points in ascending original index,
/// polygons renumbered, corner order kept.
fn part(mesh: &PolyMesh, which: &[usize]) -> PolyMesh {
    let mut used: Vec<u32> = which
        .iter()
        .flat_map(|i| mesh.polys[*i].iter().copied())
        .collect();
    used.sort_unstable();
    used.dedup();
    let map: BTreeMap<u32, u32> = used
        .iter()
        .enumerate()
        .map(|(k, v)| (*v, k as u32))
        .collect();
    PolyMesh {
        points: used.iter().map(|v| mesh.points[*v as usize]).collect(),
        polys: which
            .iter()
            .map(|i| mesh.polys[*i].iter().map(|v| map[v]).collect())
            .collect(),
    }
}

/// Moves the points so that the bounding box centre is the origin; returns the centre.
fn centred(mut mesh: PolyMesh) -> Fragment {
    let (mut lo, mut hi) = (DVec3::splat(f64::MAX), DVec3::splat(f64::MIN));
    for p in &mesh.points {
        lo = lo.min(*p);
        hi = hi.max(*p);
    }
    let pivot = if mesh.points.is_empty() {
        DVec3::ZERO
    } else {
        (lo + hi) * 0.5
    };
    for p in &mut mesh.points {
        *p -= pivot;
    }
    Fragment { mesh, pivot }
}

pub(crate) fn evaluate_fracture(
    scene: &Scene,
    id: NodeId,
    values: &BTreeMap<String, Value>,
    ctx: &EvalCtx,
) -> Result<Option<PolyMesh>, BuildError> {
    let (mode, tab) = fracture_settings(values);
    if mode == FractureMode::ExplodeSegmentsConnect {
        return Err(BuildError::Unsupported(
            "Fracture mode Explode Segments & Connect",
        ));
    }
    let mut fragments = Vec::new();
    for child in inputs_of(scene, id) {
        let Some(mut mesh) = evaluate(scene, child, ctx)? else {
            continue;
        };
        let m = scene.local_transform(child).expect("child").matrix();
        for p in &mut mesh.points {
            *p = m.transform_point3(*p);
        }
        match mode {
            FractureMode::Straight => fragments.push(centred(mesh)),
            _ => {
                for island in polygon_islands(mesh.points.len(), &mesh.polys) {
                    fragments.push(centred(part(&mesh, &island)));
                }
            }
        }
    }
    if fragments.is_empty() {
        return Ok(None);
    }
    let pivots: Vec<DVec3> = fragments.iter().map(|f| f.pivot).collect();
    let mut clones = fragment_clones(&pivots, &tab);
    apply_effectors(scene, id, &mut clones, ctx).map_err(|e| match e {
        forge_clone_engine::CloneError::Unsupported(w) => BuildError::Unsupported(w),
        forge_clone_engine::CloneError::TooMany => BuildError::Unsupported("too many fragments"),
    })?;
    let mut out = PolyMesh::default();
    for (i, f) in fragments.iter().enumerate() {
        if !clones.visible[i] {
            continue;
        }
        let base = out.points.len() as u32;
        out.points.extend(
            f.mesh
                .points
                .iter()
                .map(|p| clones.matrices[i].transform_point3(*p)),
        );
        out.polys.extend(
            f.mesh
                .polys
                .iter()
                .map(|q| q.iter().map(|v| v + base).collect::<Vec<_>>()),
        );
    }
    Ok(Some(out))
}
