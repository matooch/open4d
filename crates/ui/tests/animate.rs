//! docs/specs/ANIM.md §2: timeline, record, autokey, rule 5, navigation, playback.

mod common;

use common::{create, state};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use forge_core::transform::{POS, Transform};
use forge_core::{Interp, Key, NodeId, SceneSettings, Value, file};
use forge_ui::animate::{
    AnimCtx, AnimEdit, KeyState, RECORD_TRACKS, frame_time, key_frames, key_states, next_key,
    prev_key, time_frame,
};
use forge_ui::app::Editor;
use forge_ui::attributes::{Gesture, apply_edit_anim};
use forge_ui::commands::{Command, execute};
use forge_ui::coordinates::{Row, Space, set_coord_anim};
use forge_ui::manipulate::apply_drag_anim;
use forge_ui::state::EditorState;
use forge_ui::timeline::Timeline;
use forge_ui::viewport_panel::scene_meshes;
use forge_viewport::gizmo::DragDelta;
use glam::DVec3;

fn ctx(frame: i64, autokey: bool) -> AnimCtx {
    AnimCtx {
        frame,
        fps: 30,
        autokey,
    }
}

fn tracks(st: &EditorState, id: NodeId) -> Vec<String> {
    st.doc
        .scene()
        .node(id)
        .unwrap()
        .tracks
        .keys()
        .cloned()
        .collect()
}

fn keys(st: &EditorState, id: NodeId, track: &str) -> Vec<(f64, f64, Interp)> {
    st.doc.scene().node(id).unwrap().tracks[track]
        .keys
        .iter()
        .map(|k| (time_frame(k.time, 30), k.value, k.interp))
        .collect()
}

fn undo_len(st: &EditorState) -> usize {
    st.doc.history().undo_labels().len()
}

fn go(st: &mut EditorState, frame: i64) {
    execute(st, Command::SetFrame(frame)).unwrap();
}

fn edit(st: &mut EditorState, id: NodeId, param: &str, v: Value, autokey: bool, g: Gesture) {
    let c = ctx(st.timeline.current, autokey);
    apply_edit_anim(&mut st.doc, id, param, v, g, Some(&c)).unwrap();
}

fn stored(st: &EditorState, id: NodeId, param: &str) -> Value {
    st.doc.scene().param(id, param).unwrap().clone()
}

fn shown(st: &EditorState, id: NodeId, param: &str) -> Value {
    st.evaluated().param(id, param).unwrap().clone()
}

#[test]
fn frame_time_round_trip() {
    assert_eq!(frame_time(15, 30), 0.5);
    assert_eq!(frame_time(12, 24), 0.5);
    assert_eq!(time_frame(0.5, 30), 15.0);
}

#[test]
fn record_makes_nine_tracks_per_object_in_one_undo_step() {
    let mut st = state();
    let a = create(&mut st, "forge.cube");
    let b = create(&mut st, "forge.null");
    st.selection.click(a);
    st.selection.toggle(b);
    go(&mut st, 15);
    let before = undo_len(&st);
    execute(&mut st, Command::RecordKeys).unwrap();
    assert_eq!(undo_len(&st), before + 1, "one undo step");
    assert_eq!(st.doc.history().undo_labels().last(), Some(&"Record"));
    for id in [a, b] {
        let mut got = tracks(&st, id);
        got.sort();
        let mut want: Vec<String> = RECORD_TRACKS.iter().map(|s| s.to_string()).collect();
        want.sort();
        assert_eq!(got, want);
        for t in RECORD_TRACKS {
            let k = st.doc.scene().node(id).unwrap().tracks[t].keys.clone();
            assert_eq!(k.len(), 1);
            assert_eq!(k[0].time, 0.5);
            assert_eq!(k[0].interp, Interp::Spline);
        }
    }
    // recorded values are the object's current ones: scale 1, position 0
    assert_eq!(keys(&st, a, "transform.scale.y")[0].1, 1.0);
    execute(&mut st, Command::Undo).unwrap();
    assert!(tracks(&st, a).is_empty() && tracks(&st, b).is_empty());
    execute(&mut st, Command::Redo).unwrap();
    assert_eq!(tracks(&st, a).len(), 9);
}

