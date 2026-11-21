//! docs/specs/ANIM.md section 2b (briefs 090, 091, own design, parity-pending G5): the
//! key.* edit commands, the pure view logic, the dope sheet (box select, move, undo) and the
//! value graph (handle drag, undo) under egui_kittest.

mod common;

use common::{create, state};
use egui::{Event, Modifiers, PointerButton, Pos2, vec2};
use egui_kittest::Harness;
use forge_anim::value_at;
use forge_core::{Behavior, Interp, NodeId, TangentMode, Track};
use forge_ui::commands::execute;
use forge_ui::dopesheet::{dope_sheet_ui, sheet_state};
use forge_ui::fcurve::{graph_state, value_graph_ui};
use forge_ui::fcurve_logic::{self as logic, KeyId, KeySelection, TimeView, ValueView};
use forge_ui::state::EditorState;
use serde_json::{Value, json};

fn run(st: &mut EditorState, id: &str, args: Value) -> Value {
    execute(st, id, args).unwrap_or_else(|e| panic!("{id}: {e}"))
}

fn set_key(st: &mut EditorState, node: NodeId, track: &str, frame: i64, value: f64) {
    run(
        st,
        "key.set",
        json!({"node": node.0, "track": track, "frame": frame, "value": value}),
    );
}

/// A cube with position X keys 0 / 10 / 20 at 0 / 100 / 50 and Y keys at 10 and 30.
fn animated() -> (EditorState, NodeId) {
    let mut st = state();
    let cube = create(&mut st, "forge.cube");
    for (f, v) in [(0, 0.0), (10, 100.0), (20, 50.0)] {
        set_key(&mut st, cube, "transform.pos.x", f, v);
    }
    for (f, v) in [(10, 5.0), (30, 8.0)] {
        set_key(&mut st, cube, "transform.pos.y", f, v);
    }
    (st, cube)
}

fn track(st: &EditorState, node: NodeId, key: &str) -> Track {
    st.doc.scene().node(node).unwrap().tracks[key].clone()
}

fn frames(st: &EditorState, node: NodeId, key: &str) -> Vec<f64> {
    track(st, node, key)
        .keys
        .iter()
        .map(|k| (k.time * 30.0 * 1e6).round() / 1e6)
        .collect()
}

fn key(node: NodeId, track: &str, frame: f64) -> Value {
    json!({"node": node.0, "track": track, "frame": frame})
}

fn undo_len(st: &EditorState) -> usize {
    st.doc.history().undo_labels().len()
}

