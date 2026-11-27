//! Keyframe drafts from every edit route (docs/specs/UI.md §12 "Keyframe drafts", ANIM.md
//! §2.4b; brief 232, owner note N10): with Auto-Key off, an edit of a property that has a
//! key track shows in the viewport and the panels and marks the key button, but writes
//! nothing until the key button is clicked.

mod common;

use common::{create, state};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use forge_core::transform::{POS, Transform};
use forge_core::{NodeId, Value, file};
use forge_ui::app::Editor;
use forge_ui::attributes::Gesture;
use forge_ui::commands::execute;
use forge_ui::manipulate::apply_drag_edit;
use forge_ui::state::EditorState;
use forge_viewport::gizmo::DragDelta;
use glam::DVec3;
use serde_json::json;

fn go(st: &mut EditorState, frame: i64) {
    execute(st, "timeline.set_frame", json!({"frame": frame})).unwrap();
}

/// A cube with Position keys at frames 0 (origin) and 30 (x = 100): at frame 15 it is at
/// x = 50. Frame 15 is the current frame and the cube is the selection.
fn keyed(st: &mut EditorState) -> NodeId {
    let id = create(st, "forge.cube");
    for (frame, x) in [(0, 0.0), (30, 100.0)] {
        for (axis, v) in ["x", "y", "z"].iter().zip([x, 0.0, 0.0]) {
            execute(
                st,
                "key.set",
                json!({"node": id.0, "track": format!("transform.pos.{axis}"), "frame": frame, "value": v}),
            )
            .unwrap();
        }
    }
    go(st, 15);
    id
}

fn tracks(st: &EditorState, id: NodeId) -> String {
    format!("{:?}", st.doc.scene().node(id).unwrap().tracks)
}

fn steps(st: &EditorState) -> usize {
    st.doc.history().undo_labels().len()
}

fn shown_pos(st: &EditorState, id: NodeId) -> DVec3 {
    st.evaluated().local_transform(id).unwrap().pos
}

fn animated_pos(st: &EditorState, id: NodeId) -> DVec3 {
    st.animated().local_transform(id).unwrap().pos
}

fn near(a: DVec3, b: DVec3) {
    // the position tracks are splines: 50.00014 at the middle, not 50
    assert!((a - b).abs().max_element() < 1e-3, "{a} != {b}");
}

/// A gizmo drag of the node by `delta` world units from where it is now, as the editor
/// applies one step.
fn drag(st: &mut EditorState, id: NodeId, delta: DVec3, n: u64) {
    let start = st.evaluated().local_transform(id).unwrap();
    drag_from(st, id, &start, delta, n);
}

/// A step of a drag that began at `start` (the gizmo keeps its start for the whole drag).
fn drag_from(st: &mut EditorState, id: NodeId, start: &Transform, delta: DVec3, n: u64) {
    apply_drag_edit(
        &mut st.app,
        id,
        start,
        DragDelta::Translate(delta),
        Gesture::Drag(n),
    )
    .unwrap();
}

fn harness(state: EditorState) -> Harness<'static, Editor> {
    Harness::builder()
        .with_size([1280.0, 1400.0])
        .build_ui_state(|ui, e: &mut Editor| e.show(ui, None), Editor::new(state))
}

fn st<'a>(h: &'a Harness<'_, Editor>) -> &'a EditorState {
    &h.state().state
}

#[test]
fn a_gizmo_move_of_a_keyed_object_is_a_draft_until_the_key_is_clicked() {
    let mut s = state();
    let id = keyed(&mut s);
    let (before, n) = (tracks(&s, id), steps(&s));
    // two steps of one drag
    let start = s.evaluated().local_transform(id).unwrap();
    drag_from(&mut s, id, &start, DVec3::new(0.0, 20.0, 0.0), 1);
    drag_from(&mut s, id, &start, DVec3::new(0.0, 40.0, 0.0), 1);
    assert_eq!(tracks(&s, id), before, "the animation is not written");
    assert_eq!(steps(&s), n, "a draft is not an undo step");
    near(shown_pos(&s, id), DVec3::new(50.0, 40.0, 0.0));
    near(animated_pos(&s, id), DVec3::new(50.0, 0.0, 0.0));
    assert!(s.app.draft(id, POS).is_some());

    let mut h = harness(s);
    h.run();
    h.get_by_label("Position changed").click();
    h.run();
    let s = st(&h);
    assert_eq!(steps(s), n + 1, "the key click is one undo step");
    assert!(s.app.drafts().is_empty());
    let track = &s.doc.scene().node(id).unwrap().tracks["transform.pos.y"];
    assert_eq!(track.keys.len(), 3, "one new key beside the two");
    assert_eq!(track.keys[1].time, 0.5, "at the current frame");
    assert_eq!(track.keys[1].value, 40.0);
    near(animated_pos(s, id), DVec3::new(50.0, 40.0, 0.0));
    h.get_by_label("Position keyed");
    execute(&mut h.state_mut().state, "edit.undo", json!({})).unwrap();
    h.run();
    assert_eq!(tracks(st(&h), id), before);
    near(shown_pos(st(&h), id), DVec3::new(50.0, 0.0, 0.0));
}

