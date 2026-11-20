//! The Melt deformer (docs/specs/MODIFIERS.md §18): a structural model of the reference
//! result with our own noise. Values are parity-pending; the structure is derived.

use forge_noise::perlin;
use glam::DVec3;

/// Melt parameters; percentages as the app shows them.
#[derive(Debug, Clone, Copy)]
pub struct MeltParams {
    pub strength: f64,
    pub radius: f64,
    pub vertical_randomness: f64,
    pub radial_randomness: f64,
    pub melted_size: f64,
    pub noise_scale: f64,
}

impl Default for MeltParams {
    fn default() -> Self {
        MeltParams {
            strength: 25.0,
            radius: 100.0,
            vertical_randomness: 100.0,
            radial_randomness: 100.0,
            melted_size: 400.0,
            noise_scale: 10.0,
        }
    }
}

/// A noise of the direction angle in `[0, 1]`: Perlin on the unit circle, one lattice cell
/// per `10 · noise_scale` (so the default 10 % is one cell per unit of the circle).
fn angle_noise(seed: u32, theta: f64, scale: f64) -> f64 {
    let f = 1.0 / (10.0 * scale).max(1e-3);
    let q = [theta.cos() * f, theta.sin() * f];
    0.5 + 0.5 * perlin::<2>(seed, q).clamp(-1.0, 1.0)
}

/// Observed spread `a(s)` with Randomness 0 (spec §18): `k = 1 + m · a(s)` for `m = melted_size/100`.
/// The nodes are `melt/no-random-s10`, `-s50`, `-s100` (`k = 1.792066268, 4.18024039, 4.96319944` at
/// `m = 4`) and `s = ¼` from the measured ratio `a(½)/a(¼) = 1.6905` of the noisy cases; `a(0) = 0`.
const SPREAD: [(f64, f64); 5] = [
    (0.0, 0.0),
    (0.1, 0.198_016_567),
    (0.25, 0.795_060_098 / 1.6905),
    (0.5, 0.795_060_098),
    (1.0, 0.990_799_86),
];

/// `a(s)`: monotone cubic Hermite through [`SPREAD`] (Fritsch-Carlson slopes). Exact at the
/// nodes; between them parity-pending.
fn spread_curve(s: f64) -> f64 {
    let n = SPREAD.len();
    let h: Vec<f64> = (0..n - 1).map(|i| SPREAD[i + 1].0 - SPREAD[i].0).collect();
    let d: Vec<f64> = (0..n - 1)
        .map(|i| (SPREAD[i + 1].1 - SPREAD[i].1) / h[i])
        .collect();
    let mut m = vec![0.0; n];
    m[0] = d[0];
    m[n - 1] = d[n - 2];
    for i in 1..n - 1 {
        m[i] = if d[i - 1] * d[i] <= 0.0 {
            0.0
        } else {
            let w1 = 2.0 * h[i] + h[i - 1];
            let w2 = h[i] + 2.0 * h[i - 1];
            (w1 + w2) / (w1 / d[i - 1] + w2 / d[i])
        };
    }
    let s = s.clamp(0.0, 1.0);
    let i = (0..n - 1).rev().find(|&i| s >= SPREAD[i].0).unwrap_or(0);
    let t = (s - SPREAD[i].0) / h[i];
    let (t2, t3) = (t * t, t * t * t);
    (2.0 * t3 - 3.0 * t2 + 1.0) * SPREAD[i].1
        + (t3 - 2.0 * t2 + t) * h[i] * m[i]
        + (-2.0 * t3 + 3.0 * t2) * SPREAD[i + 1].1
        + (t3 - t2) * h[i] * m[i + 1]
}

