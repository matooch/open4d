//! The Delta Mush deformer (docs/specs/MODIFIERS.md §14.6).

use forge_geometry::PolyMesh;
use glam::DVec3;

/// Delta Mush parameters (`strength` as a fraction).
#[derive(Debug, Clone, Copy)]
pub struct DeltaMushParams {
    pub strength: f64,
    pub iterations: usize,
    pub pin_border: bool,
}

/// Points that lie on an edge used by one polygon only.
fn border_points(mesh: &PolyMesh) -> Vec<bool> {
    let mut count = std::collections::BTreeMap::new();
    for poly in &mesh.polys {
        for (i, &a) in poly.iter().enumerate() {
            let b = poly[(i + 1) % poly.len()];
            *count.entry((a.min(b), a.max(b))).or_insert(0usize) += 1;
        }
    }
    let mut out = vec![false; mesh.points.len()];
    for ((a, b), c) in count {
        if c == 1 {
            out[a as usize] = true;
            out[b as usize] = true;
        }
    }
    out
}

/// `iterations` Jacobi steps of `x ← ½x + ½ mean(neighbours)`.
pub fn smooth(
    pts: &[DVec3],
    nb: &[Vec<u32>],
    iterations: usize,
    pinned: Option<&[bool]>,
) -> Vec<DVec3> {
    let mut x = pts.to_vec();
    for _ in 0..iterations {
        let prev = x.clone();
        for (i, xi) in x.iter_mut().enumerate() {
            if nb[i].is_empty() || pinned.is_some_and(|p| p[i]) {
                continue;
            }
            let mean: DVec3 =
                nb[i].iter().map(|j| prev[*j as usize]).sum::<DVec3>() / nb[i].len() as f64;
            *xi = (prev[i] + mean) * 0.5;
        }
    }
    x
}

/// One `(t, b, n)` frame per point on `pts` (edge tangent mode).
fn frames(mesh: &PolyMesh, pts: &[DVec3], nb: &[Vec<u32>]) -> Vec<[DVec3; 3]> {
    let mut normal = vec![DVec3::ZERO; pts.len()];
    for poly in &mesh.polys {
        // Newell normal
        let mut n = DVec3::ZERO;
        for (i, &a) in poly.iter().enumerate() {
            let b = poly[(i + 1) % poly.len()];
            n += pts[a as usize].cross(pts[b as usize]);
        }
        for &i in poly {
            normal[i as usize] += n;
        }
    }
    (0..pts.len())
        .map(|i| {
            let n = if normal[i].length() > 1e-12 {
                normal[i].normalize()
            } else {
                DVec3::Y
            };
            let edge = nb[i]
                .first()
                .map_or(DVec3::X, |j| pts[*j as usize] - pts[i]);
            let mut t = edge - n * edge.dot(n);
            if t.length() < 1e-12 {
                t = n.any_orthonormal_vector();
            }
            let t = t.normalize();
            [t, n.cross(t), n]
        })
        .collect()
}

/// The deltas of the bind pose, in the frames of the smoothed bind pose.
pub fn bind_deltas(mesh: &PolyMesh, bind: &[DVec3], p: &DeltaMushParams) -> Vec<DVec3> {
    let nb = crate::helper::neighbours(mesh);
    let pinned = p.pin_border.then(|| border_points(mesh));
    let s = smooth(bind, &nb, p.iterations, pinned.as_deref());
    let f = frames(mesh, &s, &nb);
    (0..bind.len())
        .map(|i| {
            let d = bind[i] - s[i];
            DVec3::new(d.dot(f[i][0]), d.dot(f[i][1]), d.dot(f[i][2]))
        })
        .collect()
}

/// Deforms `mesh.points` (the current pose) with the deltas of `bind`.
pub fn delta_mush(mesh: &mut PolyMesh, bind: &[DVec3], p: &DeltaMushParams) {
    if bind.len() != mesh.points.len() {
        return;
    }
    let nb = crate::helper::neighbours(mesh);
    let pinned = p.pin_border.then(|| border_points(mesh));
    let deltas = bind_deltas(mesh, bind, p);
    let s = smooth(&mesh.points, &nb, p.iterations, pinned.as_deref());
    let f = frames(mesh, &s, &nb);
    for (i, q) in mesh.points.iter_mut().enumerate() {
        let out = s[i] + f[i][0] * deltas[i].x + f[i][1] * deltas[i].y + f[i][2] * deltas[i].z;
        *q += (out - *q) * p.strength;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grid() -> PolyMesh {
        let n = 5u32;
        let mut points = Vec::new();
        for j in 0..n {
            for i in 0..n {
                points.push(DVec3::new(i as f64 * 10.0, j as f64 * 10.0, 0.0));
            }
        }
        let mut polys = Vec::new();
        for j in 0..n - 1 {
            for i in 0..n - 1 {
                let a = j * n + i;
                polys.push(vec![a, a + 1, a + n + 1, a + n]);
            }
        }
        PolyMesh { points, polys }
    }

    #[test]
    fn the_bind_pose_is_reproduced() {
        let mut m = grid();
        m.points[7].z = 25.0;
        let bind = m.points.clone();
        let p = DeltaMushParams {
            strength: 1.0,
            iterations: 10,
            pin_border: false,
        };
        delta_mush(&mut m, &bind, &p);
        for (a, b) in m.points.iter().zip(&bind) {
            assert!((*a - *b).length() < 1e-9);
        }
    }

    #[test]
    fn a_rigid_move_keeps_the_detail_and_strength_zero_does_nothing() {
        let mut m = grid();
        m.points[12].z = 15.0;
        let bind = m.points.clone();
        let shift = DVec3::new(100.0, -20.0, 5.0);
        let mut moved = PolyMesh {
            points: bind.iter().map(|q| *q + shift).collect(),
            polys: m.polys.clone(),
        };
        let p = DeltaMushParams {
            strength: 1.0,
            iterations: 5,
            pin_border: true,
        };
        delta_mush(&mut moved, &bind, &p);
        for (a, b) in moved.points.iter().zip(&bind) {
            assert!((*a - (*b + shift)).length() < 1e-6, "{a:?} {b:?}");
        }
        let mut z = grid();
        z.points[3].y += 50.0;
        let before = z.points.clone();
        delta_mush(&mut z, &bind, &DeltaMushParams { strength: 0.0, ..p });
        assert_eq!(z.points, before);
    }

    #[test]
    fn a_spike_is_smoothed_out_of_a_deformed_pose() {
        let m = grid();
        let bind = m.points.clone();
        let mut spiky = m.clone();
        spiky.points[12].z = 100.0;
        let p = DeltaMushParams {
            strength: 1.0,
            iterations: 10,
            pin_border: false,
        };
        delta_mush(&mut spiky, &bind, &p);
        assert!(spiky.points[12].z < 100.0);
    }
}
