//! docs/specs/MANIPULATE.md §6.3 and §6.7: cursor snapping and translate-to-target drags.
//! Default camera: eye (0, 0, 10) looking down -Z; the unit box spans -1..1.

mod common;

use common::{H, W, close, unit_box};
use forge_modeling::snap::{SnapKind, SnapMode, SnapSettings, SnapSource, Workplane};
use forge_viewport::camera::{Camera, Projection};
use forge_viewport::gizmo::{Axis, Drag, DragDelta, Gizmo, GizmoMode, Handle, Plane};
use forge_viewport::snap::snap_cursor;
use glam::DVec3;

fn source() -> Vec<SnapSource> {
    vec![SnapSource {
        mesh: unit_box(),
        origin: Some(DVec3::ZERO),
        skip_points: vec![],
    }]
}

fn only(kinds: &[SnapKind]) -> SnapSettings {
    let mut s = SnapSettings::default();
    for k in SnapKind::ALL {
        s.targets.set(k, kinds.contains(&k));
    }
    s
}

fn px(c: &Camera, p: DVec3) -> (f64, f64) {
    let s = c.project(p, W, H).unwrap();
    (s.x, s.y)
}

fn at(
    src: &[SnapSource],
    c: &Camera,
    p: DVec3,
    off: (f64, f64),
    s: &SnapSettings,
) -> Option<forge_modeling::snap::SnapHit> {
    let (x, y) = px(c, p);
    snap_cursor(src, c, (x + off.0, y + off.1), (W, H), s)
}

#[test]
fn each_target_type_under_the_cursor() {
    let (c, src) = (Camera::default(), source());
    // vertex (front-top-left is point 6: x=-1, y=1, z=1)
    let h = at(
        &src,
        &c,
        DVec3::new(-1., 1., 1.),
        (2.0, -1.0),
        &only(&[SnapKind::Vertex]),
    )
    .unwrap();
    assert_eq!(
        (h.kind, h.index, h.source),
        (SnapKind::Vertex, Some(6), Some(0))
    );
    close(h.position, DVec3::new(-1., 1., 1.), 1e-12);
    assert!(h.distance < 3.0);
    // edge midpoint of the front-top edge
    let h = at(
        &src,
        &c,
        DVec3::new(0., 1., 1.),
        (1.0, 2.0),
        &only(&[SnapKind::EdgeMidpoint]),
    )
    .unwrap();
    assert_eq!(h.kind, SnapKind::EdgeMidpoint);
    close(h.position, DVec3::new(0., 1., 1.), 1e-12);
    // polygon centre of the front face
    let h = at(
        &src,
        &c,
        DVec3::new(0., 0., 1.),
        (3.0, 3.0),
        &only(&[SnapKind::PolygonCenter]),
    )
    .unwrap();
    assert_eq!((h.kind, h.index), (SnapKind::PolygonCenter, Some(3)));
    close(h.position, DVec3::new(0., 0., 1.), 1e-12);
    // edge: a point a quarter along the front-top edge, cursor 4 px below it
    let h = at(
        &src,
        &c,
        DVec3::new(-0.5, 1., 1.),
        (0.0, 4.0),
        &only(&[SnapKind::Edge]),
    )
    .unwrap();
    assert_eq!(h.kind, SnapKind::Edge);
    assert!((h.position.x + 0.5).abs() < 0.05, "{}", h.position);
    assert!((h.position.y - 1.0).abs() < 1e-9 && (h.position.z - 1.0).abs() < 1e-9);
    // polygon surface: a cursor in the middle of the front face
    let h = at(
        &src,
        &c,
        DVec3::new(0.3, -0.2, 1.),
        (0.0, 0.0),
        &only(&[SnapKind::Polygon]),
    )
    .unwrap();
    assert_eq!((h.kind, h.index), (SnapKind::Polygon, Some(3)));
    close(h.position, DVec3::new(0.3, -0.2, 1.), 1e-9);
    assert_eq!(h.distance, 0.0);
    // object axis
    let h = at(
        &src,
        &c,
        DVec3::ZERO,
        (2.0, 2.0),
        &only(&[SnapKind::ObjectAxis]),
    )
    .unwrap();
    assert_eq!(h.kind, SnapKind::ObjectAxis);
    close(h.position, DVec3::ZERO, 1e-12);
}

