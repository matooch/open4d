//! Component modes and selection (docs/specs/EDITABLE.md §2). UI-free: the egui code in
//! `app` only turns pointer input into these calls.

use forge_core::{EvalCtx, NodeId, Scene};
use forge_generators::evaluate;
use forge_geometry::PolyMesh;
use forge_geometry::selection::{IndexSet, Mode as Combine, Selection, polys_to_edges};
use forge_viewport::camera::Camera;
use forge_viewport::picking::{Element, pick_edge, pick_point, pick_poly, rect_select};

use crate::manipulate::pick_node;
use crate::state::EditorState;

pub use forge_app::select::{
    EditMode, SelectCmd, base_mesh, selection_of, stored_selection, write_selection,
    write_selections,
};

/// The viewport element a component mode picks.
fn element(mode: EditMode) -> Option<Element> {
    match mode {
        EditMode::Model => None,
        EditMode::Points => Some(Element::Points),
        EditMode::Edges => Some(Element::Edges),
        EditMode::Polys => Some(Element::Polys),
    }
}

/// The active node, when a component mode is on and it is a polygon object.
pub fn target(s: &EditorState) -> Option<NodeId> {
    forge_app::select::target(&s.app)
}

/// The component targets: the selected polygon objects while a component mode is on and the
/// active node is a polygon object (EDITABLE.md §6.1).
pub fn targets(s: &EditorState) -> Vec<NodeId> {
    forge_app::select::targets(&s.app)
}

/// Select All / Deselect All / Invert / Grow / Shrink on the current mode's set.
pub fn run_select(s: &mut EditorState, cmd: SelectCmd) -> Result<(), String> {
    forge_app::select::run_select(&mut s.app, cmd, None)
}

/// Pick radius of points and edges, in render pixels (EDITABLE.md §2.4).
pub const POINT_RADIUS: f64 = 8.0;
pub const EDGE_RADIUS: f64 = 6.0;

/// How a picked element combines with the current set.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectOp {
    Replace,
    Add,
    Remove,
}

/// The modifier keys of a click or drag (`command` is Ctrl, or Cmd on macOS).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ClickModifiers {
    pub shift: bool,
    pub command: bool,
}

impl From<ClickModifiers> for SelectOp {
    /// Ctrl/Cmd removes (and beats Shift), Shift adds, otherwise replace.
    fn from(m: ClickModifiers) -> SelectOp {
        if m.command {
            SelectOp::Remove
        } else if m.shift {
            SelectOp::Add
        } else {
            SelectOp::Replace
        }
    }
}

/// The three sets as the renderer draws them in `mode` (EDITABLE.md §2.6): Points and Edges
/// show their own set, Polys also outlines the selected polygons.
pub fn display_selection(mesh: &PolyMesh, stored: &Selection, mode: EditMode) -> Selection {
    match mode {
        EditMode::Model => Selection::default(),
        EditMode::Points => Selection {
            points: stored.points.clone(),
            ..Selection::default()
        },
        EditMode::Edges => Selection {
            edges: stored.edges.clone(),
            ..Selection::default()
        },
        EditMode::Polys => Selection {
            polys: stored.polys.clone(),
            edges: polys_to_edges(mesh, &stored.polys),
            ..Selection::default()
        },
    }
}

/// Combines a picked set with the current one.
pub fn apply_op(current: &IndexSet, picked: &IndexSet, op: SelectOp) -> IndexSet {
    match op {
        SelectOp::Replace => picked.clone(),
        SelectOp::Add => current.union(picked),
        SelectOp::Remove => current.difference(picked),
    }
}

/// The set after a click that picked `picked` (`None`: empty space).
pub fn apply_click(current: &IndexSet, picked: Option<u32>, op: SelectOp) -> IndexSet {
    apply_op(current, &picked.into_iter().collect(), op)
}

/// The mesh components are picked in: the evaluated (deformed) mesh in world space.
/// Indices equal those of the stored mesh (deformers keep the topology).
pub fn pick_mesh(scene: &Scene, id: NodeId) -> Option<PolyMesh> {
    let base = base_mesh(scene, id)?;
    let world = scene.world_matrix(id)?;
    let mut mesh = match evaluate(scene, id, &EvalCtx::default()) {
        Ok(Some(m)) if m.points.len() == base.points.len() && m.polys.len() == base.polys.len() => {
            m
        }
        _ => base,
    };
    for p in &mut mesh.points {
        *p = world.transform_point3(*p);
    }
    Some(mesh)
}

/// The element under the cursor for `mode` (pixels; `None` in Model mode or on nothing).
pub fn pick_component(
    mesh: &PolyMesh,
    cam: &Camera,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    mode: EditMode,
) -> Option<u32> {
    match mode {
        EditMode::Model => None,
        EditMode::Points => pick_point(mesh, cam, x, y, w, h, POINT_RADIUS, true),
        EditMode::Edges => pick_edge(mesh, cam, x, y, w, h, EDGE_RADIUS, true),
        EditMode::Polys => pick_poly(mesh, &cam.ray(x, y, w, h)).map(|hit| hit.poly),
    }
}

