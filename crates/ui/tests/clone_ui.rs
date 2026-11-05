//! Clone toolset UI (docs/specs/UI.md §16; brief 079): cloner from selection, the effector and
//! field lists in the attribute manager (add, drag, reorder, enable, strength), the Create
//! menu, the index overlay and the instanced viewport. Every edit is one undo step.

use std::collections::BTreeSet;
use std::sync::Arc;

use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use forge_clone_engine::{CloneData, CloneItem};
use forge_core::{EvalCtx, NodeId, Value};
use forge_generators::clone_data;
use forge_ui::app::Editor;
use forge_ui::clone_overlay::{MAX_MARKS, OverlayOptions, clone_marks};
use forge_ui::commands::{clone_toolset_menu, execute};
use forge_ui::state::EditorState;
use forge_ui::viewport_panel::{DrawItem, scene_draw_items_at};
use glam::{DMat4, DVec3};
use serde_json::json;

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

/// A linear cloner (3 clones, 50 cm apart along Y) with a cube, plus two effectors:
/// "Absolute" (absolute scale 2) and "Relative" (relative scale 1). Cloner selected.
fn scene() -> (EditorState, NodeId, NodeId, NodeId) {
    let mut s = state();
    let cube = make(&mut s, "forge.cube", "Cube");
    let abs = make(&mut s, "forge.effector.plain", "Absolute");
    let rel = make(&mut s, "forge.effector.plain", "Relative");
    for (e, absolute, value) in [(abs, true, 2.0), (rel, false, 1.0)] {
        set(&mut s, e, "position", json!(false));
        set(&mut s, e, "scale", json!(true));
        set(&mut s, e, "uniform_scale", json!(true));
        set(&mut s, e, "absolute_scale", json!(absolute));
        // plain floats are plain numbers; percent-less
        set(&mut s, e, "scale_value", json!(value));
    }
    execute(&mut s, "object.create_cloner", json!({"nodes": [cube.0]})).unwrap();
    let cloner = s.selection.active.unwrap();
    set(&mut s, cloner, "mode", json!("linear"));
    (s, cloner, abs, rel)
}

fn harness(state: EditorState) -> Harness<'static, Editor> {
    Harness::builder()
        .with_size([1280.0, 1400.0])
        .build_ui_state(|ui, e: &mut Editor| e.show(ui, None), Editor::new(state))
}

fn st<'a>(h: &'a Harness<'_, Editor>) -> &'a EditorState {
    &h.state().state
}

fn list(h: &Harness<'_, Editor>, id: NodeId, param: &str) -> Vec<NodeId> {
    match st(h).doc.scene().param(id, param) {
        Some(Value::LinkList(l)) => l.clone(),
        Some(Value::LinkRows(rows)) => rows.iter().map(|r| r.target).collect(),
        other => panic!("not a list: {other:?}"),
    }
}

/// The rows of a field list parameter.
fn rows(h: &Harness<'_, Editor>, id: NodeId, param: &str) -> Vec<forge_core::LinkRow> {
    match st(h).doc.scene().param(id, param) {
        Some(Value::LinkRows(rows)) => rows.clone(),
        other => panic!("not a row list: {other:?}"),
    }
}

fn undo_steps(h: &Harness<'_, Editor>) -> usize {
    st(h).doc.history().undo_labels().len()
}

fn undo(h: &mut Harness<'_, Editor>) {
    execute(&mut h.state_mut().state, "edit.undo", json!({})).unwrap();
    h.run();
}

fn data(h: &Harness<'_, Editor>, cloner: NodeId) -> CloneData {
    clone_data(st(h).doc.scene(), cloner, &EvalCtx::default())
        .unwrap()
        .unwrap()
}

fn scale0(h: &Harness<'_, Editor>, cloner: NodeId) -> f64 {
    data(h, cloner).matrices[0].x_axis.length()
}

fn combo(h: &mut Harness<'_, Editor>, label: &str) {
    h.get_by_role_and_label(egui::accesskit::Role::ComboBox, label)
        .click();
    h.run();
}

