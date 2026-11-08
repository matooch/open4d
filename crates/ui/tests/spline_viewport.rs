//! docs/specs/VIEWPORT.md section 11: splines in the editor. Draw data, Points-mode selection,
//! scripted drags (one undo step each), mirrored and broken handles, snapping, screenshot.

mod common;

use common::{create, state};
use forge_core::NodeId;
use forge_generators::editable_spline::{SplineData, data_of};
use forge_ui::commands::execute;
use forge_ui::components::{ClickModifiers, EditMode};
use forge_ui::spline_edit::{SplineDrag, click, editor_draws};
use forge_ui::state::EditorState;
use forge_viewport::camera::Camera;
use forge_viewport::curves::{ControlHit, PICK_RADIUS, pick};
use glam::DVec3;
use serde_json::json;

const SIZE: [f64; 2] = [800.0, 600.0];
const PLAIN: ClickModifiers = ClickModifiers {
    shift: false,
    command: false,
};
const SHIFT: ClickModifiers = ClickModifiers {
    shift: true,
    command: false,
};

fn front_camera() -> Camera {
    Camera {
        distance: 1500.0,
        ..Camera::default()
    }
}

/// An editable circle (radius 200, 4 Bezier points) in Points mode, seen from the front.
fn editing() -> (EditorState, NodeId) {
    let mut st = state();
    create(&mut st, "forge.circle_spline");
    execute(&mut st, "mesh.make_editable", json!({})).unwrap();
    let id = st.selection.active.unwrap();
    points_mode(&mut st);
    st.camera = front_camera();
    (st, id)
}

fn points_mode(st: &mut EditorState) {
    execute(
        st,
        "select.mode",
        json!({"mode": forge_command::ModeArg::from(EditMode::Points)}),
    )
    .unwrap();
}

fn data(st: &EditorState, id: NodeId) -> SplineData {
    data_of(st.doc.scene(), id).unwrap()
}

fn steps(st: &EditorState) -> usize {
    st.doc.history().undo_labels().len()
}

fn px(st: &EditorState, p: DVec3) -> [f64; 2] {
    let s = st.camera.project(p, SIZE[0], SIZE[1]).unwrap();
    [s.x, s.y]
}

fn wpp(st: &EditorState) -> f64 {
    st.camera.world_per_pixel(SIZE[1])
}

fn close(a: DVec3, b: DVec3, eps: f64) {
    assert!((a - b).abs().max_element() < eps, "{a} != {b}");
}

#[test]
fn primitives_and_editable_splines_draw_as_polylines() {
    let mut st = state();
    create(&mut st, "forge.circle_spline");
    let scene = st.doc.scene().clone();
    let draws = editor_draws(&scene, &st);
    assert_eq!(draws.len(), 1);
    assert!(draws[0].selected, "the new object is selected");
    assert!(draws[0].controls.is_none(), "Model mode shows no controls");
    let l = &draws[0].loops[0];
    assert!(l.closed && l.points.len() > 8);
    // the circle's points are on its radius, the same sampling as the generators
    for p in &l.points {
        assert!((p.length() - 200.0).abs() < 0.5, "{p}");
    }

    execute(&mut st, "mesh.make_editable", json!({})).unwrap();
    let scene = st.doc.scene().clone();
    let model = editor_draws(&scene, &st);
    assert_eq!(model.len(), 1);
    assert!(model[0].controls.is_none());
    assert_eq!(
        model[0].loops, draws[0].loops,
        "Make Editable keeps the curve"
    );

    points_mode(&mut st);
    let edit = editor_draws(&scene, &st);
    let c = edit[0]
        .controls
        .as_ref()
        .expect("Points mode shows controls");
    assert_eq!(c.points.len(), 4);
    assert!(c.handles, "a Bezier spline has handles");
    assert!((c.points[0] - DVec3::new(200.0, 0.0, 0.0)).length() < 1e-9);
    assert!((c.right[0] - c.points[0]).length() > 100.0);
}

#[test]
fn a_spline_follows_its_object_transform() {
    let (mut st, id) = editing();
    execute(
        &mut st,
        "param.set",
        json!({"node": id.0, "param": "transform.pos", "value": [10.0, 20.0, 30.0]}),
    )
    .unwrap();
    let world = st.doc.scene().world_matrix(id).unwrap();
    assert_eq!(world.w_axis.truncate(), DVec3::new(10.0, 20.0, 30.0));
    let scene = st.doc.scene().clone();
    let c = editor_draws(&scene, &st)[0].controls.clone().unwrap();
    close(
        c.points[0],
        world.transform_point3(DVec3::new(200.0, 0.0, 0.0)),
        1e-9,
    );
}