#[test]
fn a_frame_change_undo_and_a_document_edit_discard_the_draft_but_a_selection_change_keeps_it() {
    let mut s = state();
    let id = keyed(&mut s);
    drag(&mut s, id, DVec3::new(0.0, 40.0, 0.0), 1);
    go(&mut s, 20);
    assert!(s.app.draft(id, POS).is_none(), "frame change");
    near(shown_pos(&s, id), animated_pos(&s, id));
    go(&mut s, 15);
    assert!(s.app.draft(id, POS).is_none(), "and it does not come back");

    // undo
    execute(&mut s, "object.create", json!({"type": "forge.null"})).unwrap();
    s.selection.nodes = [id].into_iter().collect();
    s.selection.active = Some(id);
    drag(&mut s, id, DVec3::new(0.0, 40.0, 0.0), 2);
    execute(&mut s, "edit.undo", json!({})).unwrap();
    assert!(s.app.draft(id, POS).is_none(), "undo");

    // a document edit
    drag(&mut s, id, DVec3::new(0.0, 40.0, 0.0), 3);
    execute(
        &mut s,
        "transform.set",
        json!({"node": id.0, "scale": [2.0, 2.0, 2.0]}),
    )
    .unwrap();
    assert!(s.app.draft(id, POS).is_none(), "document edit");

    // a selection change keeps the draft (GUI-SESSIONS.md G4 item 2: the reference kept an
    // unkeyed edit across selecting another object and reselecting this one)
    drag(&mut s, id, DVec3::new(0.0, 40.0, 0.0), 4);
    let drafted = shown_pos(&s, id);
    s.selection.nodes.clear();
    s.selection.active = None;
    assert!(s.app.draft(id, POS).is_some(), "deselect keeps the draft");
    s.selection.nodes = [id].into_iter().collect();
    s.selection.active = Some(id);
    assert!(s.app.draft(id, POS).is_some(), "reselect keeps the draft");
    near(shown_pos(&s, id), drafted);
}

#[test]
fn dragging_back_to_the_animated_value_removes_the_draft() {
    let mut s = state();
    let id = keyed(&mut s);
    drag(&mut s, id, DVec3::new(0.0, 40.0, 0.0), 1);
    assert!(s.app.draft(id, POS).is_some());
    drag(&mut s, id, DVec3::new(0.0, -40.0, 0.0), 2);
    assert!(s.app.draft(id, POS).is_none());
}

#[test]
fn a_property_without_a_track_still_edits_the_document() {
    let mut s = state();
    let _keyed = keyed(&mut s);
    let plain = create(&mut s, "forge.cube");
    let n = steps(&s);
    let start = s.evaluated().local_transform(plain).unwrap();
    drag_from(&mut s, plain, &start, DVec3::new(0.0, 40.0, 0.0), 1);
    drag_from(&mut s, plain, &start, DVec3::new(0.0, 50.0, 0.0), 1);
    assert!(s.app.drafts().is_empty());
    assert_eq!(steps(&s), n + 1, "one merged undo step");
    near(
        s.doc.scene().local_transform(plain).unwrap().pos,
        DVec3::new(0.0, 50.0, 0.0),
    );
}

#[test]
fn auto_key_still_keys_immediately() {
    let mut s = state();
    let id = keyed(&mut s);
    execute(&mut s, "key.autokey", json!({"enabled": true})).unwrap();
    let n = steps(&s);
    drag(&mut s, id, DVec3::new(0.0, 40.0, 0.0), 1);
    assert!(s.app.drafts().is_empty());
    assert_eq!(steps(&s), n + 1);
    let y = &s.doc.scene().node(id).unwrap().tracks["transform.pos.y"];
    assert!(y.keys.iter().any(|k| k.value == 40.0), "keyed at frame 15");
}

#[test]
fn the_coordinates_panel_stages_the_same_draft() {
    let mut s = state();
    let id = keyed(&mut s);
    s.dock.close(forge_ui::dock::Panel::Attributes);
    s.dock.reveal(forge_ui::dock::Panel::Coordinates);
    let (before, n) = (tracks(&s, id), steps(&s));
    let mut h = harness(s);
    h.run();
    h.get_all_by_label("Position")
        .next()
        .unwrap()
        .set_value_accesskit(70.0);
    h.run();
    let s = st(&h);
    assert_eq!(tracks(s, id), before);
    assert_eq!(steps(s), n);
    near(shown_pos(s, id), DVec3::new(70.0, 0.0, 0.0));
    near(animated_pos(s, id), DVec3::new(50.0, 0.0, 0.0));
    assert!(s.app.draft(id, POS).is_some());
}

