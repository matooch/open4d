//! docs/specs/EDITABLE.md §3: moving, rotating and scaling components, without a GPU.

mod common;

use std::sync::Arc;

use common::{create, state};
use forge_core::{MeshData, NodeId, Registry, Value};
use forge_generators::{SELECTION_EDGES, SELECTION_POINTS, SELECTION_POLYS, evaluate};
use forge_geometry::selection::{IndexSet, Selection};
use forge_ui::attributes::Gesture;
use forge_ui::commands::{Command, execute};
use forge_ui::component_edit::{
    ComponentDrag, affected_points, local_matrix, pivot, pivot_view, pivot_world, transform_points,
};
use forge_ui::components::EditMode;
use forge_ui::state::EditorState;
use forge_viewport::gizmo::DragDelta;
use glam::{DMat4, DVec3};

fn close(a: DVec3, b: DVec3) {
    assert!((a - b).abs().max_element() < 1e-9, "{a} != {b}");
}

fn pt(m: &MeshData, i: usize) -> DVec3 {
    DVec3::from_array(m.points[i])
}

fn set(items: &[u32]) -> IndexSet {
    items.iter().copied().collect()
}

/// A made-editable default cube (200 wide) in `mode`.
fn setup(mode: EditMode) -> (EditorState, NodeId) {
    let mut st = state();
    create(&mut st, "forge.cube");
    execute(&mut st, Command::MakeEditable).unwrap();
    let id = st.selection.active.unwrap();
    execute(&mut st, Command::SetMode(mode)).unwrap();
    (st, id)
}

fn stored(st: &EditorState, id: NodeId) -> Arc<MeshData> {
    match st.doc.scene().param(id, "mesh") {
        Some(Value::Mesh(m)) => m.clone(),
        _ => panic!("mesh param"),
    }
}

fn select(st: &mut EditorState, id: NodeId, param: &str, items: &[u32]) {
    st.doc
        .transact("sel", |tx| {
            tx.set_param(id, param, Value::IndexList(items.to_vec()))
        })
        .unwrap();
}

/// The polygon whose four points all have y = 100, as (poly index, its points).
fn top_poly(m: &MeshData) -> (u32, Vec<u32>) {
    let (i, p) = m
        .polys
        .iter()
        .enumerate()
        .find(|(_, p)| {
            p.iter()
                .all(|k| (m.points[*k as usize][1] - 100.0).abs() < 1e-9)
        })
        .expect("a top polygon");
    (i as u32, p.clone())
}

fn assert_only_moved(before: &MeshData, after: &MeshData, moved: &[u32]) {
    assert_eq!(before.polys, after.polys);
    for i in 0..before.points.len() {
        if moved.contains(&(i as u32)) {
            assert_ne!(before.points[i], after.points[i], "point {i} should move");
        } else {
            assert_eq!(before.points[i], after.points[i], "point {i} must not move");
        }
    }
}

#[test]
fn move_the_top_polygon_up_by_50() {
    let (st, id) = setup(EditMode::Polys);
    let m = stored(&st, id);
    let (_, ring) = top_poly(&m);
    let top: IndexSet = ring.iter().copied().collect();
    let out = transform_points(
        &m,
        &top,
        DMat4::from_translation(DVec3::new(0.0, 50.0, 0.0)),
    );
    for i in 0..8u32 {
        if top.contains(i) {
            assert_eq!(out.points[i as usize][1], 150.0);
            assert_eq!(out.points[i as usize][0], m.points[i as usize][0]);
            assert_eq!(out.points[i as usize][2], m.points[i as usize][2]);
        } else {
            assert_eq!(out.points[i as usize], m.points[i as usize]);
        }
    }
    assert_eq!(out.polys, m.polys);
}

