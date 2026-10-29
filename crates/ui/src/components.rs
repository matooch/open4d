//! Component modes and selection (docs/specs/EDITABLE.md §2). UI-free: the egui code in
//! `app` only turns pointer input into these calls.

use std::collections::BTreeSet;

use forge_core::{Document, EditError, NodeId, Scene, Value};
use forge_generators::{
    POLYGON, SELECTION_EDGES, SELECTION_POINTS, SELECTION_POLYS, evaluate, to_poly_mesh,
};
use forge_geometry::PolyMesh;
use forge_geometry::selection::{
    IndexSet, Mode as Combine, Selection, grow_points, grow_polys, polys_to_edges, shrink_points,
    shrink_polys,
};
use forge_viewport::camera::Camera;
use forge_viewport::picking::{Element, pick_edge, pick_point, pick_poly, rect_select};

use crate::manipulate::pick_node;
use crate::state::EditorState;

/// Pick radius of points and edges, in render pixels (EDITABLE.md §2.4).
pub const POINT_RADIUS: f64 = 8.0;
pub const EDGE_RADIUS: f64 = 6.0;

/// What a click or drag in the viewport selects.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EditMode {
    #[default]
    Model,
    Points,
    Edges,
    Polys,
}

impl EditMode {
    pub const ALL: [EditMode; 4] = [
        EditMode::Model,
        EditMode::Points,
        EditMode::Edges,
        EditMode::Polys,
    ];

    /// Text of the header button.
    pub fn label(self) -> &'static str {
        match self {
            EditMode::Model => "Model",
            EditMode::Points => "Points",
            EditMode::Edges => "Edges",
            EditMode::Polys => "Polygons",
        }
    }

    fn element(self) -> Option<Element> {
        match self {
            EditMode::Model => None,
            EditMode::Points => Some(Element::Points),
            EditMode::Edges => Some(Element::Edges),
            EditMode::Polys => Some(Element::Polys),
        }
    }

    /// The polygon object param holding this mode's set.
    fn param(self) -> Option<&'static str> {
        match self {
            EditMode::Model => None,
            EditMode::Points => Some(SELECTION_POINTS),
            EditMode::Edges => Some(SELECTION_EDGES),
            EditMode::Polys => Some(SELECTION_POLYS),
        }
    }
}

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

/// Edit menu and Select menu entries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectCmd {
    All,
    None,
    Invert,
    Grow,
    Shrink,
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

/// The active node, when a component mode is on and it is a polygon object.
pub fn target(s: &EditorState) -> Option<NodeId> {
    if s.mode == EditMode::Model {
        return None;
    }
    let id = s.selection.active?;
    (s.doc.scene().node(id)?.type_id == POLYGON).then_some(id)
}

/// The stored (undeformed, object space) mesh of a polygon object.
pub fn base_mesh(scene: &Scene, id: NodeId) -> Option<PolyMesh> {
    match scene.param(id, "mesh") {
        Some(Value::Mesh(m)) if scene.node(id)?.type_id == POLYGON => Some(to_poly_mesh(m)),
        _ => None,
    }
}

