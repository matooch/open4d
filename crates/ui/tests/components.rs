//! docs/specs/EDITABLE.md §2: component modes and selection, driven without a GPU.

mod common;

use std::collections::BTreeSet;
use std::sync::Arc;

use common::{create, state};
use egui::{Key, Modifiers};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use forge_core::{NodeId, Registry, Value, file};
use forge_generators::{POLYGON, SELECTION_EDGES, SELECTION_POINTS, SELECTION_POLYS};
use forge_geometry::PolyMesh;
use forge_geometry::selection::IndexSet;
use forge_ui::app::Editor;
use forge_ui::commands::{Command, execute};
use forge_ui::components::{
    ClickModifiers, EditMode, SelectCmd, SelectOp, apply_click, display_selection, pick_component,
    pick_mesh, rect_components, selection_of, stored_selection, target, viewport_click,
    viewport_rect,
};
use forge_ui::state::EditorState;
use glam::DVec3;

const W: f64 = 800.0;
const H: f64 = 600.0;
const SIZE: [f64; 2] = [W, H];

const NONE: ClickModifiers = ClickModifiers {
    shift: false,
    command: false,
};
const SHIFT: ClickModifiers = ClickModifiers {
    shift: true,
    command: false,
};
const CMD: ClickModifiers = ClickModifiers {
    shift: false,
    command: true,
};

/// A made-editable default cube (200 wide, 8 points, 12 edges, 6 polygons) at the origin,
/// seen by the editor's default camera, in `mode`. Returns the state and the polygon object.
fn setup(mode: EditMode) -> (EditorState, NodeId) {
    let mut st = state();
    create(&mut st, "forge.cube");
    execute(&mut st, Command::MakeEditable).unwrap();
    let id = st.selection.active.unwrap();
    assert_eq!(st.doc.scene().node(id).unwrap().type_id, POLYGON);
    execute(&mut st, Command::SetMode(mode)).unwrap();
    (st, id)
}

fn mesh_of(st: &EditorState, id: NodeId) -> PolyMesh {
    pick_mesh(st.doc.scene(), id).expect("polygon object has a mesh")
}

/// Pixel of a world point in the test viewport.
fn px(st: &EditorState, p: DVec3) -> [f64; 2] {
    let s = st.camera.project(p, W, H).expect("in front of the camera");
    [s.x, s.y]
}

/// Clicks where the world point `p` appears.
fn click_at(st: &mut EditorState, p: DVec3, mods: ClickModifiers) {
    let cursor = px(st, p);
    viewport_click(st, cursor, SIZE, mods).unwrap();
}

fn depth(st: &EditorState, p: DVec3) -> f64 {
    st.camera.project(p, W, H).unwrap().z
}

fn set(items: &[u32]) -> IndexSet {
    items.iter().copied().collect()
}

fn sel(st: &EditorState, id: NodeId, mode: EditMode) -> IndexSet {
    selection_of(st.doc.scene(), id, mode)
}

fn undo_steps(st: &EditorState) -> usize {
    st.doc.history().undo_labels().len()
}

/// The point nearest to the camera, and the point farthest from it.
fn near_far(st: &EditorState, mesh: &PolyMesh) -> (u32, u32) {
    let d: Vec<f64> = mesh.points.iter().map(|p| depth(st, *p)).collect();
    let near = (0..d.len()).min_by(|a, b| d[*a].total_cmp(&d[*b])).unwrap();
    let far = (0..d.len()).max_by(|a, b| d[*a].total_cmp(&d[*b])).unwrap();
    (near as u32, far as u32)
}

#[test]
fn the_cube_has_the_expected_topology() {
    let (st, id) = setup(EditMode::Points);
    let m = mesh_of(&st, id);
    assert_eq!((m.points.len(), m.edges().len(), m.polys.len()), (8, 12, 6));
    assert_eq!(target(&st), Some(id));
}