fn pick(h: &mut Harness<'_, Editor>, label: &str) {
    h.get_all_by_label(label).last().unwrap().click();
    h.run();
}

fn add_via_list(h: &mut Harness<'_, Editor>, name: &str) {
    combo(h, "Effectors");
    pick(h, name);
}

#[test]
fn a_cloner_from_the_selection_adopts_the_objects_and_undoes_in_one_step() {
    let mut s = state();
    let a = make(&mut s, "forge.cube", "A");
    let b = make(&mut s, "forge.sphere", "B");
    let e = make(&mut s, "forge.effector.plain", "Eff");
    let steps = s.doc.history().undo_labels().len();
    execute(
        &mut s,
        "object.create_cloner",
        json!({"nodes": [a.0, b.0, e.0]}),
    )
    .unwrap();
    let c = s.selection.active.unwrap();
    let scene = s.doc.scene();
    assert_eq!(scene.node(c).unwrap().type_id, "forge.cloner");
    assert_eq!(scene.node(c).unwrap().children, vec![a, b]);
    // an effector is linked, not adopted
    assert_eq!(scene.node(e).unwrap().parent, None);
    assert_eq!(scene.param(c, "effectors"), Some(&Value::LinkList(vec![e])));
    assert_eq!(s.doc.history().undo_labels().len(), steps + 1);
    execute(&mut s, "edit.undo", json!({})).unwrap();
    let scene = s.doc.scene();
    assert!(!scene.contains(c));
    assert_eq!(scene.node(a).unwrap().parent, None);
    assert_eq!(scene.node(b).unwrap().parent, None);
}

#[test]
fn a_cloner_from_nothing_is_empty_and_takes_the_first_objects_place() {
    let mut s = state();
    execute(&mut s, "object.create_cloner", json!({"nodes": []})).unwrap();
    let c = s.selection.active.unwrap();
    assert!(s.doc.scene().node(c).unwrap().children.is_empty());
    let a = make(&mut s, "forge.cube", "A");
    let b = make(&mut s, "forge.cube", "B");
    execute(
        &mut s,
        "object.create_cloner",
        json!({"nodes": [b.0], "name": "Mine"}),
    )
    .unwrap();
    let m = s.selection.active.unwrap();
    // the roots are now: first cloner, A, then the new cloner where B was
    assert_eq!(s.doc.scene().roots(), &[c, a, m]);
    assert_eq!(s.doc.scene().node(m).unwrap().name, "Mine");
}

#[test]
fn the_create_menu_lists_the_clone_toolset_from_the_registry() {
    let reg = forge_app::default_registry().unwrap();
    let labels: Vec<String> = clone_toolset_menu(&reg)
        .into_iter()
        .map(|e| e.label)
        .collect();
    assert_eq!(labels[0], "Cloner");
    assert_eq!(labels[1], "Cloner from Selection");
    for want in ["Plain", "Random", "Linear", "Spherical", "Box"] {
        assert!(
            labels.iter().any(|l| l.contains(want)),
            "{want} missing in {labels:?}"
        );
    }
    let entries = clone_toolset_menu(&reg);
    assert_eq!(entries[1].command, "object.create_cloner");
    assert!(entries[2..].iter().all(|e| e.command == "object.create"));
}

#[test]
fn the_create_menu_runs_cloner_from_selection() {
    let mut s = state();
    let a = make(&mut s, "forge.cube", "Solo");
    let mut h = harness(s);
    h.run();
    h.get_by_label("Create").click();
    h.run();
    h.get_by_label("Clone toolset").click();
    h.run();
    h.get_by_label("Cloner from Selection").click();
    h.run();
    let c = st(&h).selection.active.unwrap();
    assert_eq!(st(&h).doc.scene().node(c).unwrap().type_id, "forge.cloner");
    assert_eq!(st(&h).doc.scene().node(a).unwrap().parent, Some(c));
}

