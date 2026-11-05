//! docs/specs/MANIPULATE.md §6–7 in the editor: snapped gizmo drags (objects, components,
//! axis mode), the pen and cutting-tool snaps, the axis mode toggle. No GPU needed.

mod common;

use common::{create, state};
use egui::{Key, Modifiers, Pos2, Vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use forge_core::{NodeId, Value};
use forge_generators::axis::AxisEdit;
use forge_geometry::PolyMesh;
use forge_modeling::PenCorner;
use forge_ui::app::Editor;
use forge_ui::attributes::Gesture;
use forge_ui::building;
use forge_ui::commands::execute;
use forge_ui::cutting;
use forge_ui::manipulate::apply_axis_drag;
use forge_ui::snapping::{collect_sources, drag_step, hit_at, snap_pixel};
use forge_ui::state::EditorState;
use forge_ui::tools::activate_tool;
use forge_viewport::gizmo::{Axis, Drag, DragDelta, Gizmo, GizmoMode, Handle};
use glam::{DMat4, DVec3};
use serde_json::json;

const W: f64 = 800.0;
const H: f64 = 600.0;

fn mesh_of(st: &EditorState, id: NodeId) -> PolyMesh {
    match st.doc.scene().param(id, "mesh") {
        Some(Value::Mesh(m)) => forge_generators::to_poly_mesh(m),
        _ => panic!("mesh"),
    }
}

fn steps(st: &EditorState) -> usize {
    st.doc.history().undo_labels().len()
}

fn pos_of(st: &EditorState, id: NodeId) -> DVec3 {
    st.doc.scene().local_transform(id).unwrap().pos
}

fn world(st: &EditorState, id: NodeId) -> DMat4 {
    st.doc.scene().world_matrix(id).unwrap()
}

fn world_points(st: &EditorState, id: NodeId) -> Vec<DVec3> {
    let w = world(st, id);
    mesh_of(st, id)
        .points
        .iter()
        .map(|p| w.transform_point3(*p))
        .collect()
}

fn snap_set(st: &mut EditorState, args: serde_json::Value) {
    execute(st, "snap.set", args).unwrap();
}

/// A generator cube A at the origin and an editable cube B at x = 500, A selected.
fn two_cubes() -> (EditorState, NodeId, NodeId) {
    let mut st = state();
    let b = create(&mut st, "forge.cube");
    execute(&mut st, "mesh.make_editable", json!({})).unwrap();
    let b = st.selection.active.unwrap_or(b);
    execute(
        &mut st,
        "transform.set",
        json!({"node": b.0, "pos": [500.0, 0.0, 0.0]}),
    )
    .unwrap();
    let a = create(&mut st, "forge.cube");
    (st, a, b)
}

// ---- pure: sources and drag steps -------------------------------------------------------

#[test]
fn sources_leave_out_the_dragged_object_and_its_children() {
    let (mut st, a, b) = two_cubes();
    let kid = create(&mut st, "forge.cube");
    execute(
        &mut st,
        "object.reparent",
        json!({"node": kid.0, "parent": a.0}),
    )
    .unwrap();
    let all = collect_sources(&st, &[], None);
    assert!(all.nodes.contains(&a) && all.nodes.contains(&b) && all.nodes.contains(&kid));
    let rest = collect_sources(&st, &[a], None);
    assert_eq!(rest.nodes, vec![b]);
    assert_eq!(rest.sources[0].origin, Some(DVec3::new(500.0, 0.0, 0.0)));
    // moved points of the edited object are skipped, nothing else
    let skipped = collect_sources(&st, &[], Some((b, vec![5, 1, 5])));
    let i = skipped.nodes.iter().position(|n| *n == b).unwrap();
    assert_eq!(skipped.sources[i].skip_points, vec![1, 5]);
    assert!(
        skipped
            .sources
            .iter()
            .enumerate()
            .all(|(k, s)| k == i || s.skip_points.is_empty())
    );
}

fn x_axis_drag(st: &EditorState, from: DVec3) -> (Drag, Gizmo) {
    let g = Gizmo {
        origin: from,
        mode: GizmoMode::Translate,
        ..Gizmo::default()
    };
    let l = g.world_length(&st.camera, H);
    let p = st.camera.project(from + DVec3::X * 0.8 * l, W, H).unwrap();
    let d = Drag::begin(&g, Handle::Axis(Axis::X), &st.camera, p.x, p.y, W, H).unwrap();
    (d, g)
}

#[test]
fn a_target_hit_moves_the_origin_onto_it_and_quantize_is_the_fallback() {
    let (mut st, a, b) = two_cubes();
    snap_set(&mut st, json!({"enabled": true, "visible_only": false}));
    let scene = collect_sources(&st, &[a], None);
    // B's point nearest the gizmo: index with the smallest x then any
    let target = world_points(&st, b)[0];
    let px = st.camera.project(target, W, H).unwrap();
    let (mut drag, _) = x_axis_drag(&st, DVec3::ZERO);
    let settings = st.app.transform_settings.snap.clone();
    let (delta, hit) = drag_step(
        &settings,
        &scene.sources,
        &mut drag,
        &st.camera,
        (px.x + 2.0, px.y - 1.0),
        (W, H),
        GizmoMode::Translate,
        false,
    );
    let hit = hit.expect("vertex hit");
    assert_eq!(hit.kind, forge_modeling::snap::SnapKind::Vertex);
    assert_eq!(scene.node_of(&hit), Some(b));
    let DragDelta::Translate(v) = delta else {
        panic!()
    };
    assert!((v - DVec3::new(target.x, 0.0, 0.0)).length() < 1e-9, "{v}");
    // far from any target: no hit, free drag; with quantize the X delta is a multiple of the step
    let mut s2 = settings.clone();
    s2.quantize.enabled = true;
    s2.quantize.move_step = 25.0;
    let (mut drag, _) = x_axis_drag(&st, DVec3::ZERO);
    let p = st.camera.project(DVec3::new(37.0, 0.0, 0.0), W, H).unwrap();
    let (delta, hit) = drag_step(
        &s2,
        &scene.sources,
        &mut drag,
        &st.camera,
        (p.x + 60.0, p.y),
        (W, H),
        GizmoMode::Translate,
        false,
    );
    assert!(hit.is_none() || hit.unwrap().distance <= s2.radius);
    if hit.is_none() {
        let DragDelta::Translate(v) = delta else {
            panic!()
        };
        assert!((v.x / 25.0 - (v.x / 25.0).round()).abs() < 1e-9, "{v}");
    }
    // snapping off: Ctrl quantizes with the default 10 units
    let mut s3 = settings.clone();
    s3.enabled = false;
    let (mut drag, _) = x_axis_drag(&st, DVec3::ZERO);
    let (delta, hit) = drag_step(
        &s3,
        &scene.sources,
        &mut drag,
        &st.camera,
        (p.x + 33.0, p.y),
        (W, H),
        GizmoMode::Translate,
        true,
    );
    assert!(hit.is_none());
    let DragDelta::Translate(v) = delta else {
        panic!()
    };
    assert!((v.x / 10.0 - (v.x / 10.0).round()).abs() < 1e-9, "{v}");
    // rotate and scale only quantize
    let g = Gizmo {
        origin: DVec3::ZERO,
        mode: GizmoMode::Rotate,
        ..Gizmo::default()
    };
    let l = g.world_length(&st.camera, H);
    let ring = g.ring_points(Axis::Y, l)[5];
    let q = st.camera.project(ring, W, H).unwrap();
    let mut drag = Drag::begin(&g, Handle::Axis(Axis::Y), &st.camera, q.x, q.y, W, H).unwrap();
    let mut s4 = settings;
    s4.quantize.enabled = true;
    let ring2 = g.ring_points(Axis::Y, l)[12];
    let q2 = st.camera.project(ring2, W, H).unwrap();
    let (delta, hit) = drag_step(
        &s4,
        &scene.sources,
        &mut drag,
        &st.camera,
        (q2.x, q2.y),
        (W, H),
        GizmoMode::Rotate,
        false,
    );
    assert!(hit.is_none());
    let DragDelta::Rotate { angle, .. } = delta else {
        panic!()
    };
    let step = 15f64.to_radians();
    assert!(
        (angle / step - (angle / step).round()).abs() < 1e-9,
        "{angle}"
    );
}

#[test]
fn hit_at_needs_snapping_on() {
    let (mut st, a, _) = two_cubes();
    let scene = collect_sources(&st, &[a], None);
    let p = st
        .camera
        .project(DVec3::new(400.0, 100.0, 100.0), W, H)
        .unwrap();
    assert!(
        hit_at(&st, &scene, p.x, p.y, W, H).is_none(),
        "off by default"
    );
    snap_set(&mut st, json!({"enabled": true, "visible_only": false}));
    assert!(hit_at(&st, &scene, p.x, p.y, W, H).is_some());
    assert_eq!(
        snap_pixel(&st, p.x + 3.0, p.y + 2.0, W, H).0.round(),
        p.x.round()
    );
}

// ---- axis drags ----------------------------------------------------------------------------

#[test]
fn axis_drags_keep_the_world_and_are_one_undo_step() {
    let mut st = state();
    create(&mut st, "forge.cube");
    execute(&mut st, "mesh.make_editable", json!({})).unwrap();
    let p = st.selection.active.unwrap();
    execute(
        &mut st,
        "transform.set",
        json!({"node": p.0, "pos": [10.0, 20.0, 30.0], "rot": [20.0, 10.0, 5.0]}),
    )
    .unwrap();
    let kid = create(&mut st, "forge.null");
    execute(
        &mut st,
        "object.reparent",
        json!({"node": kid.0, "parent": p.0}),
    )
    .unwrap();
    execute(
        &mut st,
        "transform.set",
        json!({"node": kid.0, "pos": [5.0, 6.0, 7.0]}),
    )
    .unwrap();
    let (pts, kw) = (world_points(&st, p), world(&st, kid));
    let before = steps(&st);
    let edit = AxisEdit::capture(st.doc.scene(), p).unwrap();
    for (n, delta) in [
        DragDelta::Translate(DVec3::new(10.0, 0.0, 0.0)),
        DragDelta::Translate(DVec3::new(40.0, -5.0, 2.0)),
        DragDelta::Rotate {
            axis: DVec3::Y,
            angle: 0.5,
        },
        DragDelta::Scale(DVec3::new(2.0, 2.0, 2.0)),
    ]
    .into_iter()
    .enumerate()
    {
        apply_axis_drag(&mut st.doc, &edit, delta, Gesture::Drag(100)).unwrap();
        for (a, b) in pts.iter().zip(world_points(&st, p)) {
            assert!((*a - b).abs().max_element() < 1e-9, "{n}: {a} {b}");
        }
    }
    assert_eq!(steps(&st), before + 1, "one drag, one step");
    // the last delta was a uniform scale: the new scale is twice the start scale
    let now = st.doc.scene().local_transform(p).unwrap();
    assert!((now.scale - edit.start.scale * 2.0).abs().max_element() < 1e-9);
    assert!((world(&st, kid).w_axis - kw.w_axis).abs().max_element() < 1e-9);
    // a committed single step labelled for the axis
    let edit = AxisEdit::capture(st.doc.scene(), p).unwrap();
    apply_axis_drag(
        &mut st.doc,
        &edit,
        DragDelta::Translate(DVec3::X),
        Gesture::Commit,
    )
    .unwrap();
    assert_eq!(*st.doc.history().undo_labels().last().unwrap(), "Move axis");
}

// ---- pen and cutting tools -----------------------------------------------------------------

fn editable_cube() -> (EditorState, NodeId) {
    let mut st = state();
    create(&mut st, "forge.cube");
    execute(&mut st, "mesh.make_editable", json!({})).unwrap();
    let id = st.selection.active.unwrap();
    (st, id)
}

#[test]
fn the_pen_places_corners_on_snap_targets() {
    let (mut st, id) = editable_cube();
    snap_set(
        &mut st,
        json!({"enabled": true, "visible_only": false,
               "targets": {"edge_midpoint": true, "polygon_center": false,
                           "vertex": true, "edge": false}}),
    );
    activate_tool(&mut st, "polygon_pen");
    let m = mesh_of(&st, id);
    // a vertex: the Point corner (also without the old pick radius rule)
    let p0 = st.camera.project(m.points[0], W, H).unwrap();
    building::hover(&mut st, p0.x + 4.0, p0.y + 3.0, W, H);
    assert!(st.snap_hit.is_some(), "the marker follows the pen");
    building::click(&mut st).unwrap();
    // an edge midpoint: a New corner exactly there
    let e = m.edges()[0];
    let mid = (m.points[e.0 as usize] + m.points[e.1 as usize]) * 0.5;
    let pm = st.camera.project(mid, W, H).unwrap();
    building::hover(&mut st, pm.x - 2.0, pm.y + 2.0, W, H);
    building::click(&mut st).unwrap();
    let corners = st.tool.as_ref().unwrap().interaction.build.corners.clone();
    assert_eq!(corners[0], PenCorner::Point(0));
    let PenCorner::New(v) = corners[1] else {
        panic!("{corners:?}")
    };
    assert!((v - mid).length() < 1e-9, "{v} != {mid}");
}

#[test]
fn the_pen_without_snapping_keeps_the_plane_rule() {
    let (mut st, _) = editable_cube();
    activate_tool(&mut st, "polygon_pen");
    building::hover(&mut st, 60.0, 80.0, W, H);
    building::click(&mut st).unwrap();
    let corners = st.tool.as_ref().unwrap().interaction.build.corners.clone();
    assert!(matches!(corners[0], PenCorner::New(_)));
    assert!(st.snap_hit.is_none());
}

#[test]
fn the_knife_end_points_snap_to_projected_targets() {
    let (mut st, id) = editable_cube();
    activate_tool(&mut st, "knife_plane");
    snap_set(&mut st, json!({"enabled": true, "visible_only": false}));
    let m = mesh_of(&st, id);
    let a = st.camera.project(m.points[0], W, H).unwrap();
    let b = st.camera.project(m.points[7], W, H).unwrap();
    cutting::drag_begin(&mut st, a.x + 3.0, a.y - 2.0, [W, H]);
    cutting::drag_update(&mut st, b.x - 2.0, b.y + 3.0);
    let (rect, _) = st.tool.as_ref().unwrap().interaction.drag.unwrap();
    assert!(
        (rect[0] - a.x).abs() < 1e-9 && (rect[1] - a.y).abs() < 1e-9,
        "{rect:?}"
    );
    assert!(
        (rect[2] - b.x).abs() < 1e-9 && (rect[3] - b.y).abs() < 1e-9,
        "{rect:?}"
    );
    // off: the raw pixels
    snap_set(&mut st, json!({"enabled": false}));
    cutting::drag_update(&mut st, b.x - 2.0, b.y + 3.0);
    let (rect, _) = st.tool.as_ref().unwrap().interaction.drag.unwrap();
    assert_eq!((rect[2], rect[3]), (b.x - 2.0, b.y + 3.0));
}

#[test]
fn the_loop_cut_offset_snaps_to_the_edge_midpoint() {
    let (mut st, id) = editable_cube();
    activate_tool(&mut st, "loop_cut");
    let m = mesh_of(&st, id);
    // find an edge whose midpoint hovers that edge, then hover 4 px along it
    let mut found = None;
    for (i, e) in m.edges().iter().enumerate() {
        let (pa, pb) = (m.points[e.0 as usize], m.points[e.1 as usize]);
        let (Some(sa), Some(sb)) = (st.camera.project(pa, W, H), st.camera.project(pb, W, H))
        else {
            continue;
        };
        let mid = st.camera.project((pa + pb) * 0.5, W, H).unwrap();
        let dir = (sb - sa).truncate().normalize();
        let (x, y) = (mid.x + dir.x * 4.0, mid.y + dir.y * 4.0);
        cutting::hover(&mut st, x, y, W, H);
        if let Some(h) = st.tool.as_ref().unwrap().interaction.hover
            && h.edge == i as u32
        {
            found = Some((x, y, h.t));
            break;
        }
    }
    let (x, y, t_free) = found.expect("an edge to hover");
    assert!((t_free - 0.5).abs() > 1e-6, "off: the free offset {t_free}");
    snap_set(&mut st, json!({"enabled": true}));
    cutting::hover(&mut st, x, y, W, H);
    assert_eq!(st.tool.as_ref().unwrap().interaction.hover.unwrap().t, 0.5);
    // the midpoint target off: free again
    snap_set(&mut st, json!({"targets": {"edge_midpoint": false}}));
    cutting::hover(&mut st, x, y, W, H);
    assert!((st.tool.as_ref().unwrap().interaction.hover.unwrap().t - t_free).abs() < 1e-12);
}

// ---- kittest: pointer drags and the axis mode toggle ----------------------------------------

fn harness(st: EditorState) -> Harness<'static, Editor> {
    let mut h = Harness::builder()
        .with_size([1280.0, 800.0])
        .build_ui_state(|ui, e: &mut Editor| e.show(ui, None), Editor::new(st));
    h.run();
    h
}

