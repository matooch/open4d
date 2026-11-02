//! Screen-size level of detail (docs/specs/VIEWPORT.md §10.6).
//! Ref: Rossignac & Borrel, "Multi-resolution 3D approximations for rendering complex scenes" (1993).

use forge_geometry::PolyMesh;
use glam::DVec3;

use crate::camera::{Camera, Projection};
use crate::cull::distance_to_aabb;

/// Projected diameter in pixels of a sphere of `radius` whose nearest point is `distance` away.
pub fn screen_diameter_px(radius: f64, distance: f64, cam: &Camera, viewport_h: f64) -> f64 {
    match cam.projection {
        Projection::Perspective { fov_y } => {
            2.0 * radius * viewport_h / (2.0 * distance.max(1e-9) * (fov_y / 2.0).tan())
        }
        Projection::Orthographic { height } => 2.0 * radius * viewport_h / height.max(1e-12),
    }
}

/// Whether a box of half-diagonal `radius` (the box `min..max`) is small enough on screen that
/// its proxy should be drawn. `min_px <= 0` never selects the proxy.
pub fn use_proxy(min: DVec3, max: DVec3, cam: &Camera, viewport_h: f64, min_px: f64) -> bool {
    if min_px <= 0.0 {
        return false;
    }
    let radius = (max - min).length() / 2.0;
    let d = distance_to_aabb(cam.eye_world(), min, max);
    screen_diameter_px(radius, d, cam, viewport_h) < min_px
}

/// Vertex-clustering decimation: the box of the mesh is cut into `cells`³ cells, the points of a
/// cell collapse to their mean, polygons are re-indexed, repeated corners merged and polygons with
/// fewer than three distinct corners dropped. Deterministic.
pub fn lod_proxy(mesh: &PolyMesh, cells: u32) -> PolyMesh {
    let cells = cells.max(1) as usize;
    let Some(first) = mesh.points.first().copied() else {
        return PolyMesh::default();
    };
    let (lo, hi) = mesh
        .points
        .iter()
        .fold((first, first), |(a, b), p| (a.min(*p), b.max(*p)));
    let size = (hi - lo).max(DVec3::splat(1e-12));
    let cell_of = |p: DVec3| {
        let t = (p - lo) / size * cells as f64;
        let c = |v: f64| (v as usize).min(cells - 1);
        (c(t.x) * cells + c(t.y)) * cells + c(t.z)
    };
    let mut sums: std::collections::BTreeMap<usize, (DVec3, f64)> = Default::default();
    let ids: Vec<usize> = mesh.points.iter().map(|p| cell_of(*p)).collect();
    for (p, id) in mesh.points.iter().zip(&ids) {
        let e = sums.entry(*id).or_insert((DVec3::ZERO, 0.0));
        e.0 += *p;
        e.1 += 1.0;
    }
    let mut index = std::collections::BTreeMap::new();
    let mut points = Vec::with_capacity(sums.len());
    for (id, (sum, n)) in &sums {
        index.insert(*id, points.len() as u32);
        points.push(*sum / *n);
    }
    let mut polys = Vec::new();
    for poly in &mesh.polys {
        let mut out: Vec<u32> = Vec::with_capacity(poly.len());
        for v in poly {
            let m = index[&ids[*v as usize]];
            if out.last() != Some(&m) {
                out.push(m);
            }
        }
        while out.len() > 1 && out.first() == out.last() {
            out.pop();
        }
        let mut distinct = out.clone();
        distinct.sort_unstable();
        distinct.dedup();
        if out.len() >= 3 && distinct.len() == out.len() {
            polys.push(out);
        }
    }
    PolyMesh { points, polys }
}