/// Melts `points` (deformer space): the distance from the y axis grows by `k`, the height decays
/// as `y·exp(−8 s² F)` with `F` independent of the strength (spec §18).
pub fn melt(points: &mut [DVec3], p: &MeltParams, seed: u64) {
    let s = (p.strength / 100.0).clamp(0.0, 1.0);
    let a = spread_curve(s);
    let m = (p.melted_size / 100.0).max(0.0);
    let rr = (p.radial_randomness / 100.0).clamp(0.0, 1.0);
    let vr = (p.vertical_randomness / 100.0).clamp(0.0, 1.0);
    let scale = (p.noise_scale / 100.0).max(0.0);
    let seed = seed as u32;
    for q in points.iter_mut() {
        let r = q.x.hypot(q.z);
        let theta = q.z.atan2(q.x);
        let n1 = angle_noise(seed, theta, scale);
        let n2 = angle_noise(seed.wrapping_add(1), theta, scale);
        // Randomness 0 gives the plain spread; at 100 % the reference's factor runs
        // 0.60 .. 0.80 over the rays (fit of `melt/strength`, spec §18).
        let k = 1.0 + m * a * (1.0 - rr * (0.3 - 0.1 * (2.0 * n1 - 1.0)));
        // `F = 1 + r/20` exactly with Randomness 0 (`melt/no-random-s10`); the vertical
        // randomness scales the radial part by about 0.7 (fit of the noisy cases).
        let f = 1.0 + (r / 20.0) * (1.0 - vr * (0.3 - 0.1 * (2.0 * n2 - 1.0)));
        q.x *= k;
        q.z *= k;
        q.y = if q.y <= 0.0 {
            0.0
        } else {
            q.y * (-8.0 * s * s * f).exp()
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn column(r: f64) -> Vec<DVec3> {
        vec![DVec3::new(r, -50.0, 0.0), DVec3::new(r, 150.0, 0.0)]
    }

    #[test]
    fn lower_half_goes_flat_upper_half_decays() {
        for r in [0.0, 50.0, 100.0] {
            let mut col = column(r);
            melt(&mut col, &MeltParams::default(), 0);
            assert_eq!(col[0].y, 0.0);
            assert!(col[1].y > 0.0 && col[1].y < 150.0);
        }
    }

    #[test]
    fn axis_point_follows_exp_of_minus_8_s_squared() {
        // Spec §18 fact 3: (0, 200, 0) ends at 200·e^(−8 s²) (121.306 at 25 %, 27.067 at 50 %).
        for (s, want) in [(25.0, 121.306), (50.0, 27.067)] {
            let mut pts = vec![DVec3::new(0.0, 200.0, 0.0)];
            let p = MeltParams {
                strength: s,
                ..MeltParams::default()
            };
            melt(&mut pts, &p, 5);
            assert!((pts[0].y - want).abs() < 1e-3, "{}", pts[0].y);
        }
    }

    #[test]
    fn direction_is_kept_and_distance_grows() {
        let mut pts = vec![DVec3::new(-100.0, 10.0, -100.0)];
        melt(&mut pts, &MeltParams::default(), 3);
        assert!((pts[0].x - pts[0].z).abs() < 1e-9);
        assert!(pts[0].x < -100.0);
    }

    #[test]
    fn more_strength_melts_more() {
        let run = |s: f64| {
            let mut pts = column(141.0);
            let p = MeltParams {
                strength: s,
                ..MeltParams::default()
            };
            melt(&mut pts, &p, 0);
            (pts[1].x, pts[1].y)
        };
        let (x1, y1) = run(25.0);
        let (x2, y2) = run(50.0);
        assert!(x2 > x1 && y2 < y1);
    }

    #[test]
    fn spread_is_exact_at_the_observed_strengths() {
        // melt/no-random-s*: k = 1 + 4 a(s) for Melted Size 400 %.
        for (s, k) in [(0.1, 1.792_066_268), (0.5, 4.180_240_39), (1.0, 4.963_199_44)] {
            assert!((1.0 + 4.0 * spread_curve(s) - k).abs() < 1e-8, "{s}");
        }
        assert_eq!(spread_curve(0.0), 0.0);
        // Monotone between the nodes.
        let mut last = 0.0;
        for i in 1..=100 {
            let a = spread_curve(i as f64 / 100.0);
            assert!(a >= last, "{i}");
            last = a;
        }
    }

    #[test]
    fn without_randomness_every_ray_spreads_alike_and_f_is_one_plus_r_over_20() {
        let p = MeltParams {
            strength: 10.0,
            vertical_randomness: 0.0,
            radial_randomness: 0.0,
            ..MeltParams::default()
        };
        let mut pts = vec![DVec3::new(100.0, 100.0, 100.0), DVec3::new(-70.0, 100.0, 0.0)];
        melt(&mut pts, &p, 4);
        assert!((pts[0].x / 100.0 - 1.792_066_268).abs() < 1e-8);
        assert!((pts[1].x / -70.0 - 1.792_066_268).abs() < 1e-8);
        let want = 100.0 * (-8.0_f64 * 0.01 * (1.0 + 141.421_356_237 / 20.0)).exp();
        assert!((pts[0].y - want).abs() < 1e-6, "{} vs {want}", pts[0].y);
    }

    #[test]
    fn deterministic() {
        let mut a = column(120.0);
        let mut b = column(120.0);
        melt(&mut a, &MeltParams::default(), 9);
        melt(&mut b, &MeltParams::default(), 9);
        assert_eq!(a, b);
    }
}
