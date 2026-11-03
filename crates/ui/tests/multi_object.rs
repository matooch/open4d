//! docs/specs/EDITABLE.md §6: component editing across several polygon objects in the
//! editor: picking, rectangle select, gizmo drags, numeric coordinates, snapping and a
//! kittest box select and drag. No GPU needed.

mod common;

use common::{create, state};
use egui::{Modifiers, Pos2, Vec2};
use egui_kittest::Harness;
use forge_core::{NodeId, Value};
use forge_generators::SELECTION_POINTS;
use forge_geometry::PolyMesh;
use forge_geometry::selection::IndexSet;
use forge_ui::app::Editor;
use forge_ui::attributes::Gesture;
use forge_ui::commands::execute;
use forge_ui::component_edit::{
    ComponentDrag, CoordEntry, component_coords, edit_parts, pivot_view, pivot_world,
};
use forge_ui::components::{
    ClickModifiers, EditMode, pick_mesh, selection_of, targets, viewport_click, viewport_rect,
};
use forge_ui::snapping::collect_sources_multi;
use forge_ui::state::EditorState;
use forge_viewport::gizmo::{DragDelta, Gizmo, GizmoMode};
use glam::DVec3;
use serde_json::json;

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

fn editable_at(st: &mut EditorState, pos: DVec3) -> NodeId {
    create(st, "forge.cube");
    execute(st, "mesh.make_editable", json!({})).unwrap();
    let id = st.selection.active.unwrap();
    execute(
        st,
        "transform.set",
        json!({"node": id.0, "pos": pos.to_array()}),
    )
    .unwrap();
    id
}

/// Two editable cubes at x = -250 and x = 250, both selected (B active), in `mode`.
fn two(mode: EditMode) -> (EditorState, NodeId, NodeId) {
    let mut st = state();
    let a = editable_at(&mut st, DVec3::new(-250.0, 0.0, 0.0));
    let b = editable_at(&mut st, DVec3::new(250.0, 0.0, 0.0));
    st.selection.nodes = [a, b].into_iter().collect();
    st.selection.active = Some(b);
    execute(
        &mut st,
        "select.mode",
        json!({"mode": forge_command::ModeArg::from(mode)}),
    )
    .unwrap();
    (st, a, b)
}

fn world_points(st: &EditorState, id: NodeId) -> Vec<DVec3> {
    pick_mesh(st.doc.scene(), id).unwrap().points
}

fn sel(st: &EditorState, id: NodeId, mode: EditMode) -> IndexSet {
    selection_of(st.doc.scene(), id, mode)
}

fn steps(st: &EditorState) -> usize {
    st.doc.history().undo_labels().len()
}

fn px(st: &EditorState, p: DVec3) -> [f64; 2] {
    let s = st.camera.project(p, W, H).expect("in front of the camera");
    [s.x, s.y]
}

fn depth(st: &EditorState, p: DVec3) -> f64 {
    st.camera.project(p, W, H).unwrap().z
}

fn near_corner(st: &EditorState, mesh: &PolyMesh) -> u32 {
    let d: Vec<f64> = mesh.points.iter().map(|p| depth(st, *p)).collect();
    (0..d.len()).min_by(|a, b| d[*a].total_cmp(&d[*b])).unwrap() as u32
}

fn close(a: DVec3, b: DVec3) {
    assert!((a - b).abs().max_element() < 1e-9, "{a} != {b}");
}

fn select_all_points(st: &mut EditorState) {
    execute(st, "select.all", json!({})).unwrap();
}

#[test]
fn the_targets_are_the_selected_polygon_objects() {
    let (mut st, a, b) = two(EditMode::Points);
    assert_eq!(targets(&st), vec![a, b]);
    // an unselected polygon object is not a target
    let c = editable_at(&mut st, DVec3::new(0.0, 300.0, 0.0));
    st.selection.nodes = [a, b].into_iter().collect();
    st.selection.active = Some(b);
    assert_eq!(targets(&st), vec![a, b]);
    st.selection.click(c);
    assert_eq!(targets(&st), vec![c]);
}

