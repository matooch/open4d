//! docs/specs/VIEWPORT.md §1.

mod common;

use std::f64::consts::{FRAC_PI_2, FRAC_PI_4};

use common::{H, W, close};
use forge_viewport::camera::{AxisView, Camera, Projection};
use glam::DVec3;

#[test]
fn default_looks_down_negative_z() {
    let c = Camera::default();
    close(c.eye(), DVec3::new(0., 0., 10.), 1e-12);
    close(c.forward(), DVec3::NEG_Z, 1e-12);
    close(c.right(), DVec3::X, 1e-12);
    close(c.up(), DVec3::Y, 1e-12);
    assert_eq!(c.projection, Projection::Perspective { fov_y: FRAC_PI_4 });
}

#[test]
fn eye_follows_yaw_and_pitch() {
    let mut c = Camera::default();
    c.orbit(FRAC_PI_2, 0.0);
    close(c.eye(), DVec3::new(10., 0., 0.), 1e-9);
    c.orbit(-FRAC_PI_2, FRAC_PI_4);
    let s = 10.0 * FRAC_PI_4.sin();
    close(c.eye(), DVec3::new(0., s, s), 1e-9);
}

#[test]
fn pitch_is_clamped_short_of_the_poles() {
    let mut c = Camera::default();
    c.orbit(0.0, 10.0);
    assert!((c.pitch - (FRAC_PI_2 - 1e-4)).abs() < 1e-15);
    c.orbit(0.0, -20.0);
    assert!((c.pitch + (FRAC_PI_2 - 1e-4)).abs() < 1e-15);
    assert!(c.right().is_finite() && c.up().is_finite());
}

#[test]
fn project_and_ray_are_inverse() {
    for projection in [
        Projection::Perspective { fov_y: 0.8 },
        Projection::Orthographic { height: 7.0 },
    ] {
        let mut c = Camera {
            projection,
            ..Camera::default()
        };
        c.orbit(0.4, -0.3);
        c.target = DVec3::new(1., 2., -3.);
        // the target projects to the viewport centre
        let t = c.project(c.target, W, H).unwrap();
        close(
            DVec3::new(t.x, t.y, 0.0),
            DVec3::new(W / 2.0, H / 2.0, 0.0),
            1e-9,
        );
        assert!((0.0..=1.0).contains(&t.z));
        for (x, y) in [(10.0, 20.0), (400.0, 300.0), (799.0, 1.0)] {
            let r = c.ray(x, y, W, H);
            assert!((r.dir.length() - 1.0).abs() < 1e-12);
            let p = c.project(r.origin + r.dir * 5.0, W, H).unwrap();
            close(DVec3::new(p.x, p.y, 0.0), DVec3::new(x, y, 0.0), 1e-6);
        }
    }
}

#[test]
fn points_behind_the_camera_do_not_project() {
    let c = Camera::default();
    assert!(c.project(DVec3::new(0., 0., 20.), W, H).is_none());
}

#[test]
fn screen_y_grows_downward() {
    let c = Camera::default();
    let above = c.project(DVec3::new(0., 1., 0.), W, H).unwrap();
    assert!(above.y < H / 2.0);
    let right = c.project(DVec3::new(1., 0., 0.), W, H).unwrap();
    assert!(right.x > W / 2.0);
}

#[test]
fn pan_keeps_content_under_the_cursor() {
    for projection in [
        Projection::Perspective { fov_y: 0.8 },
        Projection::Orthographic { height: 7.0 },
    ] {
        let mut c = Camera {
            projection,
            ..Camera::default()
        };
        c.orbit(0.3, 0.2);
        let before = c.project(c.target, W, H).unwrap();
        let world_target = c.target;
        c.pan(30.0, -12.0, H);
        let after = c.project(world_target, W, H).unwrap();
        close(
            DVec3::new(after.x - before.x, after.y - before.y, 0.),
            DVec3::new(30., -12., 0.),
            1e-6,
        );
    }
}

#[test]
fn world_per_pixel() {
    let c = Camera::default();
    let want = 2.0 * 10.0 * (FRAC_PI_4 / 2.0).tan() / H;
    assert!((c.world_per_pixel(H) - want).abs() < 1e-15);
    let o = Camera {
        projection: Projection::Orthographic { height: 6.0 },
        ..Camera::default()
    };
    assert_eq!(o.world_per_pixel(H), 0.01);
}

#[test]
fn dolly_scales_distance_or_height() {
    let mut c = Camera::default();
    c.dolly(0.5);
    assert_eq!(c.distance, 5.0);
    c.dolly(0.0);
    c.dolly(-2.0);
    assert_eq!(c.distance, 5.0);
    c.dolly(1e-12);
    assert_eq!(c.distance, 1e-6);
    let mut o = Camera {
        projection: Projection::Orthographic { height: 4.0 },
        ..Camera::default()
    };
    o.dolly(2.0);
    assert_eq!(o.projection, Projection::Orthographic { height: 8.0 });
}

