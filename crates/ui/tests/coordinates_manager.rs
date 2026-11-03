//! Coordinates manager (docs/specs/UI.md §20; brief 151): space, scale/size, Manual + Apply,
//! several objects and component rows, all through registry commands.

mod common;

use common::{create, state};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use forge_core::{NodeId, Value};
use forge_ui::app::Editor;
use forge_ui::commands::execute;
use forge_ui::component_edit::component_transform_args;
use forge_ui::coordinates::{CoordDraft, Space, Third, coord_view_multi};
use forge_ui::state::EditorState;
use glam::DVec3;
use serde_json::json;

fn close(a: DVec3, b: DVec3) {
    assert!((a - b).abs().max_element() < 1e-6, "{a} != {b}");
}

fn harness(state: EditorState) -> Harness<'static, Editor> {
    Harness::builder()
        .with_size([1280.0, 1400.0])
        .build_ui_state(|ui, e: &mut Editor| e.show(ui, None), Editor::new(state))
}

/// Builds the editor and shows the Coordinates tab (the bottom area has it behind Timeline).
fn open(mut state: EditorState) -> Harness<'static, Editor> {
    // the attribute manager has Position / Rotation rows of its own: close it
    state.dock.close(forge_ui::dock::Panel::Attributes);
    state.dock.reveal(forge_ui::dock::Panel::Coordinates);
    let mut h = harness(state);
    h.run();
    h
}

fn steps(h: &Harness<'_, Editor>) -> usize {
    h.state().state.doc.history().undo_labels().len()
}

fn pos(h: &Harness<'_, Editor>, id: NodeId) -> DVec3 {
    h.state().state.doc.scene().local_transform(id).unwrap().pos
}

fn scale(h: &Harness<'_, Editor>, id: NodeId) -> DVec3 {
    h.state()
        .state
        .doc
        .scene()
        .local_transform(id)
        .unwrap()
        .scale
}

/// Two cubes at x = 10 and x = 100, both selected, the second active.
fn two() -> (EditorState, NodeId, NodeId) {
    let mut st = state();
    create(&mut st, "forge.cube");
    let a = st.selection.active.unwrap();
    create(&mut st, "forge.cube");
    let b = st.selection.active.unwrap();
    for (id, x) in [(a, 10.0), (b, 100.0)] {
        execute(
            &mut st,
            "transform.set",
            json!({"node": id.0, "pos": [x, 5.0, 0.0]}),
        )
        .unwrap();
    }
    st.selection.nodes = [a, b].into_iter().collect();
    st.selection.active = Some(b);
    (st, a, b)
}

#[test]
fn mixed_axes_are_found_per_row() {
    let (st, a, b) = two();
    let scene = st.doc.scene();
    let (view, mixed) = coord_view_multi(scene, &[b, a], Space::Local, Third::Scale).unwrap();
    close(view.pos, DVec3::new(100.0, 5.0, 0.0));
    assert_eq!(mixed[0], [true, false, false]);
    assert_eq!(mixed[1], [false; 3]);
    assert_eq!(mixed[2], [false; 3]);
}

#[test]
fn a_typed_field_sets_that_axis_on_every_selected_object_in_one_step() {
    let (st, a, b) = two();
    let mut h = open(st);
    let before = steps(&h);
    h.get_all_by_label("Position")
        .nth(1)
        .unwrap()
        .set_value_accesskit(42.0);
    h.run();
    assert_eq!(steps(&h), before + 1);
    close(pos(&h, a), DVec3::new(10.0, 42.0, 0.0));
    close(pos(&h, b), DVec3::new(100.0, 42.0, 0.0));
    execute(&mut h.state_mut().state, "edit.undo", json!({})).unwrap();
    close(pos(&h, a), DVec3::new(10.0, 5.0, 0.0));
}

#[test]
fn world_space_moves_a_child_by_its_world_position() {
    let mut st = state();
    create(&mut st, "forge.null");
    let parent = st.selection.active.unwrap();
    create(&mut st, "forge.cube");
    let child = st.selection.active.unwrap();
    execute(
        &mut st,
        "transform.set",
        json!({"node": parent.0, "pos": [100.0, 0.0, 0.0]}),
    )
    .unwrap();
    execute(
        &mut st,
        "object.move",
        json!({"nodes": [child.0], "parent": parent.0, "index": 0}),
    )
    .unwrap();
    st.selection.click(child);
    let mut h = open(st);
    h.get_by_label("World").click();
    h.run();
    h.get_all_by_label("Position")
        .next()
        .unwrap()
        .set_value_accesskit(250.0);
    h.run();
    close(pos(&h, child), DVec3::new(150.0, 0.0, 0.0));
}

#[test]
fn the_size_row_scales_the_selection_from_the_bounding_box() {
    let (st, a, b) = two();
    let mut h = open(st);
    h.get_by_label("Size").click();
    h.run();
    let scene = h.state().state.doc.scene();
    let (view, _) = coord_view_multi(scene, &[a, b], Space::Local, Third::Size).unwrap();
    close(view.third.unwrap(), DVec3::splat(200.0));
    // the first "Size" is the radio button, then the X, Y, Z fields
    h.get_all_by_label("Size")
        .nth(2)
        .unwrap()
        .set_value_accesskit(400.0);
    h.run();
    close(scale(&h, a), DVec3::new(1.0, 2.0, 1.0));
    close(scale(&h, b), DVec3::new(1.0, 2.0, 1.0));
}

