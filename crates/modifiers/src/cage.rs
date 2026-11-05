//! The Mesh (cage) deformer (docs/specs/MODIFIERS.md §14.1): mean value coordinates.

use std::f64::consts::PI;

use glam::DVec3;

const EPS: f64 = 1e-9;

/// Mean value coordinates (Ju, Schaefer, Warren 2005) of `v` in the triangle mesh
/// `pts`/`tris`, normalised to sum 1. `None` when they are degenerate.
pub fn mvc_weights(v: DVec3, pts: &[DVec3], tris: &[[u32; 3]]) -> Option<Vec<f64>> {
    let n = pts.len();
    let mut d = vec![0.0; n];
    let mut u = vec![DVec3::ZERO; n];
    for i in 0..n {
        let e = pts[i] - v;
        d[i] = e.length();
        if d[i] < EPS {
            let mut w = vec![0.0; n];
            w[i] = 1.0;
            return Some(w);
        }
        u[i] = e / d[i];
    }
    let mut w = vec![0.0; n];
    for t in tris {
        let idx = t.map(|i| i as usize);
        let uu = idx.map(|i| u[i]);
        let l = [
            (uu[1] - uu[2]).length(),
            (uu[2] - uu[0]).length(),
            (uu[0] - uu[1]).length(),
        ];
        let th = l.map(|x| 2.0 * (x / 2.0).clamp(-1.0, 1.0).asin());
        let h = (th[0] + th[1] + th[2]) / 2.0;
        if PI - h < EPS {
            // the point lies in the triangle: 2-D barycentric weights
            let mut w2 = vec![0.0; n];
            for m in 0..3 {
                w2[idx[m]] = th[m].sin() * d[idx[(m + 1) % 3]] * d[idx[(m + 2) % 3]];
            }
            let s: f64 = w2.iter().sum();
            if s.abs() < 1e-300 {
                return None;
            }
            return Some(w2.into_iter().map(|x| x / s).collect());
        }
        let sin = th.map(f64::sin);
        if sin.iter().any(|s| s.abs() < EPS) {
            continue;
        }
        let c: [f64; 3] = std::array::from_fn(|m| {
            2.0 * h.sin() * (h - th[m]).sin() / (sin[(m + 1) % 3] * sin[(m + 2) % 3]) - 1.0
        });
        let sgn = uu[0].dot(uu[1].cross(uu[2])).signum();
        let s = c.map(|c| sgn * (1.0 - c * c).max(0.0).sqrt());
        if s.iter().any(|s| s.abs() < EPS) {
            continue;
        }
        for m in 0..3 {
            let (m1, m2) = ((m + 1) % 3, (m + 2) % 3);
            w[idx[m]] += (th[m] - c[m1] * th[m2] - c[m2] * th[m1]) / (d[idx[m]] * sin[m1] * s[m2]);
        }
    }
    let sum: f64 = w.iter().sum();
    if sum.abs() < 1e-12 || !sum.is_finite() {
        return None;
    }
    Some(w.into_iter().map(|x| x / sum).collect())
}

/// Moves `points` (world) from the `bind` cage to the `current` one.
pub fn cage_deform(
    points: &mut [DVec3],
    bind: &[DVec3],
    current: &[DVec3],
    tris: &[[u32; 3]],
    strength: f64,
) {
    if bind.len() != current.len() {
        return;
    }
    for p in points.iter_mut() {
        if let Some(w) = mvc_weights(*p, bind, tris) {
            let q: DVec3 = w.iter().zip(current).map(|(w, c)| *c * *w).sum();
            *p += (q - *p) * strength;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cube(a: f64) -> (Vec<DVec3>, Vec<[u32; 3]>) {
        let p: Vec<DVec3> = (0..8)
            .map(|i| {
                DVec3::new(
                    if i & 1 == 0 { -a } else { a },
                    if i & 2 == 0 { -a } else { a },
                    if i & 4 == 0 { -a } else { a },
                )
            })
            .collect();
        let quads = [
            [0, 2, 3, 1],
            [4, 5, 7, 6],
            [0, 1, 5, 4],
            [2, 6, 7, 3],
            [0, 4, 6, 2],
            [1, 3, 7, 5],
        ];
        let tris = quads
            .iter()
            .flat_map(|q| [[q[0], q[1], q[2]], [q[0], q[2], q[3]]])
            .collect();
        (p, tris)
    }

    #[test]
    fn weights_sum_to_one_and_reproduce_the_point() {
        let (p, t) = cube(100.0);
        for v in [DVec3::new(10.0, -20.0, 30.0), DVec3::new(99.0, 50.0, -70.0)] {
            let w = mvc_weights(v, &p, &t).unwrap();
            assert!((w.iter().sum::<f64>() - 1.0).abs() < 1e-9);
            let q: DVec3 = w.iter().zip(&p).map(|(w, c)| *c * *w).sum();
            assert!((q - v).length() < 1e-6, "{q:?} vs {v:?}");
        }
    }

    #[test]
    fn an_unmoved_cage_is_the_identity_and_a_scale_scales() {
        let (p, t) = cube(100.0);
        let pts0 = vec![DVec3::new(10.0, 20.0, -30.0), DVec3::new(-70.0, 5.0, 60.0)];
        let mut a = pts0.clone();
        cage_deform(&mut a, &p, &p, &t, 1.0);
        for (x, y) in a.iter().zip(&pts0) {
            assert!((*x - *y).length() < 1e-6);
        }
        let scaled: Vec<DVec3> = p.iter().map(|q| *q * 2.0).collect();
        let mut b = pts0.clone();
        cage_deform(&mut b, &p, &scaled, &t, 1.0);
        for (x, y) in b.iter().zip(&pts0) {
            assert!((*x - *y * 2.0).length() < 1e-6);
        }
        let mut c = pts0.clone();
        cage_deform(&mut c, &p, &scaled, &t, 0.0);
        assert_eq!(c, pts0);
    }
}