/// The mesh components are picked in: the evaluated (deformed) mesh in world space.
/// Indices equal those of the stored mesh (deformers keep the topology).
pub fn pick_mesh(scene: &Scene, id: NodeId) -> Option<PolyMesh> {
    let base = base_mesh(scene, id)?;
    let world = scene.world_matrix(id)?;
    let mut mesh = match evaluate(scene, id) {
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
    match mode.element() {
        Some(e) => rect_select(mesh, cam, rect, w, h, e, Combine::All, true),
        None => IndexSet::new(),
    }
}

fn indices(scene: &Scene, id: NodeId, param: &str) -> IndexSet {
    match scene.param(id, param) {
        Some(Value::IndexList(l)) => l.iter().copied().collect(),
        _ => IndexSet::new(),
    }
}

/// The stored set of `mode` (empty for Model mode).
pub fn selection_of(scene: &Scene, id: NodeId, mode: EditMode) -> IndexSet {
    mode.param()
        .map_or_else(IndexSet::new, |p| indices(scene, id, p))
}

/// All three stored sets.
pub fn stored_selection(scene: &Scene, id: NodeId) -> Selection {
    Selection {
        points: indices(scene, id, SELECTION_POINTS),
        edges: indices(scene, id, SELECTION_EDGES),
        polys: indices(scene, id, SELECTION_POLYS),
    }
}

/// Stores `set` as the `mode` set in one "Select" step. Returns false, writing nothing,
/// if it equals the stored set.
pub fn write_selection(
    doc: &mut Document,
    id: NodeId,
    mode: EditMode,
    set: &IndexSet,
) -> Result<bool, EditError> {
    let Some(param) = mode.param() else {
        return Ok(false);
    };
    if selection_of(doc.scene(), id, mode) == *set {
        return Ok(false);
    }
    doc.transact("Select", |tx| {
        tx.set_param(id, param, Value::IndexList(set.iter().collect()))
    })?;
    Ok(true)
}

fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

/// A left click at pixel `cursor` in a viewport of `size`. With a component target it
/// edits that object's set of the current mode; otherwise it picks objects (the
/// Model-mode rules: Ctrl/Cmd toggles, an empty plain click clears).
pub fn viewport_click(
    s: &mut EditorState,
    cursor: [f64; 2],
    size: [f64; 2],
    mods: ClickModifiers,
) -> Result<(), String> {
    let ([x, y], [w, h]) = (cursor, size);
    let Some(id) = target(s) else {
        match pick_node(&s.evaluated(), &s.camera, x, y, w, h) {
            Some(id) if mods.command => s.selection.toggle(id),
            Some(id) => s.selection.click(id),
            None if !mods.command => s.selection.clear(),
            None => {}
        }
        return Ok(());
    };
    let Some(mesh) = pick_mesh(s.doc.scene(), id) else {
        return Ok(());
    };
    let picked = pick_component(&mesh, &s.camera, x, y, w, h, s.mode);
    let next = apply_click(
        &selection_of(s.doc.scene(), id, s.mode),
        picked,
        mods.into(),
    );
    write_selection(&mut s.doc, id, s.mode, &next).map_err(err)?;
    Ok(())
}

/// A rectangle drag `[x0, y0, x1, y1]` released in a viewport of `size`; one transaction.
/// Without a component target nothing happens.
pub fn viewport_rect(
    s: &mut EditorState,
    rect: [f64; 4],
    size: [f64; 2],
    mods: ClickModifiers,
) -> Result<(), String> {
    let Some(id) = target(s) else {
        return Ok(());
    };
    let Some(mesh) = pick_mesh(s.doc.scene(), id) else {
        return Ok(());
    };
    let picked = rect_components(&mesh, &s.camera, rect, size[0], size[1], s.mode);
    let next = apply_op(
        &selection_of(s.doc.scene(), id, s.mode),
        &picked,
        mods.into(),
    );
    write_selection(&mut s.doc, id, s.mode, &next).map_err(err)?;
    Ok(())
}

/// Edges sharing a point with each edge.
fn edge_neighbors(mesh: &PolyMesh) -> Vec<BTreeSet<u32>> {
    let edges = mesh.edges();
    let mut at_point: Vec<Vec<u32>> = vec![Vec::new(); mesh.points.len()];
    for (i, e) in edges.iter().enumerate() {
        at_point[e.0 as usize].push(i as u32);
        at_point[e.1 as usize].push(i as u32);
    }
    edges
        .iter()
        .enumerate()
        .map(|(i, e)| {
            [e.0, e.1]
                .iter()
                .flat_map(|p| at_point[*p as usize].iter().copied())
                .filter(|n| *n as usize != i)
                .collect()
        })
        .collect()
}

fn grow_edges(mesh: &PolyMesh, sel: &IndexSet) -> IndexSet {
    let n = edge_neighbors(mesh);
    let mut out = sel.clone();
    for i in sel.iter() {
        for j in n.get(i as usize).into_iter().flatten() {
            out.insert(*j);
        }
    }
    out
}

fn shrink_edges(mesh: &PolyMesh, sel: &IndexSet) -> IndexSet {
    let n = edge_neighbors(mesh);
    sel.iter()
        .filter(|i| {
            n.get(*i as usize)
                .is_some_and(|ns| ns.iter().all(|j| sel.contains(*j)))
        })
        .collect()
}

/// The new set for `cmd` given the current one and the stored mesh.
fn component_result(
    mesh: &PolyMesh,
    mode: EditMode,
    current: &IndexSet,
    cmd: SelectCmd,
) -> IndexSet {
    let count = match mode {
        EditMode::Model => 0,
        EditMode::Points => mesh.points.len(),
        EditMode::Edges => mesh.edges().len(),
        EditMode::Polys => mesh.polys.len(),
    } as u32;
    match (cmd, mode) {
        (SelectCmd::All, _) => (0..count).collect(),
        (SelectCmd::None, _) => IndexSet::new(),
        (SelectCmd::Invert, _) => current.invert(count),
        (SelectCmd::Grow, EditMode::Points) => grow_points(mesh, current),
        (SelectCmd::Shrink, EditMode::Points) => shrink_points(mesh, current),
        (SelectCmd::Grow, EditMode::Edges) => grow_edges(mesh, current),
        (SelectCmd::Shrink, EditMode::Edges) => shrink_edges(mesh, current),
        (SelectCmd::Grow, EditMode::Polys) => grow_polys(mesh, current),
        (SelectCmd::Shrink, EditMode::Polys) => shrink_polys(mesh, current),
        (_, EditMode::Model) => current.clone(),
    }
}

/// Model mode: the set is the object selection (every hierarchy node, no tags).
fn select_objects(s: &mut EditorState, cmd: SelectCmd) {
    let scene = s.doc.scene();
    let all: BTreeSet<NodeId> = scene
        .roots()
        .iter()
        .flat_map(|r| std::iter::once(*r).chain(scene.descendants(*r)))
        .collect();
    let next: BTreeSet<NodeId> = match cmd {
        SelectCmd::All => all,
        SelectCmd::None => BTreeSet::new(),
        SelectCmd::Invert => all.difference(&s.selection.nodes).copied().collect(),
        SelectCmd::Grow | SelectCmd::Shrink => return,
    };
    match next.last().copied() {
        Some(last) => {
            s.selection.nodes = next;
            s.selection.active = Some(last);
            s.selection.anchor = Some(last);
        }
        None => s.selection.clear(),
    }
}

/// Select All / Deselect All / Invert / Grow / Shrink on the current mode's set.
pub fn run_select(s: &mut EditorState, cmd: SelectCmd) -> Result<(), String> {
    if s.mode == EditMode::Model {
        select_objects(s, cmd);
        return Ok(());
    }
    let id = target(s).ok_or("select a polygon object")?;
    let mesh = base_mesh(s.doc.scene(), id).ok_or("select a polygon object")?;
    let current = selection_of(s.doc.scene(), id, s.mode);
    let next = component_result(&mesh, s.mode, &current, cmd);
    write_selection(&mut s.doc, id, s.mode, &next).map_err(err)?;
    Ok(())
}
