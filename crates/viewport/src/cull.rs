//! Frustum maths for culling (docs/specs/VIEWPORT.md §10.4).
//! Refs: Gribb & Hartmann (2001), Assarsson & Möller, JGT 5(1) (2000), Arvo (1990).

use glam::{DMat4, DVec3, DVec4};

/// Six inward-facing, normalised planes `(n, d)`: a point is inside when `n·p + d >= 0`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Frustum {
    planes: [DVec4; 6],
}

/// Relative padding added to every box before the plane test, so a box lying on a plane is kept.
const PAD: f64 = 1e-6;

impl Frustum {
    /// The frustum of a clip-from-world matrix with depth range `[0, 1]` (wgpu, glam `*_rh`).
    pub fn from_view_proj(m: &DMat4) -> Frustum {
        let row = |i: usize| DVec4::new(m.x_axis[i], m.y_axis[i], m.z_axis[i], m.w_axis[i]);
        let (r0, r1, r2, r3) = (row(0), row(1), row(2), row(3));
        let norm = |p: DVec4| {
            let len = p.truncate().length();
            if len > 0.0 { p / len } else { p }
        };
        Frustum {
            planes: [
                norm(r3 + r0),
                norm(r3 - r0),
                norm(r3 + r1),
                norm(r3 - r1),
                norm(r2),
                norm(r3 - r2),
            ],
        }
    }

    /// Whether the point is inside (or on) all six planes.
    pub fn contains_point(&self, p: DVec3) -> bool {
        self.planes
            .iter()
            .all(|pl| pl.truncate().dot(p) + pl.w >= 0.0)
    }

    /// False only if the box lies entirely outside one plane. Conservative: a box near a
    /// frustum corner may be kept although it is outside; a box that touches is always kept.
    pub fn intersects_aabb(&self, min: DVec3, max: DVec3) -> bool {
        let pad = DVec3::splat(PAD) * (max - min).max_element().max(1.0);
        let (min, max) = (min - pad, max + pad);
        self.planes.iter().all(|pl| {
            let n = pl.truncate();
            // the corner furthest along the plane normal
            let far = DVec3::new(
                if n.x >= 0.0 { max.x } else { min.x },
                if n.y >= 0.0 { max.y } else { min.y },
                if n.z >= 0.0 { max.z } else { min.z },
            );
            n.dot(far) + pl.w >= 0.0
        })
    }

    /// False only if the sphere lies entirely outside one plane.
    pub fn intersects_sphere(&self, center: DVec3, radius: f64) -> bool {
        let r = radius + PAD * radius.max(1.0);
        self.planes
            .iter()
            .all(|pl| pl.truncate().dot(center) + pl.w >= -r)
    }
}

/// The axis-aligned box of the box `(min, max)` after the affine transform `m` (Arvo 1990).
pub fn transform_aabb(m: &DMat4, min: DVec3, max: DVec3) -> (DVec3, DVec3) {
    let cols = [
        m.x_axis.truncate(),
        m.y_axis.truncate(),
        m.z_axis.truncate(),
    ];
    let mut lo = m.w_axis.truncate();
    let mut hi = lo;
    for (c, (a, b)) in cols
        .iter()
        .zip([(min.x, max.x), (min.y, max.y), (min.z, max.z)])
    {
        let (p, q) = (*c * a, *c * b);
        lo += p.min(q);
        hi += p.max(q);
    }
    (lo, hi)
}

/// Union of two boxes.
pub fn union_aabb(a: (DVec3, DVec3), b: (DVec3, DVec3)) -> (DVec3, DVec3) {
    (a.0.min(b.0), a.1.max(b.1))
}

/// Distance from `p` to the box (0 inside).
pub fn distance_to_aabb(p: DVec3, min: DVec3, max: DVec3) -> f64 {
    (min - p).max(p - max).max(DVec3::ZERO).length()
}