#[test]
fn a_click_picks_the_nearest_object_and_clears_the_others() {
    let (mut st, a, b) = two(EditMode::Points);
    // put B right behind A: translating A's near corner along its pick ray keeps the pixel
    let mesh = pick_mesh(st.doc.scene(), a).unwrap();
    let near = near_corner(&st, &mesh);
    let corner = mesh.points[near as usize];
    let cursor = px(&st, corner);
    let ray = st.camera.ray(cursor[0], cursor[1], W, H);
    let back = ray.dir * 400.0;
    // the far object is the lower id? make it so: B (higher id) is the near one
    execute(
        &mut st,
        "transform.set",
        json!({"node": a.0, "pos": (DVec3::new(-250.0, 0.0, 0.0) + back).to_array()}),
    )
    .unwrap();
    execute(
        &mut st,
        "transform.set",
        json!({"node": b.0, "pos": [-250.0, 0.0, 0.0]}),
    )
    .unwrap();
    // A now sits behind B along the pick ray: the same corner pixel hits both objects
    let near_b = pick_mesh(st.doc.scene(), b).unwrap();
    let near_b_corner = near_corner(&st, &near_b);
    assert_eq!(near_b_corner, near, "same mesh, same nearest corner");
    let [x, y] = px(&st, near_b.points[near_b_corner as usize]);
    // seed a selection in A to see it cleared
    st.doc
        .transact("seed", |tx| {
            tx.set_param(a, SELECTION_POINTS, Value::IndexList(vec![1, 2]))
        })
        .unwrap();
    let before = steps(&st);
    viewport_click(&mut st, [x, y], SIZE, NONE).unwrap();
    assert_eq!(steps(&st), before + 1, "one Select step for both objects");
    assert_eq!(
        sel(&st, b, EditMode::Points).iter().collect::<Vec<_>>(),
        [near]
    );
    assert!(
        sel(&st, a, EditMode::Points).is_empty(),
        "the far object is cleared"
    );
}

#[test]
fn click_add_remove_and_empty_click_work_across_objects() {
    let (mut st, a, b) = two(EditMode::Points);
    let ma = pick_mesh(st.doc.scene(), a).unwrap();
    let mb = pick_mesh(st.doc.scene(), b).unwrap();
    let (na, nb) = (near_corner(&st, &ma), near_corner(&st, &mb));
    let click = |st: &mut EditorState, p: DVec3, mods| {
        let c = px(st, p);
        viewport_click(st, c, SIZE, mods).unwrap();
    };
    click(&mut st, ma.points[na as usize], NONE);
    click(&mut st, mb.points[nb as usize], SHIFT);
    assert_eq!(sel(&st, a, EditMode::Points).len(), 1);
    assert_eq!(sel(&st, b, EditMode::Points).len(), 1);
    // Ctrl removes from the object that was hit only
    click(&mut st, mb.points[nb as usize], CMD);
    assert_eq!(sel(&st, a, EditMode::Points).len(), 1);
    assert!(sel(&st, b, EditMode::Points).is_empty());
    // a plain click on an element of B replaces: A is cleared
    click(&mut st, mb.points[nb as usize], NONE);
    assert!(sel(&st, a, EditMode::Points).is_empty());
    assert_eq!(sel(&st, b, EditMode::Points).len(), 1);
    // a plain click on nothing clears every target
    click(&mut st, mb.points[nb as usize], SHIFT);
    viewport_click(&mut st, [2.0, 2.0], SIZE, NONE).unwrap();
    assert!(sel(&st, a, EditMode::Points).is_empty());
    assert!(sel(&st, b, EditMode::Points).is_empty());
}

#[test]
fn a_rectangle_selects_in_every_target_in_one_step() {
    let (mut st, a, b) = two(EditMode::Points);
    let before = steps(&st);
    viewport_rect(&mut st, [0.0, 0.0, W, H], SIZE, NONE).unwrap();
    assert_eq!(steps(&st), before + 1);
    assert!(sel(&st, a, EditMode::Points).len() >= 7);
    assert!(sel(&st, b, EditMode::Points).len() >= 7);
    // a rectangle round B only: Replace empties A
    let mb = pick_mesh(st.doc.scene(), b).unwrap();
    let cs: Vec<[f64; 2]> = mb.points.iter().map(|p| px(&st, *p)).collect();
    let (x0, x1) = (
        cs.iter().map(|c| c[0]).fold(f64::MAX, f64::min) - 3.0,
        cs.iter().map(|c| c[0]).fold(f64::MIN, f64::max) + 3.0,
    );
    viewport_rect(&mut st, [x0, 0.0, x1, H], SIZE, NONE).unwrap();
    assert!(sel(&st, a, EditMode::Points).is_empty());
    assert!(sel(&st, b, EditMode::Points).len() >= 7);
}