#[test]
fn scale_2_about_the_centroid_and_rotate_90_about_y() {
    let (st, id) = setup(EditMode::Polys);
    let m = stored(&st, id);
    let (_, ring) = top_poly(&m);
    let top: IndexSet = ring.iter().copied().collect();
    let c = pivot(&m, &top).unwrap();
    close(c, DVec3::new(0.0, 100.0, 0.0));

    let world = DMat4::IDENTITY;
    let s = local_matrix(&world, c, DragDelta::Scale(DVec3::splat(2.0)));
    let out = transform_points(&m, &top, s);
    for i in 0..8usize {
        if top.contains(i as u32) {
            close(
                pt(&out, i),
                DVec3::new(pt(&m, i).x * 2.0, 100.0, pt(&m, i).z * 2.0),
            );
        } else {
            assert_eq!(out.points[i], m.points[i]);
        }
    }

    let r = local_matrix(
        &world,
        c,
        DragDelta::Rotate {
            axis: DVec3::Y,
            angle: 90f64.to_radians(),
        },
    );
    let out = transform_points(&m, &top, r);
    for i in 0..8usize {
        if top.contains(i as u32) {
            // right-handed about +y: (x, z) -> (z, -x)
            let p = pt(&m, i);
            close(pt(&out, i), DVec3::new(p.z, 100.0, -p.x));
        } else {
            assert_eq!(out.points[i], m.points[i]);
        }
    }
}

#[test]
fn empty_set_and_out_of_range_indices_change_nothing() {
    let (st, id) = setup(EditMode::Points);
    let m = stored(&st, id);
    let t = DMat4::from_translation(DVec3::splat(5.0));
    assert_eq!(transform_points(&m, &IndexSet::new(), t), *m);
    assert_eq!(transform_points(&m, &set(&[8, 99]), t), *m);
    assert_eq!(pivot(&m, &IndexSet::new()), None);
    let sets = Selection {
        points: set(&[1, 50]),
        edges: set(&[500]),
        polys: set(&[77]),
    };
    assert_eq!(affected_points(&m, EditMode::Points, &sets), set(&[1]));
    assert!(affected_points(&m, EditMode::Edges, &sets).is_empty());
    assert!(affected_points(&m, EditMode::Polys, &sets).is_empty());
    assert!(affected_points(&m, EditMode::Model, &sets).is_empty());
}

/// Rotated by h = 30 degrees, p = 20, b = 15, scaled (2, 1, 3), moved to (10, 20, 30).
fn skew(st: &mut EditorState, id: NodeId) {
    st.doc
        .transact("skew", |tx| {
            tx.set_param(id, "transform.pos", Value::Vec3([10.0, 20.0, 30.0]))?;
            tx.set_param(
                id,
                "transform.rot",
                Value::Vec3([30f64.to_radians(), 20f64.to_radians(), 15f64.to_radians()]),
            )?;
            tx.set_param(id, "transform.scale", Value::Vec3([2.0, 1.0, 3.0]))
        })
        .unwrap();
}

fn world_points(st: &EditorState, id: NodeId, m: &MeshData) -> Vec<DVec3> {
    let w = st.doc.scene().world_matrix(id).unwrap();
    (0..m.points.len())
        .map(|i| w.transform_point3(pt(m, i)))
        .collect()
}