#[test]
fn picking_a_point_an_edge_and_a_polygon() {
    let (st, id) = setup(EditMode::Points);
    let m = mesh_of(&st, id);
    let (near, far) = near_far(&st, &m);

    // a point: exactly on the corner nearest to the camera, and a few pixels off
    let [x, y] = px(&st, m.points[near as usize]);
    for (dx, dy) in [(0.0, 0.0), (3.0, -4.0)] {
        assert_eq!(
            pick_component(&m, &st.camera, x + dx, y + dy, W, H, EditMode::Points),
            Some(near)
        );
    }
    // outside the pick radius, and on the background
    assert_eq!(
        pick_component(&m, &st.camera, x + 40.0, y, W, H, EditMode::Points).filter(|p| *p == near),
        None
    );
    assert_eq!(
        pick_component(&m, &st.camera, 5.0, 5.0, W, H, EditMode::Points),
        None
    );
    // the far corner is hidden behind the cube: visible-only picking skips it
    let [fx, fy] = px(&st, m.points[far as usize]);
    assert_eq!(
        pick_component(&m, &st.camera, fx, fy, W, H, EditMode::Points),
        None
    );

    // an edge: the midpoint of each of the three edges at the nearest corner
    let edges = m.edges();
    let at_near: Vec<usize> = (0..edges.len())
        .filter(|i| edges[*i].0 == near || edges[*i].1 == near)
        .collect();
    assert_eq!(at_near.len(), 3);
    for i in at_near {
        let mid = (m.points[edges[i].0 as usize] + m.points[edges[i].1 as usize]) / 2.0;
        let [x, y] = px(&st, mid);
        assert_eq!(
            pick_component(&m, &st.camera, x, y, W, H, EditMode::Edges),
            Some(i as u32),
            "edge {i}"
        );
    }
    // an edge of the far corner is hidden at its midpoint
    let far_edge = (0..edges.len())
        .find(|i| edges[*i].0 == far || edges[*i].1 == far)
        .unwrap();
    let mid = (m.points[edges[far_edge].0 as usize] + m.points[edges[far_edge].1 as usize]) / 2.0;
    let [x, y] = px(&st, mid);
    assert_ne!(
        pick_component(&m, &st.camera, x, y, W, H, EditMode::Edges),
        Some(far_edge as u32)
    );

    // a polygon: the centre of each of the three faces turned towards the camera
    let centre = |f: usize| {
        m.polys[f]
            .iter()
            .map(|v| m.points[*v as usize])
            .sum::<DVec3>()
            / m.polys[f].len() as f64
    };
    let front: Vec<usize> = (0..m.polys.len())
        .filter(|f| m.polys[*f].contains(&near))
        .collect();
    assert_eq!(front.len(), 3);
    for f in front {
        let [x, y] = px(&st, centre(f));
        assert_eq!(
            pick_component(&m, &st.camera, x, y, W, H, EditMode::Polys),
            Some(f as u32),
            "polygon {f}"
        );
    }
    assert_eq!(
        pick_component(&m, &st.camera, 5.0, 5.0, W, H, EditMode::Polys),
        None
    );
    assert_eq!(
        pick_component(&m, &st.camera, x, y, W, H, EditMode::Model),
        None,
        "Model mode picks no component"
    );
}

#[test]
fn click_rules_on_a_pure_set() {
    let cur = set(&[1, 2]);
    let r = SelectOp::Replace;
    assert_eq!(apply_click(&cur, Some(5), r), set(&[5]));
    assert_eq!(apply_click(&cur, None, r), set(&[]));
    assert_eq!(apply_click(&cur, Some(5), SelectOp::Add), set(&[1, 2, 5]));
    assert_eq!(apply_click(&cur, Some(2), SelectOp::Add), cur);
    assert_eq!(apply_click(&cur, None, SelectOp::Add), cur);
    assert_eq!(apply_click(&cur, Some(2), SelectOp::Remove), set(&[1]));
    assert_eq!(apply_click(&cur, Some(7), SelectOp::Remove), cur);
    assert_eq!(apply_click(&cur, None, SelectOp::Remove), cur);
    // Ctrl/Cmd beats Shift
    assert_eq!(SelectOp::from(NONE), SelectOp::Replace);
    assert_eq!(SelectOp::from(SHIFT), SelectOp::Add);
    assert_eq!(SelectOp::from(CMD), SelectOp::Remove);
    let both = ClickModifiers {
        shift: true,
        command: true,
    };
    assert_eq!(SelectOp::from(both), SelectOp::Remove);
}

