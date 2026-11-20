//! Polygon meshes, polylines and points to signed distance fields (VOLUME.md sections 2, 3).

use std::collections::HashSet;

use forge_geometry::PolyMesh;
use glam::{DVec3, IVec3};

use crate::bvh::{Bvh, triangles};
use crate::grid::SparseGrid;

/// Lattice and band of a distance field.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SdfParams {
    pub voxel_size: f64,
    pub origin: DVec3,
    /// Exterior band width in voxels.
    pub exterior_range: f64,
    /// Interior band width in voxels.
    pub interior_range: f64,
}

impl Default for SdfParams {
    fn default() -> Self {
        SdfParams {
            voxel_size: 10.0,
            origin: DVec3::ZERO,
            exterior_range: 2.0,
            interior_range: 2.0,
        }
    }
}

impl SdfParams {
    pub fn exterior_band(&self) -> f64 {
        self.exterior_range * self.voxel_size
    }

    pub fn interior_band(&self) -> f64 {
        self.interior_range * self.voxel_size
    }

    /// An empty grid on this lattice with the exterior band as background.
    pub fn empty_grid(&self) -> SparseGrid<f32> {
        SparseGrid::new(self.exterior_band() as f32, self.voxel_size, self.origin)
    }
}

/// Index box covering a world box grown by `margin`.
fn index_box(p: &SdfParams, lo: DVec3, hi: DVec3, margin: f64) -> (IVec3, IVec3) {
    let a = ((lo - DVec3::splat(margin) - p.origin) / p.voxel_size).floor();
    let b = ((hi + DVec3::splat(margin) - p.origin) / p.voxel_size).ceil();
    (a.as_ivec3(), b.as_ivec3())
}

fn collect_box(set: &mut HashSet<IVec3>, lo: IVec3, hi: IVec3) {
    for x in lo.x..=hi.x {
        for y in lo.y..=hi.y {
            for z in lo.z..=hi.z {
                set.insert(IVec3::new(x, y, z));
            }
        }
    }
}

fn sorted(set: HashSet<IVec3>) -> Vec<IVec3> {
    let mut v: Vec<IVec3> = set.into_iter().collect();
    v.sort_by_key(|i| (i.x, i.y, i.z));
    v
}

/// Samples at most this close to the surface (relative to the voxel size) are on it.
const ON_SURFACE: f64 = 1e-9;

/// Mesh to signed distance field (VOLUME.md section 2). `mesh` is in world space.
pub fn mesh_to_sdf(mesh: &PolyMesh, params: &SdfParams) -> SparseGrid<f32> {
    let tris = triangles(&mesh.points, &mesh.polys);
    let band = params.exterior_band().max(params.interior_band());
    let mut cand = HashSet::new();
    for t in &tris {
        let lo = t.a.min(t.b).min(t.c);
        let hi = t.a.max(t.b).max(t.c);
        let (a, b) = index_box(params, lo, hi, band);
        collect_box(&mut cand, a, b);
    }
    let bvh = Bvh::new(tris);
    let mut grid = params.empty_grid();
    let (ext, int) = (params.exterior_band(), params.interior_band());
    for idx in sorted(cand) {
        let p = grid.index_to_world(idx);
        let Some(d) = bvh.distance_within(p, band) else {
            continue;
        };
        if d <= ON_SURFACE * params.voxel_size {
            grid.set(idx, 0.0);
            continue;
        }
        let inside = bvh.winding_number(p).abs() > 0.5;
        if inside && d <= int {
            grid.set(idx, -(d as f32));
        } else if !inside && d <= ext {
            grid.set(idx, d as f32);
        }
    }
    grid.signed_flood_fill(int as f32);
    grid
}

fn segment_distance(a: DVec3, b: DVec3, p: DVec3) -> f64 {
    let ab = b - a;
    let l2 = ab.length_squared();
    let t = if l2 > 0.0 {
        ((p - a).dot(ab) / l2).clamp(0.0, 1.0)
    } else {
        0.0
    };
    (p - (a + ab * t)).length()
}

fn finish(mut grid: SparseGrid<f32>, params: &SdfParams) -> SparseGrid<f32> {
    grid.signed_flood_fill(params.interior_band() as f32);
    grid
}

fn put(grid: &mut SparseGrid<f32>, params: &SdfParams, idx: IVec3, value: f64) {
    if value >= 0.0 && value <= params.exterior_band()
        || value < 0.0 && -value <= params.interior_band()
    {
        grid.set(idx, value as f32);
    }
}

/// Polyline to signed distance: distance to the nearest segment minus `radius`
/// (VOLUME.md section 3). `closed` joins the last point to the first.
pub fn spline_to_sdf(
    points: &[DVec3],
    closed: bool,
    radius: f64,
    params: &SdfParams,
) -> SparseGrid<f32> {
    let mut segs: Vec<(DVec3, DVec3)> = points.windows(2).map(|w| (w[0], w[1])).collect();
    if closed && points.len() > 2 {
        segs.push((points[points.len() - 1], points[0]));
    }
    if segs.is_empty() && points.len() == 1 {
        segs.push((points[0], points[0]));
    }
    let margin = radius.max(0.0) + params.exterior_band().max(params.interior_band());
    let mut cand = HashSet::new();
    for (a, b) in &segs {
        let (lo, hi) = index_box(params, a.min(*b), a.max(*b), margin);
        collect_box(&mut cand, lo, hi);
    }
    let mut grid = params.empty_grid();
    for idx in sorted(cand) {
        let p = grid.index_to_world(idx);
        let d = segs
            .iter()
            .map(|(a, b)| segment_distance(*a, *b, p))
            .fold(f64::MAX, f64::min);
        put(&mut grid, params, idx, d - radius);
    }
    finish(grid, params)
}

/// How the spheres of [`points_to_sdf`] combine.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PointUnion {
    Min,
    /// Polynomial smooth minimum of width `k`.
    Smooth(f64),
}

/// Spheres `(centre, radius)` to a distance field: their union (VOLUME.md section 3).
pub fn points_to_sdf(
    spheres: &[(DVec3, f64)],
    union: PointUnion,
    params: &SdfParams,
) -> SparseGrid<f32> {
    let band = params.exterior_band().max(params.interior_band());
    let mut cand = HashSet::new();
    for (c, r) in spheres {
        let (lo, hi) = index_box(params, *c, *c, r.max(0.0) + band);
        collect_box(&mut cand, lo, hi);
    }
    let mut grid = params.empty_grid();
    for idx in sorted(cand) {
        let p = grid.index_to_world(idx);
        let mut v = f64::MAX;
        for (c, r) in spheres {
            let d = (p - *c).length() - r;
            v = match union {
                PointUnion::Min => v.min(d),
                PointUnion::Smooth(k) if v == f64::MAX => {
                    let _ = k;
                    d
                }
                PointUnion::Smooth(k) => crate::csg::smooth_min(v, d, k),
            };
        }
        put(&mut grid, params, idx, v);
    }
    finish(grid, params)
}
