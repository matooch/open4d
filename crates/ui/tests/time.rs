//! docs/specs/ANIM.md §2.6a, §2.9 (brief 094, own design, parity-pending G5): the timeline.*
//! and marker.* commands, headless, and the strip with the preview range and the markers.

mod common;

use common::{create, state};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use forge_command::Playback;
use forge_core::{Interp, Key, PlayMode};
use forge_ui::app::Editor;
use forge_ui::commands::execute;
use forge_ui::state::EditorState;
use serde_json::{Value, json};

fn run(st: &mut EditorState, id: &str, args: Value) -> Value {
    execute(st, id, args).unwrap_or_else(|e| panic!("{id}: {e}"))
}

fn markers(st: &mut EditorState) -> Vec<(u64, i64, String, String)> {
    run(st, "marker.list", json!({}))["markers"]
        .as_array()
        .unwrap()
        .iter()
        .map(|m| {
            (
                m["id"].as_u64().unwrap(),
                m["frame"].as_i64().unwrap(),
                m["name"].as_str().unwrap().to_string(),
                m["color"].as_str().unwrap().to_string(),
            )
        })
        .collect()
}

fn undo_len(st: &EditorState) -> usize {
    st.doc.history().undo_labels().len()
}

#[test]
fn settings_modes_and_display_are_commands_with_validation() {
    let mut st = state();
    let info = run(&mut st, "timeline.info", json!({}));
    assert_eq!(info["mode"], "loop");
    assert_eq!(info["drop_frames"], true);
    assert_eq!(info["display"], "frames");
    assert_eq!(info["preview"], Value::Null);
    assert_eq!(
        (info["fps"].as_i64(), info["end"].as_i64()),
        (Some(30), Some(90))
    );

    run(&mut st, "timeline.set_mode", json!({"mode": "ping_pong"}));
    run(
        &mut st,
        "timeline.set_frame_dropping",
        json!({"enabled": false}),
    );
    run(&mut st, "timeline.set_display", json!({"format": "smpte"}));
    run(&mut st, "timeline.set_frame", json!({"frame": 100}));
    let info = run(&mut st, "timeline.info", json!({}));
    assert_eq!(info["mode"], "ping_pong");
    assert_eq!(info["drop_frames"], false);
    assert_eq!(info["text"], "00:00:03:00", "frame 90 (clamped) at 30 fps");
    assert!(execute(&mut st, "timeline.set_mode", json!({"mode": "bounce"})).is_err());
    assert!(execute(&mut st, "timeline.set_display", json!({"format": "hex"})).is_err());

    // each setting is one undo step; setting the same value records nothing
    let n = undo_len(&st);
    run(&mut st, "timeline.set_display", json!({"format": "smpte"}));
    assert_eq!(undo_len(&st), n);
    run(
        &mut st,
        "timeline.set_display",
        json!({"format": "seconds"}),
    );
    assert_eq!(run(&mut st, "timeline.info", json!({}))["text"], "3.000 s");
    run(&mut st, "edit.undo", json!({}));
    assert_eq!(run(&mut st, "timeline.info", json!({}))["display"], "smpte");
}

#[test]
fn preview_range_is_validated_clipped_and_undoable() {
    let mut st = state();
    run(
        &mut st,
        "timeline.set_preview",
        json!({"start": 10, "end": 40}),
    );
    assert_eq!(
        run(&mut st, "timeline.info", json!({}))["preview"],
        json!([10, 40])
    );
    assert!(
        execute(
            &mut st,
            "timeline.set_preview",
            json!({"start": 40, "end": 10})
        )
        .is_err()
    );
    assert!(
        execute(
            &mut st,
            "timeline.set_preview",
            json!({"start": -1, "end": 10})
        )
        .is_err()
    );
    assert!(
        execute(
            &mut st,
            "timeline.set_preview",
            json!({"start": 0, "end": 91})
        )
        .is_err()
    );
    // the project range shrinks: the preview follows
    run(
        &mut st,
        "timeline.settings",
        json!({"fps": 30, "start": 0, "end": 30}),
    );
    assert_eq!(
        run(&mut st, "timeline.info", json!({}))["preview"],
        json!([10, 30])
    );
    run(&mut st, "timeline.clear_preview", json!({}));
    assert_eq!(
        run(&mut st, "timeline.info", json!({}))["preview"],
        Value::Null
    );
    run(&mut st, "edit.undo", json!({}));
    assert_eq!(
        run(&mut st, "timeline.info", json!({}))["preview"],
        json!([10, 30])
    );
}