#[test]
fn record_writes_the_evaluated_value_and_keeps_an_existing_keys_interpolation() {
    let mut st = state();
    let id = create(&mut st, "forge.null");
    go(&mut st, 0);
    edit(
        &mut st,
        id,
        POS,
        Value::Vec3([0.0, 0.0, 0.0]),
        true,
        Gesture::Commit,
    );
    go(&mut st, 30);
    edit(
        &mut st,
        id,
        POS,
        Value::Vec3([100.0, 0.0, 0.0]),
        true,
        Gesture::Commit,
    );
    // make the first key Linear
    st.doc
        .transact("lin", |tx| {
            tx.set_key(
                id,
                "transform.pos.x",
                Key::new(0.0, 0.0).with_interp(Interp::Linear),
            )
        })
        .unwrap();
    go(&mut st, 15);
    execute(&mut st, Command::RecordKeys).unwrap();
    let k = keys(&st, id, "transform.pos.x");
    assert_eq!(k.len(), 3);
    assert_eq!(k[1].0, 15.0);
    assert!((k[1].1 - 50.0).abs() < 1e-3, "{k:?}"); // spline midpoint of 0..100
    // recording again on the same frame replaces, keeps Linear when it was there
    go(&mut st, 0);
    execute(&mut st, Command::RecordKeys).unwrap();
    assert_eq!(keys(&st, id, "transform.pos.x")[0].2, Interp::Linear);
    assert_eq!(keys(&st, id, "transform.pos.x").len(), 3);
}

#[test]
fn record_needs_a_recordable_selection() {
    let mut st = state();
    assert!(execute(&mut st, Command::RecordKeys).is_err());
    let b = create(&mut st, "test.box");
    create(&mut st, "test.sel"); // a tag on the box; the box stays the selection
    let tag = st.doc.scene().node(b).unwrap().tags[0];
    st.selection.click(tag);
    assert!(
        execute(&mut st, Command::RecordKeys).is_err(),
        "tags have no transform"
    );
    st.selection.click(b);
    assert!(execute(&mut st, Command::RecordKeys).is_ok());
    assert_eq!(undo_len(&st), 3, "create, tag, record");
}

#[test]
fn autokey_on_keys_floats_and_all_three_vec3_components() {
    let mut st = state();
    let b = create(&mut st, "test.box");
    go(&mut st, 10);
    // Float
    edit(&mut st, b, "size", Value::Float(5.0), true, Gesture::Commit);
    assert_eq!(keys(&st, b, "size"), [(10.0, 5.0, Interp::Spline)]);
    assert_eq!(
        stored(&st, b, "size"),
        Value::Float(2.0),
        "stored value untouched"
    );
    // Vec3: all three components, though only x changed
    edit(
        &mut st,
        b,
        "offset",
        Value::Vec3([1.0, 0.0, 0.0]),
        true,
        Gesture::Commit,
    );
    for (c, v) in [("x", 1.0), ("y", 0.0), ("z", 0.0)] {
        assert_eq!(
            keys(&st, b, &format!("offset.{c}")),
            [(10.0, v, Interp::Spline)]
        );
    }
    // a second edit on the same frame updates the keys, one step each
    let n = undo_len(&st);
    edit(
        &mut st,
        b,
        "offset",
        Value::Vec3([2.0, 3.0, 4.0]),
        true,
        Gesture::Commit,
    );
    assert_eq!(undo_len(&st), n + 1);
    assert_eq!(keys(&st, b, "offset.y"), [(10.0, 3.0, Interp::Spline)]);
    // the displayed value follows the keys at any frame
    go(&mut st, 50);
    assert_eq!(shown(&st, b, "offset"), Value::Vec3([2.0, 3.0, 4.0]));
}

#[test]
fn autokey_off_on_a_plain_parameter_just_sets_the_value() {
    let mut st = state();
    let b = create(&mut st, "test.box");
    go(&mut st, 10);
    edit(
        &mut st,
        b,
        "size",
        Value::Float(5.0),
        false,
        Gesture::Commit,
    );
    edit(
        &mut st,
        b,
        "offset",
        Value::Vec3([1.0, 2.0, 3.0]),
        false,
        Gesture::Commit,
    );
    assert!(tracks(&st, b).is_empty());
    assert_eq!(stored(&st, b, "size"), Value::Float(5.0));
    assert_eq!(stored(&st, b, "offset"), Value::Vec3([1.0, 2.0, 3.0]));
}