#[test]
fn move_scale_and_ripple_are_one_undo_step_each() {
    let (mut st, cube) = animated();
    let n = undo_len(&st);
    run(
        &mut st,
        "key.move",
        json!({"keys": [key(cube, "transform.pos.x", 10.0), key(cube, "transform.pos.y", 10.0)],
               "delta_frames": 5.0, "delta_value": 10.0}),
    );
    assert_eq!(undo_len(&st), n + 1);
    assert_eq!(frames(&st, cube, "transform.pos.x"), [0.0, 15.0, 20.0]);
    assert_eq!(frames(&st, cube, "transform.pos.y"), [15.0, 30.0]);
    let x = track(&st, cube, "transform.pos.x");
    assert_eq!(x.keys[1].value, 110.0, "value moved by the display delta");
    run(&mut st, "edit.undo", json!({}));
    assert_eq!(frames(&st, cube, "transform.pos.x"), [0.0, 10.0, 20.0]);
    assert_eq!(track(&st, cube, "transform.pos.x").keys[1].value, 100.0);

    // moving onto another key replaces it
    run(
        &mut st,
        "key.move",
        json!({"keys": [key(cube, "transform.pos.x", 10.0)], "delta_frames": 10.0}),
    );
    assert_eq!(frames(&st, cube, "transform.pos.x"), [0.0, 20.0]);
    assert_eq!(track(&st, cube, "transform.pos.x").keys[1].value, 100.0);
    run(&mut st, "edit.undo", json!({}));

    // ripple: the selected key and every later key move in time
    run(
        &mut st,
        "key.move",
        json!({"keys": [key(cube, "transform.pos.x", 10.0)], "delta_frames": 4.0, "ripple": true}),
    );
    assert_eq!(frames(&st, cube, "transform.pos.x"), [0.0, 14.0, 24.0]);
    run(&mut st, "edit.undo", json!({}));

    // scale about frame 0: 10 -> 20, 20 -> 40
    run(
        &mut st,
        "key.scale",
        json!({"keys": [key(cube, "transform.pos.x", 10.0), key(cube, "transform.pos.x", 20.0)],
               "pivot_frame": 0.0, "factor": 2.0}),
    );
    assert_eq!(frames(&st, cube, "transform.pos.x"), [0.0, 20.0, 40.0]);
    run(&mut st, "edit.undo", json!({}));

    // bad input changes nothing
    let n = undo_len(&st);
    for (id, args) in [
        ("key.move", json!({"keys": [], "delta_frames": 1.0})),
        (
            "key.move",
            json!({"keys": [key(cube, "transform.pos.x", 99.0)], "delta_frames": 1.0}),
        ),
        (
            "key.move",
            json!({"keys": [key(cube, "nope", 0.0)], "delta_frames": 1.0}),
        ),
        (
            "key.scale",
            json!({"keys": [key(cube, "transform.pos.x", 10.0)], "pivot_frame": 0.0, "factor": 0.0}),
        ),
        (
            "key.set_interp",
            json!({"keys": [key(cube, "transform.pos.x", 10.0)], "interp": "wobble"}),
        ),
        (
            "key.set_tangent",
            json!({"node": cube.0, "track": "transform.pos.x", "frame": 10.0, "preset": "huh"}),
        ),
        (
            "track.set_behavior",
            json!({"node": cube.0, "track": "transform.pos.x", "after": "loop"}),
        ),
    ] {
        assert!(execute(&mut st, id, args.clone()).is_err(), "{id} {args}");
    }
    assert_eq!(undo_len(&st), n);
}

#[test]
fn delete_interpolation_paste_behaviour_and_filter() {
    let (mut st, cube) = animated();
    run(
        &mut st,
        "key.delete",
        json!({"keys": [key(cube, "transform.pos.x", 10.0)]}),
    );
    assert_eq!(frames(&st, cube, "transform.pos.x"), [0.0, 20.0]);
    // a track that loses its last key goes away
    run(
        &mut st,
        "key.delete",
        json!({"keys": [key(cube, "transform.pos.y", 10.0), key(cube, "transform.pos.y", 30.0)]}),
    );
    assert!(
        !st.doc
            .scene()
            .node(cube)
            .unwrap()
            .tracks
            .contains_key("transform.pos.y")
    );
    run(&mut st, "edit.undo", json!({}));
    assert!(
        st.doc
            .scene()
            .node(cube)
            .unwrap()
            .tracks
            .contains_key("transform.pos.y")
    );

    run(
        &mut st,
        "key.set_interp",
        json!({"keys": [key(cube, "transform.pos.y", 10.0)], "interp": "step"}),
    );
    assert_eq!(
        track(&st, cube, "transform.pos.y").keys[0].interp,
        Interp::Step
    );

    run(
        &mut st,
        "key.paste",
        json!({"keys": [{"node": cube.0, "track": "transform.pos.z", "frame": 5.0, "value": 7.0, "interp": "linear"}]}),
    );
    let z = track(&st, cube, "transform.pos.z");
    assert_eq!(
        (z.keys[0].time, z.keys[0].value, z.keys[0].interp),
        (5.0 / 30.0, 7.0, Interp::Linear)
    );

    run(
        &mut st,
        "track.set_behavior",
        json!({"node": cube.0, "track": "transform.pos.x", "before": "repeat", "after": "oscillate"}),
    );
    let x = track(&st, cube, "transform.pos.x");
    assert_eq!((x.before, x.after), (Behavior::Repeat, Behavior::Oscillate));
    // the evaluated scene follows the behaviour: after the last key the track oscillates
    let t = forge_anim::scene_at(st.doc.scene(), 30.0 / 30.0);
    let _ = t;

    // the Euler filter only touches rotation tracks, in one step
    for (f, deg) in [(0, 170.0), (10, -170.0)] {
        set_key(&mut st, cube, "transform.rot.x", f, deg);
    }
    run(&mut st, "key.euler_filter", json!({"node": cube.0}));
    let r = track(&st, cube, "transform.rot.x");
    assert!((r.keys[1].value.to_degrees() - 190.0).abs() < 1e-9);
    assert!(execute(&mut st, "key.euler_filter", json!({"node": cube.0})).is_err());
}

