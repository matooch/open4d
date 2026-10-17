//! docs/specs/MANIPULATE.md §3–5: applying drags, the coordinate panel, picking.

mod common;

use common::{create, state};
use forge_core::transform::{Transform, hpb_from_rotation, rotation_part};
use forge_core::{NodeId, Value};
use forge_ui::attributes::Gesture;
use forge_ui::coordinates::{Row, Space, Third, coord_view, set_coord};
use forge_ui::manipulate::{apply_drag, pick_node};
use forge_ui::state::EditorState;
use forge_viewport::camera::Camera;
use forge_viewport::gizmo::DragDelta;
use glam::{DMat3, DMat4, DVec3};

fn close(a: DVec3, b: DVec3) {
    assert!((a - b).abs().max_element() < 1e-9, "{a} != {b}");
}

fn mclose(a: DMat3, b: DMat3) {
    let d = (a - b)
        .to_cols_array()
        .iter()
        .fold(0.0f64, |m, x| m.max(x.abs()));
    assert!(d < 1e-9, "{a} != {b}");
}

/// A child at local (100, 0, 0) under a parent at (0, 50, 0) turned 90° (field 1).
fn rig(st: &mut EditorState) -> (NodeId, NodeId) {
    let parent = create(st, "forge.null");
    let child = create(st, "forge.null");
    st.doc
        .transact("rig", |tx| {
            tx.set_param(parent, "transform.pos", Value::Vec3([0.0, 50.0, 0.0]))?;
            tx.set_param(
                parent,
                "transform.rot",
                Value::Vec3([90f64.to_radians(), 0.0, 0.0]),
            )?;
            tx.move_node(child, Some(parent), None)?;
            tx.set_param(child, "transform.pos", Value::Vec3([100.0, 0.0, 0.0]))
        })
        .unwrap();
    (parent, child)
}

fn world(st: &EditorState, id: NodeId) -> DMat4 {
    st.doc.scene().world_matrix(id).unwrap()
}

fn local(st: &EditorState, id: NodeId) -> Transform {
    st.doc.scene().local_transform(id).unwrap()
}

#[test]
fn translate_moves_the_world_position_under_a_rotated_parent() {
    let mut st = state();
    let (_, child) = rig(&mut st);
    let before = world(&st, child).w_axis.truncate();
    let start = local(&st, child);
    apply_drag(
        &mut st.doc,
        child,
        &start,
        DragDelta::Translate(DVec3::new(0.0, 0.0, 10.0)),
        Gesture::Drag(1),
    )
    .unwrap();
    close(
        world(&st, child).w_axis.truncate(),
        before + DVec3::new(0.0, 0.0, 10.0),
    );
    // a later update of the same drag replaces, not accumulates
    apply_drag(
        &mut st.doc,
        child,
        &start,
        DragDelta::Translate(DVec3::new(0.0, 0.0, 20.0)),
        Gesture::Drag(1),
    )
    .unwrap();
    close(
        world(&st, child).w_axis.truncate(),
        before + DVec3::new(0.0, 0.0, 20.0),
    );
}

#[test]
fn rotate_applies_a_world_axis_rotation() {
    let mut st = state();
    let (_, child) = rig(&mut st);
    for (axis, angle) in [
        (DVec3::Y, 0.7),
        (DVec3::X, -0.4),
        (DVec3::new(1.0, 1.0, 0.0).normalize(), 1.1),
    ] {
        let start = local(&st, child);
        let before = rotation_part(&world(&st, child));
        apply_drag(
            &mut st.doc,
            child,
            &start,
            DragDelta::Rotate { axis, angle },
            Gesture::Commit,
        )
        .unwrap();
        mclose(
            rotation_part(&world(&st, child)),
            DMat3::from_axis_angle(axis, angle) * before,
        );
    }
}