/// The visible elements of `mode` inside `rect` (`[x0, y0, x1, y1]` pixels); edges and
/// polygons need all their points inside.
pub fn rect_components(
    mesh: &PolyMesh,
    cam: &Camera,
    rect: [f64; 4],
    w: f64,
    h: f64,
    mode: EditMode,
) -> IndexSet {
    match element(mode) {
        Some(e) => rect_select(mesh, cam, rect, w, h, e, Combine::All, true),
        None => IndexSet::new(),
    }
}

/// The nearest element for `mode` under the cursor over several objects (EDITABLE.md §6.3):
/// the object and the element index. Depth is measured along the pick ray (the point, the
/// middle of the edge, the polygon hit); ties go to the lower node id.
pub fn pick_across(
    meshes: &[(NodeId, PolyMesh)],
    cam: &Camera,
    cursor: [f64; 2],
    size: [f64; 2],
    mode: EditMode,
) -> Option<(NodeId, u32)> {
    let ([x, y], [w, h]) = (cursor, size);
    let ray = cam.ray(x, y, w, h);
    let depth = |p: glam::DVec3| (p - ray.origin).dot(ray.dir);
    meshes
        .iter()
        .filter_map(|(id, mesh)| {
            let hit = pick_component(mesh, cam, x, y, w, h, mode)?;
            let d = match mode {
                EditMode::Model => return None,
                EditMode::Points => depth(mesh.points[hit as usize]),
                EditMode::Edges => {
                    let e = mesh.edges()[hit as usize];
                    depth((mesh.points[e.0 as usize] + mesh.points[e.1 as usize]) * 0.5)
                }
                EditMode::Polys => pick_poly(mesh, &ray)?.t,
            };
            Some((d, *id, hit))
        })
        .min_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)))
        .map(|(_, id, hit)| (id, hit))
}

/// A left click at pixel `cursor` in a viewport of `size`. With component targets it edits
/// their sets of the current mode (EDITABLE.md §6.3); otherwise it picks objects (the
/// Model-mode rules: Ctrl/Cmd toggles, an empty plain click clears).
pub fn viewport_click(
    s: &mut EditorState,
    cursor: [f64; 2],
    size: [f64; 2],
    mods: ClickModifiers,
) -> Result<(), String> {
    let ([x, y], [w, h]) = (cursor, size);
    let ids = targets(s);
    if ids.is_empty() {
        match pick_node(&s.evaluated(), &s.camera, x, y, w, h) {
            Some(id) if mods.command => s.selection.toggle(id),
            Some(id) => s.selection.click(id),
            None if !mods.command => s.selection.clear(),
            None => {}
        }
        return Ok(());
    }
    let meshes: Vec<(NodeId, PolyMesh)> = ids
        .iter()
        .filter_map(|id| Some((*id, pick_mesh(s.doc.scene(), *id)?)))
        .collect();
    let hit = pick_across(&meshes, &s.camera, cursor, size, s.mode);
    let op = SelectOp::from(mods);
    let next: Vec<(NodeId, IndexSet)> = meshes
        .iter()
        .map(|(id, _)| {
            let current = selection_of(s.doc.scene(), *id, s.mode);
            let set = match hit {
                Some((hit_id, e)) if hit_id == *id => apply_click(&current, Some(e), op),
                // a plain click on an element of another object leaves nothing selected here
                Some(_) if op == SelectOp::Replace => IndexSet::new(),
                Some(_) => current,
                None => apply_click(&current, None, op),
            };
            (*id, set)
        })
        .collect();
    write_selections(&mut s.app.doc, s.app.mode, &next).map_err(|e| e.to_string())?;
    Ok(())
}

/// A rectangle drag `[x0, y0, x1, y1]` released in a viewport of `size`; one transaction for
/// all targets. Without component targets nothing happens.
pub fn viewport_rect(
    s: &mut EditorState,
    rect: [f64; 4],
    size: [f64; 2],
    mods: ClickModifiers,
) -> Result<(), String> {
    let ids = targets(s);
    let mut next = Vec::new();
    for id in ids {
        let Some(mesh) = pick_mesh(s.doc.scene(), id) else {
            continue;
        };
        let picked = rect_components(&mesh, &s.camera, rect, size[0], size[1], s.mode);
        let set = apply_op(
            &selection_of(s.doc.scene(), id, s.mode),
            &picked,
            mods.into(),
        );
        next.push((id, set));
    }
    write_selections(&mut s.app.doc, s.app.mode, &next).map_err(|e| e.to_string())?;
    Ok(())
}
