//! docs/specs/VIEWPORT.md §3–4.

mod common;

use common::{H, W, close, unit_box};
use forge_viewport::camera::{Camera, Ray};
use forge_viewport::grid::{LineKind, grid_lines};
use forge_viewport::snap::{
    line_param_closest_to_ray, ray_plane, snap_grid, snap_scalar, snap_to_point,
};
use glam::DVec3;

#[test]
fn scalar_and_grid_snapping() {
    assert_eq!(snap_scalar(0.26, 0.25), 0.25);
    assert_eq!(snap_scalar(-0.38, 0.25), -0.5);
    assert_eq!(snap_scalar(0.26, 0.0), 0.26);
    assert_eq!(snap_scalar(0.26, -1.0), 0.26);
    assert_eq!(
        snap_grid(DVec3::new(1.4, -2.6, 0.05), 0.5),
        DVec3::new(1.5, -2.5, 0.0)
    );
}

#[test]
fn ray_plane_intersection() {
    let r = Ray {
        origin: DVec3::new(0., 5., 0.),
        dir: DVec3::NEG_Y,
    };
    assert_eq!(ray_plane(&r, DVec3::ZERO, DVec3::Y), Some(5.0));
    let flat = Ray {
        origin: DVec3::new(0., 5., 0.),
        dir: DVec3::X,
    };
    assert_eq!(ray_plane(&flat, DVec3::ZERO, DVec3::Y), None);
    let away = Ray {
        origin: DVec3::new(0., 5., 0.),
        dir: DVec3::Y,
    };
    assert_eq!(ray_plane(&away, DVec3::ZERO, DVec3::Y), None);
}

#[test]
fn closest_line_parameter() {
    let r = Ray {
        origin: DVec3::new(3., 5., 1.),
        dir: DVec3::NEG_Y,
    };
    assert!((line_param_closest_to_ray(DVec3::ZERO, DVec3::X, &r).unwrap() - 3.0).abs() < 1e-12);
    // scaled direction: parameter is in units of `dir`
    assert!(
        (line_param_closest_to_ray(DVec3::ZERO, DVec3::X * 2.0, &r).unwrap() - 1.5).abs() < 1e-12
    );
    let parallel = Ray {
        origin: DVec3::new(0., 1., 0.),
        dir: DVec3::X,
    };
    assert_eq!(
        line_param_closest_to_ray(DVec3::ZERO, DVec3::X, &parallel),
        None
    );
}

#[test]
fn vertex_snap_returns_world_position() {
    let m = unit_box();
    let c = Camera::default();
    let p = c.project(DVec3::new(-1., 1., 1.), W, H).unwrap();
    close(
        snap_to_point(&m, &c, p.x + 1.0, p.y, W, H, 4.0).unwrap(),
        DVec3::new(-1., 1., 1.),
        1e-12,
    );
    assert!(snap_to_point(&m, &c, 3.0, 3.0, W, H, 4.0).is_none());
}

#[test]
fn grid_spacing_extent_and_kinds() {
    let c = Camera::default(); // 10 px ≈ 0.138 world units -> spacing 1
    let g = grid_lines(&c, H);
    assert_eq!(g.spacing, 1.0);
    assert_eq!(g.lines.len(), 202);
    let first = &g.lines[0];
    close(first.a, DVec3::new(-50., 0., -50.), 1e-12);
    close(first.b, DVec3::new(50., 0., -50.), 1e-12);
    assert_eq!(first.kind, LineKind::Major);
    let at_z = |z: f64| g.lines[..101].iter().find(|l| l.a.z == z).unwrap().kind;
    assert_eq!(at_z(0.0), LineKind::AxisX);
    assert_eq!(at_z(10.0), LineKind::Major);
    assert_eq!(at_z(3.0), LineKind::Minor);
    let along_z = &g.lines[101];
    close(along_z.a, DVec3::new(-50., 0., -50.), 1e-12);
    close(along_z.b, DVec3::new(-50., 0., 50.), 1e-12);
    assert_eq!(g.lines[151].kind, LineKind::AxisZ);
}

#[test]
fn grid_adapts_to_zoom_and_follows_target() {
    let far = Camera {
        distance: 1000.0,
        ..Camera::default()
    };
    assert_eq!(grid_lines(&far, H).spacing, 100.0);
    let c = Camera {
        target: DVec3::new(1234.0, 0.0, -567.0),
        ..Camera::default()
    };
    let g = grid_lines(&c, H);
    assert_eq!(g.lines[0].a.z, -620.0);
    assert_eq!(g.lines[100].a.z, -520.0);
    assert_eq!(g.lines[101].a.x, 1180.0);
    assert!(
        g.lines
            .iter()
            .all(|l| !matches!(l.kind, LineKind::AxisX | LineKind::AxisZ))
    );
    let _ = W;
}
