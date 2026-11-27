//! Look through camera and the object gizmos (docs/specs/SCENE-OBJECTS.md S7).

use forge_viewport::scene_view::*;
use glam::{DMat3, DMat4, DVec3};

/// The render API's pixel mapping (RENDER.md R10) for a camera of `world`, 36 mm sensor:
/// `px = w/2 + f_px * x/z`, `py = h/2 - f_px * y/z` with `f_px = w * focal / sensor`.
fn formula_pixel(world: DMat4, focal: f64, sensor: f64, p: DVec3, w: f64, h: f64) -> (f64, f64) {
    let c = world.inverse().transform_point3(p);
    let f = w * focal / sensor;
    (w * 0.5 + f * c.x / c.z, h * 0.5 - f * c.y / c.z)
}

fn hpb(pos: DVec3, h: f64, p: f64) -> DMat4 {
    // the node transform of the scene: Ry(-h) Rx(-p) (core transform.rs)
    DMat4::from_mat3_translation(DMat3::from_rotation_y(-h) * DMat3::from_rotation_x(-p), pos)
}

#[test]
fn the_viewport_camera_projects_like_the_formula() {
    let (w, h) = (320.0, 240.0);
    for (focal, hd, pd) in [
        (36.0, 0.0, 0.0),
        (18.0, -15.0, 0.0),
        (50.0, 30.0, 10.0),
        (100.0, 0.0, -20.0),
    ] {
        let world = hpb(
            DVec3::new(30.0, 40.0, -600.0),
            f64::to_radians(hd),
            f64::to_radians(pd),
        );
        let cam = look_through(
            world,
            Lens::Perspective {
                focal_mm: focal,
                sensor_mm: 36.0,
            },
            w / h,
            500.0,
        );
        // points in front of the camera, off axis
        for q in [
            DVec3::new(0.0, 0.0, 0.0),
            DVec3::new(100.0, -50.0, 50.0),
            DVec3::new(-80.0, 60.0, 10.0),
        ] {
            let ahead = world.inverse().transform_point3(q).z > 0.0;
            if !ahead {
                continue;
            }
            let want = formula_pixel(world, focal, 36.0, q, w, h);
            let got = cam.project(q, w, h).expect("in front");
            assert!(
                (got.x - want.0).abs() < 1e-6 && (got.y - want.1).abs() < 1e-6,
                "f={focal} h={hd} p={pd}: {got:?} vs {want:?}"
            );
        }
    }
}

#[test]
fn the_eye_and_view_axis_follow_the_object() {
    let world = hpb(DVec3::new(0.0, 100.0, -600.0), (-15f64).to_radians(), 0.0);
    let cam = look_through(
        world,
        Lens::Perspective {
            focal_mm: 36.0,
            sensor_mm: 36.0,
        },
        4.0 / 3.0,
        400.0,
    );
    assert!((cam.eye_world() - world.w_axis.truncate()).length() < 1e-9);
    let fwd = world.transform_vector3(DVec3::Z);
    assert!((cam.forward_world() - fwd).length() < 1e-9);
}

#[test]
fn orthographic_lens_shows_the_lens_width() {
    let world = DMat4::from_translation(DVec3::new(0.0, 0.0, -600.0));
    let cam = look_through(
        world,
        Lens::Orthographic { width: 1000.0 },
        320.0 / 240.0,
        600.0,
    );
    // 1000 units across 320 px: a point 100 units right of the axis is 32 px right of the centre
    let p = cam
        .project(DVec3::new(100.0, 0.0, 0.0), 320.0, 240.0)
        .unwrap();
    assert!(
        (p.x - 192.0).abs() < 1e-6 && (p.y - 120.0).abs() < 1e-6,
        "{p:?}"
    );
}

#[test]
fn camera_gizmo_has_the_frustum() {
    let world = DMat4::from_translation(DVec3::new(0.0, 0.0, -100.0));
    let lens = Lens::Perspective {
        focal_mm: 36.0,
        sensor_mm: 36.0,
    };
    let lines = camera_lines(world, lens, 2.0, 100.0, false);
    // 4 frame edges, 4 side edges, 2 up marker edges
    assert_eq!(lines.len(), 10);
    // the frame sits `depth` ahead and is 100 wide (36 mm sensor, 36 mm lens) by 50 high
    let far = &lines[..4];
    assert!(far.iter().all(|l| l.a.z.abs() < 1e-9 && l.b.z.abs() < 1e-9));
    let xs: Vec<f64> = far.iter().flat_map(|l| [l.a.x, l.b.x]).collect();
    assert!((xs.iter().cloned().fold(f64::MIN, f64::max) - 50.0).abs() < 1e-9);
    let sel = camera_lines(world, lens, 2.0, 100.0, true);
    assert_eq!(sel[0].color, SELECTED_COLOR);
}

#[test]
fn light_gizmos_are_closed_and_oriented() {
    let w = DMat4::IDENTITY;
    let omni = light_lines(w, LightGizmo::Omni { radius: 10.0 }, false);
    assert_eq!(omni.len(), 72);
    for l in &omni {
        assert!((l.a.length() - 10.0).abs() < 1e-9);
    }
    let spot = light_lines(
        w,
        LightGizmo::Spot {
            outer: std::f64::consts::FRAC_PI_2,
            length: 50.0,
        },
        false,
    );
    // a 90 degree cone is as wide as it is long
    assert!(
        spot.iter()
            .any(|l| (l.b.x - 50.0).abs() < 1e-9 && (l.b.z - 50.0).abs() < 1e-9)
    );
    let rect = light_lines(
        w,
        LightGizmo::AreaRect {
            width: 200.0,
            height: 100.0,
        },
        false,
    );
    assert_eq!(rect.len(), 7);
    assert!(
        rect.iter().any(|l| (l.b.z - 50.0).abs() < 1e-9),
        "direction arrow toward +Z"
    );
    assert!(
        !light_lines(
            w,
            LightGizmo::Infinite {
                radius: 5.0,
                length: 20.0
            },
            true
        )
        .is_empty()
    );
    assert!(!light_lines(w, LightGizmo::AreaDisc { radius: 5.0 }, true).is_empty());
    assert!(!light_lines(w, LightGizmo::AreaSphere { radius: 5.0 }, true).is_empty());
    assert_eq!(floor_lines(w, 100.0, 4, OBJECT_COLOR).len(), 10);
}
