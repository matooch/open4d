//! docs/specs/VIEWPORT.md §7.

mod common;

use common::{H, W, close};
use forge_viewport::camera::{Camera, Handedness, Projection};
use forge_viewport::nav::*;
use glam::DVec3;

fn cams() -> Vec<Camera> {
    let base = Camera {
        target: DVec3::new(3.0, -2.0, 5.0),
        distance: 40.0,
        yaw: 0.7,
        pitch: 0.4,
        ..Camera::default()
    };
    let ortho = Camera {
        projection: Projection::Orthographic { height: 30.0 },
        ..base
    };
    let left = Camera {
        handedness: Handedness::Left,
        ..base
    };
    vec![base, ortho, left]
}

#[test]
fn orbit_keeps_pivot_and_distance() {
    for c in cams() {
        let o = orbit_drag(&c, 120.0, -45.0);
        assert_eq!(o.target, c.target);
        assert_eq!(o.distance, c.distance);
        assert!(((o.eye() - o.target).length() - c.distance).abs() < 1e-9);
        assert!((o.yaw - (c.yaw - 1.2)).abs() < 1e-12);
        assert!((o.pitch - (c.pitch - 0.45)).abs() < 1e-12);
    }
}

#[test]
fn orbit_clamps_pitch_short_of_the_poles() {
    let o = orbit_drag(&Camera::default(), 0.0, 1e6);
    assert!(o.pitch < std::f64::consts::FRAC_PI_2);
    assert!(o.right().is_finite() && o.up().is_finite());
}

#[test]
fn pan_moves_the_pivot_in_the_view_plane() {
    for c in cams() {
        let p = pan_drag(&c, 30.0, -12.0, H);
        let d = p.target - c.target;
        assert!(d.length() > 1e-6);
        assert!(d.dot(c.forward()).abs() < 1e-9, "{d}");
        assert_eq!(p.distance, c.distance);
        // content follows the cursor
        let before = c.project(c.target_world(), W, H).unwrap();
        let after = p.project(c.target_world(), W, H).unwrap();
        assert!((after.x - before.x - 30.0).abs() < 1e-6, "{before} {after}");
        assert!((after.y - before.y + 12.0).abs() < 1e-6);
    }
}

#[test]
fn dolly_scales_distance_or_height() {
    let c = Camera {
        distance: 20.0,
        ..Camera::default()
    };
    let d = dolly_drag(&c, 100.0);
    assert!((d.distance - 20.0 * (-1.0f64).exp()).abs() < 1e-9);
    assert_eq!(d.target, c.target);
    let back = dolly_drag(&d, -100.0);
    assert!((back.distance - 20.0).abs() < 1e-9);
    let o = Camera {
        projection: Projection::Orthographic { height: 10.0 },
        ..c
    };
    let Projection::Orthographic { height } = dolly_drag(&o, -100.0).projection else {
        panic!()
    };
    assert!((height - 10.0 * 1.0f64.exp()).abs() < 1e-9);
    assert!(wheel_factor(100.0) < 1.0 && wheel_factor(-100.0) > 1.0);
}

#[test]
fn zoom_at_keeps_the_point_under_the_cursor_fixed() {
    for c in cams() {
        for cursor in [[400.0, 300.0], [100.0, 80.0], [700.0, 550.0], [10.0, 590.0]] {
            for f in [0.5, 0.9, 1.3] {
                // the pivot-plane point under the cursor
                let wpp = c.world_per_pixel(H);
                let p = c.target
                    + c.right() * (cursor[0] - W / 2.0) * wpp
                    + c.up() * (H / 2.0 - cursor[1]) * wpp;
                let p_world = p * c.mirror();
                let z = zoom_at(&c, cursor, [W, H], f);
                let s = z.project(p_world, W, H).unwrap();
                assert!((s.x - cursor[0]).abs() < 1e-6, "{s} {cursor:?} {f}");
                assert!((s.y - cursor[1]).abs() < 1e-6);
                let ratio = match (c.projection, z.projection) {
                    (Projection::Perspective { .. }, _) => z.distance / c.distance,
                    (
                        Projection::Orthographic { height: a },
                        Projection::Orthographic { height: b },
                    ) => b / a,
                    _ => unreachable!(),
                };
                assert!((ratio - f).abs() < 1e-9);
            }
        }
    }
}

