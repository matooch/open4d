//! The FFD deformer: a Bézier volume over a lattice (docs/specs/MODIFIERS.md §16).

use glam::DVec3;

/// FFD parameters. `grid` is empty (or of the wrong length) for the regular lattice.
#[derive(Debug, Clone)]
pub struct FfdParams {
    pub size: DVec3,
    pub counts: [usize; 3],
    /// Lattice point positions, x fastest (`i + nx·(j + ny·k)`).
    pub grid: Vec<DVec3>,
}

/// The regular lattice: `(i/(n−1) − ½)` of `size` per axis, x fastest.
pub fn regular_grid(size: DVec3, counts: [usize; 3]) -> Vec<DVec3> {
    let at = |i: usize, n: usize| {
        if n > 1 {
            i as f64 / (n - 1) as f64 - 0.5
        } else {
            0.0
        }
    };
    let mut out = Vec::with_capacity(counts[0] * counts[1] * counts[2]);
    for k in 0..counts[2] {
        for j in 0..counts[1] {
            for i in 0..counts[0] {
                out.push(DVec3::new(at(i, counts[0]), at(j, counts[1]), at(k, counts[2])) * size);
            }
        }
    }
    out
}

/// The Bernstein basis of degree `n − 1` (`n` values) at `t`, by the stable recurrence.
fn bernstein(n: usize, t: f64) -> Vec<f64> {
    let mut b = vec![0.0; n];
    b[0] = 1.0;
    for j in 1..n {
        let mut saved = 0.0;
        for slot in b.iter_mut().take(j) {
            let tmp = *slot;
            *slot = saved + (1.0 - t) * tmp;
            saved = t * tmp;
        }
        b[j] = saved;
    }
    b
}

/// Moves `points` (in the deformer's space): inside the lattice box by the Bézier volume of
/// the lattice offsets, outside it not at all.
pub fn ffd(points: &mut [DVec3], p: &FfdParams) {
    let [nx, ny, nz] = p.counts;
    if nx < 2 || ny < 2 || nz < 2 {
        return;
    }
    let regular = regular_grid(p.size, p.counts);
    let grid = if p.grid.len() == regular.len() {
        &p.grid
    } else {
        return; // The regular lattice is the identity.
    };
    // Only lattice points that were moved contribute.
    let moved: Vec<(usize, usize, usize, DVec3)> = grid
        .iter()
        .zip(&regular)
        .enumerate()
        .filter(|(_, (g, r))| g != r)
        .map(|(idx, (g, r))| (idx % nx, (idx / nx) % ny, idx / (nx * ny), *g - *r))
        .collect();
    if moved.is_empty() {
        return;
    }
    for q in points.iter_mut() {
        let t = *q / p.size + DVec3::splat(0.5);
        let inside = |v: f64| (0.0..=1.0).contains(&v);
        if !(inside(t.x) && inside(t.y) && inside(t.z)) {
            continue; // Also false for NaN (size 0 on an axis).
        }
        let (bx, by, bz) = (bernstein(nx, t.x), bernstein(ny, t.y), bernstein(nz, t.z));
        let mut d = DVec3::ZERO;
        for (i, j, k, delta) in &moved {
            d += *delta * (bx[*i] * by[*j] * bz[*k]);
        }
        *q += d;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn params(counts: [usize; 3]) -> FfdParams {
        let size = DVec3::splat(300.0);
        FfdParams {
            size,
            counts,
            grid: regular_grid(size, counts),
        }
    }

    #[test]
    fn regular_lattice_corners() {
        let g = regular_grid(DVec3::splat(300.0), [3, 3, 3]);
        assert_eq!(g[0], DVec3::splat(-150.0));
        assert_eq!(g[8], DVec3::new(150.0, 150.0, -150.0));
        assert_eq!(g[26], DVec3::splat(150.0));
    }

    #[test]
    fn untouched_lattice_is_the_identity() {
        let mut pts = vec![DVec3::new(10.0, -20.0, 30.0), DVec3::new(500.0, 0.0, 0.0)];
        let before = pts.clone();
        ffd(&mut pts, &params([4, 3, 5]));
        assert_eq!(pts, before);
    }

    #[test]
    fn corner_point_follows_its_lattice_point_and_outside_stays() {
        let mut p = params([3, 3, 3]);
        p.grid[26] += DVec3::new(0.0, 100.0, 0.0);
        let mut pts = vec![
            DVec3::splat(150.0),
            DVec3::new(0.0, 0.0, 0.0),
            DVec3::new(0.0, 151.0, 0.0),
        ];
        ffd(&mut pts, &p);
        assert!((pts[0].y - 250.0).abs() < 1e-9);
        // The centre has weight 1/8·... (B(½) of the corner on three axes = 1/64): 100/64.
        assert!((pts[1].y - 100.0 / 64.0).abs() < 1e-9);
        assert_eq!(pts[2], DVec3::new(0.0, 151.0, 0.0));
    }

    #[test]
    fn bernstein_partitions_unity() {
        let b = bernstein(7, 0.37);
        assert!((b.iter().sum::<f64>() - 1.0).abs() < 1e-14);
    }
}