#[test]
fn world_axis_move_and_rotate_on_a_rotated_non_uniformly_scaled_object() {
    let (mut st, id) = setup(EditMode::Polys);
    skew(&mut st, id);
    let m = stored(&st, id);
    let (_, ring) = top_poly(&m);
    let top: IndexSet = ring.iter().copied().collect();
    let c = pivot(&m, &top).unwrap();
    let w = st.doc.scene().world_matrix(id).unwrap();
    let before = world_points(&st, id, &m);

    // move +50 along world y: the world positions change by exactly that
    let d = DVec3::new(0.0, 50.0, 0.0);
    let out = transform_points(&m, &top, local_matrix(&w, c, DragDelta::Translate(d)));
    let after = world_points(&st, id, &out);
    for i in 0..8 {
        let want = if top.contains(i as u32) {
            before[i] + d
        } else {
            before[i]
        };
        close(after[i], want);
    }
    // the object-space delta is W^-1 d: not along y, because of the rotation and scale
    let local = pt(&out, ring[0] as usize) - pt(&m, ring[0] as usize);
    close(local, w.inverse().transform_vector3(d));
    assert!(local.x.abs() > 1e-6 && local.z.abs() > 1e-6);

    // rotate 90 degrees about world y through the world pivot
    let pw = w.transform_point3(c);
    let angle = 90f64.to_radians();
    let out = transform_points(
        &m,
        &top,
        local_matrix(
            &w,
            c,
            DragDelta::Rotate {
                axis: DVec3::Y,
                angle,
            },
        ),
    );
    let after = world_points(&st, id, &out);
    let rot = glam::DQuat::from_axis_angle(DVec3::Y, angle);
    for i in 0..8 {
        let want = if top.contains(i as u32) {
            pw + rot * (before[i] - pw)
        } else {
            before[i]
        };
        close(after[i], want);
    }

    // scale 2 in the object frame: object-space scale about the pivot
    let out = transform_points(
        &m,
        &top,
        local_matrix(&w, c, DragDelta::Scale(DVec3::new(2.0, 1.0, 0.5))),
    );
    for &i in &ring {
        let p = pt(&m, i as usize);
        close(
            pt(&out, i as usize),
            c + (p - c) * DVec3::new(2.0, 1.0, 0.5),
        );
    }
}

#[test]
fn points_edges_and_polygons_modes_move_the_right_points() {
    let (mut st, id) = setup(EditMode::Points);
    let m = stored(&st, id);
    let poly = forge_generators::to_poly_mesh(&m);
    let delta = DragDelta::Translate(DVec3::new(0.0, 0.0, 7.0));
    let run = |st: &mut EditorState| {
        let drag = ComponentDrag::begin(st).expect("a selection to move");
        drag.apply(&mut st.doc, delta, Gesture::Commit).unwrap();
        let out = stored(st, id);
        // undo so the next case starts from the cube
        assert!(st.doc.undo());
        out
    };

    // Points: exactly the selected points
    select(&mut st, id, SELECTION_POINTS, &[2, 5]);
    let out = run(&mut st);
    assert_only_moved(&m, &out, &[2, 5]);
    assert_eq!(out.points[2][2], m.points[2][2] + 7.0);

    // Edges: the two end points of each selected edge (edge 0 and the last edge)
    execute(&mut st, Command::SetMode(EditMode::Edges)).unwrap();
    let edges = poly.edges();
    let last = edges.len() as u32 - 1;
    select(&mut st, id, SELECTION_EDGES, &[0, last]);
    let want: Vec<u32> = [edges[0], edges[last as usize]]
        .iter()
        .flat_map(|e| [e.0, e.1])
        .collect::<IndexSet>()
        .iter()
        .collect();
    let out = run(&mut st);
    assert_only_moved(&m, &out, &want);

    // Polygons: the points of the selected polygon
    execute(&mut st, Command::SetMode(EditMode::Polys)).unwrap();
    let (pi, ring) = top_poly(&m);
    select(&mut st, id, SELECTION_POLYS, &[pi]);
    let out = run(&mut st);
    assert_only_moved(&m, &out, &ring);

    // the stored sets of other modes do not leak into this mode
    select(&mut st, id, SELECTION_POINTS, &[0]);
    let out = run(&mut st);
    assert_only_moved(&m, &out, &ring);
}

#[test]
fn nothing_to_drag_without_a_selection_or_target() {
    let (mut st, id) = setup(EditMode::Points);
    assert!(
        ComponentDrag::begin(&st).is_none(),
        "empty selection: no drag"
    );
    assert!(pivot_world(&st).is_none());
    assert!(pivot_view(&st, false).is_none());
    select(&mut st, id, SELECTION_POINTS, &[0]);
    assert!(ComponentDrag::begin(&st).is_some());
    // Model mode: no component drag even with a stored selection
    execute(&mut st, Command::SetMode(EditMode::Model)).unwrap();
    assert!(ComponentDrag::begin(&st).is_none());
    // a non-polygon active object
    let cube = create(&mut st, "forge.cube");
    execute(&mut st, Command::SetMode(EditMode::Points)).unwrap();
    assert_eq!(st.selection.active, Some(cube));
    assert!(ComponentDrag::begin(&st).is_none());
}