#[test]
fn click_shift_click_and_ctrl_click_edit_the_stored_set() {
    let (mut st, id) = setup(EditMode::Points);
    let m = mesh_of(&st, id);
    // the three visible corners at the nearest point and two of its neighbours
    let (near, _) = near_far(&st, &m);
    let nbr: Vec<u32> = m
        .edges()
        .iter()
        .filter_map(|e| match (e.0 == near, e.1 == near) {
            (true, _) => Some(e.1),
            (_, true) => Some(e.0),
            _ => None,
        })
        .collect();
    let (a, b) = (near, nbr[0]);
    let cam = st.camera;
    let at = |p: u32| {
        let s = cam.project(m.points[p as usize], W, H).unwrap();
        [s.x, s.y]
    };
    let steps = undo_steps(&st);

    viewport_click(&mut st, at(a), SIZE, NONE).unwrap();
    assert_eq!(sel(&st, id, EditMode::Points), set(&[a]));
    viewport_click(&mut st, at(b), SIZE, NONE).unwrap();
    assert_eq!(sel(&st, id, EditMode::Points), set(&[b]), "click replaces");
    viewport_click(&mut st, at(a), SIZE, SHIFT).unwrap();
    assert_eq!(sel(&st, id, EditMode::Points), set(&[a, b]), "shift adds");
    viewport_click(&mut st, at(a), SIZE, SHIFT).unwrap();
    assert_eq!(sel(&st, id, EditMode::Points), set(&[a, b]), "still there");
    viewport_click(&mut st, at(b), SIZE, CMD).unwrap();
    assert_eq!(sel(&st, id, EditMode::Points), set(&[a]), "ctrl removes");
    assert_eq!(undo_steps(&st), steps + 4, "the repeated add wrote nothing");

    // empty clicks: shift and ctrl keep the set, a plain click clears it
    viewport_click(&mut st, [5.0, 5.0], SIZE, SHIFT).unwrap();
    viewport_click(&mut st, [5.0, 5.0], SIZE, CMD).unwrap();
    assert_eq!(sel(&st, id, EditMode::Points), set(&[a]));
    assert_eq!(undo_steps(&st), steps + 4);
    viewport_click(&mut st, [5.0, 5.0], SIZE, NONE).unwrap();
    assert_eq!(sel(&st, id, EditMode::Points), set(&[]));
    assert_eq!(undo_steps(&st), steps + 5);
    assert_eq!(
        st.doc.history().undo_labels().last().copied(),
        Some("Select")
    );
    viewport_click(&mut st, [5.0, 5.0], SIZE, NONE).unwrap();
    assert_eq!(
        undo_steps(&st),
        steps + 5,
        "clearing nothing writes nothing"
    );

    // the sets are independent: only the current mode's one changed
    assert!(sel(&st, id, EditMode::Edges).is_empty());
    assert!(sel(&st, id, EditMode::Polys).is_empty());
}

#[test]
fn edges_and_polygons_are_clicked_into_their_own_sets() {
    let (mut st, id) = setup(EditMode::Edges);
    let m = mesh_of(&st, id);
    let (near, _) = near_far(&st, &m);
    let edges = m.edges();
    let e = (0..edges.len()).find(|i| edges[*i].0 == near).unwrap();
    let mid = (m.points[edges[e].0 as usize] + m.points[edges[e].1 as usize]) / 2.0;
    click_at(&mut st, mid, NONE);
    assert_eq!(sel(&st, id, EditMode::Edges), set(&[e as u32]));

    execute(&mut st, Command::SetMode(EditMode::Polys)).unwrap();
    let f = (0..m.polys.len())
        .find(|f| m.polys[*f].contains(&near))
        .unwrap();
    let c = m.polys[f]
        .iter()
        .map(|v| m.points[*v as usize])
        .sum::<DVec3>()
        / 4.0;
    click_at(&mut st, c, NONE);
    assert_eq!(sel(&st, id, EditMode::Polys), set(&[f as u32]));
    assert_eq!(
        sel(&st, id, EditMode::Edges),
        set(&[e as u32]),
        "switching the mode converts nothing"
    );
    let all = stored_selection(st.doc.scene(), id);
    assert!(all.points.is_empty());
    assert_eq!(all.edges, set(&[e as u32]));
    assert_eq!(all.polys, set(&[f as u32]));
}

