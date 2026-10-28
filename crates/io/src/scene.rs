//! Scene side of the OBJ exchange (docs/specs/IO.md §1.4).

use std::collections::BTreeSet;
use std::sync::Arc;

use forge_core::smoothing::{TAG as SMOOTHING_TAG, smoothing_angle};
use forge_core::{EditError, NodeId, Scene, Tx, Value};
use forge_generators::{POLYGON, evaluate, from_poly_mesh, is_consumed};

use crate::obj::ObjObject;

/// Every node with geometry as an OBJ object: the whole scene (`None`) or the listed nodes
/// with their descendants. Consumed inputs, deformers and nodes without polygons (or whose
/// build fails) are skipped.
pub fn export_scene(scene: &Scene, roots: Option<&[NodeId]>) -> Vec<ObjObject> {
    let start: Vec<NodeId> = match roots {
        None => scene.roots().to_vec(),
        Some(r) => r.to_vec(),
    };
    let mut order = Vec::new();
    let mut seen = BTreeSet::new();
    for r in start {
        if !scene.contains(r) {
            continue;
        }
        for id in std::iter::once(r).chain(scene.descendants(r)) {
            if seen.insert(id) {
                order.push(id);
            }
        }
    }
    order
        .into_iter()
        .filter_map(|id| {
            let node = scene.node(id)?;
            if node.is_tag() || is_consumed(scene, id) {
                return None;
            }
            let mesh = evaluate(scene, id).ok().flatten()?;
            if mesh.polys.is_empty() {
                return None;
            }
            Some(ObjObject {
                name: node.name.clone(),
                mesh,
                world: scene.world_matrix(id)?,
                smoothing_angle: smoothing_angle(scene, id),
            })
        })
        .collect()
}

/// One root polygon object per OBJ object (identity transform, the world-space points as the
/// mesh, a smoothing tag with default values). Returns the new ids in order.
pub fn import_into(tx: &mut Tx, objects: &[ObjObject]) -> Result<Vec<NodeId>, EditError> {
    let mut ids = Vec::new();
    for obj in objects {
        let mut mesh = obj.mesh.clone();
        for p in &mut mesh.points {
            *p = obj.world.transform_point3(*p);
        }
        // a polygon object's mesh must be valid: drop polygons with < 3 distinct corners
        mesh.polys.retain(|poly| {
            let distinct: BTreeSet<u32> = poly.iter().copied().collect();
            distinct.len() >= 3
        });
        if mesh.polys.is_empty() {
            continue;
        }
        let id = tx.create(POLYGON, None, None)?;
        tx.rename(id, &obj.name)?;
        tx.set_param(id, "mesh", Value::Mesh(Arc::new(from_poly_mesh(&mesh))))?;
        tx.attach_tag(SMOOTHING_TAG, id, None)?;
        ids.push(id);
    }
    Ok(ids)
}