#[test]
fn a_drag_is_one_undo_step_and_frames_do_not_accumulate() {
    let (mut st, id) = setup(EditMode::Polys);
    let m = stored(&st, id);
    let (pi, ring) = top_poly(&m);
    select(&mut st, id, SELECTION_POLYS, &[pi]);
    let steps = st.doc.history().undo_labels().len();

    let drag = ComponentDrag::begin(&st).unwrap();
    for y in [10.0, 30.0, 50.0] {
        drag.apply(
            &mut st.doc,
            DragDelta::Translate(DVec3::new(0.0, y, 0.0)),
            Gesture::Drag(7),
        )
        .unwrap();
    }
    let moved = stored(&st, id);
    for &i in &ring {
        assert_eq!(moved.points[i as usize][1], 150.0, "from the start mesh");
    }
    assert_eq!(st.doc.history().undo_labels().len(), steps + 1);
    assert_eq!(st.doc.history().undo_labels().last().copied(), Some("Move"));
    // the gizmo / pivot follows the edited mesh
    close(pivot_view(&st, false).unwrap(), DVec3::new(0.0, 150.0, 0.0));
    close(pivot_world(&st).unwrap(), DVec3::new(0.0, 150.0, 0.0));

    assert!(st.doc.undo());
    assert_eq!(*stored(&st, id), *m);
    assert_eq!(st.doc.history().undo_labels().len(), steps);
    assert!(st.doc.redo());
    assert_eq!(*stored(&st, id), *moved);

    // the selection survived the edit
    let sel = forge_ui::components::selection_of(st.doc.scene(), id, EditMode::Polys);
    assert_eq!(sel, set(&[pi]));
}

#[test]
fn a_bend_child_deforms_the_edited_mesh() {
    let mut r = Registry::with_builtins();
    r.install(&forge_generators::Primitives).unwrap();
    r.install(&forge_modifiers::Deformers).unwrap();
    let mut st = EditorState::new(Arc::new(r));
    create(&mut st, "forge.cube");
    execute(&mut st, Command::MakeEditable).unwrap();
    let id = st.selection.active.unwrap();
    st.doc
        .transact("bend", |tx| {
            let b = tx.create("forge.bend", Some(id), None)?;
            tx.set_param(b, "strength", Value::Float(1.0))
        })
        .unwrap();
    execute(&mut st, Command::SetMode(EditMode::Polys)).unwrap();
    let m = stored(&st, id);
    let (pi, ring) = top_poly(&m);
    let before = evaluate(st.doc.scene(), id).unwrap().unwrap();
    // the bend is active
    assert!(
        (0..8).any(
            |i| (before.points[i] - forge_generators::to_poly_mesh(&m).points[i]).length() > 1.0
        )
    );

    select(&mut st, id, SELECTION_POLYS, &[pi]);
    let drag = ComponentDrag::begin(&st).unwrap();
    drag.apply(
        &mut st.doc,
        DragDelta::Translate(DVec3::new(0.0, 50.0, 0.0)),
        Gesture::Commit,
    )
    .unwrap();
    // the stored mesh holds the edit, undeformed
    let edited = stored(&st, id);
    for &i in &ring {
        assert_eq!(edited.points[i as usize][1], 150.0);
    }
    let after = evaluate(st.doc.scene(), id).unwrap().unwrap();
    assert_eq!(after.polys, before.polys);
    for i in 0..8usize {
        if ring.contains(&(i as u32)) {
            // moved, and bent: not the plain edited position
            assert!((after.points[i] - before.points[i]).length() > 1.0);
            assert!((after.points[i] - DVec3::from_array(edited.points[i])).length() > 1.0);
        } else {
            // the bend acts per point, so the untouched points evaluate as before
            assert!((after.points[i] - before.points[i]).length() < 1e-9);
        }
    }
}