#[test]
fn manual_mode_waits_for_apply_and_apply_is_one_step() {
    let (st, a, b) = two();
    let mut h = open(st);
    h.get_by_label("Manual").click();
    h.run();
    let before = steps(&h);
    h.get_all_by_label("Position")
        .next()
        .unwrap()
        .set_value_accesskit(1.0);
    h.run();
    h.get_all_by_label("Rotation")
        .next()
        .unwrap()
        .set_value_accesskit(90.0);
    h.run();
    assert_eq!(steps(&h), before, "nothing is written before Apply");
    close(pos(&h, a), DVec3::new(10.0, 5.0, 0.0));
    h.get_by_label("Apply").click();
    h.run();
    assert_eq!(
        steps(&h),
        before + 1,
        "both rows and both objects, one step"
    );
    close(pos(&h, a), DVec3::new(1.0, 5.0, 0.0));
    close(pos(&h, b), DVec3::new(1.0, 5.0, 0.0));
    let r = h.state().state.doc.scene().local_transform(a).unwrap().rot;
    assert!((r.x - std::f64::consts::FRAC_PI_2).abs() < 1e-9);
}

#[test]
fn reset_drops_the_draft() {
    let (st, a, _b) = two();
    let mut h = open(st);
    h.get_by_label("Manual").click();
    h.run();
    h.get_all_by_label("Position")
        .next()
        .unwrap()
        .set_value_accesskit(1.0);
    h.run();
    h.get_by_label("Reset").click();
    h.run();
    h.get_by_label("Apply").click();
    h.run();
    close(pos(&h, a), DVec3::new(10.0, 5.0, 0.0));
}

#[test]
fn draft_arguments_are_built_for_the_command() {
    let mut d = CoordDraft::default();
    assert!(d.transform_args(Space::Local, Third::Scale, None).is_none());
    d.set(0, 1, 3.0);
    d.set(2, 0, 2.0);
    let args = d
        .transform_args(Space::World, Third::Size, Some(4))
        .unwrap();
    assert_eq!(
        args,
        json!({"space": "world", "pos": [null, 3.0, null], "size": [2.0, null, null], "gesture": 4})
    );
}

fn editable_pair() -> (EditorState, NodeId, NodeId) {
    let mut st = state();
    let mut ids = Vec::new();
    for x in [10.0, 100.0] {
        create(&mut st, "forge.cube");
        execute(&mut st, "mesh.make_editable", json!({})).unwrap();
        let id = st.selection.active.unwrap();
        execute(
            &mut st,
            "transform.set",
            json!({"node": id.0, "pos": [x, 5.0, 0.0]}),
        )
        .unwrap();
        ids.push(id);
    }
    st.selection.nodes = ids.iter().copied().collect();
    st.selection.active = Some(ids[1]);
    (st, ids[0], ids[1])
}

fn points_of(st: &EditorState, id: NodeId) -> Vec<DVec3> {
    let w = st.doc.scene().world_matrix(id).unwrap();
    match st.doc.scene().param(id, "mesh") {
        Some(Value::Mesh(m)) => m
            .points
            .iter()
            .map(|p| w.transform_point3(DVec3::from_array(*p)))
            .collect(),
        _ => panic!("mesh"),
    }
}

#[test]
fn component_rows_run_mesh_transform_in_one_step() {
    let (mut st, a, b) = editable_pair();
    execute(&mut st, "select.mode", json!({"mode": "points"})).unwrap();
    execute(&mut st, "select.all", json!({})).unwrap();
    let c = forge_ui::component_edit::component_coords(&st, true).unwrap();
    // a pivot move and a size change as one draft
    let mut d = CoordDraft::default();
    d.set(0, 0, c.pivot.x + 10.0);
    d.set(2, 1, c.size.y * 2.0);
    let args = component_transform_args(&st, &d, true, None).unwrap();
    let before = st.doc.history().undo_labels().len();
    execute(&mut st, "mesh.transform", args).unwrap();
    assert_eq!(st.doc.history().undo_labels().len(), before + 1);
    let n = forge_ui::component_edit::component_coords(&st, true).unwrap();
    close(n.pivot, c.pivot + DVec3::new(10.0, 0.0, 0.0));
    close(n.size, DVec3::new(c.size.x, c.size.y * 2.0, c.size.z));
    assert_eq!(points_of(&st, a).len(), 8);
    assert_eq!(points_of(&st, b).len(), 8);
}

#[test]
fn the_panel_shows_pivot_rows_in_a_component_mode_and_applies_them() {
    let (mut st, a, b) = editable_pair();
    execute(&mut st, "select.mode", json!({"mode": "points"})).unwrap();
    execute(&mut st, "select.all", json!({})).unwrap();
    let before = points_of(&st, a);
    let mut h = open(st);
    assert!(h.query_all_by_label("Pivot").count() >= 3);
    assert!(
        h.query_by_label("Position").is_none(),
        "no duplicate object rows"
    );
    let steps0 = steps(&h);
    h.get_all_by_label("Pivot")
        .nth(1)
        .unwrap()
        .set_value_accesskit(300.0);
    h.run();
    assert_eq!(steps(&h), steps0 + 1, "both objects, one step");
    let after = points_of(&h.state().state, a);
    assert!((after[0].y - before[0].y).abs() > 1.0);
    let _ = b;
}

#[test]
fn object_edits_leave_no_change_on_an_empty_selection() {
    let mut st = state();
    create(&mut st, "forge.cube");
    st.selection.nodes.clear();
    st.selection.active = None;
    let h = open(st);
    assert!(h.query_by_label("Position").is_none());
}
