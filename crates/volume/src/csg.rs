//! CSG on signed distance fields (VOLUME.md section 4).

use std::collections::BTreeSet;

use glam::IVec3;

use crate::VolumeError;
use crate::grid::SparseGrid;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CsgOp {
    Union,
    /// `a` minus `b`.
    Subtract,
    Intersect,
    /// Polynomial smooth union of width `k`.
    SmoothUnion(f64),
}

/// Polynomial smooth minimum (Quilez): `min(a, b) - h * h * k / 4`.
pub fn smooth_min(a: f64, b: f64, k: f64) -> f64 {
    if k <= 0.0 {
        return a.min(b);
    }
    let h = (k - (a - b).abs()).max(0.0) / k;
    a.min(b) - h * h * k / 4.0
}

fn apply(op: CsgOp, a: f32, b: f32) -> f32 {
    match op {
        CsgOp::Union => a.min(b),
        CsgOp::Subtract => a.max(-b),
        CsgOp::Intersect => a.max(b),
        CsgOp::SmoothUnion(k) => smooth_min(f64::from(a), f64::from(b), k) as f32,
    }
}

/// Combines two grids on the same lattice; active in the result where either input is active.
pub fn combine(
    a: &SparseGrid<f32>,
    b: &SparseGrid<f32>,
    op: CsgOp,
) -> Result<SparseGrid<f32>, VolumeError> {
    if !a.same_lattice(b) {
        return Err(VolumeError::GridMismatch);
    }
    let background = apply(op, a.background(), b.background());
    let mut out = SparseGrid::new(background, a.voxel_size(), a.origin());
    let mut seen: BTreeSet<(i32, i32, i32)> = BTreeSet::new();
    for grid in [a, b] {
        for (i, _, _) in grid.stored_voxels() {
            seen.insert((i.x, i.y, i.z));
        }
    }
    for (x, y, z) in seen {
        let i = IVec3::new(x, y, z);
        let v = apply(op, a.get(i), b.get(i));
        if a.is_active(i) || b.is_active(i) {
            out.set(i, v);
        } else if v != background {
            out.set_inactive(i, v);
        }
    }
    Ok(out)
}

/// Negated copy (inside becomes outside); the background is negated too.
pub fn negate(a: &SparseGrid<f32>) -> SparseGrid<f32> {
    a.map(-a.background(), |v| -v)
}