#[test]
fn the_radius_limits_vertex_candidates() {
    let (c, src) = (Camera::default(), source());
    let mut s = only(&[SnapKind::Vertex]);
    s.radius = 5.0;
    assert!(at(&src, &c, DVec3::new(-1., 1., 1.), (4.0, 0.0), &s).is_some());
    assert!(at(&src, &c, DVec3::new(-1., 1., 1.), (8.0, 0.0), &s).is_none());
    s.radius = 20.0;
    assert!(at(&src, &c, DVec3::new(-1., 1., 1.), (8.0, 0.0), &s).is_some());
}

#[test]
fn priority_point_over_edge_over_polygon() {
    let (c, src) = (Camera::default(), source());
    let all = [
        SnapKind::Vertex,
        SnapKind::Edge,
        SnapKind::Polygon,
        SnapKind::Workplane,
    ];
    let mut s = only(&all);
    s.workplane = Workplane {
        origin: DVec3::ZERO,
        normal: DVec3::Z,
    };
    // 6 px from a vertex, inside the face: the vertex wins
    let h = at(&src, &c, DVec3::new(-1., 1., 1.), (6.0, 6.0), &s).unwrap();
    assert_eq!(h.kind, SnapKind::Vertex);
    // near an edge but away from vertices: the edge beats the polygon
    let h = at(&src, &c, DVec3::new(0.2, 1., 1.), (0.0, 4.0), &s).unwrap();
    assert_eq!(h.kind, SnapKind::Edge);
    // middle of the face: the polygon beats the workplane
    let h = at(&src, &c, DVec3::new(0.2, 0.1, 1.), (0.0, 0.0), &s).unwrap();
    assert_eq!(h.kind, SnapKind::Polygon);
    // beside the box: only the workplane
    let h = at(&src, &c, DVec3::new(3., 0., 0.), (0.0, 0.0), &s).unwrap();
    assert_eq!(h.kind, SnapKind::Workplane);
    close(h.position, DVec3::new(3., 0., 0.), 1e-9);
    // nothing enabled
    assert!(at(&src, &c, DVec3::ZERO, (0.0, 0.0), &only(&[])).is_none());
}

#[test]
fn grid_point_and_grid_line_on_the_workplane() {
    let c = Camera::default();
    let mut s = only(&[SnapKind::GridPoint]);
    s.workplane = Workplane {
        origin: DVec3::ZERO,
        normal: DVec3::Z,
    };
    s.grid_spacing = 1.0;
    // 1 world unit is ~ 100 px here; the cursor is 5 px from the grid point (2, 1)
    let h = at(&[], &c, DVec3::new(2., 1., 0.), (4.0, 3.0), &s).unwrap();
    assert_eq!(h.kind, SnapKind::GridPoint);
    close(h.position, DVec3::new(2., 1., 0.), 1e-9);
    // between grid points: out of reach
    assert!(at(&[], &c, DVec3::new(2.5, 1.5, 0.), (0.0, 0.0), &s).is_none());
    // grid line: 4 px from the vertical line x = 2, anywhere along it
    let mut l = only(&[SnapKind::GridLine]);
    l.workplane = s.workplane;
    l.grid_spacing = 1.0;
    let h = at(&[], &c, DVec3::new(2.0, 1.4, 0.), (3.0, 0.0), &l).unwrap();
    assert_eq!(h.kind, SnapKind::GridLine);
    assert!((h.position.x - 2.0).abs() < 1e-9 && (h.position.y - 1.4).abs() < 0.05);
    // the automatic spacing follows the visible grid (1 for the default camera)
    l.grid_spacing = 0.0;
    let h = at(&[], &c, DVec3::new(2.0, 1.4, 0.), (3.0, 0.0), &l).unwrap();
    assert!((h.position.x - 2.0).abs() < 1e-9);
}

#[test]
fn hidden_targets_follow_visible_only() {
    let (c, src) = (Camera::default(), source());
    // the back-top-left corner (point 2: x=-1,y=1,z=-1) is behind the front face
    let back = DVec3::new(-1., 1., -1.);
    let mut s = only(&[SnapKind::Vertex]);
    s.radius = 3.0;
    s.visible_only = false;
    let h = at(&src, &c, back, (0.0, 0.0), &s).unwrap();
    assert_eq!(h.index, Some(2));
    s.visible_only = true;
    // the back vertex is occluded; a front vertex projects farther away than 3 px
    let h = at(&src, &c, back, (0.0, 0.0), &s);
    assert!(h.is_none_or(|h| h.index != Some(2)), "{h:?}");
}

