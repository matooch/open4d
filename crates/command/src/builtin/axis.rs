//! object.set_axis, object.center_axis, object.align_axis and edit.axis_mode: axis (pivot)
//! editing without moving the content (docs/specs/MANIPULATE.md §7).

use forge_core::transform::{Transform, hpb_from_rotation, rotation_part};
use forge_core::{NodeId, Value};
use forge_generators::axis::{AxisEdit, shifted_axis, with_world_rotation};
use forge_generators::{POLYGON, SELECTION_EDGES, SELECTION_POINTS, SELECTION_POLYS, to_poly_mesh};
use forge_geometry::PolyMesh;
use forge_geometry::normals::{poly_area, poly_normal};
use forge_geometry::selection::{
    IndexSet, Mode, edges_to_points, points_to_polys, polys_to_points,
};
use forge_modeling::snap::normal_basis;
use glam::{DMat3, DVec3};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value as Json, json};

use super::{spec, target_nodes};
use crate::context::CommandContext;
use crate::error::CommandError;
use crate::registry::CommandRegistry;

#[derive(Deserialize, JsonSchema, Clone, Copy, Default, PartialEq)]
#[serde(rename_all = "snake_case")]
enum AxisSpace {
    /// `pos` and `rot` are the object's own parameters (relative to its parent).
    #[default]
    Local,
    /// `pos` is a world point and `rot` a world rotation.
    World,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct SetAxisArgs {
    /// The object; the active node when absent.
    node: Option<u64>,
    /// New axis position `[x, y, z]`.
    pos: Option<[f64; 3]>,
    /// New axis rotation `[h, p, b]` in degrees.
    rot: Option<[f64; 3]>,
    /// New axis scale `[x, y, z]` (always the object's own scale).
    scale: Option<[f64; 3]>,
    /// Whether `pos` and `rot` are local (default) or world values.
    space: Option<AxisSpace>,
}

#[derive(Deserialize, JsonSchema, Clone, Copy, Default, PartialEq)]
#[serde(rename_all = "snake_case")]
enum CenterTo {
    /// The centre of the bounding box of the points (polygon objects).
    #[default]
    BoundingBox,
    /// The average of the points (polygon objects).
    Points,
    /// The average of the points of the stored point, edge and polygon selection.
    Selection,
    /// The average position of the direct children (any object).
    Children,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct CenterArgs {
    /// Objects; the current selection when absent.
    nodes: Option<Vec<u64>>,
    /// What to centre on (default `bounding_box`).
    to: Option<CenterTo>,
}

#[derive(Deserialize, JsonSchema, Clone, Copy, PartialEq)]
#[serde(rename_all = "snake_case")]
enum AlignTo {
    /// The world rotation becomes identity.
    World,
    /// The rotation relative to the parent becomes zero.
    Parent,
    /// The Z axis points along the average normal of the selected polygons.
    Normal,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct AlignArgs {
    /// Objects; the current selection when absent.
    nodes: Option<Vec<u64>>,
    to: AlignTo,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct ModeArgs {
    /// Switch axis mode on or off; toggles when absent.
    enabled: Option<bool>,
}

pub(super) fn register(r: &mut CommandRegistry) -> Result<(), CommandError> {
    r.register_fn(
        spec(
            "object.set_axis",
            "Set Axis",
            true,
            || json!({"pos": [0.0, 50.0, 0.0], "space": "world"}),
        ),
        set_axis,
    )?;
    r.register_fn(
        spec(
            "object.center_axis",
            "Center Axis",
            true,
            || json!({"to": "bounding_box"}),
        ),
        center_axis,
    )?;
    r.register_fn(
        spec(
            "object.align_axis",
            "Align Axis",
            true,
            || json!({"to": "world"}),
        ),
        align_axis,
    )?;
    r.register_fn(
        spec(
            "edit.axis_mode",
            "Axis Mode",
            false,
            || json!({"enabled": true}),
        ),
        axis_mode,
    )
}

fn axis_mode(ctx: &mut dyn CommandContext, a: ModeArgs) -> Result<Json, CommandError> {
    let t = ctx.transform_settings_mut();
    t.axis_mode = a.enabled.unwrap_or(!t.axis_mode);
    Ok(json!({"enabled": t.axis_mode}))
}

fn active_or(ctx: &dyn CommandContext, node: Option<u64>) -> Result<NodeId, CommandError> {
    match node {
        Some(n) => Ok(NodeId(n)),
        None => ctx
            .selection()
            .active
            .ok_or_else(|| CommandError::failed("select an object")),
    }
}

/// The nodes of a command: given, else the selection; at least one.
fn nodes_of(
    ctx: &dyn CommandContext,
    nodes: Option<Vec<u64>>,
) -> Result<Vec<NodeId>, CommandError> {
    let ids = target_nodes(ctx, nodes);
    if ids.is_empty() {
        return Err(CommandError::failed("select an object"));
    }
    Ok(ids)
}

fn nodes_json(ids: &[NodeId]) -> Vec<u64> {
    ids.iter().map(|n| n.0).collect()
}

/// Runs the prepared edits as one transaction.
fn commit(
    ctx: &mut dyn CommandContext,
    label: &str,
    edits: &[(AxisEdit, Transform)],
) -> Result<(), CommandError> {
    ctx.doc_mut().transact(label, |tx| {
        for (edit, new) in edits {
            edit.apply(tx, new)?;
        }
        Ok(())
    })?;
    Ok(())
}

fn set_axis(ctx: &mut dyn CommandContext, a: SetAxisArgs) -> Result<Json, CommandError> {
    if a.pos.is_none() && a.rot.is_none() && a.scale.is_none() {
        return Err(CommandError::InvalidArgs(
            "object.set_axis needs at least one of pos, rot, scale".into(),
        ));
    }
    let node = active_or(ctx, a.node)?;
    let scene = ctx.doc().scene();
    let edit = AxisEdit::capture(scene, node)
        .map_err(|_| CommandError::failed(format!("node {} cannot have an axis", node.0)))?;
    let parent = scene
        .parent_world_matrix(node)
        .ok_or_else(|| CommandError::failed("no parent transform"))?;
    let mut new = edit.start;
    let world = a.space == Some(AxisSpace::World);
    if let Some(p) = a.pos {
        let p = DVec3::from_array(p);
        new.pos = if world {
            parent.inverse().transform_point3(p)
        } else {
            p
        };
    }
    if let Some(r) = a.rot {
        let rot = DVec3::from_array(r.map(f64::to_radians));
        new.rot = if world {
            let wanted = Transform {
                rot,
                ..Transform::IDENTITY
            }
            .rotation();
            with_world_rotation(&new, rotation_part(&parent), wanted).rot
        } else {
            rot
        };
    }
    if let Some(s) = a.scale {
        new.scale = DVec3::from_array(s);
    }
    commit(ctx, "Set Axis", &[(edit, new)])?;
    Ok(json!({"nodes": [node.0]}))
}

fn stored_mesh(ctx: &dyn CommandContext, node: NodeId) -> Option<PolyMesh> {
    let scene = ctx.doc().scene();
    match scene.param(node, "mesh") {
        Some(Value::Mesh(m)) if scene.node(node)?.type_id == POLYGON => Some(to_poly_mesh(m)),
        _ => None,
    }
}

fn stored_set(ctx: &dyn CommandContext, node: NodeId, key: &str) -> IndexSet {
    match ctx.doc().scene().param(node, key) {
        Some(Value::IndexList(l)) => l.iter().copied().collect(),
        _ => IndexSet::new(),
    }
}

/// The points of the stored point, edge and polygon selection.
fn selected_points(ctx: &dyn CommandContext, node: NodeId, mesh: &PolyMesh) -> IndexSet {
    let all = stored_set(ctx, node, SELECTION_POINTS)
        .union(&edges_to_points(
            mesh,
            &stored_set(ctx, node, SELECTION_EDGES),
        ))
        .union(&polys_to_points(
            mesh,
            &stored_set(ctx, node, SELECTION_POLYS),
        ));
    all.iter()
        .filter(|i| (*i as usize) < mesh.points.len())
        .collect()
}

fn need_polygon(label: &str, node: NodeId) -> CommandError {
    CommandError::failed(format!(
        "{label}: node {} is not a polygon object (use Make Editable first)",
        node.0
    ))
}

/// The object-space centre for `to`.
fn centre(ctx: &dyn CommandContext, node: NodeId, to: CenterTo) -> Result<DVec3, CommandError> {
    let label = "Center Axis";
    if to == CenterTo::Children {
        let scene = ctx.doc().scene();
        let kids: Vec<DVec3> = scene
            .node(node)
            .map(|n| n.children.clone())
            .unwrap_or_default()
            .into_iter()
            .filter(|c| scene.param(*c, forge_core::transform::POS).is_some())
            .filter_map(|c| Some(scene.local_transform(c)?.pos))
            .collect();
        if kids.is_empty() {
            return Err(CommandError::failed(format!(
                "{label}: node {} has no children",
                node.0
            )));
        }
        return Ok(kids.iter().sum::<DVec3>() / kids.len() as f64);
    }
    let mesh = stored_mesh(ctx, node).ok_or_else(|| need_polygon(label, node))?;
    let average = |pts: &mut dyn Iterator<Item = DVec3>| {
        let (sum, n) = pts.fold((DVec3::ZERO, 0usize), |(s, n), p| (s + p, n + 1));
        (n > 0).then(|| sum / n as f64)
    };
    let empty = || CommandError::failed(format!("{label}: node {} has no points", node.0));
    match to {
        CenterTo::BoundingBox => {
            if mesh.points.is_empty() {
                return Err(empty());
            }
            let (lo, hi) = mesh.points.iter().fold(
                (DVec3::splat(f64::MAX), DVec3::splat(f64::MIN)),
                |(lo, hi), p| (lo.min(*p), hi.max(*p)),
            );
            Ok((lo + hi) * 0.5)
        }
        CenterTo::Points => average(&mut mesh.points.iter().copied()).ok_or_else(empty),
        CenterTo::Selection => {
            let sel = selected_points(ctx, node, &mesh);
            average(&mut sel.iter().map(|i| mesh.points[i as usize])).ok_or_else(|| {
                CommandError::failed(format!("{label}: node {} has no selection", node.0))
            })
        }
        CenterTo::Children => unreachable!("handled above"),
    }
}

fn center_axis(ctx: &mut dyn CommandContext, a: CenterArgs) -> Result<Json, CommandError> {
    let ids = nodes_of(ctx, a.nodes)?;
    let to = a.to.unwrap_or_default();
    let mut edits = Vec::new();
    let mut centers = serde_json::Map::new();
    for id in &ids {
        let c = centre(ctx, *id, to)?;
        let edit = AxisEdit::capture(ctx.doc().scene(), *id)
            .map_err(|_| CommandError::failed(format!("node {} cannot have an axis", id.0)))?;
        let new = shifted_axis(&edit.start, c);
        centers.insert(id.0.to_string(), json!([c.x, c.y, c.z]));
        edits.push((edit, new));
    }
    commit(ctx, "Center Axis", &edits)?;
    Ok(json!({"nodes": nodes_json(&ids), "centers": centers}))
}

/// The average normal (area weighted) and the first edge direction of the polygons that
/// decide the `normal` alignment: the stored polygon selection, else the polygons that use a
/// selected point or edge.
fn selection_normal(
    ctx: &dyn CommandContext,
    node: NodeId,
    mesh: &PolyMesh,
) -> Result<(DVec3, DVec3), CommandError> {
    let mut polys = stored_set(ctx, node, SELECTION_POLYS);
    if polys.is_empty() {
        let pts = stored_set(ctx, node, SELECTION_POINTS).union(&edges_to_points(
            mesh,
            &stored_set(ctx, node, SELECTION_EDGES),
        ));
        polys = points_to_polys(mesh, &pts, Mode::Any);
    }
    let polys: Vec<u32> = polys
        .iter()
        .filter(|p| (*p as usize) < mesh.polys.len())
        .collect();
    let Some(first) = polys.first().map(|p| &mesh.polys[*p as usize]) else {
        return Err(CommandError::failed(format!(
            "Align Axis: select polygons, edges or points of node {}",
            node.0
        )));
    };
    let n: DVec3 = polys
        .iter()
        .map(|p| {
            let poly = &mesh.polys[*p as usize];
            poly_normal(&mesh.points, poly) * poly_area(&mesh.points, poly)
        })
        .sum();
    if n.length() < 1e-12 {
        return Err(CommandError::failed(
            "Align Axis: the selection has no normal",
        ));
    }
    let hint = if first.len() >= 2 {
        mesh.points[first[1] as usize] - mesh.points[first[0] as usize]
    } else {
        DVec3::ZERO
    };
    Ok((n, hint))
}

fn align_axis(ctx: &mut dyn CommandContext, a: AlignArgs) -> Result<Json, CommandError> {
    let ids = nodes_of(ctx, a.nodes)?;
    let mut edits = Vec::new();
    for id in &ids {
        let scene = ctx.doc().scene();
        let edit = AxisEdit::capture(scene, *id)
            .map_err(|_| CommandError::failed(format!("node {} cannot have an axis", id.0)))?;
        let start = edit.start;
        let parent = scene
            .parent_world_matrix(*id)
            .ok_or_else(|| CommandError::failed("no parent transform"))?;
        let new = match a.to {
            AlignTo::World => with_world_rotation(&start, rotation_part(&parent), DMat3::IDENTITY),
            AlignTo::Parent => Transform {
                rot: DVec3::ZERO,
                ..start
            },
            AlignTo::Normal => {
                let mesh = stored_mesh(ctx, *id).ok_or_else(|| need_polygon("Align Axis", *id))?;
                let (n, hint) = selection_normal(ctx, *id, &mesh)?;
                Transform {
                    rot: hpb_from_rotation(start.rotation() * normal_basis(n, hint)),
                    ..start
                }
            }
        };
        edits.push((edit, new));
    }
    commit(ctx, "Align Axis", &edits)?;
    Ok(json!({"nodes": nodes_json(&ids)}))
}