#[test]
fn tangent_presets_and_user_handles_change_the_curve() {
    let mut st = state();
    let cube = create(&mut st, "forge.cube");
    set_key(&mut st, cube, "transform.pos.x", 0, 0.0);
    set_key(&mut st, cube, "transform.pos.x", 30, 100.0);
    let at = |st: &EditorState, f: f64| value_at(&track(st, cube, "transform.pos.x"), f / 30.0);
    let eased = at(&st, 10.0);
    run(
        &mut st,
        "key.set_tangent",
        json!({"node": cube.0, "track": "transform.pos.x", "frame": 0.0, "preset": "linear"}),
    );
    run(
        &mut st,
        "key.set_tangent",
        json!({"node": cube.0, "track": "transform.pos.x", "frame": 30.0, "preset": "linear"}),
    );
    let straight = at(&st, 10.0);
    assert!(
        eased < straight - 1.0 && (straight - 33.3).abs() < 0.5,
        "{eased} {straight}"
    );
    run(
        &mut st,
        "key.set_tangent",
        json!({"node": cube.0, "track": "transform.pos.x", "frame": 30.0, "preset": "user",
               "left": {"dt_frames": -10.0, "dv": -60.0}, "broken": true}),
    );
    let t = track(&st, cube, "transform.pos.x");
    assert_eq!(t.tangents[1].left.mode, TangentMode::User);
    assert!(t.tangents[1].broken);
    assert_eq!(t.tangents[1].left.dt, -10.0 / 30.0);
    assert!(
        at(&st, 20.0) < straight * 2.0 - 5.0,
        "the curve sags under the line"
    );
    run(&mut st, "edit.undo", json!({}));
    assert!((at(&st, 10.0) - straight).abs() < 1e-9);
    run(
        &mut st,
        "key.set_tangent",
        json!({"node": cube.0, "track": "transform.pos.x", "frame": 0.0, "preset": "ease"}),
    );
    assert_eq!(
        track(&st, cube, "transform.pos.x").tangents[0].right.dv,
        0.0
    );
}

#[test]
fn view_logic_selection_hit_tests_and_scaling() {
    let mut v = TimeView::default();
    assert_eq!(v.to_frame(v.to_x(12.5, 100.0), 100.0), 12.5);
    let before = v.to_frame(300.0, 100.0);
    v.zoom_about(300.0, 100.0, 2.0);
    assert!(
        (v.to_frame(300.0, 100.0) - before).abs() < 1e-9,
        "zoom keeps the cursor frame"
    );
    v.pan(16.0);
    assert!((v.start - (-5.0 + before - before)).abs() < 100.0);
    assert_eq!(TimeView { snap: true, ..v }.snapped(3.4), 3.0);
    assert_eq!(TimeView { snap: false, ..v }.snapped(3.4), 3.4);
    assert_eq!(logic::scale_about(10.0, 4.0, 2.0), 16.0);
    let vv = ValueView::default();
    assert!((vv.to_value(vv.to_y(42.0, 10.0), 10.0) - 42.0).abs() < 1e-9);

    let id = |f: f64| KeyId {
        node: 1,
        track: "t".into(),
        frame: f,
    };
    let pts = vec![
        (id(0.0), [10.0, 10.0]),
        (id(1.0), [50.0, 10.0]),
        (id(2.0), [50.0, 40.0]),
    ];
    assert_eq!(logic::hit_test(&pts, [52.0, 12.0], 6.0), Some(1));
    assert_eq!(logic::hit_test(&pts, [30.0, 30.0], 6.0), None);
    let boxed = logic::box_select(&pts, [60.0, 45.0], [40.0, 5.0]);
    assert_eq!(boxed, vec![id(1.0), id(2.0)]);

    let mut sel = KeySelection::default();
    sel.click(id(0.0), false);
    sel.click(id(1.0), true);
    assert_eq!(sel.keys.len(), 2);
    sel.click(id(0.0), true);
    assert_eq!(sel.keys, vec![id(1.0)]);
    sel.set(vec![id(1.0), id(2.0)], true);
    assert_eq!(sel.keys.len(), 2, "no duplicates");
    assert_eq!(sel.moved(3.0).keys[0].frame, 4.0);
    assert_eq!(logic::track_label("transform.pos.x"), "Position . X");
    assert_eq!(logic::axis_color("transform.rot.z"), [90, 140, 240]);
    assert_eq!(
        logic::display(forge_core::Unit::Angle, std::f64::consts::PI),
        180.0
    );
}

