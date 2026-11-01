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
    (app.doc.scene().node(id)?.type_id == POLYGON).then_some(id)
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

/// Select All / Deselect All / Invert / Grow / Shrink on the current mode's set.
pub fn run_select(s: &mut App, cmd: SelectCmd) -> Result<(), String> {
    if s.mode == EditMode::Model {
        select_objects(s, cmd);
        return Ok(());
    }
    let id = target(s).ok_or("select a polygon object")?;
    let mesh = base_mesh(s.doc.scene(), id).ok_or("select a polygon object")?;
    let current = selection_of(s.doc.scene(), id, s.mode);
    let next = component_result(&mesh, s.mode, &current, cmd);
    write_selection(&mut s.doc, id, s.mode, &next).map_err(|e| e.to_string())?;
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

/// Connected, Convert, Loop and Ring on the current mode's set (EDITABLE.md §2.8). Model
/// mode does nothing.
pub fn run_select_extra(s: &mut App, cmd: SelectExtra) -> Result<(), String> {
    let Some(from) = kind_of(s.mode) else {
        return Ok(());
    };
    let id = target(s).ok_or("select a polygon object")?;
    let mesh = base_mesh(s.doc.scene(), id).ok_or("select a polygon object")?;
    let current = selection_of(s.doc.scene(), id, s.mode);
    let (write_mode, next) = match cmd {
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
            let across = match cmd {
                SelectExtra::Ring { .. } => across ^ 1,
                _ => across,
            };
            let ring = matches!(cmd, SelectExtra::Ring { .. });
            let set = match from {
                Kind::Edges if ring => edge_ring(&mesh, &current),
                Kind::Edges => edge_loop(&mesh, &current),
                Kind::Polys => poly_strip(&mesh, &current, across),
                Kind::Points => return Err("loop and ring need edges or polygons".into()),
            };
            (s.mode, set)
        }
    };
    write_selection(&mut s.doc, id, write_mode, &next).map_err(|e| e.to_string())?;
    if write_mode != s.mode {
        s.mode = write_mode;
    }
    Ok(())
}