#[test]
fn adding_effectors_through_the_list_applies_them_in_order_and_undoes() {
    let (s, cloner, abs, rel) = scene();
    let mut h = harness(s);
    h.run();
    assert!((scale0(&h, cloner) - 1.0).abs() < 1e-12);
    let before = undo_steps(&h);
    add_via_list(&mut h, "Absolute");
    assert_eq!(list(&h, cloner, "effectors"), vec![abs]);
    assert_eq!(undo_steps(&h), before + 1);
    assert!((scale0(&h, cloner) - 2.0).abs() < 1e-12);
    add_via_list(&mut h, "Relative");
    assert_eq!(list(&h, cloner, "effectors"), vec![abs, rel]);
    // absolute 2, then relative adds 1
    assert!((scale0(&h, cloner) - 3.0).abs() < 1e-12);
    undo(&mut h);
    assert_eq!(list(&h, cloner, "effectors"), vec![abs]);
    assert!((scale0(&h, cloner) - 2.0).abs() < 1e-12);
    undo(&mut h);
    assert!(list(&h, cloner, "effectors").is_empty());
    assert!((scale0(&h, cloner) - 1.0).abs() < 1e-12);
}

#[test]
fn dragging_an_effector_from_the_object_manager_adds_it() {
    let (s, cloner, abs, _) = scene();
    let mut h = harness(s);
    h.run();
    let before = undo_steps(&h);
    let src = h.get_by_label("Absolute").rect().center();
    let dst = h
        .get_by_role_and_label(egui::accesskit::Role::ComboBox, "Effectors")
        .rect()
        .center();
    h.hover_at(src);
    h.run();
    h.drag_at(src);
    h.run();
    for k in 1..=4 {
        h.hover_at(src.lerp(dst, k as f32 / 4.0));
        h.run();
    }
    h.event(egui::Event::PointerButton {
        pos: dst,
        button: egui::PointerButton::Primary,
        pressed: false,
        modifiers: egui::Modifiers::NONE,
    });
    h.run();
    assert_eq!(list(&h, cloner, "effectors"), vec![abs]);
    assert_eq!(undo_steps(&h), before + 1);
    undo(&mut h);
    assert!(list(&h, cloner, "effectors").is_empty());
}

#[test]
fn reordering_the_list_changes_the_clone_matrices() {
    let (s, cloner, abs, rel) = scene();
    let mut h = harness(s);
    h.run();
    add_via_list(&mut h, "Absolute");
    add_via_list(&mut h, "Relative");
    let first = data(&h, cloner);
    assert!((first.matrices[0].x_axis.length() - 3.0).abs() < 1e-12);
    let before = undo_steps(&h);
    h.get_by_label("Move Relative up").click();
    h.run();
    assert_eq!(list(&h, cloner, "effectors"), vec![rel, abs]);
    assert_eq!(undo_steps(&h), before + 1);
    let swapped = data(&h, cloner);
    assert!((swapped.matrices[0].x_axis.length() - 2.0).abs() < 1e-12);
    assert_ne!(first.matrices, swapped.matrices);
    h.get_by_label("Move Relative down").click();
    h.run();
    assert_eq!(data(&h, cloner), first);
    undo(&mut h);
    assert_eq!(data(&h, cloner), swapped);
    undo(&mut h);
    assert_eq!(data(&h, cloner), first);
    assert_eq!(list(&h, cloner, "effectors"), vec![abs, rel]);
}

#[test]
fn the_enable_box_switches_one_effector_without_leaving_the_list() {
    let (s, cloner, abs, rel) = scene();
    let mut h = harness(s);
    h.run();
    add_via_list(&mut h, "Absolute");
    add_via_list(&mut h, "Relative");
    let before = undo_steps(&h);
    h.get_by_label("Enable Absolute").click();
    h.run();
    assert_eq!(
        st(&h).doc.scene().param(abs, "enabled"),
        Some(&Value::Bool(false))
    );
    assert_eq!(list(&h, cloner, "effectors"), vec![abs, rel]);
    assert_eq!(undo_steps(&h), before + 1);
    // only the relative effector acts: 1 + 1
    assert!((scale0(&h, cloner) - 2.0).abs() < 1e-12);
    undo(&mut h);
    assert_eq!(
        st(&h).doc.scene().param(abs, "enabled"),
        Some(&Value::Bool(true))
    );
    assert!((scale0(&h, cloner) - 3.0).abs() < 1e-12);
}

