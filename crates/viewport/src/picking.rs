//! Ray picking and rectangle selection (docs/specs/VIEWPORT.md §2).
//! Ref: Möller & Trumbore, JGT 2(1) (1997).

use forge_geometry::PolyMesh;
use forge_geometry::selection::{IndexSet, Mode};
use forge_geometry::triangulate::triangulate;
use glam::{DVec2, DVec3};

use crate::camera::{Camera, Projection, Ray};

/// Two-sided ray/triangle intersection; returns `t > 1e-12`.
pub fn ray_triangle(ray: &Ray, a: DVec3, b: DVec3, c: DVec3) -> Option<f64> {
    let (e1, e2) = (b - a, c - a);
    let p = ray.dir.cross(e2);
    let det = e1.dot(p);
    if det.abs() < 1e-15 {
        return None;
    }
    let inv = 1.0 / det;
    let s = ray.origin - a;
    let u = s.dot(p) * inv;
    if !(0.0..=1.0).contains(&u) {
        return None;
    }
    let q = s.cross(e1);
    let v = ray.dir.dot(q) * inv;
    if v < 0.0 || u + v > 1.0 {
        return None;
    }
    let t = e2.dot(q) * inv;
    (t > 1e-12).then_some(t)
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PolyHit {
    pub poly: u32,
    pub t: f64,
    pub point: DVec3,
}

type Tris = Vec<(u32, [u32; 3])>;

fn nearest_hit(mesh: &PolyMesh, tris: &Tris, ray: &Ray) -> Option<(u32, f64)> {
    let mut best: Option<(u32, f64)> = None;
    for (poly, t) in tris {
        let [a, b, c] = t.map(|i| mesh.points[i as usize]);
        if let Some(d) = ray_triangle(ray, a, b, c)
            && best.is_none_or(|(_, bt)| d < bt)
        {
            best = Some((*poly, d));
        }
    }
    best
}

pub fn pick_poly(mesh: &PolyMesh, ray: &Ray) -> Option<PolyHit> {
    nearest_hit(mesh, &triangulate(mesh), ray).map(|(poly, t)| PolyHit {
        poly,
        t,
        point: ray.at(t),
    })
}

/// True if a polygon lies between `p` and the camera.
fn occluded(mesh: &PolyMesh, tris: &Tris, cam: &Camera, p: DVec3) -> bool {
    let (dir, max_t) = match cam.projection {
        Projection::Perspective { .. } => {
            let to_eye = cam.eye_world() - p;
            (to_eye.normalize(), to_eye.length())
        }
        Projection::Orthographic { .. } => (-cam.forward_world(), f64::INFINITY),
    };
    let eps = 1e-6 * (cam.eye_world() - p).length().max(1.0);
    let ray = Ray { origin: p, dir };
    tris.iter().any(|(_, t)| {
        let [a, b, c] = t.map(|i| mesh.points[i as usize]);
        ray_triangle(&ray, a, b, c).is_some_and(|d| d > eps && d < max_t)
    })
}

fn closest_on_segment(p: DVec2, a: DVec2, b: DVec2) -> (f64, f64) {
    let ab = b - a;
    let len2 = ab.length_squared();
    let s = if len2 > 0.0 {
        ((p - a).dot(ab) / len2).clamp(0.0, 1.0)
    } else {
        0.0
    };
    (s, (a + ab * s - p).length())
}

/// Nearest point within `radius_px` of the cursor (see spec for tie-breaks).
#[allow(clippy::too_many_arguments)] // mesh, camera, cursor, viewport and options are all independent
pub fn pick_point(
    mesh: &PolyMesh,
    cam: &Camera,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    radius_px: f64,
    visible_only: bool,
) -> Option<u32> {
    let tris = triangulate(mesh);
    let cursor = DVec2::new(x, y);
    let mut best: Option<(f64, f64, u32)> = None;
    for (i, p) in mesh.points.iter().enumerate() {
        let Some(s) = cam.project(*p, w, h) else {
            continue;
        };
        let d = (s.truncate() - cursor).length();
        if d > radius_px || (visible_only && occluded(mesh, &tris, cam, *p)) {
            continue;
        }
        if best.is_none_or(|(bd, bz, _)| (d, s.z) < (bd, bz)) {
            best = Some((d, s.z, i as u32));
        }
    }
    best.map(|(_, _, i)| i)
}

/// Nearest edge (id from `PolyMesh::edges`) within `radius_px` of the cursor.
#[allow(clippy::too_many_arguments)] // see pick_point
pub fn pick_edge(
    mesh: &PolyMesh,
    cam: &Camera,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    radius_px: f64,
    visible_only: bool,
) -> Option<u32> {
    let tris = triangulate(mesh);
    let cursor = DVec2::new(x, y);
    let mut best: Option<(f64, f64, u32)> = None;
    for (id, e) in mesh.edges().iter().enumerate() {
        let (a, b) = (mesh.points[e.0 as usize], mesh.points[e.1 as usize]);
        let (Some(sa), Some(sb)) = (cam.project(a, w, h), cam.project(b, w, h)) else {
            continue;
        };
        let (s, d) = closest_on_segment(cursor, sa.truncate(), sb.truncate());
        if d > radius_px || (visible_only && occluded(mesh, &tris, cam, a.lerp(b, s))) {
            continue;
        }
        let depth = sa.z + (sb.z - sa.z) * s;
        if best.is_none_or(|(bd, bz, _)| (d, depth) < (bd, bz)) {
            best = Some((d, depth, id as u32));
        }
    }
    best.map(|(_, _, i)| i)
}

/// An edge under the cursor and where along it (docs/specs/MODELING.md §5.5).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EdgeHit {
    /// Edge id from `PolyMesh::edges`.
    pub edge: u32,
    /// Position along the edge from its lower to its higher point, in `[0, 1]`: the point of
    /// the edge's line closest to the cursor ray.
    pub t: f64,
}