#[test]
fn editing_an_animated_parameter_with_autokey_off_updates_or_creates_the_key() {
    let mut st = state();
    let b = create(&mut st, "test.box");
    go(&mut st, 0);
    edit(&mut st, b, "size", Value::Float(1.0), true, Gesture::Commit);
    go(&mut st, 30);
    edit(&mut st, b, "size", Value::Float(3.0), true, Gesture::Commit);
    // rule 5: off, at frame 15 (no key there): creates one, stored value unchanged
    go(&mut st, 15);
    edit(
        &mut st,
        b,
        "size",
        Value::Float(10.0),
        false,
        Gesture::Commit,
    );
    assert_eq!(
        keys(&st, b, "size")
            .iter()
            .map(|k| (k.0, k.1))
            .collect::<Vec<_>>(),
        [(0.0, 1.0), (15.0, 10.0), (30.0, 3.0)]
    );
    assert_eq!(stored(&st, b, "size"), Value::Float(2.0));
    assert_eq!(shown(&st, b, "size"), Value::Float(10.0), "no snap back");
    // and again on the key: updates it, keeping its interpolation
    st.doc
        .transact("lin", |tx| {
            tx.set_key(b, "size", Key::new(0.5, 10.0).with_interp(Interp::Linear))
        })
        .unwrap();
    edit(
        &mut st,
        b,
        "size",
        Value::Float(7.0),
        false,
        Gesture::Commit,
    );
    assert_eq!(keys(&st, b, "size")[1], (15.0, 7.0, Interp::Linear));
    // changing the frame and back keeps the value
    go(&mut st, 20);
    go(&mut st, 15);
    assert_eq!(shown(&st, b, "size"), Value::Float(7.0));
}

#[test]
fn rule_5_on_a_vec3_keys_only_the_changed_animated_components() {
    let mut st = state();
    let b = create(&mut st, "test.box");
    go(&mut st, 0);
    edit(
        &mut st,
        b,
        "offset",
        Value::Vec3([0.0, 0.0, 0.0]),
        true,
        Gesture::Commit,
    );
    // drop the z track: x and y animated, z plain
    st.doc
        .transact("rm", |tx| tx.set_track(b, "offset.z", None))
        .unwrap();
    go(&mut st, 20);
    // change x and z (what the panel would send after dragging X and typing Z)
    edit(
        &mut st,
        b,
        "offset",
        Value::Vec3([4.0, 0.0, 9.0]),
        false,
        Gesture::Commit,
    );
    assert_eq!(keys(&st, b, "offset.x").len(), 2, "x got a key");
    assert_eq!(keys(&st, b, "offset.y").len(), 1, "y unchanged: no key");
    assert!(!tracks(&st, b).contains(&"offset.z".to_string()));
    assert_eq!(
        stored(&st, b, "offset"),
        Value::Vec3([0.0, 0.0, 9.0]),
        "z is plain: stored"
    );
    assert_eq!(shown(&st, b, "offset"), Value::Vec3([4.0, 0.0, 9.0]));
}

#[test]
fn a_drag_with_autokey_is_one_undo_step_and_keys_only_what_moved() {
    let mut st = state();
    let id = create(&mut st, "forge.null");
    go(&mut st, 12);
    let start = st.evaluated().local_transform(id).unwrap();
    let n = undo_len(&st);
    for d in [10.0, 20.0, 35.0] {
        let eval = st.evaluated();
        let anim = AnimEdit {
            ctx: ctx(12, true),
            eval: &eval,
        };
        apply_drag_anim(
            &mut st.doc,
            id,
            &start,
            DragDelta::Translate(DVec3::new(d, 0.0, 0.0)),
            Gesture::Drag(1),
            Some(&anim),
        )
        .unwrap();
    }
    assert_eq!(undo_len(&st), n + 1, "merged into one step");
    let mut got = tracks(&st, id);
    got.sort();
    assert_eq!(
        got,
        ["transform.pos.x", "transform.pos.y", "transform.pos.z"]
    );
    assert_eq!(
        keys(&st, id, "transform.pos.x"),
        [(12.0, 35.0, Interp::Spline)]
    );
    execute(&mut st, Command::Undo).unwrap();
    assert!(tracks(&st, id).is_empty());
}