#[test]
fn removing_an_effector_keeps_the_node_and_undoes() {
    let (s, cloner, abs, _) = scene();
    let mut h = harness(s);
    h.run();
    add_via_list(&mut h, "Absolute");
    h.get_by_label("Remove Absolute").click();
    h.run();
    assert!(list(&h, cloner, "effectors").is_empty());
    assert!(st(&h).doc.scene().contains(abs));
    undo(&mut h);
    assert_eq!(list(&h, cloner, "effectors"), vec![abs]);
}

/// An effector moving the clones 100 cm along X, with a spherical field.
fn field_scene() -> (EditorState, NodeId, NodeId, NodeId) {
    let mut s = state();
    let cube = make(&mut s, "forge.cube", "Cube");
    let eff = make(&mut s, "forge.effector.plain", "Mover");
    set(&mut s, eff, "position", json!(true));
    set(&mut s, eff, "p", json!([100.0, 0.0, 0.0]));
    let field = make(&mut s, "forge.field.spherical", "Ball");
    execute(&mut s, "object.create_cloner", json!({"nodes": [cube.0]})).unwrap();
    let cloner = s.selection.active.unwrap();
    set(&mut s, cloner, "mode", json!("linear"));
    set(&mut s, cloner, "effectors", json!([eff.0]));
    // clone 1 sits at y = 75: inside the field's radius, outside its inner half (value 0.5)
    set(
        &mut s,
        cloner,
        "transform_position",
        json!([0.0, 75.0, 0.0]),
    );
    s.selection.click(eff);
    (s, cloner, eff, field)
}

fn shift1(h: &Harness<'_, Editor>, cloner: NodeId) -> f64 {
    data(h, cloner).matrices[1].w_axis.x
}

#[test]
fn a_field_row_has_an_enable_box_and_a_strength_that_drive_the_effect() {
    let (s, cloner, eff, field) = field_scene();
    let mut h = harness(s);
    h.run();
    let none = shift1(&h, cloner);
    assert!((none - 100.0).abs() < 1e-9, "{none}");
    combo(&mut h, "Fields");
    pick(&mut h, "Ball");
    assert_eq!(list(&h, eff, "fields"), vec![field]);
    let full = shift1(&h, cloner);
    assert!((full - 50.0).abs() < 1e-9, "{full}");
    // the row strength is stored in the list row of the effector (brief 059), shown in percent
    let before = undo_steps(&h);
    h.get_by_label("Strength Ball").set_value_accesskit(50.0);
    h.run();
    assert_eq!(rows(&h, eff, "fields")[0].strength, 0.5);
    assert_eq!(undo_steps(&h), before + 1);
    assert!((shift1(&h, cloner) - 0.5 * full).abs() < 1e-9);
    // the enable box switches the layer off: back to full effect (empty list)
    h.get_by_label("Enable Ball").click();
    h.run();
    assert!(!rows(&h, eff, "fields")[0].enabled);
    // the field node itself is untouched
    assert_eq!(
        st(&h).doc.scene().param(field, "object.enabled"),
        Some(&Value::Bool(true))
    );
    assert_eq!(list(&h, eff, "fields"), vec![field]);
    assert!((shift1(&h, cloner) - 100.0).abs() < 1e-9);
    undo(&mut h);
    assert!((shift1(&h, cloner) - 0.5 * full).abs() < 1e-9);
    undo(&mut h);
    assert!((shift1(&h, cloner) - full).abs() < 1e-9);
}

#[test]
fn a_deformer_field_list_uses_the_same_rows() {
    let mut s = state();
    make(&mut s, "forge.cube", "Cube");
    let bend = make(&mut s, "forge.bend", "Bender");
    let field = make(&mut s, "forge.field.linear", "Ramp");
    s.selection.click(bend);
    let mut h = harness(s);
    h.run();
    combo(&mut h, "Fields");
    pick(&mut h, "Ramp");
    assert_eq!(list(&h, bend, "fields"), vec![field]);
    h.get_by_label("Strength Ramp").set_value_accesskit(25.0);
    h.run();
    assert_eq!(rows(&h, bend, "fields")[0].strength, 0.25);
    // the row belongs to the deformer: a field used in two lists has two strengths
    assert!(st(&h).doc.scene().param(field, "layer_strength").is_none());
}