#[test]
fn zoom_at_the_centre_is_a_plain_dolly_and_ignores_bad_factors() {
    let c = Camera::default();
    let z = zoom_at(&c, [W / 2.0, H / 2.0], [W, H], 0.5);
    close(z.target, c.target, 1e-12);
    assert!((z.distance - 5.0).abs() < 1e-12);
    assert_eq!(zoom_at(&c, [1.0, 1.0], [W, H], 0.0), c);
    assert_eq!(zoom_at(&c, [1.0, 1.0], [W, H], -2.0), c);
}

fn corners(min: DVec3, max: DVec3) -> Vec<DVec3> {
    (0..8)
        .map(|i| {
            DVec3::new(
                if i & 1 == 1 { max.x } else { min.x },
                if i & 2 == 2 { max.y } else { min.y },
                if i & 4 == 4 { max.z } else { min.z },
            )
        })
        .collect()
}

#[test]
fn frame_fits_the_box_for_every_aspect_projection_and_handedness() {
    let (min, max) = (DVec3::new(-30.0, 0.0, 10.0), DVec3::new(50.0, 20.0, 40.0));
    for c in cams() {
        for (w, h) in [(800.0, 600.0), (300.0, 900.0), (1600.0, 200.0)] {
            let f = frame_box(&c, min, max, w / h);
            close(f.target_world(), (min + max) / 2.0, 1e-9);
            assert_eq!((f.yaw, f.pitch), (c.yaw, c.pitch));
            let mut extent = 0.0f64;
            for p in corners(min, max) {
                let s = f.project(p, w, h).expect("in front");
                assert!(
                    s.x >= 0.0 && s.x <= w && s.y >= 0.0 && s.y <= h,
                    "{s} {w}x{h}"
                );
                extent = extent
                    .max(((s.x - w / 2.0) / w).abs() * 2.0)
                    .max(((s.y - h / 2.0) / h).abs() * 2.0);
            }
            // fills a good part of the limiting direction
            assert!(extent > 0.35, "{extent}");
        }
    }
}

#[test]
fn frame_of_a_point_only_recentres() {
    let c = Camera::default();
    let p = DVec3::new(5.0, 6.0, 7.0);
    let f = frame_box(&c, p, p, 1.5);
    close(f.target, p, 1e-12);
    assert_eq!(f.distance, c.distance);
}

#[test]
fn standard_views_look_along_their_axis() {
    let right_handed = Camera {
        distance: 12.0,
        ..Camera::default()
    };
    let left_handed = Camera {
        handedness: Handedness::Left,
        ..right_handed
    };
    for c in [right_handed, left_handed] {
        let mirror = c.mirror();
        let eye_dir = |v| {
            let n = view_camera(&c, v);
            assert!(matches!(n.projection, Projection::Orthographic { .. }));
            assert_eq!(n.target, c.target);
            (n.eye_world() - n.target_world()).normalize()
        };
        close(eye_dir(StandardView::Right), DVec3::X, 1e-9);
        close(eye_dir(StandardView::Left), DVec3::NEG_X, 1e-9);
        close(eye_dir(StandardView::Top), DVec3::Y, 1e-3);
        close(eye_dir(StandardView::Bottom), DVec3::NEG_Y, 1e-3);
        // front is where the default camera sits on Z, back the other side
        let front = eye_dir(StandardView::Front);
        let back = eye_dir(StandardView::Back);
        close(front, DVec3::new(0.0, 0.0, mirror.z), 1e-9);
        close(back, DVec3::new(0.0, 0.0, -mirror.z), 1e-9);
    }
}

#[test]
fn perspective_view_round_trips_the_apparent_size() {
    let c = Camera {
        distance: 25.0,
        ..Camera::default()
    };
    let top = view_camera(&c, StandardView::Top);
    let Projection::Orthographic { height } = top.projection else {
        panic!()
    };
    // same visible height at the pivot as the perspective camera had
    assert!((height - c.world_per_pixel(1.0)).abs() < 1e-9);
    let p = view_camera(&top, StandardView::Perspective);
    assert!(matches!(p.projection, Projection::Perspective { .. }));
    assert!((p.distance - 25.0).abs() < 1e-9);
    assert_eq!((p.yaw, p.pitch), (DEFAULT_YAW, DEFAULT_PITCH));
    // already perspective: unchanged
    assert_eq!(view_camera(&c, StandardView::Perspective), c);
}