#[test]
fn markers_add_rename_move_colour_remove_and_jump() {
    let mut st = state();
    let a = run(&mut st, "marker.add", json!({"frame": 20, "name": "Hit"}));
    assert_eq!((a["id"].as_u64(), a["frame"].as_i64()), (Some(1), Some(20)));
    run(&mut st, "timeline.set_frame", json!({"frame": 5}));
    let b = run(&mut st, "marker.add", json!({"color": "#38C6E8"}));
    assert_eq!(b["frame"], 5, "default: the current frame");
    run(&mut st, "marker.add", json!({"frame": 60, "name": ""}));
    assert_eq!(
        markers(&mut st),
        [
            (2, 5, "Marker 2".into(), "#38c6e8".into()),
            (1, 20, "Hit".into(), "#e0a030".into()),
            (3, 60, "".into(), "#e0a030".into()),
        ]
    );
    assert!(execute(&mut st, "marker.add", json!({"frame": 91})).is_err());
    assert!(execute(&mut st, "marker.add", json!({"color": "red"})).is_err());
    assert!(execute(&mut st, "marker.rename", json!({"id": 9, "name": "x"})).is_err());

    run(&mut st, "marker.rename", json!({"id": 1, "name": "Land"}));
    run(
        &mut st,
        "marker.set_color",
        json!({"id": 1, "color": "#ff0000"}),
    );
    run(&mut st, "marker.move", json!({"id": 3, "frame": 30}));
    assert_eq!(
        markers(&mut st)[1],
        (1, 20, "Land".into(), "#ff0000".into())
    );
    assert_eq!(markers(&mut st)[2].1, 30, "kept sorted by frame");
    assert!(execute(&mut st, "marker.move", json!({"id": 3, "frame": -2})).is_err());

    // jump: next / previous from the current frame 5
    let at = |st: &mut EditorState, forward: bool| {
        let r = run(st, "marker.jump", json!({"forward": forward}));
        (r["frame"].as_i64().unwrap(), r["found"].as_bool().unwrap())
    };
    assert_eq!(at(&mut st, false), (5, false), "nothing before");
    assert_eq!(at(&mut st, true), (20, true));
    assert_eq!(at(&mut st, true), (30, true));
    assert_eq!(at(&mut st, true), (30, false));
    assert_eq!(at(&mut st, false), (20, true));

    run(&mut st, "marker.remove", json!({"id": 1}));
    assert_eq!(markers(&mut st).len(), 2);
    run(&mut st, "edit.undo", json!({}));
    assert_eq!(markers(&mut st).len(), 3, "remove is undoable");
    // ids are not reused while a marker keeps the maximum
    run(&mut st, "marker.remove", json!({"id": 2}));
    assert_eq!(run(&mut st, "marker.add", json!({"frame": 1}))["id"], 4);
}

#[test]
fn dragging_a_marker_is_one_undo_step() {
    let mut st = state();
    run(&mut st, "marker.add", json!({"frame": 10}));
    let n = undo_len(&st);
    for f in [11, 12, 13, 14] {
        run(&mut st, "marker.move", json!({"id": 1, "frame": f}));
    }
    assert_eq!(undo_len(&st), n + 1, "steps of one drag merge");
    run(&mut st, "edit.undo", json!({}));
    assert_eq!(markers(&mut st)[0].1, 10);
}