/// Parameter on the line `a + t (b - a)` of the point closest to `ray`, clamped to `[0, 1]`.
pub fn closest_on_edge(ray: &Ray, a: DVec3, b: DVec3) -> f64 {
    let (d1, d2) = (ray.dir, b - a);
    let r = ray.origin - a;
    let (a11, a12, a22) = (d1.dot(d1), d1.dot(d2), d2.dot(d2));
    let det = a11 * a22 - a12 * a12;
    if det.abs() < 1e-18 {
        return 0.5;
    }
    // minimise |r + s d1 - t d2|^2 over s, t: [a11 -a12; -a12 a22][s t] = [-r.d1, r.d2]
    let (b1, b2) = (-r.dot(d1), r.dot(d2));
    ((a11 * b2 + a12 * b1) / det).clamp(0.0, 1.0)
}

/// [`pick_edge`] plus the position along the edge under the cursor.
#[allow(clippy::too_many_arguments)] // see pick_point
pub fn pick_edge_param(
    mesh: &PolyMesh,
    cam: &Camera,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    radius_px: f64,
    visible_only: bool,
) -> Option<EdgeHit> {
    let edge = pick_edge(mesh, cam, x, y, w, h, radius_px, visible_only)?;
    let e = mesh.edges()[edge as usize];
    let (a, b) = (mesh.points[e.0 as usize], mesh.points[e.1 as usize]);
    Some(EdgeHit {
        edge,
        t: closest_on_edge(&cam.ray(x, y, w, h), a, b),
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Element {
    Points,
    Edges,
    Polys,
}

/// Elements inside the pixel rectangle `[x0, y0, x1, y1]` (any corner order, inclusive).
#[allow(clippy::too_many_arguments)] // see pick_point
pub fn rect_select(
    mesh: &PolyMesh,
    cam: &Camera,
    rect: [f64; 4],
    w: f64,
    h: f64,
    element: Element,
    mode: Mode,
    visible_only: bool,
) -> IndexSet {
    let (x0, x1) = (rect[0].min(rect[2]), rect[0].max(rect[2]));
    let (y0, y1) = (rect[1].min(rect[3]), rect[1].max(rect[3]));
    let tris = triangulate(mesh);
    let inside: IndexSet = (0..mesh.points.len() as u32)
        .filter(|&i| {
            let p = mesh.points[i as usize];
            cam.project(p, w, h)
                .is_some_and(|s| (x0..=x1).contains(&s.x) && (y0..=y1).contains(&s.y))
                && !(visible_only && occluded(mesh, &tris, cam, p))
        })
        .collect();
    match element {
        Element::Points => inside,
        Element::Edges => forge_geometry::selection::points_to_edges(mesh, &inside, mode),
        Element::Polys => forge_geometry::selection::points_to_polys(mesh, &inside, mode),
    }
}

/// The clone hit by [`pick_instances`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct InstanceHit {
    pub clone: u32,
    pub proto: u32,
    pub poly: u32,
    /// World ray parameter.
    pub t: f64,
}

/// Slab test of the ray against a box; the entry/exit parameters bracket a hit.
fn ray_hits_box(origin: DVec3, dir: DVec3, lo: DVec3, hi: DVec3) -> bool {
    let (mut t0, mut t1) = (f64::NEG_INFINITY, f64::INFINITY);
    for k in 0..3 {
        if dir[k].abs() < 1e-300 {
            if origin[k] < lo[k] || origin[k] > hi[k] {
                return false;
            }
            continue;
        }
        let (a, b) = ((lo[k] - origin[k]) / dir[k], (hi[k] - origin[k]) / dir[k]);
        t0 = t0.max(a.min(b));
        t1 = t1.min(a.max(b));
    }
    t0 <= t1 && t1 >= 0.0
}

/// Nearest clone under the ray, without expanding the geometry (docs/specs/VIEWPORT.md §10.7).
/// Clone `i` shows `prototypes[input.child[i]]` placed by `world · input.matrices[i]`.
pub fn pick_instances(
    prototypes: &[PolyMesh],
    input: &crate::instancing::InstanceInput,
    world: &glam::DMat4,
    ray: &Ray,
) -> Option<InstanceHit> {
    let tris: Vec<Tris> = prototypes.iter().map(triangulate).collect();
    let boxes: Vec<Option<(DVec3, DVec3)>> = prototypes
        .iter()
        .map(|m| {
            let first = *m.points.first()?;
            Some(
                m.points
                    .iter()
                    .fold((first, first), |(a, b), p| (a.min(*p), b.max(*p))),
            )
        })
        .collect();
    let mut best: Option<InstanceHit> = None;
    for i in 0..input.len() {
        if !input.visible.get(i).copied().unwrap_or(true) {
            continue;
        }
        let proto = input.child.get(i).copied().unwrap_or(0) as usize;
        let Some(Some((lo, hi))) = boxes.get(proto) else {
            continue;
        };
        let m = *world * input.matrices[i];
        if m.determinant().abs() < 1e-18 {
            continue;
        }
        let inv = m.inverse();
        // unnormalised direction: `t` stays the world ray parameter
        let local = Ray {
            origin: inv.transform_point3(ray.origin),
            dir: inv.transform_vector3(ray.dir),
        };
        if !ray_hits_box(local.origin, local.dir, *lo, *hi) {
            continue;
        }
        if let Some((poly, t)) = nearest_hit(&prototypes[proto], &tris[proto], &local)
            && best.is_none_or(|b| t < b.t)
        {
            best = Some(InstanceHit {
                clone: i as u32,
                proto: proto as u32,
                poly,
                t,
            });
        }
    }
    best
}
