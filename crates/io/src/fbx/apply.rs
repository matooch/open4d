//! Installs an [`ImportScene`] as one undoable edit (IO.md §8.3).

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use forge_core::smoothing::TAG as SMOOTHING_TAG;
use forge_core::tags::POLYGON_SELECTION;
use forge_core::transform::{POS, ROT, SCALE};
use forge_core::{EditError, NodeId, Tx, Value};
use forge_material::node_material::{GRAPH, NODE_MATERIAL};
use forge_rig::weights::matrix_to_entries;

use super::{ImportScene, Kind, MeshPart};

impl ImportScene {
    /// Creates the objects, materials and tags; returns the new root objects.
    pub fn apply(&self, tx: &mut Tx<'_>) -> Result<Vec<NodeId>, EditError> {
        let mats = self
            .materials
            .iter()
            .map(|m| {
                let id = tx.create_material(NODE_MATERIAL, None)?;
                tx.rename(id, &m.name)?;
                if !m.graph.is_empty() {
                    tx.set_param(id, GRAPH, Value::String(m.graph.clone()))?;
                }
                Ok(id)
            })
            .collect::<Result<Vec<_>, EditError>>()?;
        let mut ids: BTreeMap<usize, NodeId> = BTreeMap::new();
        let mut skinned = Vec::new();
        let mut roots = Vec::new();
        let mut pending: Vec<_> = self.roots.iter().rev().map(|&i| (i, None)).collect();
        while let Some((i, parent)) = pending.pop() {
            let n = &self.nodes[i];
            let kind = match &n.kind {
                Kind::Null => "forge.null",
                Kind::Joint => forge_rig::JOINT,
                Kind::Mesh(_) => forge_generators::POLYGON,
                Kind::Camera(_) => "forge.camera",
                Kind::Light(_) => "forge.light",
            };
            let id = tx.create(kind, parent, None)?;
            if parent.is_none() {
                roots.push(id);
            }
            ids.insert(i, id);
            tx.rename(id, &n.name)?;
            for (param, val) in [(POS, n.pos), (ROT, n.rot), (SCALE, n.scale)] {
                tx.set_param(id, param, Value::Vec3(val.to_array()))?;
            }
            for (key, track) in &n.tracks {
                tx.set_track(id, key, Some(track.clone()))?;
            }
            match &n.kind {
                Kind::Mesh(part) => {
                    apply_mesh(tx, id, part, &mats)?;
                    if part.skin.is_some() {
                        skinned.push((id, part));
                    }
                }
                Kind::Camera(c) => {
                    tx.set_param(id, "sensor_size", Value::Float(c.sensor))?;
                    tx.set_param(id, "fov_horizontal", Value::Float(c.fov))?;
                    if let Some(f) = c.focus {
                        tx.set_param(id, "focus_distance", Value::Float(f))?;
                    }
                    if let Some(zoom) = c.ortho_zoom {
                        tx.set_param(id, "projection", Value::Enum("parallel".into()))?;
                        tx.set_param(id, "zoom", Value::Float(zoom))?;
                    }
                    if c.near.is_finite() && c.near > 0.0 {
                        tx.set_param(id, "enable_near_clipping", Value::Bool(true))?;
                        tx.set_param(id, "near_clipping", Value::Float(c.near))?;
                    }
                    if c.far.is_finite() && c.far > 0.0 {
                        tx.set_param(id, "enable_far_clipping", Value::Bool(true))?;
                        tx.set_param(id, "far_clipping", Value::Float(c.far))?;
                    }
                }
                Kind::Light(l) => {
                    tx.set_param(id, "type", Value::Enum(l.kind.into()))?;
                    tx.set_param(id, "color", Value::Color(l.color))?;
                    tx.set_param(id, "intensity", Value::Float(l.intensity.max(0.0)))?;
                    if l.kind == "spot" {
                        tx.set_param(id, "inner_angle", Value::Float(l.inner))?;
                        tx.set_param(id, "outer_angle", Value::Float(l.outer))?;
                    }
                }
                Kind::Null | Kind::Joint => {}
            }
            pending.extend(n.children.iter().rev().map(|&c| (c, Some(id))));
        }
        for (mesh, part) in skinned {
            let Some(skin) = &part.skin else { continue };
            let joints = skin
                .joints
                .iter()
                .map(|j| {
                    ids.get(j).copied().ok_or_else(|| {
                        EditError::WrongCategory("FBX skin names a joint outside the scene".into())
                    })
                })
                .collect::<Result<Vec<_>, _>>()?;
            let tag = tx.attach_tag(forge_rig::WEIGHT_TAG, mesh, None)?;
            tx.set_param(tag, "joints", Value::LinkList(joints))?;
            tx.set_param(tag, "weights", skin.weights.to_value())?;
            let bind = skin.bind.iter().flat_map(matrix_to_entries).collect();
            tx.set_param(tag, "bind_pose", Value::Vec3List(bind))?;
            // FBX linear skinning is the plain weighted sum of the cluster transforms, the
            // unweighted remainder staying at the bind position (ufbx's
            // `get_skin_vertex_matrix`): our skin must not renormalise the rows (IO.md §8.3)
            let deformer = tx.create(forge_rig::SKIN, Some(mesh), None)?;
            tx.set_param(deformer, "normalise", Value::Bool(false))?;
        }
        Ok(roots)
    }
}

fn apply_mesh(
    tx: &mut Tx<'_>,
    id: NodeId,
    part: &MeshPart,
    mats: &[NodeId],
) -> Result<(), EditError> {
    tx.set_param(
        id,
        "mesh",
        Value::Mesh(Arc::new(forge_generators::from_poly_mesh(&part.mesh))),
    )?;
    if part.smooth {
        tx.attach_tag(SMOOTHING_TAG, id, None)?;
    }
    if let Some(uv) = &part.uv {
        let tag = tx.attach_tag(forge_material::UVW_TAG, id, None)?;
        tx.set_param(
            tag,
            "uvw",
            Value::FloatList(uv.corners.iter().flatten().flatten().copied().collect()),
        )?;
    }
    // one texture tag per used material slot; several slots select their polygons
    let used: BTreeSet<u32> = part.face_slot.iter().copied().collect();
    let used: Vec<(u32, usize)> = used
        .into_iter()
        .filter_map(|s| Some((s, part.slots.get(s as usize).copied().flatten()?)))
        .collect();
    let split = used.len() > 1;
    let mut names = BTreeSet::new();
    for (slot, material) in used {
        let tag = tx.attach_tag(forge_material::TEXTURE_TAG, id, None)?;
        tx.set_param(tag, "material", Value::Link(Some(mats[material])))?;
        if part.uv.is_some() {
            tx.set_param(tag, "projection", Value::Enum("uvw".into()))?;
        }
        if split {
            let base = tx
                .scene()
                .node(mats[material])
                .map(|n| n.name.clone())
                .unwrap_or_default();
            let mut name = base.clone();
            let mut k = 1;
            while !names.insert(name.clone()) {
                k += 1;
                name = format!("{base} {k}");
            }
            let sel = tx.attach_tag(POLYGON_SELECTION, id, None)?;
            tx.rename(sel, &name)?;
            let polys: Vec<u32> = part
                .face_slot
                .iter()
                .enumerate()
                .filter(|(_, s)| **s == slot)
                .map(|(p, _)| p as u32)
                .collect();
            tx.set_param(sel, "indices", Value::IndexList(polys))?;
            tx.set_param(tag, "selection", Value::String(name))?;
        }
    }
    Ok(())
}