#[test]
fn a_gizmo_drag_moves_the_points_of_both_objects_as_one_step() {
    let (mut st, a, b) = two(EditMode::Points);
    select_all_points(&mut st);
    let (wa, wb) = (world_points(&st, a), world_points(&st, b));
    close(pivot_world(&st).unwrap(), DVec3::ZERO);
    assert_eq!(edit_parts(&st).len(), 2);
    let drag = ComponentDrag::begin(&st).unwrap();
    assert_eq!(drag.moved().len(), 2);
    let before = steps(&st);
    for (n, d) in [(1u64, 5.0), (1, 15.0), (1, 30.0)] {
        drag.apply(
            &mut st.doc,
            DragDelta::Translate(DVec3::new(0.0, d, 0.0)),
            Gesture::Drag(n),
        )
        .unwrap();
    }
    assert_eq!(
        steps(&st),
        before + 1,
        "a drag over several frames is one step"
    );
    for (id, old) in [(a, &wa), (b, &wb)] {
        for (n, o) in world_points(&st, id).iter().zip(old) {
            close(*n, *o + DVec3::Y * 30.0);
        }
    }
    close(pivot_world(&st).unwrap(), DVec3::new(0.0, 30.0, 0.0));
    st.doc.undo();
    assert_eq!(world_points(&st, a), wa);
    assert_eq!(world_points(&st, b), wb);

    // rotate 90 degrees about y round the common pivot: A and B swap sides
    let drag = ComponentDrag::begin(&st).unwrap();
    drag.apply(
        &mut st.doc,
        DragDelta::Rotate {
            axis: DVec3::Y,
            angle: std::f64::consts::FRAC_PI_2,
        },
        Gesture::Commit,
    )
    .unwrap();
    let centre = |pts: Vec<DVec3>| pts.iter().copied().sum::<DVec3>() / pts.len() as f64;
    close(centre(world_points(&st, a)), DVec3::new(0.0, 0.0, 250.0));
    close(centre(world_points(&st, b)), DVec3::new(0.0, 0.0, -250.0));
}

#[test]
fn a_drag_with_soft_selection_stays_inside_each_object() {
    let (mut st, a, b) = two(EditMode::Points);
    execute(
        &mut st,
        "edit.soft_selection",
        json!({"enabled": true, "radius": 2000.0}),
    )
    .unwrap();
    st.doc
        .transact("seed", |tx| {
            tx.set_param(a, SELECTION_POINTS, Value::IndexList(vec![0]))
        })
        .unwrap();
    let wb = world_points(&st, b);
    let drag = ComponentDrag::begin(&st).unwrap();
    assert_eq!(drag.moved().len(), 1, "only A has affected points");
    drag.apply(
        &mut st.doc,
        DragDelta::Translate(DVec3::new(0.0, 20.0, 0.0)),
        Gesture::Commit,
    )
    .unwrap();
    assert_eq!(world_points(&st, b), wb, "the falloff never reaches B");
}