#[test]
fn a_coordinates_edit_of_a_mixed_selection_drafts_the_keyed_one_and_writes_the_other() {
    let mut s = state();
    let a = keyed(&mut s);
    let b = create(&mut s, "forge.cube");
    s.selection.nodes = [a, b].into_iter().collect();
    s.selection.active = Some(b);
    s.dock.close(forge_ui::dock::Panel::Attributes);
    s.dock.reveal(forge_ui::dock::Panel::Coordinates);
    let (before, n) = (tracks(&s, a), steps(&s));
    let mut h = harness(s);
    h.run();
    h.get_all_by_label("Position")
        .next()
        .unwrap()
        .set_value_accesskit(70.0);
    h.run();
    let s = st(&h);
    assert_eq!(tracks(s, a), before, "the keyed object is not written");
    near(shown_pos(s, a), DVec3::new(70.0, 0.0, 0.0));
    assert!(s.app.draft(a, POS).is_some());
    assert!(s.app.draft(b, POS).is_none());
    near(
        s.doc.scene().local_transform(b).unwrap().pos,
        DVec3::new(70.0, 0.0, 0.0),
    );
    assert_eq!(steps(s), n + 1, "the plain object's edit, one step");
    assert!(
        s.app.draft(a, POS).is_some(),
        "that document edit kept the other draft"
    );
}

#[test]
fn the_inspector_stages_a_row_edit_and_the_key_button_commits_it() {
    let mut s = state();
    let id = keyed(&mut s);
    let (before, n) = (tracks(&s, id), steps(&s));
    let mut h = harness(s);
    h.run();
    h.get_all_by_label("Position")
        .next()
        .unwrap()
        .set_value_accesskit(70.0);
    h.run();
    assert_eq!(tracks(st(&h), id), before);
    near(shown_pos(st(&h), id), DVec3::new(70.0, 0.0, 0.0));
    h.get_by_label("Position changed").click();
    h.run();
    assert_eq!(steps(st(&h)), n + 1);
    let x = &st(&h).doc.scene().node(id).unwrap().tracks["transform.pos.x"];
    assert!(x.keys.iter().any(|k| k.value == 70.0));
}

#[test]
fn record_keys_the_drafts_of_the_selection() {
    let mut s = state();
    let id = keyed(&mut s);
    drag(&mut s, id, DVec3::new(0.0, 40.0, 0.0), 1);
    let n = steps(&s);
    execute(&mut s, "key.record", json!({})).unwrap();
    assert_eq!(steps(&s), n + 1);
    assert!(s.app.drafts().is_empty());
    let y = &s.doc.scene().node(id).unwrap().tracks["transform.pos.y"];
    assert_eq!(y.keys.len(), 3);
    assert_eq!(
        y.keys[1].value, 40.0,
        "the drafted value, not the animated one"
    );
    let x = &s.doc.scene().node(id).unwrap().tracks["transform.pos.x"];
    assert!(x.keys.iter().any(|k| (k.value - 50.0).abs() < 1e-3));
}

#[test]
fn render_export_and_save_ignore_drafts() {
    let mut s = state();
    let id = keyed(&mut s);
    let text_before = file::save(s.doc.scene());
    let mesh_before = s.app.eval_mesh(id, 15).unwrap().points.clone();
    drag(&mut s, id, DVec3::new(0.0, 40.0, 0.0), 1);
    // the viewport shows the draft
    let drawn = s.app.scheduled_draw(15);
    near(
        drawn.scene.local_transform(id).unwrap().pos,
        DVec3::new(50.0, 40.0, 0.0),
    );
    // render and export read the animation only
    assert_eq!(s.app.eval_mesh(id, 15).unwrap().points, mesh_before);
    let scheduled = s.app.scheduled(15);
    near(
        scheduled.scene.local_transform(id).unwrap().pos,
        DVec3::new(50.0, 0.0, 0.0),
    );
    let path = std::env::temp_dir().join("forge_draft_export.obj");
    s.app.export_obj(path.to_str().unwrap(), None).unwrap();
    let obj = std::fs::read_to_string(&path).unwrap();
    let _ = std::fs::remove_file(&path);
    assert!(obj.contains("v "), "exported");
    // the saved file is the document
    assert_eq!(file::save(s.doc.scene()), text_before);
}

#[test]
fn escape_over_the_viewport_discards_the_drafts() {
    let mut s = state();
    let id = keyed(&mut s);
    drag(&mut s, id, DVec3::new(0.0, 40.0, 0.0), 1);
    let mut h = harness(s);
    h.run();
    // the viewport is the large central area
    h.hover_at(egui::pos2(640.0, 450.0));
    h.run();
    h.key_press(egui::Key::Escape);
    h.run();
    assert!(st(&h).app.drafts().is_empty());
    near(shown_pos(st(&h), id), DVec3::new(50.0, 0.0, 0.0));
}

#[test]
fn a_vector_value_in_the_layer_is_one_whole_value() {
    let mut s = state();
    let id = keyed(&mut s);
    s.app
        .set_draft(id, POS, Value::Vec3([1.0, 2.0, 3.0]))
        .unwrap();
    near(shown_pos(&s, id), DVec3::new(1.0, 2.0, 3.0));
    s.app.clear_drafts();
    near(shown_pos(&s, id), DVec3::new(50.0, 0.0, 0.0));
}
