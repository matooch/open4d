//! Polygon object, Make Editable and Current State to Object (docs/specs/EDITABLE.md §1).

use std::sync::Arc;

use forge_core::transform::PREFIX;
use forge_core::{Category, EditError, MeshData, NodeId, NodeType, ParamDesc, Scene, Tx, Value};
use forge_geometry::PolyMesh;
use glam::DVec3;

use crate::{BuildError, evaluate, evaluate_geometry, is_consumed};

pub const TYPE: &str = "forge.polygon";

/// Component selection params of a polygon object (docs/specs/EDITABLE.md §2.1).
pub const SELECTION_POINTS: &str = "selection.points";
pub const SELECTION_EDGES: &str = "selection.edges";
pub const SELECTION_POLYS: &str = "selection.polys";

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum EditableError {
    /// The node makes no mesh (Null, spline, deformer, tag, missing node).
    #[error("{0} has no mesh")]
    NoMesh(String),
    #[error(transparent)]
    Build(#[from] BuildError),
    #[error(transparent)]
    Edit(#[from] EditError),
}

pub(crate) fn node_type() -> NodeType {
    NodeType::new(TYPE, "Polygon Object", Category::Object)
        .param(ParamDesc::mesh("mesh", "Mesh"))
        .param(ParamDesc::index_list(SELECTION_POINTS, "Selected points"))
        .param(ParamDesc::index_list(SELECTION_EDGES, "Selected edges"))
        .param(ParamDesc::index_list(SELECTION_POLYS, "Selected polygons"))
        .not_in_create_menu()
}

pub fn to_poly_mesh(m: &MeshData) -> PolyMesh {
    PolyMesh {
        points: m.points.iter().map(|p| DVec3::from_array(*p)).collect(),
        polys: m.polys.clone(),
    }
}

pub fn from_poly_mesh(m: &PolyMesh) -> MeshData {
    MeshData {
        points: m.points.iter().map(|p| p.to_array()).collect(),
        polys: m.polys.clone(),
    }
}

/// The stored mesh of a polygon object node.
pub(crate) fn stored_mesh(scene: &Scene, id: NodeId) -> PolyMesh {
    match scene.param(id, "mesh") {
        Some(Value::Mesh(m)) => to_poly_mesh(m),
        _ => PolyMesh::default(),
    }
}

fn name_of(scene: &Scene, id: NodeId) -> String {
    scene
        .node(id)
        .map_or_else(|| id.to_string(), |n| n.name.clone())
}

/// Index of `id` in its parent's child list (or the root list).
fn index_of(scene: &Scene, id: NodeId) -> Option<(Option<NodeId>, usize)> {
    let parent = scene.node(id)?.parent;
    let list = match parent {
        Some(p) => &scene.node(p)?.children,
        None => scene.roots(),
    };
    Some((parent, list.iter().position(|c| *c == id)?))
}

/// Creates a polygon object at `parent[index]` named like `src`, with copies of its tags,
/// the `transform.*` params and tracks of `xform_src` and the given mesh.
fn spawn_like(
    tx: &mut Tx,
    src: NodeId,
    xform_src: NodeId,
    parent: Option<NodeId>,
    index: usize,
    mesh: &PolyMesh,
) -> Result<NodeId, EditError> {
    let (name, params, tracks, tags) = {
        let n = tx.scene().node(src).ok_or(EditError::UnknownNode(src))?;
        let params: Vec<(String, Value)> = tx
            .scene()
            .node(xform_src)
            .ok_or(EditError::UnknownNode(xform_src))?
            .params
            .iter()
            .filter(|(k, _)| k.starts_with(PREFIX))
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        // transform animation is copied with the transform (docs/specs/ANIM.md §2.8)
        let tracks: Vec<(String, forge_core::Track)> = tx
            .scene()
            .node(xform_src)
            .ok_or(EditError::UnknownNode(xform_src))?
            .tracks
            .iter()
            .filter(|(k, _)| k.starts_with(PREFIX))
            .map(|(k, t)| (k.clone(), t.clone()))
            .collect();
        let tags: Vec<_> = n
            .tags
            .iter()
            .filter_map(|t| tx.scene().node(*t).cloned())
            .collect();
        (n.name.clone(), params, tracks, tags)
    };
    let id = tx.create(TYPE, parent, Some(index))?;
    tx.rename(id, &name)?;
    for (k, v) in params {
        tx.set_param(id, &k, v)?;
    }
    for (k, t) in tracks {
        tx.set_track(id, &k, Some(t))?;
    }
    for tag in tags {
        let t = tx.attach_tag(&tag.type_id, id, None)?;
        tx.rename(t, &tag.name)?;
        for (k, v) in tag.params {
            tx.set_param(t, &k, v)?;
        }
    }
    tx.set_param(id, "mesh", Value::Mesh(Arc::new(from_poly_mesh(mesh))))?;
    Ok(id)
}

/// Points every `Link` param of any other node (tags included) that targets `old` to `new`.
fn redirect_links(tx: &mut Tx, old: NodeId, new: NodeId) -> Result<(), EditError> {
    let scene = tx.scene();
    let mut found = Vec::new();
    for root in scene.roots() {
        for id in std::iter::once(*root).chain(scene.descendants(*root)) {
            let Some(node) = scene.node(id) else { continue };
            for owner in std::iter::once(id).chain(node.tags.iter().copied()) {
                if owner == new {
                    continue;
                }
                let params = scene.node(owner).map(|n| &n.params);
                for (k, v) in params.into_iter().flatten() {
                    if *v == Value::Link(Some(old)) {
                        found.push((owner, k.clone()));
                    }
                }
            }
        }
    }
    for (owner, k) in found {
        tx.set_param(owner, &k, Value::Link(Some(new)))?;
    }
    Ok(())
}

/// Make Editable (EDITABLE.md §1.2): replaces `id` by a polygon object holding its mesh
/// without deformers; consumed children are deleted, other children move to the new
/// object. Returns the new id (`id` itself if it already is a polygon object).
pub fn make_editable(tx: &mut Tx, id: NodeId) -> Result<NodeId, EditableError> {
    let scene = tx.scene();
    let node = scene
        .node(id)
        .filter(|n| !n.is_tag())
        .ok_or_else(|| EditableError::NoMesh(name_of(scene, id)))?;
    if node.type_id == TYPE {
        return Ok(id);
    }
    let mesh =
        evaluate_geometry(scene, id)?.ok_or_else(|| EditableError::NoMesh(name_of(scene, id)))?;
    let (parent, index) = index_of(scene, id).ok_or(EditError::UnknownNode(id))?;
    let (consumed, kept): (Vec<NodeId>, Vec<NodeId>) =
        node.children.iter().partition(|c| is_consumed(scene, **c));
    let new = spawn_like(tx, id, id, parent, index, &mesh)?;
    redirect_links(tx, id, new)?;
    for c in kept {
        tx.move_node(c, Some(new), None)?;
    }
    for c in consumed {
        tx.delete(c)?;
    }
    tx.delete(id)?;
    Ok(new)
}

/// Current State to Object (EDITABLE.md §1.3): a new polygon object right after `id`
/// holding its mesh with deformers applied; `id` is untouched. Returns the new id.
pub fn current_state_to_object(tx: &mut Tx, id: NodeId) -> Result<NodeId, EditableError> {
    let scene = tx.scene();
    if scene.node(id).is_none_or(|n| n.is_tag()) {
        return Err(EditableError::NoMesh(name_of(scene, id)));
    }
    let mut mesh = evaluate(scene, id)?.ok_or_else(|| EditableError::NoMesh(name_of(scene, id)))?;
    let (parent, index) = index_of(scene, id).ok_or(EditError::UnknownNode(id))?;
    // Deformers inherited from a geometry-less parent would deform the copy again under
    // that parent: put it after the parent instead, in the parent's space (§1.3).
    if let Some(p) = crate::sibling_deformer_parent(scene, id) {
        let m = scene
            .local_transform(id)
            .ok_or(EditError::UnknownNode(id))?
            .matrix();
        for pt in &mut mesh.points {
            *pt = m.transform_point3(*pt);
        }
        let (gp, pi) = index_of(scene, p).ok_or(EditError::UnknownNode(p))?;
        return Ok(spawn_like(tx, id, p, gp, pi + 1, &mesh)?);
    }
    Ok(spawn_like(tx, id, id, parent, index + 1, &mesh)?)
}