#[test]
fn clicks_select_points_with_shift_adding_and_empty_space_clearing() {
    let (mut st, id) = editing();
    let base = steps(&st);
    let p0 = px(&st, DVec3::new(200.0, 0.0, 0.0));
    let p1 = px(&st, DVec3::new(0.0, 200.0, 0.0));
    assert!(click(&mut st, p0, SIZE, PLAIN));
    assert_eq!(st.spline_sel.get(id).into_iter().collect::<Vec<_>>(), [0]);
    click(&mut st, p1, SIZE, SHIFT);
    assert_eq!(
        st.spline_sel.get(id).into_iter().collect::<Vec<_>>(),
        [0, 1]
    );
    click(&mut st, p1, SIZE, PLAIN);
    assert_eq!(st.spline_sel.get(id).into_iter().collect::<Vec<_>>(), [1]);
    click(
        &mut st,
        p0,
        SIZE,
        ClickModifiers {
            shift: false,
            command: true,
        },
    );
    assert_eq!(
        st.spline_sel.get(id).into_iter().collect::<Vec<_>>(),
        [0, 1]
    );
    // the selection shows in the draw data
    let scene = st.doc.scene().clone();
    let c = editor_draws(&scene, &st)[0].controls.clone().unwrap();
    assert_eq!(c.selected, [true, true, false, false]);
    // empty space: Shift keeps, a plain click clears
    click(&mut st, [20.0, 20.0], SIZE, SHIFT);
    assert_eq!(st.spline_sel.get(id).len(), 2);
    click(&mut st, [20.0, 20.0], SIZE, PLAIN);
    assert!(st.spline_sel.get(id).is_empty());
    assert_eq!(steps(&st), base, "selecting is not an undo step");
}

#[test]
fn clicks_do_nothing_without_an_edited_spline() {
    let mut st = state();
    create(&mut st, "forge.circle_spline");
    assert!(!click(&mut st, [400.0, 300.0], SIZE, PLAIN), "Model mode");
    execute(&mut st, "mesh.make_editable", json!({})).unwrap();
    assert!(!click(&mut st, [400.0, 300.0], SIZE, PLAIN), "Model mode");
    // Points mode on a primitive: not editable
    let mut st = state();
    create(&mut st, "forge.circle_spline");
    points_mode(&mut st);
    assert!(!click(&mut st, [400.0, 300.0], SIZE, PLAIN));
    assert!(SplineDrag::begin(&mut st, [400.0, 300.0], SIZE, PLAIN, 1).is_none());
}

#[test]
fn a_point_drag_is_one_undo_step_and_undoes_in_one_go() {
    let (mut st, id) = editing();
    let before = data(&st, id);
    let base = steps(&st);
    let start = px(&st, before.points[0]);
    let mut drag = SplineDrag::begin(&mut st, start, SIZE, PLAIN, 7).expect("a point is there");
    assert_eq!(st.spline_sel.get(id).into_iter().collect::<Vec<_>>(), [0]);
    for k in 1..=5 {
        let at = [start[0] + 10.0 * k as f64, start[1] - 4.0 * k as f64];
        drag.update(&mut st, at, SIZE, false).unwrap();
    }
    assert_eq!(steps(&st), base + 1, "one gesture, one undo step");
    let after = data(&st, id);
    let d = wpp(&st);
    // 50 px right, 20 px up on the camera plane (camera looks down -Z, +Y up)
    close(
        after.points[0],
        before.points[0] + DVec3::new(50.0 * d, 20.0 * d, 0.0),
        1e-6,
    );
    assert_eq!(after.points[1..], before.points[1..], "other points stay");
    assert_eq!(
        after.left, before.left,
        "handles are offsets: they travel along"
    );
    assert_eq!(after.right, before.right);
    // a second drag is a second step
    let p1 = px(&st, after.points[1]);
    let mut again = SplineDrag::begin(&mut st, p1, SIZE, PLAIN, 8).unwrap();
    again
        .update(&mut st, [p1[0] + 5.0, p1[1]], SIZE, false)
        .unwrap();
    assert_eq!(steps(&st), base + 2);
    execute(&mut st, "edit.undo", json!({})).unwrap();
    execute(&mut st, "edit.undo", json!({})).unwrap();
    assert_eq!(data(&st, id), before);
    assert_eq!(steps(&st), base);
}

