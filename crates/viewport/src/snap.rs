//! Snapping and constraint math (docs/specs/VIEWPORT.md §3).
//! Ref: Ericson, *Real-Time Collision Detection* (2005) §5.1.8, §5.3.

use forge_geometry::PolyMesh;
use glam::DVec3;

use crate::camera::{Camera, Ray};
use crate::picking::pick_point;

pub fn snap_scalar(v: f64, inc: f64) -> f64 {
    if inc > 0.0 {
        (v / inc).round() * inc
    } else {
        v
    }
}

pub fn snap_grid(p: DVec3, spacing: f64) -> DVec3 {
    DVec3::new(
        snap_scalar(p.x, spacing),
        snap_scalar(p.y, spacing),
        snap_scalar(p.z, spacing),
    )
}

/// Ray parameter of the hit with the plane, if in front of the ray.
pub fn ray_plane(ray: &Ray, point: DVec3, normal: DVec3) -> Option<f64> {
    let denom = ray.dir.dot(normal);
    if denom.abs() < 1e-12 {
        return None;
    }
    let t = (point - ray.origin).dot(normal) / denom;
    (t >= 0.0).then_some(t)
}

/// Parameter `s` on `origin + dir·s` closest to the ray's line; `None` if parallel.
pub fn line_param_closest_to_ray(origin: DVec3, dir: DVec3, ray: &Ray) -> Option<f64> {
    let r = origin - ray.origin;
    let (a, b, e) = (dir.dot(dir), dir.dot(ray.dir), ray.dir.dot(ray.dir));
    let (c, f) = (dir.dot(r), ray.dir.dot(r));
    let denom = a * e - b * b;
    if a == 0.0 || denom / (a * e) < 1e-12 {
        return None;
    }
    Some((b * f - c * e) / denom)
}

/// World position of the visible vertex nearest the cursor within `radius_px`.
pub fn snap_to_point(
    mesh: &PolyMesh,
    cam: &Camera,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    radius_px: f64,
) -> Option<DVec3> {
    pick_point(mesh, cam, x, y, w, h, radius_px, true).map(|i| mesh.points[i as usize])
}
