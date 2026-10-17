//! docs/specs/VIEWPORT.md §5. Gizmo at the origin; default camera looks down -Z.

mod common;

use common::{H, W, close};
use forge_viewport::camera::Camera;
use forge_viewport::gizmo::{Axis, Drag, DragDelta, Gizmo, GizmoMode, Handle, Plane};
use glam::DVec3;

fn setup(mode: GizmoMode) -> (Camera, Gizmo, f64) {
    let c = Camera::default();
    let g = Gizmo {
        origin: DVec3::ZERO,
        mode,
        size_px: 100.0,
        ..Gizmo::default()
    };
    let l = g.world_length(&c, H);
    (c, g, l)
}

fn px(c: &Camera, p: DVec3) -> (f64, f64) {
    let s = c.project(p, W, H).unwrap();
    (s.x, s.y)
}

#[test]
fn world_length_matches_pixel_size() {
    let (c, g, l) = setup(GizmoMode::Translate);
    assert!((l - 100.0 * c.world_per_pixel(H)).abs() < 1e-12);
    let (x0, _) = px(&c, g.origin);
    let (x1, _) = px(&c, DVec3::X * l);
    assert!((x1 - x0 - 100.0).abs() < 1e-9);
}

#[test]
fn hit_test_priorities() {
    let (c, g, l) = setup(GizmoMode::Translate);
    let hit = |p: DVec3| {
        let (x, y) = px(&c, p);
        g.hit_test(&c, x, y, W, H)
    };
    assert_eq!(hit(DVec3::ZERO), Some(Handle::Center));
    assert_eq!(hit(DVec3::X * 0.8 * l), Some(Handle::Axis(Axis::X)));
    assert_eq!(hit(DVec3::Y * 0.9 * l), Some(Handle::Axis(Axis::Y)));
    assert_eq!(
        hit(DVec3::new(0.3 * l, 0.3 * l, 0.0)),
        Some(Handle::Plane(Plane::XY))
    );
    assert_eq!(hit(DVec3::new(0.8 * l, 0.8 * l, 0.0)), None);
    let (x, y) = px(&c, DVec3::X * 0.8 * l);
    assert_eq!(
        g.hit_test(&c, x, y + 5.0, W, H),
        Some(Handle::Axis(Axis::X))
    );
    assert_eq!(g.hit_test(&c, x, y + 7.0, W, H), None);
}

fn drag(c: &Camera, g: &Gizmo, h: Handle, from: DVec3, to: DVec3, snap: Option<f64>) -> DragDelta {
    let (x0, y0) = px(c, from);
    let mut d = Drag::begin(g, h, c, x0, y0, W, H).expect("drag starts");
    let (x1, y1) = px(c, to);
    d.update(c, x1, y1, W, H, snap)
}

fn translate(d: DragDelta) -> DVec3 {
    match d {
        DragDelta::Translate(v) => v,
        other => panic!("expected translate, got {other:?}"),
    }
}

fn scale(d: DragDelta) -> DVec3 {
    match d {
        DragDelta::Scale(v) => v,
        other => panic!("expected scale, got {other:?}"),
    }
}

#[test]
fn translate_drags_follow_their_constraint() {
    let (c, g, l) = setup(GizmoMode::Translate);
    let a = DVec3::X * 0.5 * l;
    let d = translate(drag(
        &c,
        &g,
        Handle::Axis(Axis::X),
        a,
        a + DVec3::new(2.0, 0.0, 0.0),
        None,
    ));
    close(d, DVec3::new(2.0, 0.0, 0.0), 1e-6);
    let d = translate(drag(
        &c,
        &g,
        Handle::Axis(Axis::X),
        a,
        a + DVec3::X * 1.3,
        Some(0.5),
    ));
    close(d, DVec3::new(1.5, 0.0, 0.0), 1e-12);

    let p = DVec3::new(0.3 * l, 0.3 * l, 0.0);
    let d = translate(drag(
        &c,
        &g,
        Handle::Plane(Plane::XY),
        p,
        p + DVec3::new(1.0, -2.0, 0.0),
        None,
    ));
    close(d, DVec3::new(1.0, -2.0, 0.0), 1e-6);
    let d = translate(drag(
        &c,
        &g,
        Handle::Center,
        DVec3::ZERO,
        DVec3::new(0.7, 0.2, 0.0),
        None,
    ));
    close(d, DVec3::new(0.7, 0.2, 0.0), 1e-6);
}

#[test]
fn scale_drags() {
    let (c, g, l) = setup(GizmoMode::Scale);
    let a = DVec3::X * 0.5 * l;
    close(
        scale(drag(&c, &g, Handle::Axis(Axis::X), a, a * 2.0, None)),
        DVec3::new(2.0, 1.0, 1.0),
        1e-6,
    );
    let p = DVec3::new(0.3 * l, 0.3 * l, 0.0);
    close(
        scale(drag(&c, &g, Handle::Plane(Plane::XY), p, p * 2.0, None)),
        DVec3::new(2.0, 2.0, 1.0),
        1e-6,
    );

    let (x0, y0) = px(&c, DVec3::ZERO);
    let mut d = Drag::begin(&g, Handle::Center, &c, x0, y0, W, H).unwrap();
    close(
        scale(d.update(&c, x0 + 50.0, y0 + 30.0, W, H, None)),
        DVec3::splat(1.5),
        1e-12,
    );
    close(
        scale(d.update(&c, x0 + 52.0, y0, W, H, Some(0.25))),
        DVec3::splat(1.5),
        1e-12,
    );
}