#[test]
fn an_animated_object_does_not_jump_when_dragged_or_edited_in_coordinates() {
    let mut st = state();
    let id = create(&mut st, "forge.null");
    go(&mut st, 0);
    edit(
        &mut st,
        id,
        POS,
        Value::Vec3([0.0; 3]),
        true,
        Gesture::Commit,
    );
    go(&mut st, 30);
    edit(
        &mut st,
        id,
        POS,
        Value::Vec3([100.0, 0.0, 0.0]),
        true,
        Gesture::Commit,
    );
    go(&mut st, 15);
    let before = st.evaluated().local_transform(id).unwrap();
    assert!((before.pos.x - 50.0).abs() < 1e-3);
    // a zero-length drag with rule 5 changes nothing and adds no key
    let eval = st.evaluated();
    let anim = AnimEdit {
        ctx: ctx(15, false),
        eval: &eval,
    };
    apply_drag_anim(
        &mut st.doc,
        id,
        &before,
        DragDelta::Translate(DVec3::ZERO),
        Gesture::Commit,
        Some(&anim),
    )
    .unwrap();
    assert_eq!(keys(&st, id, "transform.pos.x").len(), 2);
    // a Coordinates edit of Y at frame 15 keys only Y there
    let eval = st.evaluated();
    let anim = AnimEdit {
        ctx: ctx(15, false),
        eval: &eval,
    };
    let pos = before.pos + DVec3::new(0.0, 7.0, 0.0);
    set_coord_anim(
        &mut st.doc,
        id,
        Space::Local,
        Row::Position,
        pos,
        Gesture::Commit,
        Some(&anim),
    )
    .unwrap();
    assert_eq!(
        keys(&st, id, "transform.pos.y").len(),
        3,
        "y got the frame-15 key"
    );
    assert_eq!(
        keys(&st, id, "transform.pos.x").len(),
        2,
        "x unchanged: no new key"
    );
    assert_eq!(
        keys(&st, id, "transform.pos.z").len(),
        2,
        "z unchanged: no new key"
    );
    let after: Transform = st.evaluated().local_transform(id).unwrap();
    assert!((after.pos.y - 7.0).abs() < 1e-9);
}

#[test]
fn previous_and_next_key_follow_the_selected_objects_only() {
    let mut st = state();
    let a = create(&mut st, "forge.null");
    let b = create(&mut st, "forge.null");
    for (id, frames) in [(a, vec![10, 40]), (b, vec![25, 70])] {
        st.selection.click(id);
        for f in frames {
            go(&mut st, f);
            execute(&mut st, Command::RecordKeys).unwrap();
        }
    }
    st.selection.click(a);
    go(&mut st, 0);
    let mut seen = Vec::new();
    for _ in 0..3 {
        execute(&mut st, Command::JumpKey(true)).unwrap();
        seen.push(st.timeline.current);
    }
    assert_eq!(seen, [10, 40, 40], "no key after 40: stays");
    execute(&mut st, Command::JumpKey(false)).unwrap();
    assert_eq!(st.timeline.current, 10);
    execute(&mut st, Command::JumpKey(false)).unwrap();
    assert_eq!(st.timeline.current, 10);
    st.selection.toggle(b);
    go(&mut st, 11);
    execute(&mut st, Command::JumpKey(true)).unwrap();
    assert_eq!(st.timeline.current, 25);
    let frames = key_frames(st.doc.scene(), &[a, b]);
    assert_eq!(frames, [10.0, 25.0, 40.0, 70.0]);
    assert_eq!(prev_key(&frames, 25), Some(10));
    assert_eq!(next_key(&frames, 25), Some(40));
    assert_eq!(next_key(&frames, 70), None);
    assert_eq!(prev_key(&frames, 10), None);
    // off-grid keys round away from the current frame
    assert_eq!(next_key(&[10.4], 10), Some(11));
    assert_eq!(prev_key(&[9.6], 10), Some(9));
}