#[test]
fn typed_coordinates_move_rotate_and_size_the_selection() {
    let (mut st, a, b) = two(EditMode::Points);
    select_all_points(&mut st);
    let c = component_coords(&st, true).unwrap();
    close(c.pivot, DVec3::ZERO);
    close(c.size, DVec3::new(700.0, 200.0, 200.0));
    // several targets: Local shows the world pivot as well
    close(pivot_view(&st, false).unwrap(), DVec3::ZERO);

    let before = steps(&st);
    let drag = ComponentDrag::begin(&st).unwrap();
    drag.apply_entry(
        &mut st.doc,
        CoordEntry::Position(DVec3::new(10.0, 20.0, 30.0)),
        true,
        Gesture::Commit,
    )
    .unwrap();
    assert_eq!(steps(&st), before + 1, "both objects, one step");
    close(
        component_coords(&st, true).unwrap().pivot,
        DVec3::new(10.0, 20.0, 30.0),
    );
    close(
        component_coords(&st, true).unwrap().size,
        DVec3::new(700.0, 200.0, 200.0),
    );

    let drag = ComponentDrag::begin(&st).unwrap();
    drag.apply_entry(
        &mut st.doc,
        CoordEntry::Size(DVec3::new(350.0, 200.0, 100.0)),
        true,
        Gesture::Commit,
    )
    .unwrap();
    let c = component_coords(&st, true).unwrap();
    close(c.size, DVec3::new(350.0, 200.0, 100.0));
    close(c.pivot, DVec3::new(10.0, 20.0, 30.0));

    let drag = ComponentDrag::begin(&st).unwrap();
    drag.apply_entry(
        &mut st.doc,
        CoordEntry::Rotation(DVec3::new(std::f64::consts::FRAC_PI_2, 0.0, 0.0)),
        true,
        Gesture::Commit,
    )
    .unwrap();
    let c = component_coords(&st, true).unwrap();
    // 90 degrees of heading swaps the x and z extents
    close(c.size, DVec3::new(100.0, 200.0, 350.0));
    // A and B keep their own topology and stay separate meshes
    assert_eq!(world_points(&st, a).len(), 8);
    assert_eq!(world_points(&st, b).len(), 8);
}

#[test]
fn typed_coordinates_of_one_object_work_in_its_object_space() {
    let mut st = state();
    let a = editable_at(&mut st, DVec3::new(100.0, 0.0, 0.0));
    execute(&mut st, "select.mode", json!({"mode": "points"})).unwrap();
    select_all_points(&mut st);
    let obj = component_coords(&st, false).unwrap();
    close(obj.pivot, DVec3::ZERO);
    let world = component_coords(&st, true).unwrap();
    close(world.pivot, DVec3::new(100.0, 0.0, 0.0));
    let drag = ComponentDrag::begin(&st).unwrap();
    drag.apply_entry(
        &mut st.doc,
        CoordEntry::Position(DVec3::new(0.0, 50.0, 0.0)),
        false,
        Gesture::Commit,
    )
    .unwrap();
    close(
        component_coords(&st, false).unwrap().pivot,
        DVec3::new(0.0, 50.0, 0.0),
    );
    close(
        component_coords(&st, true).unwrap().pivot,
        DVec3::new(100.0, 50.0, 0.0),
    );
    let _ = a;
}

#[test]
fn the_moving_points_of_every_target_are_not_snap_targets() {
    let (mut st, a, b) = two(EditMode::Points);
    select_all_points(&mut st);
    let drag = ComponentDrag::begin(&st).unwrap();
    let skips = drag.moved();
    let all = collect_sources_multi(&st, &[], &[]);
    let rest = collect_sources_multi(&st, &[], &skips);
    for id in [a, b] {
        let i = all.nodes.iter().position(|n| *n == id).unwrap();
        let j = rest.nodes.iter().position(|n| *n == id).unwrap();
        assert!(all.sources[i].skip_points.is_empty());
        assert_eq!(rest.sources[j].skip_points.len(), 8, "{id:?}");
    }
}

#[test]
fn the_tool_panel_says_when_a_tool_works_on_one_object_only() {
    let (mut st, _a, _b) = two(EditMode::Polys);
    assert!(forge_ui::tools::activate_tool(&mut st, "loop_cut"));
    assert_eq!(
        forge_ui::tools::scope_note(&st).as_deref(),
        Some("Loop/Path Cut works on the active object only")
    );
    assert!(forge_ui::tools::activate_tool(&mut st, "extrude"));
    assert!(forge_ui::tools::scope_note(&st).is_none());
}

#[test]
fn a_tool_previews_and_applies_to_every_object_with_a_selection() {
    let (mut st, a, b) = two(EditMode::Polys);
    st.doc
        .transact("seed", |tx| {
            tx.set_param(a, "selection.polys", Value::IndexList(vec![0]))?;
            tx.set_param(b, "selection.polys", Value::IndexList(vec![1]))
        })
        .unwrap();
    assert!(forge_ui::tools::activate_tool(&mut st, "extrude"));
    assert_eq!(forge_ui::tools::tool_previews(&st).len(), 2);
    let before = steps(&st);
    forge_ui::tools::apply_tool(&mut st).unwrap();
    assert_eq!(steps(&st), before + 1, "one Extrude step for both objects");
    for id in [a, b] {
        let n = match st.doc.scene().param(id, "mesh") {
            Some(Value::Mesh(m)) => m.points.len(),
            _ => 0,
        };
        assert!(n > 8, "{id:?} was extruded");
    }
}