#[test]
fn degenerate_drags_do_not_start() {
    let (c, g, _) = setup(GizmoMode::Translate);
    // the view ray through the centre is parallel to the Z axis
    assert!(Drag::begin(&g, Handle::Axis(Axis::Z), &c, W / 2.0, H / 2.0, W, H).is_none());
    let (cs, gs, _) = setup(GizmoMode::Scale);
    // scaling from the axis origin has no reference length
    assert!(Drag::begin(&gs, Handle::Axis(Axis::X), &cs, W / 2.0, H / 2.0, W, H).is_none());
}

// ---- docs/specs/MANIPULATE.md §2: object frames and rotation ----

fn rotated_frame() -> [DVec3; 3] {
    // 90° about +Y (right-handed): X -> -Z, Z -> +X
    [DVec3::NEG_Z, DVec3::Y, DVec3::X]
}

#[test]
fn axis_handles_follow_the_frame() {
    let c = Camera::default();
    let mut g = Gizmo {
        axes: rotated_frame(),
        ..Gizmo::default()
    };
    g.origin = DVec3::ZERO;
    let l = g.world_length(&c, H);
    // the gizmo's X handle now points along world -Z, i.e. at the camera: not pickable;
    // its Z handle lies along world +X
    let (x, y) = px(&c, DVec3::X * 0.8 * l);
    assert_eq!(g.hit_test(&c, x, y, W, H), Some(Handle::Axis(Axis::Z)));
    let d = translate(drag(
        &c,
        &g,
        Handle::Axis(Axis::Z),
        DVec3::X * 0.5 * l,
        DVec3::X * 0.5 * l + DVec3::X * 3.0,
        None,
    ));
    close(d, DVec3::new(3.0, 0.0, 0.0), 1e-6);
    assert_eq!(Axis::Y.index(), 1);
}

fn rotate(d: DragDelta) -> (DVec3, f64) {
    match d {
        DragDelta::Rotate { axis, angle } => (axis, angle),
        other => panic!("expected rotate, got {other:?}"),
    }
}

#[test]
fn rotate_rings_hit_and_measure_signed_angles() {
    let c = Camera::default(); // looking down -Z at the origin
    let g = Gizmo {
        mode: GizmoMode::Rotate,
        ..Gizmo::default()
    };
    let l = g.world_length(&c, H);
    // the Z ring faces the camera: a circle of radius l in the XY plane
    let (x, y) = px(
        &c,
        DVec3::new(l, 0.0, 0.0) * std::f64::consts::FRAC_1_SQRT_2
            + DVec3::new(0.0, l, 0.0) * std::f64::consts::FRAC_1_SQRT_2,
    );
    assert_eq!(g.hit_test(&c, x, y, W, H), Some(Handle::Axis(Axis::Z)));
    // inside the Z ring, away from the edge-on X and Y rings (lines through the centre)
    let (ix, iy) = px(&c, DVec3::new(0.4 * l, 0.3 * l, 0.0));
    assert_eq!(
        g.hit_test(&c, ix, iy, W, H),
        None,
        "no centre handle in rotate mode"
    );

    let from = DVec3::X * l;
    let to = DVec3::Y * l; // +90° about +Z (right-handed)
    let (axis, angle) = rotate(drag(&c, &g, Handle::Axis(Axis::Z), from, to, None));
    close(axis, DVec3::Z, 1e-12);
    assert!(
        (angle - std::f64::consts::FRAC_PI_2).abs() < 1e-6,
        "{angle}"
    );
    let (_, back) = rotate(drag(&c, &g, Handle::Axis(Axis::Z), to, from, None));
    assert!((back + std::f64::consts::FRAC_PI_2).abs() < 1e-6, "{back}");
    let near = DVec3::new(
        l * 20f64.to_radians().cos(),
        l * 20f64.to_radians().sin(),
        0.0,
    );
    let (_, snapped) = rotate(drag(
        &c,
        &g,
        Handle::Axis(Axis::Z),
        from,
        near,
        Some(15f64.to_radians()),
    ));
    assert!((snapped - 15f64.to_radians()).abs() < 1e-9, "{snapped}");
}

#[test]
fn rotate_drags_need_a_ring_and_a_plane() {
    let c = Camera::default();
    let g = Gizmo {
        mode: GizmoMode::Rotate,
        ..Gizmo::default()
    };
    let (cx, cy) = px(&c, DVec3::ZERO);
    assert!(Drag::begin(&g, Handle::Center, &c, cx, cy, W, H).is_none());
    // a ray through the origin hits the Z ring's plane at the origin: no start vector
    assert!(Drag::begin(&g, Handle::Axis(Axis::Z), &c, cx, cy, W, H).is_none());
}
