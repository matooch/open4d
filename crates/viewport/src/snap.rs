//! Snapping and constraint math (docs/specs/VIEWPORT.md §3).
//! Ref: Ericson, *Real-Time Collision Detection* (2005) §5.1.8, §5.3.

use forge_geometry::PolyMesh;
use forge_geometry::triangulate::triangulate;
use forge_modeling::snap::{
    Candidate, SnapHit, SnapKind, SnapSettings, SnapSource, best, point_targets,
};
use glam::{DVec2, DVec3};

use crate::camera::{Camera, Projection, Ray};
use crate::grid::grid_spacing;
use crate::picking::{closest_on_edge, pick_point, ray_triangle};

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

type Tris = Vec<(u32, [u32; 3])>;

/// True if a polygon of any source lies between `p` and the camera.
fn hidden(sources: &[SnapSource], tris: &[Tris], cam: &Camera, p: DVec3) -> bool {
    let (dir, max_t) = match cam.projection {
        Projection::Perspective { .. } => {
            let to_eye = cam.eye_world() - p;
            (to_eye.normalize(), to_eye.length())
        }
        Projection::Orthographic { .. } => (-cam.forward_world(), f64::INFINITY),
    };
    let eps = 1e-6 * (cam.eye_world() - p).length().max(1.0);
    let ray = Ray { origin: p, dir };
    sources.iter().zip(tris).any(|(s, tris)| {
        tris.iter().any(|(_, t)| {
            let [a, b, c] = t.map(|i| s.mesh.points[i as usize]);
            ray_triangle(&ray, a, b, c).is_some_and(|d| d > eps && d < max_t)
        })
    })
}

/// Resolves the cursor `(x, y)` (pixels of a `w x h` viewport) against the enabled targets of
/// `sources` (docs/specs/MANIPULATE.md §6.3). `settings.enabled` is not looked at.
pub fn snap_cursor(
    sources: &[SnapSource],
    cam: &Camera,
    cursor: (f64, f64),
    size: (f64, f64),
    settings: &SnapSettings,
) -> Option<SnapHit> {
    let (x, y) = cursor;
    let (w, h) = size;
    let t = &settings.targets;
    let radius = settings.radius;
    let cur = DVec2::new(x, y);
    let ray = cam.ray(x, y, w, h);
    let need_tris = t.polygon || settings.visible_only;
    let tris: Vec<Tris> = sources
        .iter()
        .map(|s| {
            if need_tris {
                triangulate(&s.mesh)
            } else {
                Vec::new()
            }
        })
        .collect();
    let mut cands: Vec<Candidate> = Vec::new();
    // the screen distance and depth of a world position, if it is in reach and visible
    let reach = |p: DVec3, check_hidden: bool| -> Option<(f64, f64)> {
        let s = cam.project(p, w, h)?;
        let d = (s.truncate() - cur).length();
        if d > radius || (check_hidden && settings.visible_only && hidden(sources, &tris, cam, p)) {
            return None;
        }
        Some((d, s.z))
    };
    for (kind, p, si, idx) in point_targets(sources, settings) {
        // an object axis lies inside its mesh: it is not hidden by it
        let check = kind != SnapKind::ObjectAxis;
        if let Some((distance, depth)) = reach(p, check) {
            cands.push(Candidate {
                kind,
                position: p,
                distance,
                depth,
                source: Some(si),
                index: idx,
            });
        }
    }
    if t.edge {
        for (si, s) in sources.iter().enumerate() {
            for (i, e) in s.mesh.edges().iter().enumerate() {
                if s.skips(e.0) || s.skips(e.1) {
                    continue;
                }
                let (a, b) = (s.mesh.points[e.0 as usize], s.mesh.points[e.1 as usize]);
                let p = a.lerp(b, closest_on_edge(&ray, a, b));
                if let Some((distance, depth)) = reach(p, true) {
                    cands.push(Candidate {
                        kind: SnapKind::Edge,
                        position: p,
                        distance,
                        depth,
                        source: Some(si),
                        index: Some(i as u32),
                    });
                }
            }
        }
    }
    if t.polygon {
        let mut nearest: Option<(f64, usize, u32)> = None;
        for (si, (s, tris)) in sources.iter().zip(&tris).enumerate() {
            for (poly, tri) in tris {
                if s.skips_any(&s.mesh.polys[*poly as usize]) {
                    continue;
                }
                let [a, b, c] = tri.map(|i| s.mesh.points[i as usize]);
                if let Some(d) = ray_triangle(&ray, a, b, c)
                    && nearest.is_none_or(|(bd, _, _)| d < bd)
                {
                    nearest = Some((d, si, *poly));
                }
            }
        }
        if let Some((d, si, poly)) = nearest {
            let p = ray.at(d);
            cands.push(Candidate {
                kind: SnapKind::Polygon,
                position: p,
                distance: 0.0,
                depth: cam.project(p, w, h).map_or(0.0, |s| s.z),
                source: Some(si),
                index: Some(poly),
            });
        }
    }
    let wp = &settings.workplane;
    if (t.grid_point || t.grid_line || t.workplane)
        && let Some(d) = ray_plane(&ray, wp.origin, wp.normal)
    {
        let hit = ray.at(d);
        let spacing = settings.spacing_or(grid_spacing(cam, h));
        let mut push = |kind, p: DVec3, in_reach: Option<(f64, f64)>| {
            if let Some((distance, depth)) = in_reach {
                cands.push(Candidate {
                    kind,
                    position: p,
                    distance,
                    depth,
                    source: None,
                    index: None,
                });
            }
        };
        if t.workplane {
            let depth = cam.project(hit, w, h).map_or(0.0, |s| s.z);
            push(SnapKind::Workplane, hit, Some((0.0, depth)));
        }
        if t.grid_point {
            let p = wp.grid_point(hit, spacing);
            push(SnapKind::GridPoint, p, reach(p, false));
        }
        if t.grid_line {
            let p = wp.grid_line(hit, spacing);
            push(SnapKind::GridLine, p, reach(p, false));
        }
    }
    best(&cands, settings)
}