// ---- kittest: a box select and a drag across two objects -------------------------------------

fn harness(st: EditorState) -> Harness<'static, Editor> {
    let mut h = Harness::builder()
        .with_size([1280.0, 800.0])
        .build_ui_state(|ui, e: &mut Editor| e.show(ui, None), Editor::new(st));
    h.run();
    h
}

fn drag_pointer(h: &mut Harness<'_, Editor>, from: Pos2, dir: Vec2, to: Pos2) {
    h.hover_at(from);
    h.run();
    h.event(egui::Event::PointerButton {
        pos: from,
        button: egui::PointerButton::Primary,
        pressed: true,
        modifiers: Modifiers::NONE,
    });
    h.run();
    for p in [from + dir * 3.0, from + dir * 8.0, from + dir * 12.0, to] {
        h.hover_at(p);
        h.run();
    }
    h.event(egui::Event::PointerButton {
        pos: to,
        button: egui::PointerButton::Primary,
        pressed: false,
        modifiers: Modifiers::NONE,
    });
    h.run();
}

fn screen(h: &Harness<'_, Editor>, p: DVec3) -> Pos2 {
    let rect = h.state().view_rect();
    let cam = h.state().state.camera;
    let s = cam
        .project(p, f64::from(rect.width()), f64::from(rect.height()))
        .expect("in front of the camera");
    rect.min + Vec2::new(s.x as f32, s.y as f32)
}

#[test]
fn a_box_select_and_a_drag_work_across_two_objects() {
    let (st, a, b) = two(EditMode::Points);
    let mut h = harness(st);
    let rect = h.state().view_rect();
    assert!(rect.width() > 200.0, "the viewport was laid out");

    // box select: drag a rectangle from the top-left to the bottom-right of the viewport
    let before = h.state().state.doc.history().undo_labels().len();
    let from = rect.min + Vec2::new(4.0, 4.0);
    let to = rect.max - Vec2::new(4.0, 4.0);
    drag_pointer(&mut h, from, (to - from).normalized(), to);
    {
        let st = &h.state().state;
        assert!(sel(st, a, EditMode::Points).len() >= 7, "A has its points");
        assert!(sel(st, b, EditMode::Points).len() >= 7, "B has its points");
        assert_eq!(st.doc.history().undo_labels().len(), before + 1);
    }

    // drag the X handle of the gizmo at the common pivot
    let (wa, wb) = (
        world_points(&h.state().state, a),
        world_points(&h.state().state, b),
    );
    let pivot = pivot_world(&h.state().state).expect("a pivot");
    let g = Gizmo {
        origin: pivot,
        mode: GizmoMode::Translate,
        ..Gizmo::default()
    };
    let cam = h.state().state.camera;
    let l = g.world_length(&cam, f64::from(rect.height()));
    let p1 = screen(&h, pivot + DVec3::X * 0.8 * l);
    let p2 = screen(&h, pivot + DVec3::X * 0.9 * l);
    let dir = (p2 - p1).normalized();
    let steps_before = h.state().state.doc.history().undo_labels().len();
    drag_pointer(&mut h, p1, dir, p1 + dir * 60.0);
    let st = &h.state().state;
    let moved = |id: NodeId, old: &[DVec3]| {
        world_points(st, id)
            .iter()
            .zip(old)
            .map(|(n, o)| (*n - *o).x)
            .fold(f64::MIN, f64::max)
    };
    assert!(moved(a, &wa) > 1.0, "A moved along x");
    assert!(moved(b, &wb) > 1.0, "B moved along x");
    // the same shift for both objects
    let ia = sel(st, a, EditMode::Points).iter().next().unwrap() as usize;
    let ib = sel(st, b, EditMode::Points).iter().next().unwrap() as usize;
    let shift_a = (world_points(st, a)[ia] - wa[ia]).x;
    let shift_b = (world_points(st, b)[ib] - wb[ib]).x;
    assert!((shift_a - shift_b).abs() < 1e-6, "{shift_a} vs {shift_b}");
    assert_eq!(
        st.doc.history().undo_labels().len(),
        steps_before + 1,
        "one drag, one undo step for both objects"
    );
}