/// Presses on `from`, moves along `dir` past the drag threshold (so the handle is still under
/// the pointer when the drag starts), then to `to`, and releases.
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

/// Egui position of a world point in the active viewport pane.
fn screen(h: &Harness<'_, Editor>, p: DVec3) -> Pos2 {
    let rect = h.state().view_rect();
    let cam = h.state().state.camera;
    let s = cam
        .project(p, f64::from(rect.width()), f64::from(rect.height()))
        .expect("in front of the camera");
    rect.min + Vec2::new(s.x as f32, s.y as f32)
}

/// The start of an X-handle drag at the object's origin: the handle point and its screen direction.
fn x_handle(h: &Harness<'_, Editor>, origin: DVec3) -> (Pos2, Vec2) {
    let rect = h.state().view_rect();
    let cam = h.state().state.camera;
    let g = Gizmo {
        origin,
        mode: GizmoMode::Translate,
        ..Gizmo::default()
    };
    let l = g.world_length(&cam, f64::from(rect.height()));
    let a = screen(h, origin + DVec3::X * 0.8 * l);
    let b = screen(h, origin + DVec3::X * 0.9 * l);
    (a, (b - a).normalized())
}

#[test]
fn a_snapped_gizmo_drag_lands_on_the_target_vertex() {
    let (mut st, a, b) = two_cubes();
    snap_set(&mut st, json!({"enabled": true, "visible_only": false}));
    let target = world_points(&st, b)[0];
    let mut h = harness(st);
    assert!(
        h.state().view_rect().width() > 100.0,
        "the viewport was laid out"
    );
    let before = steps(&h.state().state);
    let (from, dir) = x_handle(&h, DVec3::ZERO);
    let to = screen(&h, target) + Vec2::new(2.0, 1.0);
    drag_pointer(&mut h, from, dir, to);
    let st = &h.state().state;
    let p = pos_of(st, a);
    assert!(
        (p - DVec3::new(target.x, 0.0, 0.0)).length() < 1e-9,
        "{p} vs {target}"
    );
    assert_eq!(steps(st), before + 1, "one drag, one undo step");
    assert!(st.snap_hit.is_none(), "the marker goes with the drag");
    // the same drag with snapping off lands elsewhere
    let (mut st2, a2, b2) = two_cubes();
    let target2 = world_points(&st2, b2)[0];
    st2.camera = st.camera;
    let mut h2 = harness(st2);
    let (from, dir) = x_handle(&h2, DVec3::ZERO);
    let to = screen(&h2, target2) + Vec2::new(2.0, 1.0);
    drag_pointer(&mut h2, from, dir, to);
    let free = pos_of(&h2.state().state, a2);
    assert!(free.x.abs() > 1.0, "the free drag moved: {free}");
    assert!(
        (free.x - target2.x).abs() > 1e-6,
        "and did not land on the vertex: {free}"
    );
}