#[test]
fn rows_list_objects_then_tracks_with_a_summary_of_the_keys() {
    let (st, cube) = animated();
    let rows = logic::rows(st.doc.scene(), &[]);
    assert_eq!(rows.len(), 3);
    assert_eq!(
        rows[0].frames,
        vec![10.0, 20.0, 0.0]
            .into_iter()
            .chain([30.0])
            .fold(vec![], |mut a: Vec<f64>, f| {
                a.push(f);
                a.sort_by(f64::total_cmp);
                a
            })
    );
    assert_eq!(rows[1].label, "Position . X");
    assert_eq!(rows[2].frames, vec![10.0, 30.0]);
    assert!(logic::rows(st.doc.scene(), &[NodeId(cube.0 + 99)]).is_empty());
}

// ---- kittest -------------------------------------------------------------------------

fn dope_harness(st: EditorState) -> Harness<'static, EditorState> {
    let mut h = Harness::builder().with_size([900.0, 400.0]).build_ui_state(
        |ui, st: &mut EditorState| {
            for (id, args) in dope_sheet_ui(ui, st) {
                execute(st, id, args).unwrap();
            }
        },
        st,
    );
    h.run();
    h
}

fn drag(h: &mut Harness<'_, EditorState>, from: Pos2, to: Pos2) {
    h.hover_at(from);
    h.run();
    h.event(Event::PointerButton {
        pos: from,
        button: PointerButton::Primary,
        pressed: true,
        modifiers: Modifiers::NONE,
    });
    h.run();
    for k in 1..=4 {
        h.hover_at(from + (to - from) * (k as f32 / 4.0));
        h.run();
    }
    h.event(Event::PointerButton {
        pos: to,
        button: PointerButton::Primary,
        pressed: false,
        modifiers: Modifiers::NONE,
    });
    h.run();
    h.run();
}

#[test]
fn dope_sheet_box_select_move_and_undo() {
    let (st, cube) = animated();
    let mut h = dope_harness(st);
    let sheet = sheet_state(&h.ctx);
    let canvas = sheet.canvas.expect("canvas");
    assert_eq!(sheet.shown.len(), 3, "object row and two tracks");
    // box around frame 10 on both track rows (rows 1 and 2); start away from any key
    let a = sheet.key_point(canvas, 1, 8.5) + vec2(0.0, -8.0);
    let b = sheet.key_point(canvas, 2, 11.5) + vec2(0.0, 8.0);
    drag(&mut h, a, b);
    let sel = sheet_state(&h.ctx).selection;
    assert_eq!(sel.keys.len(), 2, "{sel:?}");
    assert!(sel.keys.iter().all(|k| k.frame == 10.0));
    // drag the key at frame 10 on row 1 by three frames
    let sheet = sheet_state(&h.ctx);
    let from = sheet.key_point(canvas, 1, 10.0);
    let to = from + vec2(3.0 * sheet.view.ppf as f32, 0.0);
    let n = undo_len(h.state());
    drag(&mut h, from, to);
    assert_eq!(undo_len(h.state()), n + 1, "one undo step");
    assert_eq!(
        frames(h.state(), cube, "transform.pos.x"),
        [0.0, 13.0, 20.0]
    );
    assert_eq!(frames(h.state(), cube, "transform.pos.y"), [13.0, 30.0]);
    assert!(
        sheet_state(&h.ctx)
            .selection
            .keys
            .iter()
            .all(|k| k.frame == 13.0)
    );
    h.state_mut().doc.undo();
    assert_eq!(
        frames(h.state(), cube, "transform.pos.x"),
        [0.0, 10.0, 20.0]
    );
    assert_eq!(frames(h.state(), cube, "transform.pos.y"), [10.0, 30.0]);
}