#[test]
fn a_field_row_has_a_blend_combo_and_reordering_keeps_the_row_data() {
    let mut s = state();
    make(&mut s, "forge.cube", "Cube");
    let bend = make(&mut s, "forge.bend", "Bender");
    let a = make(&mut s, "forge.field.linear", "Ramp");
    let b = make(&mut s, "forge.field.spherical", "Ball");
    s.selection.click(bend);
    let mut h = harness(s);
    h.run();
    combo(&mut h, "Fields");
    pick(&mut h, "Ramp");
    combo(&mut h, "Fields");
    pick(&mut h, "Ball");
    assert_eq!(list(&h, bend, "fields"), vec![a, b]);
    let before = undo_steps(&h);
    h.get_by_role_and_label(egui::accesskit::Role::ComboBox, "Blend Ball")
        .click();
    h.run();
    pick(&mut h, "Add");
    assert_eq!(rows(&h, bend, "fields")[1].blend, "add");
    assert_eq!(undo_steps(&h), before + 1);
    h.get_by_label("Strength Ball").set_value_accesskit(40.0);
    h.run();
    // moving the row up carries its blend mode and strength with it
    h.get_by_label("Move Ball up").click();
    h.run();
    let r = rows(&h, bend, "fields");
    assert_eq!(
        (r[0].target, r[0].blend.as_str(), r[0].strength),
        (b, "add", 0.4)
    );
    assert_eq!(r[1].blend, "normal");
    h.get_by_label("Remove Ball").click();
    h.run();
    assert_eq!(list(&h, bend, "fields"), vec![a]);
    assert_eq!(rows(&h, bend, "fields")[0].blend, "normal");
}

fn clones_at(points: &[DVec3]) -> CloneData {
    let mut d = CloneData::default();
    for p in points {
        d.push(CloneItem::at(DMat4::from_translation(*p), DVec3::ZERO));
    }
    d
}

const VIEW: [f64; 2] = [800.0, 600.0];

#[test]
fn clone_marks_carry_index_position_and_axes() {
    let camera = EditorState::default_camera();
    let d = clones_at(&[DVec3::ZERO, DVec3::new(0.0, 0.0, 0.0)]);
    let both = OverlayOptions {
        indices: true,
        axes: true,
    };
    let marks = clone_marks(&d, &DMat4::IDENTITY, &camera, VIEW, both);
    assert_eq!(marks.len(), 2);
    assert_eq!(marks[1].index, 1);
    // the camera looks at the origin: the clone sits in the middle of the view
    assert!((marks[0].origin[0] - 400.0).abs() < 1e-3);
    assert!((marks[0].origin[1] - 300.0).abs() < 1e-3);
    let axes = marks[0].axes.unwrap();
    for end in axes {
        let len = ((end[0] - 400.0).powi(2) + (end[1] - 300.0).powi(2)).sqrt();
        assert!(len > 1.0 && len < 30.0, "{len}");
    }
    // indices alone: no axes; neither: no marks
    let idx = OverlayOptions {
        indices: true,
        axes: false,
    };
    assert!(
        clone_marks(&d, &DMat4::IDENTITY, &camera, VIEW, idx)[0]
            .axes
            .is_none()
    );
    assert!(
        clone_marks(
            &d,
            &DMat4::IDENTITY,
            &camera,
            VIEW,
            OverlayOptions::default()
        )
        .is_empty()
    );
}