#[test]
fn rectangle_select_takes_the_visible_elements_in_one_transaction() {
    let (mut st, id) = setup(EditMode::Points);
    let m = mesh_of(&st, id);
    let (near, far) = near_far(&st, &m);
    let whole = [0.0, 0.0, W, H];
    let steps = undo_steps(&st);

    // points: everything but the hidden far corner
    viewport_rect(&mut st, whole, SIZE, NONE).unwrap();
    let visible: IndexSet = (0..8).filter(|p| *p != far).collect();
    assert_eq!(sel(&st, id, EditMode::Points), visible);
    assert_eq!(undo_steps(&st), steps + 1, "one rectangle, one step");
    assert_eq!(
        rect_components(&m, &st.camera, whole, W, H, EditMode::Points),
        visible
    );
    // the corner order of the rectangle does not matter
    assert_eq!(
        rect_components(&m, &st.camera, [W, H, 0.0, 0.0], W, H, EditMode::Points),
        visible
    );

    // a small rectangle round the nearest corner: replace, add, remove
    let [x, y] = px(&st, m.points[near as usize]);
    let small = [x - 5.0, y - 5.0, x + 5.0, y + 5.0];
    viewport_rect(&mut st, small, SIZE, NONE).unwrap();
    assert_eq!(sel(&st, id, EditMode::Points), set(&[near]));
    viewport_rect(&mut st, whole, SIZE, SHIFT).unwrap();
    assert_eq!(sel(&st, id, EditMode::Points), visible);
    viewport_rect(&mut st, small, SIZE, CMD).unwrap();
    let mut without = visible.clone();
    without.remove(near);
    assert_eq!(sel(&st, id, EditMode::Points), without);
    // an empty rectangle with a plain click modifier clears
    viewport_rect(&mut st, [1.0, 1.0, 9.0, 9.0], SIZE, NONE).unwrap();
    assert!(sel(&st, id, EditMode::Points).is_empty());

    // edges and polygons need all their points inside and visible
    execute(&mut st, Command::SetMode(EditMode::Edges)).unwrap();
    viewport_rect(&mut st, whole, SIZE, NONE).unwrap();
    assert_eq!(
        sel(&st, id, EditMode::Edges).len(),
        9,
        "12 minus the 3 at the far corner"
    );
    execute(&mut st, Command::SetMode(EditMode::Polys)).unwrap();
    viewport_rect(&mut st, whole, SIZE, NONE).unwrap();
    let polys = sel(&st, id, EditMode::Polys);
    assert_eq!(polys.len(), 3);
    assert!(polys.iter().all(|f| !m.polys[f as usize].contains(&far)));
}

#[test]
fn undo_and_redo_restore_the_previous_selection() {
    let (mut st, id) = setup(EditMode::Points);
    let m = mesh_of(&st, id);
    let (near, _) = near_far(&st, &m);
    click_at(&mut st, m.points[near as usize], NONE);
    viewport_rect(&mut st, [0.0, 0.0, W, H], SIZE, NONE).unwrap();
    let all = sel(&st, id, EditMode::Points);
    assert_eq!(all.len(), 7);

    execute(&mut st, Command::Undo).unwrap();
    assert_eq!(sel(&st, id, EditMode::Points), set(&[near]));
    execute(&mut st, Command::Undo).unwrap();
    assert!(sel(&st, id, EditMode::Points).is_empty());
    execute(&mut st, Command::Redo).unwrap();
    execute(&mut st, Command::Redo).unwrap();
    assert_eq!(sel(&st, id, EditMode::Points), all);
    assert_eq!(st.mode, EditMode::Points, "undo does not change the mode");
}