#[test]
fn a_press_without_motion_changes_nothing() {
    let (mut st, id) = editing();
    let before = data(&st, id);
    let base = steps(&st);
    let at = px(&st, before.points[2]);
    let mut drag = SplineDrag::begin(&mut st, at, SIZE, PLAIN, 3).unwrap();
    drag.update(&mut st, at, SIZE, false).unwrap();
    assert_eq!(steps(&st), base);
    assert_eq!(data(&st, id), before);
}

#[test]
fn dragging_a_selected_point_moves_all_selected_points() {
    let (mut st, id) = editing();
    let before = data(&st, id);
    let (a, b) = (px(&st, before.points[0]), px(&st, before.points[2]));
    click(&mut st, a, SIZE, PLAIN);
    click(&mut st, b, SIZE, SHIFT);
    let start = px(&st, before.points[2]);
    let mut drag = SplineDrag::begin(&mut st, start, SIZE, PLAIN, 2).unwrap();
    drag.update(&mut st, [start[0], start[1] + 30.0], SIZE, false)
        .unwrap();
    let after = data(&st, id);
    let delta = DVec3::new(0.0, -30.0 * wpp(&st), 0.0);
    close(after.points[0], before.points[0] + delta, 1e-6);
    close(after.points[2], before.points[2] + delta, 1e-6);
    assert_eq!(after.points[1], before.points[1]);
    // pressing an unselected point replaces the selection
    let p1 = px(&st, after.points[1]);
    SplineDrag::begin(&mut st, p1, SIZE, PLAIN, 3).unwrap();
    assert_eq!(st.spline_sel.get(id).into_iter().collect::<Vec<_>>(), [1]);
}

#[test]
fn a_handle_drag_mirrors_the_opposite_handle_and_shift_breaks_the_link() {
    let (mut st, id) = editing();
    let before = data(&st, id);
    let end = before.points[0] + before.right[0];
    let start = px(&st, end);
    assert_eq!(
        {
            let scene = st.doc.scene().clone();
            let c = editor_draws(&scene, &st)[0].clone();
            pick(
                &c,
                &st.camera,
                (start[0], start[1]),
                (SIZE[0], SIZE[1]),
                PICK_RADIUS,
            )
        },
        Some(ControlHit::Right(0))
    );
    let base = steps(&st);
    let mut drag = SplineDrag::begin(&mut st, start, SIZE, PLAIN, 11).unwrap();
    drag.update(&mut st, [start[0] + 20.0, start[1]], SIZE, false)
        .unwrap();
    drag.update(&mut st, [start[0] + 40.0, start[1]], SIZE, false)
        .unwrap();
    assert_eq!(steps(&st), base + 1);
    let after = data(&st, id);
    let moved = before.right[0] + DVec3::new(40.0 * wpp(&st), 0.0, 0.0);
    close(after.right[0], moved, 1e-6);
    close(after.left[0], -moved, 1e-6);
    assert_eq!(after.points, before.points, "the point stays");
    assert_eq!(after.right[1..], before.right[1..]);

    // Shift at the press: only the dragged handle changes
    execute(&mut st, "edit.undo", json!({})).unwrap();
    let mut drag = SplineDrag::begin(&mut st, start, SIZE, SHIFT, 12).unwrap();
    drag.update(&mut st, [start[0] + 40.0, start[1]], SIZE, false)
        .unwrap();
    let broken = data(&st, id);
    close(broken.right[0], moved, 1e-6);
    assert_eq!(broken.left[0], before.left[0], "the link is broken");
}

#[test]
fn the_left_handle_drags_too() {
    let (mut st, id) = editing();
    let before = data(&st, id);
    let start = px(&st, before.points[1] + before.left[1]);
    let mut drag = SplineDrag::begin(&mut st, start, SIZE, PLAIN, 5).unwrap();
    drag.update(&mut st, [start[0], start[1] - 25.0], SIZE, false)
        .unwrap();
    let after = data(&st, id);
    let moved = before.left[1] + DVec3::new(0.0, 25.0 * wpp(&st), 0.0);
    close(after.left[1], moved, 1e-6);
    close(after.right[1], -moved, 1e-6);
}