#[test]
fn frame_fits_the_bounding_sphere() {
    let mut c = Camera::default();
    c.orbit(0.7, 0.3);
    let (lo, hi) = (DVec3::new(-1., 0., 2.), DVec3::new(3., 2., 6.));
    c.frame(lo, hi);
    close(c.target, DVec3::new(1., 1., 4.), 1e-12);
    let r = (hi - lo).length() / 2.0;
    assert!((c.distance - r / (FRAC_PI_4 / 2.0).sin()).abs() < 1e-12);
    assert!((c.yaw - 0.7).abs() < 1e-15);
    let mut o = Camera {
        projection: Projection::Orthographic { height: 1.0 },
        ..Camera::default()
    };
    o.frame(lo, hi);
    assert_eq!(o.projection, Projection::Orthographic { height: 2.0 * r });
}

#[test]
fn axis_views_place_the_eye_on_that_side() {
    let cases = [
        (AxisView::PosX, DVec3::X),
        (AxisView::NegX, DVec3::NEG_X),
        (AxisView::PosZ, DVec3::Z),
        (AxisView::NegZ, DVec3::NEG_Z),
        (AxisView::PosY, DVec3::Y),
        (AxisView::NegY, DVec3::NEG_Y),
    ];
    for (view, side) in cases {
        let mut c = Camera {
            target: DVec3::new(1., 1., 1.),
            ..Camera::default()
        };
        c.look_along(view);
        let dir = (c.eye() - c.target).normalize();
        assert!(dir.dot(side) > 0.999_999, "{view:?}: {dir}");
    }
}

// ---- docs/specs/SMOOTHING.md §4: left-handed display ----

use forge_viewport::camera::Handedness;

fn left() -> Camera {
    Camera {
        handedness: Handedness::Left,
        ..Camera::default()
    }
}

#[test]
fn left_handed_front_view_shows_x_right_y_up_looking_along_z() {
    let mut c = left();
    c.look_along(AxisView::NegZ); // eye on world -Z, looking along +Z
    assert!(c.eye_world().z < 0.0);
    assert!(c.forward_world().z > 0.999);
    let o = c.project(DVec3::ZERO, W, H).unwrap();
    let x = c.project(DVec3::X, W, H).unwrap();
    let y = c.project(DVec3::Y, W, H).unwrap();
    assert!(x.x > o.x + 1.0 && (x.y - o.y).abs() < 1e-6, "+X right");
    assert!(y.y < o.y - 1.0, "+Y up");
    // the same view in a right-handed camera is mirrored
    let mut r = Camera::default();
    r.look_along(AxisView::NegZ);
    assert!(r.project(DVec3::X, W, H).unwrap().x < W / 2.0);
}

#[test]
fn editor_default_pose_matches_the_reference_directions() {
    let c = Camera {
        yaw: 0.68,
        pitch: 0.52,
        distance: 1000.0,
        ..left()
    };
    let o = c.project(DVec3::ZERO, W, H).unwrap();
    let x = c.project(DVec3::new(200.0, 0.0, 0.0), W, H).unwrap();
    let y = c.project(DVec3::new(0.0, 200.0, 0.0), W, H).unwrap();
    let z = c.project(DVec3::new(0.0, 0.0, 200.0), W, H).unwrap();
    assert!(x.x > o.x && x.y > o.y, "+X right and slightly down");
    assert!(x.z < o.z, "+X nearer");
    assert!(y.y < o.y && (y.x - o.x).abs() < 1.0, "+Y straight up");
    assert!(z.x > o.x && z.y < o.y, "+Z up-right");
    assert!(z.z > o.z, "+Z farther");
    assert!(c.eye_world().y > 0.0, "seen from above");
}

#[test]
fn left_handed_project_ray_pan_and_world_vectors() {
    for projection in [
        Projection::Perspective { fov_y: 0.8 },
        Projection::Orthographic { height: 7.0 },
    ] {
        let mut c = Camera {
            projection,
            ..left()
        };
        c.orbit(0.4, -0.3);
        c.target = DVec3::new(1., 2., -3.);
        let t = c.project(c.target_world(), W, H).unwrap();
        close(
            DVec3::new(t.x, t.y, 0.0),
            DVec3::new(W / 2.0, H / 2.0, 0.0),
            1e-9,
        );
        let ahead = c
            .project(c.target_world() + c.forward_world(), W, H)
            .unwrap();
        close(
            DVec3::new(ahead.x, ahead.y, 0.0),
            DVec3::new(W / 2.0, H / 2.0, 0.0),
            1e-6,
        );
        let r = c.project(c.target_world() + c.right_world(), W, H).unwrap();
        let u = c.project(c.target_world() + c.up_world(), W, H).unwrap();
        assert!(r.x > t.x && u.y < t.y);
        for (x, y) in [(10.0, 20.0), (400.0, 300.0)] {
            let ray = c.ray(x, y, W, H);
            let p = c.project(ray.origin + ray.dir * 5.0, W, H).unwrap();
            close(DVec3::new(p.x, p.y, 0.0), DVec3::new(x, y, 0.0), 1e-6);
        }
        // a point at the target's depth follows the cursor exactly
        let anchor = c.target_world();
        let before = c.project(anchor, W, H).unwrap();
        c.pan(30.0, -12.0, H);
        let after = c.project(anchor, W, H).unwrap();
        close(
            DVec3::new(after.x - before.x, after.y - before.y, 0.),
            DVec3::new(30., -12., 0.),
            1e-6,
        );
    }
    let mut f = left();
    f.frame(DVec3::new(0., 0., 4.), DVec3::new(2., 2., 6.));
    close(f.target_world(), DVec3::new(1., 1., 5.), 1e-12);
}