#[test]
fn the_selection_survives_save_and_load() {
    let (mut st, id) = setup(EditMode::Points);
    let m = mesh_of(&st, id);
    viewport_rect(&mut st, [0.0, 0.0, W, H], SIZE, NONE).unwrap();
    execute(&mut st, Command::SetMode(EditMode::Edges)).unwrap();
    viewport_rect(&mut st, [0.0, 0.0, W, H], SIZE, NONE).unwrap();
    execute(&mut st, Command::SetMode(EditMode::Polys)).unwrap();
    viewport_rect(&mut st, [0.0, 0.0, W, H], SIZE, NONE).unwrap();
    let want = stored_selection(st.doc.scene(), id);
    assert_eq!(
        (want.points.len(), want.edges.len(), want.polys.len()),
        (7, 9, 3)
    );

    // the file API directly
    let text = file::save(st.doc.scene());
    let (scene, report) = file::load(common::registry(), &text).unwrap();
    assert!(report.warnings.is_empty(), "{:?}", report.warnings);
    assert_eq!(stored_selection(&scene, id), want);
    assert_eq!(file::save(&scene), text);

    // through the editor's Save and Open with an injected path
    let dir = std::env::temp_dir().join(format!("forge-components-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("sel.forge");
    let mut e = Editor::new(st);
    assert!(e.save_to(&path));
    let mut other = Editor::new(state());
    other.open_path(&path);
    assert_eq!(stored_selection(other.state.doc.scene(), id), want);
    assert_eq!(other.state.mode, EditMode::Model, "Open resets the mode");
    assert_eq!(
        pick_mesh(other.state.doc.scene(), id).unwrap().points,
        m.points
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_version_2_file_without_selection_params_loads_with_empty_sets() {
    let mut st = state();
    create(&mut st, "forge.cube");
    execute(&mut st, Command::MakeEditable).unwrap();
    let id = st.selection.active.unwrap();
    let text = file::save(st.doc.scene());
    // strip the selection params and claim version 2
    let old: String = text
        .lines()
        .filter(|l| !l.contains("selection."))
        .collect::<Vec<_>>()
        .join("\n")
        .replace("version: 3", "version: 2");
    assert!(!old.contains("selection."));
    let (scene, report) = file::load(common::registry(), &old).unwrap();
    assert!(report.warnings.is_empty(), "{:?}", report.warnings);
    let s = stored_selection(&scene, id);
    assert!(s.points.is_empty() && s.edges.is_empty() && s.polys.is_empty());
    for key in [SELECTION_POINTS, SELECTION_EDGES, SELECTION_POLYS] {
        assert_eq!(scene.param(id, key), Some(&Value::IndexList(vec![])));
    }
}

#[test]
fn without_a_polygon_target_clicks_pick_objects() {
    let mut st = state();
    let cube = create(&mut st, "forge.cube");
    st.selection.clear();
    execute(&mut st, Command::SetMode(EditMode::Points)).unwrap();
    assert_eq!(target(&st), None);
    let steps = undo_steps(&st);
    // the centre of the (non-editable) cube
    viewport_click(&mut st, [W / 2.0, H / 2.0], SIZE, NONE).unwrap();
    assert_eq!(st.selection.active, Some(cube));
    assert_eq!(undo_steps(&st), steps, "an object pick is no document edit");
    // an empty click clears the object selection as in Model mode
    viewport_click(&mut st, [5.0, 5.0], SIZE, NONE).unwrap();
    assert!(st.selection.nodes.is_empty());
    // a rectangle does nothing without a target
    viewport_rect(&mut st, [0.0, 0.0, W, H], SIZE, NONE).unwrap();
    assert_eq!(undo_steps(&st), steps);

    // Model mode on a polygon object: also object picking, no component written
    let (mut st, id) = setup(EditMode::Model);
    st.selection.clear();
    assert_eq!(target(&st), None);
    viewport_click(&mut st, [W / 2.0, H / 2.0], SIZE, NONE).unwrap();
    assert_eq!(st.selection.active, Some(id));
    let s = stored_selection(st.doc.scene(), id);
    assert!(s.points.is_empty() && s.edges.is_empty() && s.polys.is_empty());
}

#[test]
fn picking_uses_the_deformed_mesh_and_stores_base_indices() {
    let mut r = Registry::with_builtins();
    r.install(&forge_generators::Primitives).unwrap();
    r.install(&forge_modifiers::Deformers).unwrap();
    let mut st = EditorState::new(Arc::new(r));
    execute(&mut st, Command::Create("forge.cube".into())).unwrap();
    execute(&mut st, Command::MakeEditable).unwrap();
    let id = st.selection.active.unwrap();
    let base = mesh_of(&st, id);
    st.doc
        .transact("deform", |tx| {
            let d = tx.create("forge.spherify", Some(id), None)?;
            tx.set_param(d, "radius", Value::Float(100.0))?;
            tx.set_param(d, "strength", Value::Float(100.0))
        })
        .unwrap();
    execute(&mut st, Command::SetMode(EditMode::Points)).unwrap();
    let deformed = mesh_of(&st, id);
    assert_eq!(deformed.polys, base.polys, "topology is unchanged");
    let (near, _) = near_far(&st, &deformed);
    assert!(
        (deformed.points[near as usize] - base.points[near as usize]).length() > 10.0,
        "the deformer moved the corner"
    );
    // clicking where the deformed corner is picks it; the base corner's place does not
    click_at(&mut st, deformed.points[near as usize], NONE);
    assert_eq!(sel(&st, id, EditMode::Points), set(&[near]));
    click_at(&mut st, base.points[near as usize], NONE);
    assert!(
        !sel(&st, id, EditMode::Points).contains(near),
        "the undeformed place is empty space now"
    );
    // the stored mesh itself is untouched by selecting
    let Some(Value::Mesh(stored)) = st.doc.scene().param(id, "mesh") else {
        panic!("mesh param")
    };
    assert_eq!(stored.points.len(), 8);
}

#[test]
fn select_all_none_invert_grow_and_shrink_act_on_the_current_set() {
    let (mut st, id) = setup(EditMode::Points);
    let run = |st: &mut EditorState, c| execute(st, Command::Select(c)).unwrap();
    let pts = |st: &EditorState| sel(st, id, EditMode::Points);

    run(&mut st, SelectCmd::All);
    assert_eq!(pts(&st).len(), 8);
    run(&mut st, SelectCmd::Invert);
    assert!(pts(&st).is_empty());
    run(&mut st, SelectCmd::Invert);
    assert_eq!(pts(&st).len(), 8);
    run(&mut st, SelectCmd::None);
    assert!(pts(&st).is_empty());
    // Grow adds the edge-neighbours (3 on a cube), Shrink takes them away again
    st.doc
        .transact("sel", |tx| {
            tx.set_param(id, SELECTION_POINTS, Value::IndexList(vec![0]))
        })
        .unwrap();
    run(&mut st, SelectCmd::Grow);
    assert_eq!(pts(&st).len(), 4);
    run(&mut st, SelectCmd::Shrink);
    assert_eq!(pts(&st), set(&[0]));
    run(&mut st, SelectCmd::Shrink);
    assert!(
        pts(&st).is_empty(),
        "a lone point has unselected neighbours"
    );

    // edges: an edge touches 4 others on a cube
    execute(&mut st, Command::SetMode(EditMode::Edges)).unwrap();
    st.doc
        .transact("sel", |tx| {
            tx.set_param(id, SELECTION_EDGES, Value::IndexList(vec![3]))
        })
        .unwrap();
    run(&mut st, SelectCmd::Grow);
    assert_eq!(sel(&st, id, EditMode::Edges).len(), 5);
    run(&mut st, SelectCmd::Shrink);
    assert_eq!(sel(&st, id, EditMode::Edges), set(&[3]));
    run(&mut st, SelectCmd::All);
    assert_eq!(sel(&st, id, EditMode::Edges).len(), 12);

    // polygons: a face has 4 edge-neighbours
    execute(&mut st, Command::SetMode(EditMode::Polys)).unwrap();
    st.doc
        .transact("sel", |tx| {
            tx.set_param(id, SELECTION_POLYS, Value::IndexList(vec![2]))
        })
        .unwrap();
    run(&mut st, SelectCmd::Grow);
    assert_eq!(sel(&st, id, EditMode::Polys).len(), 5);
    run(&mut st, SelectCmd::Shrink);
    assert_eq!(sel(&st, id, EditMode::Polys), set(&[2]));

    // one undo step each, labelled Select
    assert_eq!(
        st.doc.history().undo_labels().last().copied(),
        Some("Select")
    );
    // a no-op writes nothing
    let steps = undo_steps(&st);
    run(&mut st, SelectCmd::Shrink); // {2} -> {} writes ...
    let steps_after = undo_steps(&st);
    assert_eq!(steps_after, steps + 1);
    run(&mut st, SelectCmd::Shrink); // ... but empty stays empty
    assert_eq!(undo_steps(&st), steps_after);
}

#[test]
fn select_commands_without_a_target_and_in_model_mode() {
    let (mut st, id) = setup(EditMode::Points);
    st.selection.clear();
    assert_eq!(
        execute(&mut st, Command::Select(SelectCmd::All)),
        Err("select a polygon object".to_string())
    );
    assert!(sel(&st, id, EditMode::Points).is_empty());

    // Model mode: the set is the object selection
    let mut st = state();
    let a = create(&mut st, "forge.cube");
    let b = create(&mut st, "forge.null");
    execute(&mut st, Command::SetMode(EditMode::Model)).unwrap();
    st.selection.clear();
    execute(&mut st, Command::Select(SelectCmd::All)).unwrap();
    assert_eq!(st.selection.nodes, BTreeSet::from([a, b]));
    execute(&mut st, Command::Select(SelectCmd::Invert)).unwrap();
    assert!(st.selection.nodes.is_empty());
    st.selection.click(a);
    execute(&mut st, Command::Select(SelectCmd::Invert)).unwrap();
    assert_eq!(st.selection.nodes, BTreeSet::from([b]));
    execute(&mut st, Command::Select(SelectCmd::None)).unwrap();
    assert!(st.selection.nodes.is_empty());
}

fn harness(st: EditorState) -> Harness<'static, Editor> {
    let mut h = Harness::builder()
        .with_size([1280.0, 800.0])
        .build_ui_state(|ui, e: &mut Editor| e.show(ui, None), Editor::new(st));
    h.run();
    h
}

#[test]
fn header_buttons_switch_the_mode() {
    let (st, id) = setup(EditMode::Model);
    let mut h = harness(st);
    for (label, mode) in [
        ("Points", EditMode::Points),
        ("Edges", EditMode::Edges),
        ("Polygons", EditMode::Polys),
        ("Model", EditMode::Model),
    ] {
        h.get_by_label(label).click();
        h.run();
        assert_eq!(h.state().state.mode, mode, "{label}");
    }
    assert_eq!(h.state().state.selection.active, Some(id));
}

#[test]
fn shortcuts_and_menus_act_on_the_current_set() {
    let (st, id) = setup(EditMode::Points);
    let mut h = harness(st);
    let points = |h: &Harness<'_, Editor>| sel(&h.state().state, id, EditMode::Points);

    h.key_press_modifiers(Modifiers::COMMAND, Key::A);
    h.run();
    assert_eq!(points(&h).len(), 8, "Ctrl+A selects all points");
    h.key_press_modifiers(Modifiers::COMMAND | Modifiers::SHIFT, Key::A);
    h.run();
    assert!(points(&h).is_empty(), "Ctrl+Shift+A deselects, not selects");

    h.get_by_label("Edit").click();
    h.run();
    h.get_by_label_contains("Select All").click();
    h.run();
    assert_eq!(points(&h).len(), 8);
    h.get_by_label("Edit").click();
    h.run();
    h.get_by_label_contains("Invert Selection").click();
    h.run();
    assert!(points(&h).is_empty());
    h.get_by_label("Edit").click();
    h.run();
    h.get_by_label_contains("Deselect All").click();
    h.run();

    h.state_mut()
        .state
        .doc
        .transact("sel", |tx| {
            tx.set_param(id, SELECTION_POINTS, Value::IndexList(vec![0]))
        })
        .unwrap();
    h.get_by_label("Select").click();
    h.run();
    h.get_by_label_contains("Grow Selection").click();
    h.run();
    assert_eq!(points(&h).len(), 4);
    h.get_by_label("Select").click();
    h.run();
    h.get_by_label_contains("Shrink Selection").click();
    h.run();
    assert_eq!(points(&h), set(&[0]));
}

#[test]
fn display_selection_outlines_selected_polygons_and_hides_the_other_sets() {
    let (st, id) = setup(EditMode::Polys);
    let m = mesh_of(&st, id);
    let stored = forge_geometry::selection::Selection {
        points: set(&[1]),
        edges: set(&[2]),
        polys: set(&[0]),
    };
    let polys = display_selection(&m, &stored, EditMode::Polys);
    assert_eq!(polys.polys, set(&[0]));
    assert_eq!(polys.edges.len(), 4, "the outline of one quad");
    assert!(polys.points.is_empty());
    let edges = display_selection(&m, &stored, EditMode::Edges);
    assert_eq!((edges.edges, edges.polys.len()), (set(&[2]), 0));
    let points = display_selection(&m, &stored, EditMode::Points);
    assert_eq!((points.points, points.edges.len()), (set(&[1]), 0));
    assert_eq!(
        display_selection(&m, &stored, EditMode::Model),
        Default::default()
    );
}

#[test]
fn component_modes_show_the_wire() {
    use forge_ui::viewport_panel::component_shading;
    use forge_viewport::render::Shading::*;
    assert_eq!(component_shading(Smooth), SmoothWire);
    assert_eq!(component_shading(Flat), FlatWire);
    for s in [Wire, SmoothWire, FlatWire] {
        assert_eq!(component_shading(s), s);
    }
}