#[test]
fn skipped_points_are_not_snapped_to() {
    let (c, mut src) = (Camera::default(), source());
    src[0].skip_points = vec![6];
    let s = only(&[SnapKind::Vertex]);
    let h = at(&src, &c, DVec3::new(-1., 1., 1.), (1.0, 1.0), &s);
    assert!(h.is_none_or(|h| h.index != Some(6)));
    let mid = only(&[SnapKind::EdgeMidpoint, SnapKind::Edge]);
    // the edge from point 6 to 7 is skipped; its midpoint is not a target
    let h = at(&src, &c, DVec3::new(0., 1., 1.), (0.0, 0.0), &mid);
    assert!(
        h.is_none_or(|h| h.position != DVec3::new(0., 1., 1.)),
        "{h:?}"
    );
}

#[test]
fn two_d_mode_projects_onto_the_workplane() {
    let (c, src) = (Camera::default(), source());
    let mut s = only(&[SnapKind::Vertex]);
    s.workplane = Workplane {
        origin: DVec3::new(0., 0., 5.),
        normal: DVec3::Z,
    };
    s.mode = SnapMode::D2;
    let h = at(&src, &c, DVec3::new(-1., 1., 1.), (0.0, 0.0), &s).unwrap();
    close(h.raw, DVec3::new(-1., 1., 1.), 1e-12);
    close(h.position, DVec3::new(-1., 1., 5.), 1e-12);
}

#[test]
fn orthographic_cameras_snap_too() {
    let c = Camera {
        projection: Projection::Orthographic { height: 6.0 },
        ..Camera::default()
    };
    let src = source();
    let h = at(
        &src,
        &c,
        DVec3::new(1., -1., 1.),
        (2.0, 2.0),
        &only(&[SnapKind::Vertex]),
    )
    .unwrap();
    assert_eq!(h.kind, SnapKind::Vertex);
    close(h.position, DVec3::new(1., -1., 1.), 1e-12);
}

fn translate_drag(mode: GizmoMode, handle: Handle) -> (Drag, Camera) {
    let c = Camera::default();
    let g = Gizmo {
        origin: DVec3::new(1.0, 2.0, 3.0),
        mode,
        size_px: 100.0,
        ..Gizmo::default()
    };
    let l = g.world_length(&c, H);
    let from = match handle {
        Handle::Axis(a) => g.origin + g.axis(a) * 0.5 * l,
        Handle::Plane(Plane::XY) => g.origin + DVec3::new(0.3 * l, 0.3 * l, 0.0),
        _ => g.origin,
    };
    let (x, y) = px(&c, from);
    (Drag::begin(&g, handle, &c, x, y, W, H).expect("begins"), c)
}

fn delta(d: DragDelta) -> DVec3 {
    match d {
        DragDelta::Translate(v) => v,
        other => panic!("{other:?}"),
    }
}

#[test]
fn translate_to_respects_the_handle() {
    let target = DVec3::new(5.0, -4.0, 10.0);
    // axis handle: only the component along X
    let (mut d, _) = translate_drag(GizmoMode::Translate, Handle::Axis(Axis::X));
    close(
        delta(d.translate_to(target)),
        DVec3::new(4.0, 0.0, 0.0),
        1e-12,
    );
    // plane handle XY: z stays
    let (mut d, _) = translate_drag(GizmoMode::Translate, Handle::Plane(Plane::XY));
    close(
        delta(d.translate_to(target)),
        DVec3::new(4.0, -6.0, 0.0),
        1e-12,
    );
    // centre: the whole difference
    let (mut d, _) = translate_drag(GizmoMode::Translate, Handle::Center);
    close(
        delta(d.translate_to(target)),
        DVec3::new(4.0, -6.0, 7.0),
        1e-12,
    );
    // the origin lands on the target for a free drag, and a second call replaces the first
    close(
        delta(d.translate_to(DVec3::new(1.0, 2.0, 3.0))),
        DVec3::ZERO,
        1e-12,
    );
}

#[test]
fn translate_to_leaves_rotate_and_scale_drags_alone() {
    let (mut d, c) = translate_drag(GizmoMode::Scale, Handle::Axis(Axis::X));
    let before = d.translate_to(DVec3::splat(9.0));
    assert!(matches!(before, DragDelta::Scale(v) if v == DVec3::ONE));
    let _ = c;
}
