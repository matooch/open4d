//! Component modes and selection without any UI (docs/specs/EDITABLE.md §2, APP.md §1).

use std::collections::BTreeSet;

use forge_command::{ConvertMode, ModeArg, SelectExtra, SelectOp};
use forge_core::{Document, EditError, NodeId, Scene, Value};
use forge_generators::{POLYGON, SELECTION_EDGES, SELECTION_POINTS, SELECTION_POLYS, to_poly_mesh};
use forge_geometry::PolyMesh;
use forge_geometry::selection::Mode;
use forge_geometry::selection::{
    IndexSet, Selection, grow_points, grow_polys, shrink_points, shrink_polys,
};
use forge_geometry::selection_ops::{
    Kind, connected, convert, edge_loop, edge_ring, grow_edges, poly_strip, shrink_edges,
};

use crate::App;

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

    /// The polygon object param holding this mode's set.
    pub fn param(self) -> Option<&'static str> {
        match self {
            EditMode::Model => None,
            EditMode::Points => Some(SELECTION_POINTS),
            EditMode::Edges => Some(SELECTION_EDGES),
            EditMode::Polys => Some(SELECTION_POLYS),
        }
    }
}

impl From<ModeArg> for EditMode {
    fn from(m: ModeArg) -> EditMode {
        match m {
            ModeArg::Model => EditMode::Model,
            ModeArg::Points => EditMode::Points,
            ModeArg::Edges => EditMode::Edges,
            ModeArg::Polys => EditMode::Polys,
        }
    }
}