#[test]
fn clone_marks_skip_hidden_behind_and_off_screen_clones_and_the_cloner_world_moves_them() {
    let camera = EditorState::default_camera();
    let behind = camera.eye_world() - camera.forward_world() * 10.0;
    let mut d = clones_at(&[DVec3::ZERO, behind, DVec3::new(1e6, 0.0, 0.0), DVec3::ZERO]);
    d.visible[3] = false;
    let idx = OverlayOptions {
        indices: true,
        axes: false,
    };
    let marks = clone_marks(&d, &DMat4::IDENTITY, &camera, VIEW, idx);
    assert_eq!(marks.iter().map(|m| m.index).collect::<Vec<_>>(), vec![0]);
    // the world matrix of the cloner places the clones
    let moved = DMat4::from_translation(DVec3::new(1e6, 0.0, 0.0));
    let one = clones_at(&[DVec3::ZERO]);
    assert!(clone_marks(&one, &moved, &camera, VIEW, idx).is_empty());
}

#[test]
fn clone_marks_are_capped_with_ten_thousand_clones() {
    let camera = EditorState::default_camera();
    let points: Vec<DVec3> = (0..10_000)
        .map(|i| DVec3::new(f64::from(i % 100) - 50.0, 0.0, f64::from(i / 100) - 50.0))
        .collect();
    let d = clones_at(&points);
    let idx = OverlayOptions {
        indices: true,
        axes: true,
    };
    let marks = clone_marks(&d, &DMat4::IDENTITY, &camera, VIEW, idx);
    assert_eq!(marks.len(), MAX_MARKS);
    assert_eq!(marks[0].index, 0);
    assert_eq!(marks[MAX_MARKS - 1].index, MAX_MARKS as u32 - 1);
}

#[test]
fn view_clone_display_sets_the_options_and_needs_an_argument() {
    let mut s = state();
    assert!(!s.views.clone_indices && !s.views.clone_axes);
    execute(&mut s, "view.clone_display", json!({"indices": true})).unwrap();
    assert!(s.views.clone_indices && !s.views.clone_axes);
    execute(
        &mut s,
        "view.clone_display",
        json!({"axes": true, "indices": false}),
    )
    .unwrap();
    assert!(!s.views.clone_indices && s.views.clone_axes);
    assert!(execute(&mut s, "view.clone_display", json!({})).is_err());
    assert!(s.doc.history().undo_labels().is_empty());
}

#[test]
fn the_view_menu_toggles_the_clone_overlay() {
    let mut h = harness(state());
    h.run();
    h.get_by_label("View").click();
    h.run();
    h.get_by_label("Clone Indices").click();
    h.run();
    assert!(st(&h).views.clone_indices);
}

#[test]
fn ten_thousand_clones_are_one_instanced_draw_item() {
    let (mut s, cloner, _, _) = scene();
    set(&mut s, cloner, "linear_count", json!(10_000));
    let items = scene_draw_items_at(s.doc.scene(), &BTreeSet::new(), &EvalCtx::default());
    let DrawItem::Instanced { id, instances, .. } = &items[0] else {
        panic!("the cloner is drawn by instancing")
    };
    assert_eq!(items.len(), 1);
    assert_eq!(*id, cloner);
    assert_eq!(instances.clones.len(), 10_000);
    // one prototype mesh, not 10 000 copies
    assert_eq!(instances.prototypes.len(), 1);
}

#[test]
fn selected_cloners_show_marks_and_matrix_mode_turns_the_axes_on() {
    use forge_ui::clone_overlay::selected_cloner_marks;
    let (mut s, cloner, _, _) = scene();
    let camera = EditorState::default_camera();
    let ctx = EvalCtx::default();
    let none = OverlayOptions::default();
    let marks = |s: &EditorState, sel: &BTreeSet<NodeId>, o| {
        selected_cloner_marks(s.doc.scene(), sel, &ctx, &camera, VIEW, o)
    };
    let sel: BTreeSet<NodeId> = [cloner].into();
    assert!(marks(&s, &sel, none).is_empty());
    let idx = OverlayOptions {
        indices: true,
        axes: false,
    };
    assert_eq!(marks(&s, &sel, idx).len(), 3);
    assert!(marks(&s, &BTreeSet::new(), idx).is_empty());
    set(&mut s, cloner, "viewport_mode", json!("matrix"));
    let m = marks(&s, &sel, none);
    assert_eq!(m.len(), 3);
    assert!(m.iter().all(|k| k.axes.is_some()));
}
