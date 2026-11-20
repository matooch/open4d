//! Field list folders and field display (docs/specs/UI.md §16.6; brief 064): the Folder
//! button of a field list, the gizmo strips of the selected fields and the value preview.

use std::collections::BTreeSet;
use std::sync::Arc;

use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use forge_core::{EvalCtx, NodeId, Value};
use forge_ui::app::Editor;
use forge_ui::commands::execute;
use forge_ui::field_overlay::{
    MAX_DOTS, local_gizmo, preview_dots, project_strip, selected_preview, shown_fields,
    value_colour,
};
use forge_ui::state::EditorState;
use glam::DVec3;
use serde_json::json;

const VIEW: [f64; 2] = [800.0, 600.0];

fn state() -> EditorState {
    EditorState::new(Arc::new(forge_app::default_registry().unwrap()))
}

fn make(s: &mut EditorState, ty: &str, name: &str) -> NodeId {
    execute(s, "object.create", json!({"type": ty})).unwrap();
    let id = s.selection.active.unwrap();
    execute(s, "object.rename", json!({"node": id.0, "name": name})).unwrap();
    id
}

fn set(s: &mut EditorState, id: NodeId, param: &str, value: serde_json::Value) {
    execute(
        s,
        "param.set",
        json!({"node": id.0, "param": param, "value": value}),
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

fn rows(s: &EditorState, id: NodeId) -> Vec<forge_core::LinkRow> {
    match s.doc.scene().param(id, "fields") {
        Some(Value::LinkRows(rows)) => rows.clone(),
        other => panic!("not a row list: {other:?}"),
    }
}

#[test]
fn the_folder_button_adds_a_group_row_and_keeps_the_owner_active() {
    let mut s = state();
    make(&mut s, "forge.cube", "Cube");
    let bend = make(&mut s, "forge.bend", "Bender");
    let ball = make(&mut s, "forge.field.spherical", "Ball");
    s.selection.click(bend);
    let mut h = harness(s);
    h.run();
    h.get_by_role_and_label(egui::accesskit::Role::ComboBox, "Fields")
        .click();
    h.run();
    h.get_all_by_label("Ball").last().unwrap().click();
    h.run();
    h.get_by_label("Add Folder").click();
    h.run();
    let list = rows(st(&h), bend);
    assert_eq!(list.len(), 2);
    assert_eq!(list[0].target, ball);
    let group = list[1].target;
    assert_eq!(
        st(&h).doc.scene().node(group).unwrap().type_id,
        "forge.field.group"
    );
    assert_eq!(
        (list[1].blend.as_str(), list[1].strength, list[1].enabled),
        ("normal", 1.0, true)
    );
    // the owner is the active object again, so its panel still shows the list
    assert_eq!(st(&h).selection.active, Some(bend));
    // the group is a field of its own: selecting it shows its own list, edited the same way
    let mut s2 = std::mem::replace(&mut h.state_mut().state, state());
    s2.selection.click(group);
    let fields = shown_fields(s2.doc.scene(), &[group].into());
    assert!(fields.is_empty(), "an empty group has no gizmo of its own");
    // two undo steps: the row edit, then the group creation
    execute(&mut s2, "edit.undo", json!({})).unwrap();
    assert_eq!(rows(&s2, bend).len(), 1);
    execute(&mut s2, "edit.undo", json!({})).unwrap();
    assert!(!s2.doc.scene().contains(group));
}

#[test]
fn field_gizmos_follow_the_field_parameters() {
    let mut s = state();
    let ball = make(&mut s, "forge.field.spherical", "Ball");
    set(&mut s, ball, "radius", json!(80.0));
    let strips = local_gizmo(s.doc.scene(), ball);
    assert_eq!(strips.len(), 3);
    assert!(
        strips
            .iter()
            .flatten()
            .all(|p| (p.length() - 80.0).abs() < 1e-9)
    );
    let cube = make(&mut s, "forge.field.box", "Box");
    set(&mut s, cube, "size", json!([10.0, 20.0, 30.0]));
    let edges = local_gizmo(s.doc.scene(), cube);
    assert_eq!(edges.len(), 12);
    let max = edges
        .iter()
        .flatten()
        .fold(DVec3::ZERO, |m, p| m.max(p.abs()));
    assert_eq!(max, DVec3::new(10.0, 20.0, 30.0));
    let lin = make(&mut s, "forge.field.linear", "Ramp");
    set(&mut s, lin, "length", json!(50.0));
    set(&mut s, lin, "direction", json!("z_neg"));
    let strips = local_gizmo(s.doc.scene(), lin);
    let axis = strips.last().unwrap();
    assert_eq!(
        axis,
        &vec![DVec3::new(0.0, 0.0, 50.0), DVec3::new(0.0, 0.0, -50.0)]
    );
    let cone = make(&mut s, "forge.field.cone", "Cone");
    set(&mut s, cone, "height", json!(200.0));
    let strips = local_gizmo(s.doc.scene(), cone);
    // base circle at -height/2 along Y+, lines reach the apex at +height/2
    assert!(strips[0].iter().all(|p| (p.y + 100.0).abs() < 1e-9));
    assert_eq!(strips[1][1], DVec3::new(0.0, 100.0, 0.0));
    let rnd = make(&mut s, "forge.field.random", "Noise");
    assert!(local_gizmo(s.doc.scene(), rnd).is_empty());
    // the world gizmo is placed by the field's transform
    set(&mut s, ball, "transform.pos", json!([100.0, 0.0, 0.0]));
    let world = forge_ui::field_overlay::field_gizmo(s.doc.scene(), ball);
    assert!(
        world
            .iter()
            .flatten()
            .all(|p| ((*p - DVec3::X * 100.0).length() - 80.0).abs() < 1e-9)
    );
}

#[test]
fn shown_fields_are_the_selection_and_the_enabled_rows_of_selected_lists() {
    let mut s = state();
    let bend = make(&mut s, "forge.bend", "Bender");
    let a = make(&mut s, "forge.field.linear", "A");
    let b = make(&mut s, "forge.field.spherical", "B");
    let group = make(&mut s, "forge.field.group", "G");
    let inner = make(&mut s, "forge.field.box", "Inner");
    let invert = make(&mut s, "forge.field.invert_layer", "Inv");
    set(&mut s, group, "fields", json!([inner.0]));
    set(
        &mut s,
        bend,
        "fields",
        json!([
            {"target": a.0},
            {"target": b.0, "enabled": false},
            {"target": group.0},
            {"target": invert.0},
        ]),
    );
    let sel: BTreeSet<NodeId> = [bend].into();
    // the disabled row is left out, the group is opened, a modifier layer is a field-category row
    // that draws nothing
    let shown = shown_fields(s.doc.scene(), &sel);
    assert_eq!(shown, vec![a, inner, invert]);
    assert!(local_gizmo(s.doc.scene(), invert).is_empty());
    // a selected field shows itself
    assert_eq!(shown_fields(s.doc.scene(), &[b].into()), vec![b]);
    assert!(shown_fields(s.doc.scene(), &BTreeSet::new()).is_empty());
}

#[test]
fn the_preview_colours_follow_the_field_list_value() {
    let mut s = state();
    let lin = make(&mut s, "forge.field.linear", "Ramp");
    set(&mut s, lin, "direction", json!("y_pos"));
    set(&mut s, lin, "length", json!(100.0));
    let camera = EditorState::default_camera();
    let ctx = EvalCtx::default();
    let points = [
        DVec3::new(0.0, -100.0, 0.0),
        DVec3::new(0.0, 0.0, 0.0),
        DVec3::new(0.0, 100.0, 0.0),
    ];
    let dots = preview_dots(s.doc.scene(), lin, &points, &ctx, &camera, VIEW);
    assert_eq!(dots.len(), 3);
    let values: Vec<f32> = dots.iter().map(|d| d.value).collect();
    assert!(
        (values[0] - 0.0).abs() < 1e-6
            && (values[1] - 0.5).abs() < 1e-6
            && (values[2] - 1.0).abs() < 1e-6
    );
    assert!(dots.iter().all(|d| d.at[0] >= 0.0 && d.at[0] <= 800.0));
    // blue, white, red
    assert!(value_colour(0.0)[2] > value_colour(0.0)[0]);
    assert_eq!(value_colour(0.5), [235, 235, 235]);
    assert!(value_colour(1.0)[0] > value_colour(1.0)[2]);
    assert_eq!(value_colour(-3.0), value_colour(0.0));
    assert_eq!(value_colour(9.0), value_colour(1.0));
    // a deformer's list and a field itself are both owners; nothing selected, nothing drawn
    let bend = make(&mut s, "forge.bend", "Bender");
    set(&mut s, bend, "fields", json!([lin.0]));
    let sel: BTreeSet<NodeId> = [bend].into();
    let via = selected_preview(s.doc.scene(), &sel, &points, &ctx, &camera, VIEW);
    assert_eq!(via, dots);
    assert!(
        selected_preview(
            s.doc.scene(),
            &BTreeSet::new(),
            &points,
            &ctx,
            &camera,
            VIEW
        )
        .is_empty()
    );
    // the cap, and points behind the camera or outside the pane are skipped
    let many = vec![DVec3::ZERO; MAX_DOTS + 10];
    assert_eq!(
        preview_dots(s.doc.scene(), lin, &many, &ctx, &camera, VIEW).len(),
        MAX_DOTS
    );
    let far = [DVec3::new(1e7, 0.0, 0.0)];
    assert!(preview_dots(s.doc.scene(), lin, &far, &ctx, &camera, VIEW).is_empty());
}

#[test]
fn strips_are_cut_behind_the_camera() {
    let camera = EditorState::default_camera();
    let ahead = [DVec3::ZERO, DVec3::new(10.0, 0.0, 0.0)];
    assert_eq!(project_strip(&ahead, &camera, VIEW).len(), 1);
    let behind = camera.eye_world() - camera.forward_world() * 10.0;
    assert!(project_strip(&[behind, behind + DVec3::X], &camera, VIEW).is_empty());
    let cut = project_strip(&[DVec3::ZERO, behind, DVec3::X, DVec3::Y], &camera, VIEW);
    assert_eq!(cut.len(), 1);
}

#[test]
fn the_view_menu_toggles_the_field_preview() {
    let mut h = harness(state());
    h.run();
    assert!(!st(&h).views.field_preview);
    h.get_by_label("View").click();
    h.run();
    h.get_by_label("Field Preview").click();
    h.run();
    assert!(st(&h).views.field_preview);
}