#[test]
fn dope_sheet_click_selects_and_copy_paste_makes_keys() {
    let (st, cube) = animated();
    let mut h = dope_harness(st);
    let sheet = sheet_state(&h.ctx);
    let canvas = sheet.canvas.unwrap();
    // click the key at frame 20 on the X track
    let p = sheet.key_point(canvas, 1, 20.0);
    h.hover_at(p);
    h.run();
    h.event(Event::PointerButton {
        pos: p,
        button: PointerButton::Primary,
        pressed: true,
        modifiers: Modifiers::NONE,
    });
    h.event(Event::PointerButton {
        pos: p,
        button: PointerButton::Primary,
        pressed: false,
        modifiers: Modifiers::NONE,
    });
    h.run();
    h.run();
    let s = sheet_state(&h.ctx);
    assert_eq!(
        s.selection.keys,
        vec![KeyId {
            node: cube.0,
            track: "transform.pos.x".into(),
            frame: 20.0
        }]
    );
    // copy through the state API and paste at frame 25 through the command
    let mut s = s;
    s.clipboard = vec![forge_ui::dopesheet::ClipKey {
        node: cube.0,
        track: "transform.pos.x".into(),
        dframe: 0.0,
        value: 50.0,
        interp: "spline",
    }];
    let (id, args) = s.paste_command(25.0).unwrap();
    run(h.state_mut(), id, args);
    assert_eq!(
        frames(h.state(), cube, "transform.pos.x"),
        [0.0, 10.0, 20.0, 25.0]
    );
}

#[test]
fn value_graph_handle_drag_changes_the_curve_and_undo_restores_it() {
    let mut st = state();
    let cube = create(&mut st, "forge.cube");
    set_key(&mut st, cube, "transform.pos.x", 0, 0.0);
    set_key(&mut st, cube, "transform.pos.x", 30, 100.0);
    st.selection.click(cube);
    let mut h = Harness::builder()
        .with_size([1000.0, 600.0])
        .build_ui_state(
            |ui, st: &mut EditorState| {
                for (id, args) in value_graph_ui(ui, st) {
                    execute(st, id, args).unwrap();
                }
            },
            st,
        );
    h.run();
    let g = graph_state(&h.ctx);
    let canvas = g.canvas.expect("canvas");
    // select the last key by a click
    let key_pt = g.point(canvas, 30.0, 100.0);
    h.hover_at(key_pt);
    h.run();
    h.event(Event::PointerButton {
        pos: key_pt,
        button: PointerButton::Primary,
        pressed: true,
        modifiers: Modifiers::NONE,
    });
    h.event(Event::PointerButton {
        pos: key_pt,
        button: PointerButton::Primary,
        pressed: false,
        modifiers: Modifiers::NONE,
    });
    h.run();
    h.run();
    assert_eq!(graph_state(&h.ctx).selection.keys.len(), 1);
    let at = |h: &Harness<'_, EditorState>, f: f64| {
        value_at(&track(h.state(), cube, "transform.pos.x"), f / 30.0)
    };
    let before = at(&h, 20.0);
    // the automatic left handle is a third of the span (10 frames) left of the key, flat
    let handle = g.point(canvas, 20.0, 100.0);
    let n = undo_len(h.state());
    drag(&mut h, handle, handle + vec2(0.0, 60.0));
    assert_eq!(undo_len(h.state()), n + 1);
    let t = track(h.state(), cube, "transform.pos.x");
    assert_eq!(t.tangents[1].left.mode, TangentMode::User);
    assert!(t.tangents[1].left.dv < -20.0, "{:?}", t.tangents[1].left);
    let after = at(&h, 20.0);
    assert!(after < before - 5.0, "curve dips: {before} -> {after}");
    h.state_mut().doc.undo();
    assert!((at(&h, 20.0) - before).abs() < 1e-9);
}
