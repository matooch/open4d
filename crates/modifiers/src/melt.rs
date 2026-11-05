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

/// The spread of the strength: `g(0) = 0`, `g(1) = 1`, `g(½)/g(¼) = 1.6905` (spec §18, fact 2).
fn spread_curve(s: f64) -> f64 {
    const C: f64 = 1.4803;
    (1.0 - (-C * s).exp()) / (1.0 - (-C).exp())
}

/// Melts `points` (deformer space): the distance from the y axis grows, the height decays as
/// `y·exp(−8 s² F)` with `F` independent of the strength (spec §18, facts 1 to 3).
pub fn melt(points: &mut [DVec3], p: &MeltParams, seed: u64) {
    let s = (p.strength / 100.0).clamp(0.0, 1.0);
    let g = spread_curve(s);
    let m = (p.melted_size / 100.0).max(1.0);
    let rr = (p.radial_randomness / 100.0).clamp(0.0, 1.0);
    let vr = (p.vertical_randomness / 100.0).clamp(0.0, 1.0);
    let scale = (p.noise_scale / 100.0).max(0.0);
    let seed = seed as u32;
    // Distances are measured in half radii; a zero radius keeps the profile finite.
    let half = (0.5 * p.radius).max(1e-6);
    for q in points.iter_mut() {
        let r = q.x.hypot(q.z);
        let theta = q.z.atan2(q.x);
        let n1 = angle_noise(seed, theta, scale);
        let n2 = angle_noise(seed.wrapping_add(1), theta, scale);
        let k = 1.0 + (m - 1.0) * g * (1.1 + 0.15 * rr * (2.0 * n1 - 1.0));
        // Fit of the cap points of both Melt cases (spec §18): 1 on the axis, 2.7 at
        // r = 50, 3.7 at r = 100, 4.3 at r = 141, with a scatter of about 12 %.
        let f = (1.0 + 1.7 * (r / half).powf(0.65)) * (1.0 + 0.12 * vr * (2.0 * n2 - 1.0));
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
    fn deterministic() {
        let mut a = column(120.0);
        let mut b = column(120.0);
        melt(&mut a, &MeltParams::default(), 9);
        melt(&mut b, &MeltParams::default(), 9);
        assert_eq!(a, b);
    }
}
