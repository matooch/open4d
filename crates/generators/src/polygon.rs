//! Polygon object, Make Editable and Current State to Object (docs/specs/EDITABLE.md §1).

use std::sync::Arc;

use forge_core::transform::PREFIX;
use forge_core::{
    Category, EditError, EvalCtx, MeshData, NodeId, NodeType, ParamDesc, Scene, Tx, Value,
};
use forge_geometry::PolyMesh;
use glam::DVec3;

use crate::{BuildError, editable_spline, evaluate, evaluate_geometry, is_consumed};

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

/// Tag type of a named set of polygons (the `C1` / `C2` tags of the cap objects).
pub const POLYGON_SELECTION: &str = "forge.polygon_selection";

pub(crate) fn selection_tag_type() -> NodeType {
    NodeType::new(POLYGON_SELECTION, "Polygon Selection", Category::Tag)
        .param(ParamDesc::index_list("indices", "Polygons"))
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

/// What a new editable object is made of: its type and the params written after creation.
struct Payload {
    type_id: &'static str,
    params: Vec<(String, Value)>,
}

fn mesh_payload(mesh: &PolyMesh) -> Payload {
    Payload {
        type_id: TYPE,
        params: vec![(
            "mesh".to_string(),
            Value::Mesh(Arc::new(from_poly_mesh(mesh))),
        )],
    }
}

/// Attaches a copy of every tag in `tags` to `host`, in order.
fn copy_tags(tx: &mut Tx, tags: Vec<forge_core::Node>, host: NodeId) -> Result<(), EditError> {
    for tag in tags {
        let t = tx.attach_tag(&tag.type_id, host, None)?;
        tx.rename(t, &tag.name)?;
        for (k, v) in tag.params {
            tx.set_param(t, &k, v)?;
        }
    }
    Ok(())
}

/// Creates an editable object at `parent[index]` named like `src`, with copies of its tags,
/// the `transform.*` params and tracks of `xform_src` and the payload's params.
fn spawn_like(
    tx: &mut Tx,
    src: NodeId,
    xform_src: NodeId,
    parent: Option<NodeId>,
    index: usize,
    payload: Payload,
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
        let mut tags: Vec<forge_core::Node> = n
            .tags
            .iter()
            .filter_map(|t| tx.scene().node(*t).cloned())
            .collect();
        // The tags of a consumed input come along (`subdivision`: the smoothing tag is the
        // cube's), except a type the object already has.
        for c in n.children.iter().filter(|c| is_consumed(tx.scene(), **c)) {
            for t in &tx.scene().node(*c).ok_or(EditError::UnknownNode(*c))?.tags {
                if let Some(t) = tx.scene().node(*t)
                    && !tags.iter().any(|have| have.type_id == t.type_id)
                {
                    tags.push(t.clone());
                }
            }
        }
        (n.name.clone(), params, tracks, tags)
    };
    let id = tx.create(payload.type_id, parent, Some(index))?;
    tx.rename(id, &name)?;
    for (k, v) in params {
        tx.set_param(id, &k, v)?;
    }
    for (k, t) in tracks {
        tx.set_track(id, &k, Some(t))?;
    }
    copy_tags(tx, tags, id)?;
    for (k, v) in payload.params {
        tx.set_param(id, &k, v)?;
    }
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

/// The editable spline replacing a spline primitive (SPLINES.md §3).
fn spline_payload(
    type_id: &str,
    values: &std::collections::BTreeMap<String, Value>,
) -> Result<Option<Payload>, BuildError> {
    Ok(
        editable_spline::from_primitive(type_id, values)?.map(|params| Payload {
            type_id: editable_spline::TYPE,
            params,
        }),
    )
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
    if node.type_id == TYPE || node.type_id == editable_spline::TYPE {
        return Ok(id);
    }
    let mut caps = None;
    let payload = match spline_payload(&node.type_id, &node.params)? {
        Some(p) => p,
        None => match crate::spline_gens::split_caps(scene, id)? {
            Some(split) => {
                let sides = mesh_payload(&split.sides);
                caps = Some(split);
                sides
            }
            None => mesh_payload(
                &evaluate_geometry(scene, id, &EvalCtx::default())?
                    .ok_or_else(|| EditableError::NoMesh(name_of(scene, id)))?,
            ),
        },
    };
    let (parent, index) = index_of(scene, id).ok_or(EditError::UnknownNode(id))?;
    let (consumed, kept): (Vec<NodeId>, Vec<NodeId>) =
        node.children.iter().partition(|c| is_consumed(scene, **c));
    let new = spawn_like(tx, id, id, parent, index, payload)?;
    redirect_links(tx, id, new)?;
    if let Some(split) = caps {
        let cap_objects = [("Cap 1", "C1", split.start), ("Cap 2", "C2", split.end)];
        let mut at = 0;
        for (name, tag, polys) in cap_objects {
            if !polys.is_empty() {
                spawn_cap(tx, id, new, at, name, tag, &split.sides.points, &polys)?;
                at += 1;
            }
        }
    }
    for c in kept {
        tx.move_node(c, Some(new), None)?;
    }
    for c in consumed {
        tx.delete(c)?;
    }
    tx.delete(id)?;
    Ok(new)
}

/// A cap object under `parent` at `index` (§1.2 step 5): the points the cap polygons use
/// (ascending index) with the polygons renumbered, a polygon selection tag `tag_name` over
/// all of them, and a copy of the source's tags.
#[allow(clippy::too_many_arguments)]
fn spawn_cap(
    tx: &mut Tx,
    src: NodeId,
    parent: NodeId,
    index: usize,
    name: &str,
    tag_name: &str,
    points: &[DVec3],
    polys: &[Vec<u32>],
) -> Result<NodeId, EditableError> {
    let used: std::collections::BTreeSet<u32> = polys.iter().flatten().copied().collect();
    let renumber: std::collections::BTreeMap<u32, u32> = used
        .iter()
        .enumerate()
        .map(|(new, old)| (*old, new as u32))
        .collect();
    let mesh = PolyMesh {
        points: used.iter().map(|i| points[*i as usize]).collect(),
        polys: polys
            .iter()
            .map(|p| p.iter().map(|i| renumber[i]).collect())
            .collect(),
    };
    let tags: Vec<_> = tx
        .scene()
        .node(src)
        .ok_or(EditError::UnknownNode(src))?
        .tags
        .iter()
        .filter_map(|t| tx.scene().node(*t).cloned())
        .collect();
    let cap = tx.create(TYPE, Some(parent), Some(index))?;
    tx.rename(cap, name)?;
    tx.set_param(cap, "mesh", Value::Mesh(Arc::new(from_poly_mesh(&mesh))))?;
    let sel = tx.attach_tag(POLYGON_SELECTION, cap, None)?;
    tx.rename(sel, tag_name)?;
    tx.set_param(
        sel,
        "indices",
        Value::IndexList((0..mesh.polys.len() as u32).collect()),
    )?;
    copy_tags(tx, tags, cap)?;
    Ok(cap)
}

/// Split (docs/specs/MODELING.md §4.7): a polygon object right after the polygon object `id`
/// (same parent, transform, tags) holding `mesh`, named `<name>.N` with the lowest free `N`.
pub fn spawn_split(tx: &mut Tx, id: NodeId, mesh: &PolyMesh) -> Result<NodeId, EditableError> {
    let scene = tx.scene();
    let (parent, index) = index_of(scene, id).ok_or(EditError::UnknownNode(id))?;
    let base = name_of(scene, id);
    let taken: std::collections::BTreeSet<String> = scene
        .roots()
        .iter()
        .flat_map(|r| std::iter::once(*r).chain(scene.descendants(*r)))
        .filter_map(|n| scene.node(n).map(|n| n.name.clone()))
        .collect();
    let name = (1..)
        .map(|n| format!("{base}.{n}"))
        .find(|candidate| !taken.contains(candidate))
        .unwrap_or_else(|| base.clone());
    let new = spawn_like(tx, id, id, parent, index + 1, mesh_payload(mesh))?;
    tx.rename(new, &name)?;
    Ok(new)
}

/// Current State to Object (EDITABLE.md §1.3): a new polygon object right after `id`
/// holding its mesh with deformers applied; `id` is untouched. Returns the new id.
pub fn current_state_to_object(tx: &mut Tx, id: NodeId) -> Result<NodeId, EditableError> {
    let scene = tx.scene();
    if scene.node(id).is_none_or(|n| n.is_tag()) {
        return Err(EditableError::NoMesh(name_of(scene, id)));
    }
    let (parent, index) = index_of(scene, id).ok_or(EditError::UnknownNode(id))?;
    // a spline primitive gives an editable spline next to it (deformers do not act on splines)
    if let Some(node) = scene.node(id)
        && let Some(payload) = spline_payload(&node.type_id, &node.params)?
    {
        return Ok(spawn_like(tx, id, id, parent, index + 1, payload)?);
    }
    let mut mesh = evaluate(scene, id, &EvalCtx::default())?
        .ok_or_else(|| EditableError::NoMesh(name_of(scene, id)))?;
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
        return Ok(spawn_like(tx, id, p, gp, pi + 1, mesh_payload(&mesh))?);
    }
    Ok(spawn_like(
        tx,
        id,
        id,
        parent,
        index + 1,
        mesh_payload(&mesh),
    )?)
}