#[test]
fn the_axis_mode_toggle_and_an_axis_drag() {
    let mut st = state();
    create(&mut st, "forge.cube");
    execute(&mut st, "mesh.make_editable", json!({})).unwrap();
    let p = st.selection.active.unwrap();
    let mut h = harness(st);
    assert!(!h.state().state.app.transform_settings.axis_mode);
    // the key
    h.key_press(Key::L);
    h.run();
    assert!(
        h.state().state.app.transform_settings.axis_mode,
        "L toggles axis mode on"
    );
    // the menu shows it as selected and toggles it
    h.get_by_label("Edit").click();
    h.run();
    h.get_by_label_contains("Axis mode").click();
    h.run();
    assert!(!h.state().state.app.transform_settings.axis_mode);
    h.key_press(Key::L);
    h.run();
    assert!(h.state().state.app.transform_settings.axis_mode);
    // drag the X handle: the axis moves, the points stay
    let (pts, before) = (world_points(&h.state().state, p), steps(&h.state().state));
    let (from, dir) = x_handle(&h, DVec3::ZERO);
    drag_pointer(&mut h, from, dir, from + dir * 60.0);
    let st = &h.state().state;
    let moved = pos_of(st, p);
    assert!(
        moved.x.abs() > 1.0 && moved.y.abs() < 1e-9 && moved.z.abs() < 1e-9,
        "{moved}"
    );
    for (a, b) in pts.iter().zip(world_points(st, p)) {
        assert!((*a - b).abs().max_element() < 1e-9, "{a} {b}");
    }
    assert_eq!(steps(st), before + 1, "one undo step");
    assert_eq!(*st.doc.history().undo_labels().last().unwrap(), "Move axis");
    // undo puts the axis and the points back
    h.state_mut().state.doc.undo();
    let st = &h.state().state;
    assert_eq!(pos_of(st, p), DVec3::ZERO);
    assert_eq!(world_points(st, p), pts);
    // the mode off again: the same drag moves the object and its points with it
    h.key_press(Key::L);
    h.run();
    assert!(!h.state().state.app.transform_settings.axis_mode);
    let (from, dir) = x_handle(&h, DVec3::ZERO);
    drag_pointer(&mut h, from, dir, from + dir * 60.0);
    let st = &h.state().state;
    assert!(pos_of(st, p).x.abs() > 1.0);
    let moved_pts = world_points(st, p);
    assert!((moved_pts[0] - pts[0]).length() > 1.0, "the object moved");
}