impl From<EditMode> for ModeArg {
    fn from(m: EditMode) -> ModeArg {
        match m {
            EditMode::Model => ModeArg::Model,
            EditMode::Points => ModeArg::Points,
            EditMode::Edges => ModeArg::Edges,
            EditMode::Polys => ModeArg::Polys,
        }
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

impl From<SelectOp> for SelectCmd {
    fn from(o: SelectOp) -> SelectCmd {
        match o {
            SelectOp::All => SelectCmd::All,
            SelectOp::None => SelectCmd::None,
            SelectOp::Invert => SelectCmd::Invert,
            SelectOp::Grow => SelectCmd::Grow,
            SelectOp::Shrink => SelectCmd::Shrink,
        }
    }
}

/// The active node, when a component mode is on and it is a polygon object.
pub fn target(app: &App) -> Option<NodeId> {
    if app.mode == EditMode::Model {
        return None;
    }
    let id = app.selection.active?;
    is_polygon(app.doc.scene(), id).then_some(id)
}

fn is_polygon(scene: &Scene, id: NodeId) -> bool {
    scene.node(id).is_some_and(|n| n.type_id == POLYGON)
}

/// The component targets (EDITABLE.md §6.1): the selected polygon objects in ascending id
/// order, when a component mode is on and the active node is a polygon object.
pub fn targets(app: &App) -> Vec<NodeId> {
    let Some(active) = target(app) else {
        return Vec::new();
    };
    let scene = app.doc.scene();
    let mut out: BTreeSet<NodeId> = app
        .selection
        .nodes
        .iter()
        .copied()
        .filter(|id| is_polygon(scene, *id))
        .collect();
    out.insert(active);
    out.into_iter().collect()
}

/// The objects a selection command works on: the explicit `nodes` (each must be a polygon
/// object) or the component targets.
pub fn resolve_targets(app: &App, nodes: Option<&[NodeId]>) -> Result<Vec<NodeId>, String> {
    let Some(nodes) = nodes else {
        let all = targets(app);
        return if all.is_empty() {
            Err("select a polygon object".into())
        } else {
            Ok(all)
        };
    };
    let scene = app.doc.scene();
    let mut out = BTreeSet::new();
    for id in nodes {
        if !is_polygon(scene, *id) {
            return Err(format!("node {} is not a polygon object", id.0));
        }
        out.insert(*id);
    }
    if out.is_empty() {
        return Err("nodes is empty".into());
    }
    Ok(out.into_iter().collect())
}

/// The stored (undeformed, object space) mesh of a polygon object.
pub fn base_mesh(scene: &Scene, id: NodeId) -> Option<PolyMesh> {
    match scene.param(id, "mesh") {
        Some(Value::Mesh(m)) if scene.node(id)?.type_id == POLYGON => Some(to_poly_mesh(m)),
        _ => None,
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
    write_selections(doc, mode, &[(id, set.clone())])
}

/// Stores each `(object, set)` as that object's `mode` set, all in one "Select" step.
/// Objects whose stored set already equals the new one are left alone; returns false,
/// writing nothing, if no object changes.
pub fn write_selections(
    doc: &mut Document,
    mode: EditMode,
    sets: &[(NodeId, IndexSet)],
) -> Result<bool, EditError> {
    let Some(param) = mode.param() else {
        return Ok(false);
    };
    let changed: Vec<&(NodeId, IndexSet)> = sets
        .iter()
        .filter(|(id, set)| selection_of(doc.scene(), *id, mode) != *set)
        .collect();
    if changed.is_empty() {
        return Ok(false);
    }
    doc.transact("Select", |tx| {
        for (id, set) in changed {
            tx.set_param(*id, param, Value::IndexList(set.iter().collect()))?;
        }
        Ok(())
    })?;
    Ok(true)
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
fn select_objects(s: &mut App, cmd: SelectCmd) {
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

/// Select All / Deselect All / Invert / Grow / Shrink on the current mode's set of every
/// target (`nodes`: the objects to use instead, EDITABLE.md §6.3): one "Select" step.
pub fn run_select(s: &mut App, cmd: SelectCmd, nodes: Option<&[NodeId]>) -> Result<(), String> {
    if s.mode == EditMode::Model {
        if nodes.is_some() {
            return Err("nodes needs a component mode".into());
        }
        select_objects(s, cmd);
        return Ok(());
    }
    let ids = resolve_targets(s, nodes)?;
    let mut next = Vec::new();
    for id in ids {
        let mesh = base_mesh(s.doc.scene(), id).ok_or("select a polygon object")?;
        let current = selection_of(s.doc.scene(), id, s.mode);
        next.push((id, component_result(&mesh, s.mode, &current, cmd)));
    }
    write_selections(&mut s.doc, s.mode, &next).map_err(|e| e.to_string())?;
    Ok(())
}

fn kind_of(mode: EditMode) -> Option<Kind> {
    match mode {
        EditMode::Model => None,
        EditMode::Points => Some(Kind::Points),
        EditMode::Edges => Some(Kind::Edges),
        EditMode::Polys => Some(Kind::Polys),
    }
}

/// Connected, Convert, Loop and Ring on the current mode's set of every target (EDITABLE.md
/// §2.8, §6.3). Model mode does nothing. One "Select" step; an error in any object writes
/// nothing.
pub fn run_select_extra(
    s: &mut App,
    cmd: SelectExtra,
    nodes: Option<&[NodeId]>,
) -> Result<(), String> {
    let Some(from) = kind_of(s.mode) else {
        return if nodes.is_some() {
            Err("nodes needs a component mode".into())
        } else {
            Ok(())
        };
    };
    let ids = resolve_targets(s, nodes)?;
    let mut write_mode = s.mode;
    let mut next = Vec::new();
    for id in ids {
        let mesh = base_mesh(s.doc.scene(), id).ok_or("select a polygon object")?;
        let current = selection_of(s.doc.scene(), id, s.mode);
        let (mode, set) = match cmd {
            SelectExtra::Connected => (s.mode, connected(&mesh, from, &current)),
            SelectExtra::Convert { to, mode } => {
                let to_mode = EditMode::from(to);
                let to_kind = kind_of(to_mode).ok_or("convert needs points, edges or polys")?;
                let rule = match mode {
                    ConvertMode::Strict => Mode::All,
                    ConvertMode::Tolerant => Mode::Any,
                };
                (to_mode, convert(&mesh, &current, from, to_kind, rule))
            }
            SelectExtra::Loop { across } | SelectExtra::Ring { across } => {
                let ring = matches!(cmd, SelectExtra::Ring { .. });
                let across = if ring { across ^ 1 } else { across };
                let set = match from {
                    Kind::Edges if ring => edge_ring(&mesh, &current),
                    Kind::Edges => edge_loop(&mesh, &current),
                    Kind::Polys => poly_strip(&mesh, &current, across),
                    Kind::Points => return Err("loop and ring need edges or polygons".into()),
                };
                (s.mode, set)
            }
        };
        write_mode = mode;
        next.push((id, set));
    }
    write_selections(&mut s.doc, write_mode, &next).map_err(|e| e.to_string())?;
    if write_mode != s.mode {
        s.mode = write_mode;
    }
    Ok(())
}