#[test]
fn delete_keys_removes_the_current_frame_in_one_step() {
    let mut st = state();
    let a = create(&mut st, "forge.null");
    for f in [0, 10] {
        go(&mut st, f);
        execute(&mut st, Command::RecordKeys).unwrap();
    }
    let n = undo_len(&st);
    go(&mut st, 10);
    execute(&mut st, Command::DeleteKeys).unwrap();
    assert_eq!(undo_len(&st), n + 1);
    assert_eq!(st.doc.history().undo_labels().last(), Some(&"Delete Keys"));
    for t in RECORD_TRACKS {
        assert_eq!(keys(&st, a, t).len(), 1);
    }
    // deleting the last key of a track removes the track; nothing on the frame: no step
    go(&mut st, 0);
    execute(&mut st, Command::DeleteKeys).unwrap();
    assert!(tracks(&st, a).is_empty());
    let n = undo_len(&st);
    execute(&mut st, Command::DeleteKeys).unwrap();
    assert_eq!(undo_len(&st), n);
    execute(&mut st, Command::Undo).unwrap();
    assert_eq!(tracks(&st, a).len(), 9);
}

#[test]
fn playback_follows_the_clock_drops_frames_and_loops() {
    let s = SceneSettings {
        fps: 30,
        start: 0,
        end: 59,
    };
    let mut t = Timeline::default();
    t.play();
    assert!(!t.update(100.0, &s), "first update only anchors");
    assert_eq!(t.current, 0);
    assert!(t.update(100.0 + 1.0, &s));
    assert_eq!(t.current, 30, "1 s at 30 fps is 30 frames");
    // a long stall drops frames instead of slowing down
    t.update(100.0 + 1.5, &s);
    assert_eq!(t.current, 45);
    t.update(100.0 + 2.0, &s);
    assert_eq!(t.current, 0, "loops: 60 frames in 2 s");
    t.update(100.0 + 2.0 + 1.0 / 30.0, &s);
    assert_eq!(t.current, 1);
    t.update(100.0 + 3.5, &s);
    assert_eq!(t.current, 45);
    // stop keeps the frame and ignores the clock
    t.stop();
    assert!(!t.update(500.0, &s));
    assert_eq!(t.current, 45);
    // resuming continues from the current frame
    t.play();
    t.update(600.0, &s);
    t.update(600.0 + 0.5, &s);
    assert_eq!(t.current, (45 + 15) % 60);
    // setting the frame by hand while playing re-anchors
    t.set_current(10, &s);
    t.update(700.0, &s);
    t.update(700.0 + 1.0 / 30.0, &s);
    assert_eq!(t.current, 11);
}

#[test]
fn playback_uses_the_scene_fps_and_a_default_range_of_91_frames() {
    let s = SceneSettings {
        fps: 24,
        start: 10,
        end: 20,
    };
    let mut t = Timeline::default();
    t.set_current(10, &s);
    t.play();
    t.update(0.0, &s);
    t.update(0.5, &s);
    assert_eq!(t.current, 10 + 12 % 11);
    let d = SceneSettings::default();
    let mut t = Timeline::default();
    t.play();
    t.update(0.0, &d);
    t.update(91.0 / 30.0, &d);
    assert_eq!(t.current, 0, "frames 0..=90 are 91 frames");
}

#[test]
fn the_evaluated_scene_at_frame_15_moves_the_viewport_mesh() {
    let mut st = state();
    let cube = create(&mut st, "forge.cube");
    go(&mut st, 0);
    edit(
        &mut st,
        cube,
        POS,
        Value::Vec3([0.0; 3]),
        true,
        Gesture::Commit,
    );
    go(&mut st, 30);
    edit(
        &mut st,
        cube,
        POS,
        Value::Vec3([100.0, 0.0, 0.0]),
        true,
        Gesture::Commit,
    );
    st.doc
        .transact("linear", |tx| {
            tx.set_key(
                cube,
                "transform.pos.x",
                Key::new(0.0, 0.0).with_interp(Interp::Linear),
            )
        })
        .unwrap();
    let centre_x = |scene: &forge_core::Scene| {
        let m = &scene_meshes(scene, &Default::default())[0].1;
        let (lo, hi) = m.points.iter().fold((f64::MAX, f64::MIN), |(lo, hi), p| {
            (lo.min(p.x), hi.max(p.x))
        });
        (lo + hi) / 2.0
    };
    go(&mut st, 0);
    assert!(centre_x(&st.evaluated()).abs() < 1e-9);
    go(&mut st, 15);
    assert!((centre_x(&st.evaluated()) - 50.0).abs() < 1e-3);
    assert!(
        centre_x(st.doc.scene()).abs() < 1e-9,
        "the stored scene is not animated"
    );
    go(&mut st, 30);
    assert!((centre_x(&st.evaluated()) - 100.0).abs() < 1e-9);
    // cached: the same frame and revision give the same scene
    let a = st.evaluated();
    assert!(std::sync::Arc::ptr_eq(&a, &st.evaluated()));
}