#[test]
fn set_fps_converts_ranges_and_markers_but_keys_keep_their_seconds() {
    let mut st = state();
    let id = create(&mut st, "test.box");
    // keys at 0.5 s (frame 15 at 30 fps) and 1/3 s (frame 10)
    st.doc
        .transact("keys", |tx| {
            tx.set_key(id, "size", Key::new(0.5, 1.0))?;
            let mut k = Key::new(1.0 / 3.0, 2.0);
            k.interp = Interp::Linear;
            tx.set_key(id, "size", k)
        })
        .unwrap();
    let keys_before = st.doc.scene().node(id).unwrap().tracks["size"].keys.clone();
    run(
        &mut st,
        "timeline.settings",
        json!({"fps": 30, "start": 0, "end": 90}),
    );
    run(
        &mut st,
        "timeline.set_preview",
        json!({"start": 30, "end": 60}),
    );
    run(&mut st, "marker.add", json!({"frame": 10, "name": "a"}));
    run(&mut st, "marker.add", json!({"frame": 90, "name": "b"}));
    run(&mut st, "timeline.set_frame", json!({"frame": 45}));
    let n = undo_len(&st);

    let r = run(&mut st, "timeline.set_fps", json!({"fps": 24}));
    assert_eq!(
        r,
        json!({"fps": 24, "start": 0, "end": 72, "frame": 36}),
        "90 frames = 3 s = 72 frames"
    );
    assert_eq!(undo_len(&st), n + 1, "one undo step");
    let info = run(&mut st, "timeline.info", json!({}));
    assert_eq!(info["preview"], json!([24, 48]));
    let m: Vec<i64> = markers(&mut st).iter().map(|m| m.1).collect();
    assert_eq!(m, [8, 72], "frame 10 = 0.333 s = frame 8");
    // keys did not move: same seconds, hence frame 12 and 8 at the new rate
    let keys_after = st.doc.scene().node(id).unwrap().tracks["size"].keys.clone();
    assert_eq!(keys_after, keys_before);
    assert!((keys_after[1].time * 24.0 - 12.0).abs() < 1e-9);
    assert!((keys_after[0].time * 24.0 - 8.0).abs() < 1e-9);
    // key.jump now lands on the converted frames
    run(&mut st, "timeline.set_frame", json!({"frame": 0}));
    assert_eq!(
        run(&mut st, "key.jump", json!({"forward": true}))["frame"],
        8
    );

    assert!(execute(&mut st, "timeline.set_fps", json!({"fps": 0})).is_err());
    assert!(execute(&mut st, "timeline.set_fps", json!({"fps": 241})).is_err());
    run(&mut st, "edit.undo", json!({}));
    let info = run(&mut st, "timeline.info", json!({}));
    assert_eq!(
        (info["fps"].as_i64(), info["end"].as_i64()),
        (Some(30), Some(90))
    );
    assert_eq!(info["preview"], json!([30, 60]));
    assert_eq!(markers(&mut st)[0].1, 10);
}

#[test]
fn the_play_commands_use_the_preview_range_and_the_mode() {
    let mut st = state();
    run(
        &mut st,
        "timeline.set_preview",
        json!({"start": 10, "end": 12}),
    );
    run(&mut st, "timeline.set_mode", json!({"mode": "once"}));
    run(&mut st, "timeline.set_frame", json!({"frame": 12}));
    run(&mut st, "timeline.play", json!({}));
    assert_eq!(
        st.timeline.current, 10,
        "Once at the end restarts at the start"
    );
    assert!(st.timeline.playing);
    let scene = st.doc.scene();
    let p = Playback::of(&scene.settings(), scene.time());
    assert_eq!((p.lo, p.hi, p.mode), (10, 12, PlayMode::Once));
    st.timeline.update_with(0.0, &p);
    st.timeline.update_with(1.0, &p);
    assert_eq!((st.timeline.current, st.timeline.playing), (12, false));
    run(&mut st, "timeline.toggle_play", json!({}));
    assert!(st.timeline.playing);
    assert_eq!(st.timeline.current, 10);
}

// --- the strip --------------------------------------------------------------------------

fn harness(st: EditorState) -> Harness<'static, Editor> {
    Harness::builder()
        .with_size([1280.0, 800.0])
        .build_ui_state(
            |ui, editor: &mut Editor| editor.show(ui, None),
            Editor::new(st),
        )
}

fn press(h: &mut Harness<'_, Editor>, pos: egui::Pos2, pressed: bool) {
    h.event(egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    });
    h.run();
}

/// Drags from `from` to `to` in small steps.
fn drag(h: &mut Harness<'_, Editor>, from: egui::Pos2, to: egui::Pos2) {
    h.hover_at(from);
    h.run();
    press(h, from, true);
    for k in 1..=6 {
        h.hover_at(from + (to - from) * (k as f32 / 6.0));
        h.run();
    }
    press(h, to, false);
}

fn editor<'a>(h: &'a Harness<'_, Editor>) -> &'a EditorState {
    &h.state().state
}

#[test]
fn the_preview_checkbox_and_handles_edit_the_preview_range() {
    let mut h = harness(state());
    h.run();
    assert!(h.query_by_label("Preview start handle").is_none());
    h.get_by_label("Preview range").click();
    h.run();
    assert_eq!(editor(&h).doc.scene().time().preview, Some((0, 90)));
    h.get_by_label("Preview start handle");
    h.get_by_label("Preview end handle");
    h.get_by_label("Preview start");
    h.get_by_label("Preview end");

    // drag the end handle left by about a third of the strip
    let strip = h.get_by_label("Frame strip").rect();
    let end = h.get_by_label("Preview end handle").rect().center();
    let target = egui::pos2(strip.left() + 8.0 + (strip.width() - 16.0) / 3.0, end.y);
    drag(&mut h, end, target);
    let (a, b) = editor(&h).doc.scene().time().preview.unwrap();
    assert_eq!(a, 0);
    assert!((28..=32).contains(&b), "dragged to about frame 30, got {b}");
    // one undo step for the whole drag, plus the checkbox
    h.get_by_label("Preview range").click();
    h.run();
    assert_eq!(editor(&h).doc.scene().time().preview, None);
}