#[test]
fn scale_multiplies_along_object_axes_and_drags_are_one_step() {
    let mut st = state();
    let id = create(&mut st, "forge.null");
    st.doc
        .transact("s", |tx| {
            tx.set_param(id, "transform.scale", Value::Vec3([1.0, 2.0, 1.0]))
        })
        .unwrap();
    let start = local(&st, id);
    let steps = st.doc.history().undo_labels().len();
    for f in [1.5, 2.0] {
        apply_drag(
            &mut st.doc,
            id,
            &start,
            DragDelta::Scale(DVec3::new(f, 1.0, 1.0)),
            Gesture::Drag(7),
        )
        .unwrap();
    }
    close(local(&st, id).scale, DVec3::new(2.0, 2.0, 1.0));
    assert_eq!(st.doc.history().undo_labels().len(), steps + 1);
}

#[test]
fn coordinate_panel_local_and_world() {
    let mut st = state();
    let (_, child) = rig(&mut st);
    let s = st.doc.scene();
    let l = coord_view(s, child, Space::Local, Third::Scale).unwrap();
    close(l.pos, DVec3::new(100.0, 0.0, 0.0));
    close(l.third.unwrap(), DVec3::ONE);
    let w = coord_view(s, child, Space::World, Third::Scale).unwrap();
    close(w.pos, world(&st, child).w_axis.truncate());
    close(w.rot, DVec3::new(90f64.to_radians(), 0.0, 0.0));

    set_coord(
        &mut st.doc,
        child,
        Space::World,
        Row::Position,
        DVec3::new(1.0, 2.0, 3.0),
        Gesture::Commit,
    )
    .unwrap();
    close(
        world(&st, child).w_axis.truncate(),
        DVec3::new(1.0, 2.0, 3.0),
    );
    let target = DVec3::new(0.3, 0.2, 0.1);
    set_coord(
        &mut st.doc,
        child,
        Space::World,
        Row::Rotation,
        target,
        Gesture::Commit,
    )
    .unwrap();
    close(hpb_from_rotation(rotation_part(&world(&st, child))), target);
    set_coord(
        &mut st.doc,
        child,
        Space::Local,
        Row::Rotation,
        DVec3::ZERO,
        Gesture::Commit,
    )
    .unwrap();
    close(local(&st, child).rot, DVec3::ZERO);
}

#[test]
fn size_row_converts_through_the_mesh_bounds() {
    let mut st = state();
    let cube = create(&mut st, "forge.cube");
    let v = coord_view(st.doc.scene(), cube, Space::Local, Third::Size).unwrap();
    close(v.third.unwrap(), DVec3::splat(200.0));
    set_coord(
        &mut st.doc,
        cube,
        Space::Local,
        Row::Third(Third::Size),
        DVec3::new(400.0, 200.0, 100.0),
        Gesture::Commit,
    )
    .unwrap();
    close(local(&st, cube).scale, DVec3::new(2.0, 1.0, 0.5));
    let null = create(&mut st, "forge.null");
    assert!(
        coord_view(st.doc.scene(), null, Space::Local, Third::Size)
            .unwrap()
            .third
            .is_none()
    );
}

#[test]
fn pick_node_finds_the_object_under_the_cursor() {
    let mut st = state();
    let cube = create(&mut st, "forge.cube");
    let _null = create(&mut st, "forge.null");
    let cam = Camera {
        distance: 1000.0,
        ..EditorState::default_camera()
    };
    let (w, h) = (800.0, 600.0);
    assert_eq!(
        pick_node(st.doc.scene(), &cam, w / 2.0, h / 2.0, w, h),
        Some(cube)
    );
    assert_eq!(pick_node(st.doc.scene(), &cam, 2.0, 2.0, w, h), None);
}

#[test]
fn subdivision_inputs_are_drawn_only_through_their_generator() {
    use forge_ui::viewport_panel::scene_meshes;
    let mut st = state();
    let sub = create(&mut st, "forge.subdivision");
    let cube = create(&mut st, "forge.cube");
    forge_ui::commands::execute(
        &mut st,
        forge_ui::commands::Command::Reparent(cube, Some(sub)),
    )
    .unwrap();
    let drawn: Vec<NodeId> = scene_meshes(st.doc.scene(), &Default::default())
        .into_iter()
        .map(|(id, ..)| id)
        .collect();
    assert_eq!(drawn, [sub]);
    let (_, mesh, _, _) = &scene_meshes(st.doc.scene(), &Default::default())[0];
    assert_eq!(mesh.points.len(), 98, "default viewport level 2");
}