#[test]
fn key_states_mark_keyed_and_animated_rows() {
    let mut st = state();
    let b = create(&mut st, "test.box");
    go(&mut st, 0);
    edit(&mut st, b, "size", Value::Float(1.0), true, Gesture::Commit);
    go(&mut st, 10);
    edit(
        &mut st,
        b,
        "offset",
        Value::Vec3([1.0; 3]),
        true,
        Gesture::Commit,
    );
    let m = key_states(st.doc.scene(), b, 10);
    assert_eq!(m.get("offset"), Some(&KeyState::Keyed));
    assert_eq!(m.get("size"), Some(&KeyState::Animated));
    assert_eq!(m.get("twist"), None);
    go(&mut st, 0);
    let m = key_states(st.doc.scene(), b, 0);
    assert_eq!(m.get("size"), Some(&KeyState::Keyed));
    assert_eq!(m.get("offset"), Some(&KeyState::Animated));
}

#[test]
fn settings_are_an_undoable_command_and_move_the_current_frame_into_range() {
    let mut st = state();
    go(&mut st, 80);
    let s = SceneSettings {
        fps: 24,
        start: 0,
        end: 40,
    };
    execute(&mut st, Command::SetSettings(s)).unwrap();
    assert_eq!(st.settings(), s);
    assert_eq!(st.timeline.current, 40);
    assert!(execute(&mut st, Command::SetSettings(SceneSettings { fps: 0, ..s })).is_err());
    assert_eq!(st.settings(), s);
    execute(&mut st, Command::Undo).unwrap();
    assert_eq!(st.settings(), SceneSettings::default());
    go(&mut st, 500);
    assert_eq!(st.timeline.current, 90);
    execute(&mut st, Command::StepFrame(-1)).unwrap();
    assert_eq!(st.timeline.current, 89);
}

#[test]
fn save_and_load_keep_tracks_fps_and_range() {
    let mut st = state();
    let id = create(&mut st, "forge.cube");
    execute(
        &mut st,
        Command::SetSettings(SceneSettings {
            fps: 25,
            start: -5,
            end: 125,
        }),
    )
    .unwrap();
    go(&mut st, 20);
    execute(&mut st, Command::RecordKeys).unwrap();
    st.doc
        .transact("step", |tx| {
            tx.set_key(
                id,
                "transform.pos.y",
                Key::new(2.0, 9.0).with_interp(Interp::Step),
            )
        })
        .unwrap();
    let text = file::save(st.doc.scene());
    let (loaded, report) = file::load(common::registry(), &text).unwrap();
    assert!(report.warnings.is_empty(), "{:?}", report.warnings);
    assert_eq!(
        loaded.settings(),
        SceneSettings {
            fps: 25,
            start: -5,
            end: 125
        }
    );
    assert_eq!(
        loaded.node(id).unwrap().tracks,
        st.doc.scene().node(id).unwrap().tracks
    );
    assert_eq!(file::save(&loaded), text);
}