#[test]
fn the_preview_start_handle_cannot_pass_the_end() {
    let mut st = state();
    run(
        &mut st,
        "timeline.set_preview",
        json!({"start": 10, "end": 30}),
    );
    let mut h = harness(st);
    h.run();
    let strip = h.get_by_label("Frame strip").rect();
    let start = h.get_by_label("Preview start handle").rect().center();
    drag(&mut h, start, egui::pos2(strip.right() - 8.0, start.y));
    let (a, b) = editor(&h).doc.scene().time().preview.unwrap();
    assert_eq!((a, b), (30, 30), "clamped at the end handle");
}

#[test]
fn markers_are_added_clicked_dragged_and_deleted_in_the_strip() {
    let mut st = state();
    run(&mut st, "timeline.set_frame", json!({"frame": 20}));
    let mut h = harness(st);
    h.run();
    h.get_by_label("Add Marker").click();
    h.run();
    assert_eq!(editor(&h).doc.scene().time().markers.len(), 1);
    assert_eq!(editor(&h).doc.scene().time().markers[0].frame, 20);
    h.get_by_label("Marker Marker 1");

    // clicking a marker goes to its frame
    h.state_mut().state.timeline.current = 70;
    h.run();
    h.get_by_label("Marker Marker 1").click();
    h.run();
    assert_eq!(editor(&h).state_current(), 20);

    // dragging it moves it
    let strip = h.get_by_label("Frame strip").rect();
    let from = h.get_by_label("Marker Marker 1").rect().center();
    let to = egui::pos2(strip.left() + 8.0 + (strip.width() - 16.0) * 0.5, from.y);
    drag(&mut h, from, to);
    let f = editor(&h).doc.scene().time().markers[0].frame;
    assert!((43..=47).contains(&f), "moved to about frame 45, got {f}");

    // previous / next marker buttons
    run(
        &mut h.state_mut().state,
        "marker.add",
        json!({"frame": 80, "name": "End"}),
    );
    h.state_mut().state.timeline.current = 0;
    h.run();
    h.get_by_label("Next Marker").click();
    h.run();
    assert_eq!(editor(&h).state_current(), f);
    h.get_by_label("Next Marker").click();
    h.run();
    assert_eq!(editor(&h).state_current(), 80);
    h.get_by_label("Previous Marker").click();
    h.run();
    assert_eq!(editor(&h).state_current(), f);

    // the context menu deletes
    h.get_by_label("Marker End").click_secondary();
    h.run();
    h.get_by_label("Delete Marker").click();
    h.run();
    assert_eq!(editor(&h).doc.scene().time().markers.len(), 1);
    assert!(h.query_by_label("Marker End").is_none());
}

#[test]
fn marker_keys_add_and_jump() {
    let mut st = state();
    run(&mut st, "marker.add", json!({"frame": 50}));
    let mut h = harness(st);
    h.run();
    h.state_mut().state.timeline.current = 7;
    h.run();
    h.key_press(egui::Key::M);
    h.run();
    let frames: Vec<i64> = editor(&h)
        .doc
        .scene()
        .time()
        .markers
        .iter()
        .map(|m| m.frame)
        .collect();
    assert_eq!(frames, [7, 50]);
    h.key_press_modifiers(egui::Modifiers::ALT, egui::Key::Period);
    h.run();
    assert_eq!(editor(&h).state_current(), 50);
    h.key_press_modifiers(egui::Modifiers::ALT, egui::Key::Comma);
    h.run();
    assert_eq!(editor(&h).state_current(), 7);
}

trait Current {
    fn state_current(&self) -> i64;
}

impl Current for EditorState {
    fn state_current(&self) -> i64 {
        self.timeline.current
    }
}

#[test]
fn the_frame_label_follows_the_display_format() {
    let mut st = state();
    run(&mut st, "timeline.set_frame", json!({"frame": 45}));
    let mut h = harness(st);
    h.run();
    h.get_by_label("1.500 s");
    run(
        &mut h.state_mut().state,
        "timeline.set_display",
        json!({"format": "smpte"}),
    );
    h.run();
    h.get_by_label("00:00:01:15");
}