#[test]
fn point_drags_snap_to_mesh_points_and_quantize() {
    let mut st = state();
    create(&mut st, "forge.cube"); // 200 wide: corners at +-100
    create(&mut st, "forge.circle_spline");
    execute(&mut st, "mesh.make_editable", json!({})).unwrap();
    let id = st.selection.active.unwrap();
    points_mode(&mut st);
    st.camera = front_camera();
    execute(
        &mut st,
        "snap.set",
        json!({"enabled": true, "visible_only": false}),
    )
    .unwrap();
    let before = data(&st, id);
    let corner = DVec3::new(100.0, 100.0, 100.0);
    let start = px(&st, before.points[0]);
    let target = px(&st, corner);
    let base = steps(&st);
    let mut drag = SplineDrag::begin(&mut st, start, SIZE, PLAIN, 1).unwrap();
    drag.update(&mut st, target, SIZE, false).unwrap();
    assert_eq!(steps(&st), base + 1);
    close(data(&st, id).points[0], corner, 1e-6);
    execute(&mut st, "edit.undo", json!({})).unwrap();

    // snapping off: Ctrl quantizes the world delta to the move step (default 10)
    execute(&mut st, "snap.set", json!({"enabled": false})).unwrap();
    let mut drag = SplineDrag::begin(&mut st, start, SIZE, PLAIN, 2).unwrap();
    drag.update(&mut st, [start[0] + 13.0, start[1]], SIZE, true)
        .unwrap();
    let x = data(&st, id).points[0].x - before.points[0].x;
    assert!((x / 10.0 - (x / 10.0).round()).abs() < 1e-9, "{x}");
    assert!(x > 0.0);
    // handles are not snapped: the same drag moves by the exact amount
    let now = data(&st, id);
    let hs = px(&st, now.points[0] + now.right[0]);
    execute(
        &mut st,
        "snap.set",
        json!({"enabled": true, "visible_only": false}),
    )
    .unwrap();
    let mut drag = SplineDrag::begin(&mut st, hs, SIZE, PLAIN, 3).unwrap();
    drag.update(&mut st, [hs[0] + 13.0, hs[1]], SIZE, false)
        .unwrap();
    let got = data(&st, id).right[0].x - now.right[0].x;
    assert!((got - 13.0 * wpp(&st)).abs() < 1e-6, "{got}");
}

fn skip(message: &str) {
    if std::env::var("FORGE_REQUIRE_GPU").as_deref() == Ok("1") {
        panic!("FORGE_REQUIRE_GPU=1 but the screenshot failed: {message}");
    }
    eprintln!("skipping GPU test: {message}");
}

fn decode(png_bytes: &[u8]) -> (u32, Vec<u8>) {
    let mut reader = png::Decoder::new(std::io::Cursor::new(png_bytes))
        .read_info()
        .expect("valid PNG");
    let mut pixels = vec![0; reader.output_buffer_size().unwrap()];
    let info = reader.next_frame(&mut pixels).unwrap();
    (info.width, pixels)
}

#[test]
fn screenshots_show_the_spline() {
    let mut st = state();
    create(&mut st, "forge.circle_spline");
    let (w, h) = (320u32, 240u32);
    let shot = match forge_ui::screenshot::render_png(&mut st, w, h) {
        Ok(b) => b,
        Err(e) if e.contains("no GPU") => return skip(&e),
        Err(e) => panic!("{e}"),
    };
    let (iw, px) = decode(&shot);
    assert_eq!(iw, w);
    let count = |colour: [u8; 3]| {
        px.chunks(4)
            .filter(|p| (0..3).all(|c| (p[c] as i32 - colour[c] as i32).abs() <= 2))
            .count()
    };
    // the selected circle is drawn in the selection colour (255, 140, 26)
    assert!(count([255, 140, 26]) > 40, "{}", count([255, 140, 26]));
    // an unselected spline uses the object colour
    execute(&mut st, "select.none", json!({})).ok();
    st.selection.clear();
    let shot = forge_ui::screenshot::render_png(&mut st, w, h).unwrap();
    let (_, px2) = decode(&shot);
    assert!(
        px2.chunks(4)
            .filter(|p| (0..3).all(|c| (p[c] as i32 - [179, 179, 179][c]).abs() <= 2))
            .count()
            > 40
    );
    // Points mode on an editable spline adds the control point squares
    let mut st = state();
    create(&mut st, "forge.circle_spline");
    execute(&mut st, "mesh.make_editable", json!({})).unwrap();
    points_mode(&mut st);
    let shot = forge_ui::screenshot::render_png(&mut st, w, h).unwrap();
    let (_, px3) = decode(&shot);
    let handles = px3
        .chunks(4)
        .filter(|p| p[0] == 115 && (p[1] as i32 - 191).abs() <= 2 && (p[2] as i32 - 242).abs() <= 2)
        .count();
    assert!(
        handles > 20,
        "handle lines and squares are drawn: {handles}"
    );
}
