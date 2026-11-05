//! The Wind deformer (docs/specs/MODIFIERS.md §12).

use forge_noise::perlin;
use glam::DVec3;

/// Wind parameters. `turbulence` is percent as the app shows it.
#[derive(Debug, Clone, Copy)]
pub struct WindParams {
    pub amplitude: f64,
    pub size: f64,
    pub frequency: f64,
    pub turbulence: f64,
    pub fx: f64,
    pub fy: f64,
    pub flag: bool,
}

impl Default for WindParams {
    fn default() -> Self {
        WindParams {
            amplitude: 50.0,
            size: 100.0,
            frequency: 1.0,
            turbulence: 50.0,
            fx: 1.0,
            fy: 1.0,
            flag: true,
        }
    }
}

/// Pushes `points` (in the deformer's local space) along z by a noise wave of `x`, `y`
/// and `time` seconds.
pub fn wind(points: &mut [DVec3], p: &WindParams, time: f64, seed: u64) {
    if p.size <= 0.0 || p.amplitude == 0.0 {
        return;
    }
    let turb = (p.turbulence / 100.0).clamp(0.0, 1.0);
    let seed = seed as u32;
    let t = time * p.frequency;
    for pt in points.iter_mut() {
        let q = [pt.x * p.fx / p.size, pt.y * p.fy / p.size, t];
        let n1 = perlin::<3>(seed, q).clamp(-1.0, 1.0);
        let n2 = perlin::<3>(
            seed.wrapping_add(1),
            [q[0] * 2.0 + 17.0, q[1] * 2.0 + 31.0, q[2] * 2.0 + 5.0],
        )
        .clamp(-1.0, 1.0);
        // A gust that never reverses: 0.5..1.5 times the amplitude (spec §12).
        let n = 1.0 + 0.5 * (n1 + turb * n2) / (1.0 + turb);
        let g = if p.flag {
            pt.x.signum() * (pt.x.abs() / p.size).min(1.0).sqrt()
        } else {
            1.0
        };
        pt.z += p.amplitude * n * g;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_z_moves_and_x_zero_is_pinned() {
        let mut pts = vec![DVec3::new(0.0, 10.0, 5.0), DVec3::new(50.0, 10.0, 5.0)];
        wind(&mut pts, &WindParams::default(), 1.0 / 3.0, 0);
        assert_eq!(pts[0], DVec3::new(0.0, 10.0, 5.0));
        assert_eq!((pts[1].x, pts[1].y), (50.0, 10.0));
        assert!(pts[1].z > 5.0);
    }

    #[test]
    fn the_offset_is_bounded_by_amplitude_and_turbulence() {
        let mut pts: Vec<DVec3> = (-10..=10)
            .flat_map(|i| {
                (-10..=10).map(move |j| DVec3::new(i as f64 * 20.0, j as f64 * 20.0, 0.0))
            })
            .collect();
        wind(&mut pts, &WindParams::default(), 0.7, 3);
        assert!(pts.iter().all(|p| p.z.abs() <= 75.0 + 1e-9));
        assert!(pts.iter().any(|p| p.z.abs() > 40.0));
        assert!(pts.iter().any(|p| p.z.abs() > 1.0));
    }

    #[test]
    fn it_moves_over_time() {
        let base = vec![DVec3::new(80.0, 30.0, 0.0)];
        let (mut a, mut b) = (base.clone(), base);
        wind(&mut a, &WindParams::default(), 0.0, 0);
        wind(&mut b, &WindParams::default(), 0.5, 0);
        assert_ne!(a, b);
    }
}