#[test]
fn opening_a_file_through_the_editor_resets_the_frame_and_keeps_the_animation() {
    let dir = std::env::temp_dir().join(format!("forge-anim-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("a.forge");
    let mut e = Editor::new(state());
    let id = create(&mut e.state, "forge.null");
    go(&mut e.state, 30);
    execute(&mut e.state, Command::RecordKeys).unwrap();
    assert!(e.save_to(&path));
    let mut other = Editor::new(state());
    go(&mut other.state, 50);
    other.open_path(&path);
    assert_eq!(other.state.timeline.current, 0);
    assert_eq!(tracks(&other.state, id).len(), 9);
    let _ = std::fs::remove_dir_all(&dir);
}

fn animated_cube(st: &mut EditorState) -> NodeId {
    let id = create(st, "forge.cube");
    for (f, x) in [(0, 0.0), (30, 100.0)] {
        go(st, f);
        edit(
            st,
            id,
            POS,
            Value::Vec3([x, 0.0, 0.0]),
            true,
            Gesture::Commit,
        );
    }
    // a generator parameter track, which a polygon object cannot keep
    st.doc
        .transact("size", |tx| tx.set_key(id, "size.x", Key::new(0.0, 100.0)))
        .ok();
    id
}

#[test]
fn make_editable_and_current_state_copy_transform_tracks_only() {
    for command in [Command::MakeEditable, Command::CurrentStateToObject] {
        let mut st = state();
        let src = animated_cube(&mut st);
        let before = st.doc.scene().node(src).unwrap().tracks.clone();
        assert!(before.contains_key("transform.pos.x"));
        st.selection.click(src);
        execute(&mut st, command.clone()).unwrap();
        let new = st.selection.active.unwrap();
        let node = st.doc.scene().node(new).unwrap();
        assert_eq!(node.type_id, "forge.polygon");
        let mut got: Vec<&String> = node.tracks.keys().collect();
        got.sort();
        let want: Vec<&String> = before
            .keys()
            .filter(|k| k.starts_with("transform."))
            .collect();
        assert_eq!(got, want, "{command:?}");
        assert_eq!(node.tracks["transform.pos.x"], before["transform.pos.x"]);
        // still one undo step
        execute(&mut st, Command::Undo).unwrap();
        assert!(st.doc.scene().node(new).is_none());
    }
}

fn harness_with(state: EditorState) -> Harness<'static, Editor> {
    Harness::builder()
        .with_size([1280.0, 800.0])
        .build_ui_state(
            |ui, editor: &mut Editor| editor.show(ui, None),
            Editor::new(state),
        )
}

#[test]
fn clicking_record_shows_the_key_markers_in_the_attributes_panel() {
    let mut st = state();
    let id = create(&mut st, "forge.null");
    st.selection.click(id);
    let mut h = harness_with(st);
    h.run();
    assert!(h.query_by_label("Position keyed").is_none());
    h.get_by_label("Record").click();
    h.run();
    assert_eq!(tracks(&h.state().state, id).len(), 9);
    h.get_by_label("Position keyed");
    h.get_by_label("Rotation keyed");
    h.get_by_label("Scale keyed");
    // on another frame the rows are animated without a key there
    h.state_mut().state.timeline.current = 12;
    h.run();
    h.get_by_label("Position animated");
    assert!(h.query_by_label("Position keyed").is_none());
    // previous key goes back to frame 0
    h.get_by_label("Previous Key").click();
    h.run();
    assert_eq!(h.state().state.timeline.current, 0);
    h.get_by_label("Position keyed");
}

#[test]
fn shortcuts_record_autokey_and_frame_steps() {
    let mut st = state();
    let id = create(&mut st, "forge.null");
    st.selection.click(id);
    let mut h = harness_with(st);
    h.run();
    h.key_press(egui::Key::K);
    h.run();
    assert_eq!(tracks(&h.state().state, id).len(), 9);
    h.key_press(egui::Key::Period);
    h.run();
    assert_eq!(h.state().state.timeline.current, 1);
    h.key_press(egui::Key::Comma);
    h.run();
    assert_eq!(h.state().state.timeline.current, 0);
    h.key_press_modifiers(egui::Modifiers::SHIFT, egui::Key::Period);
    h.run();
    assert_eq!(h.state().state.timeline.current, 0, "no later key");
    h.key_press_modifiers(egui::Modifiers::SHIFT, egui::Key::K);
    h.run();
    assert!(h.state().state.autokey);
    assert_eq!(
        h.state().state.doc.history().undo_labels().last(),
        Some(&"Record"),
        "Shift+K must not record"
    );
}
