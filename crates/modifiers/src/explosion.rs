//! The Explosion and Shatter deformers (docs/specs/MODIFIERS.md §11).
//!
//! Both split the mesh into its polygons and move every polygon rigidly. The random
//! stream of the reference is not known, so the motions are drawn from a seeded
//! SplitMix64 stream (deterministic, `ctx.seed`).

use forge_geometry::PolyMesh;
use glam::{DMat3, DVec3};

/// Which of the two deformers runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Explosion,
    Shatter,
}

/// Parameters of both deformers (`speed` is used by Explosion only). `strength`,
/// `end_size` and `randomness` are percent as the app shows them, `angle_speed` radians.
#[derive(Debug, Clone, Copy)]
pub struct BlastParams {
    pub strength: f64,
    pub speed: f64,
    pub angle_speed: f64,
    pub end_size: f64,
    pub randomness: f64,
}

impl Default for BlastParams {
    fn default() -> Self {
        BlastParams {
            strength: 0.0,
            speed: 100.0,
            angle_speed: 100f64.to_radians(),
            end_size: 0.0,
            randomness: 100.0,
        }
    }
}

/// SplitMix64: a small, fully specified generator.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed)
    }

    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform in `[0, 1)`.
    pub fn unit(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }
}

/// Splits `mesh` (in the deformer's local space) into its polygons and moves each one
/// (MODIFIERS.md §11). Strength 0 returns the mesh unchanged and unsplit.
pub fn blast(mesh: &mut PolyMesh, kind: Kind, p: &BlastParams, seed: u64) {
    let s = (p.strength / 100.0).clamp(0.0, 1.0);
    if s <= 0.0 {
        return;
    }
    let r = (p.randomness / 100.0).clamp(0.0, 1.0);
    let scale = 1.0 + (p.end_size / 100.0 - 1.0) * s;
    let half = 0.75 * p.angle_speed * s;
    let mut rng = Rng::new(seed ^ 0x5348_4154_5445_5231);
    // `u` in [0,1) pulled to the middle as randomness falls.
    let draw = |rng: &mut Rng| (1.0 - r) * 0.5 + r * rng.unit();
    let mut points = Vec::with_capacity(mesh.polys.len() * 4);
    let mut polys = Vec::with_capacity(mesh.polys.len());
    for poly in &mesh.polys {
        let src: Vec<DVec3> = poly.iter().map(|&i| mesh.points[i as usize]).collect();
        let c = src.iter().copied().sum::<DVec3>() / src.len().max(1) as f64;
        let ey = (2.0 * draw(&mut rng) - 1.0) * half;
        let ex = (2.0 * draw(&mut rng) - 1.0) * half;
        let ez = (2.0 * draw(&mut rng) - 1.0) * half;
        let rot =
            DMat3::from_rotation_y(ey) * DMat3::from_rotation_x(ex) * DMat3::from_rotation_z(ez);
        let centre = match kind {
            Kind::Explosion => {
                let z = 2.0 * rng.unit() - 1.0;
                let phi = std::f64::consts::TAU * rng.unit();
                let ring = (1.0 - z * z).sqrt();
                let v = DVec3::new(ring * phi.cos(), z, ring * phi.sin());
                let radial = c.normalize_or_zero();
                let dir = (radial + 1.5 * r * v).normalize_or(v);
                c + dir * (15.0 * p.speed * s * rng.unit())
            }
            Kind::Shatter => {
                let a = 100.0 * s * r * (rng.unit() + rng.unit() - 1.0);
                let b = 100.0 * s * r * (rng.unit() + rng.unit() - 1.0);
                DVec3::new(c.x + a, c.y * (1.0 - (2.0 * s).min(1.0)), c.z + b)
            }
        };
        let start = points.len() as u32;
        for q in &src {
            points.push(centre + scale * (rot * (*q - c)));
        }
        polys.push((start..start + src.len() as u32).collect());
    }
    mesh.points = points;
    mesh.polys = polys;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn quad(x: f64) -> Vec<DVec3> {
        vec![
            DVec3::new(x, 0.0, 0.0),
            DVec3::new(x + 10.0, 0.0, 0.0),
            DVec3::new(x + 10.0, 10.0, 0.0),
            DVec3::new(x, 10.0, 0.0),
        ]
    }

    fn two_quads() -> PolyMesh {
        let mut points = quad(0.0);
        points.extend(quad(20.0));
        PolyMesh {
            points,
            polys: vec![vec![0, 1, 2, 3], vec![4, 5, 6, 7]],
        }
    }

    #[test]
    fn strength_zero_changes_nothing() {
        let mut m = two_quads();
        let before = m.clone();
        blast(&mut m, Kind::Explosion, &BlastParams::default(), 0);
        assert_eq!(m, before);
    }

    #[test]
    fn polygons_stay_rigid_and_shrink_by_the_end_size() {
        for kind in [Kind::Explosion, Kind::Shatter] {
            let mut m = two_quads();
            let p = BlastParams {
                strength: 50.0,
                ..BlastParams::default()
            };
            blast(&mut m, kind, &p, 7);
            assert_eq!(m.points.len(), 8);
            for poly in &m.polys {
                let e = m.points[poly[1] as usize] - m.points[poly[0] as usize];
                assert!((e.length() - 5.0).abs() < 1e-9, "{kind:?} {e:?}");
            }
        }
    }

    #[test]
    fn same_seed_same_result() {
        let p = BlastParams {
            strength: 30.0,
            ..BlastParams::default()
        };
        let (mut a, mut b) = (two_quads(), two_quads());
        blast(&mut a, Kind::Shatter, &p, 3);
        blast(&mut b, Kind::Shatter, &p, 3);
        assert_eq!(a, b);
    }

    #[test]
    fn shatter_puts_the_centres_on_the_floor_at_half_strength() {
        let mut m = two_quads();
        let p = BlastParams {
            strength: 50.0,
            ..BlastParams::default()
        };
        blast(&mut m, Kind::Shatter, &p, 1);
        for poly in &m.polys {
            let c = poly.iter().map(|&i| m.points[i as usize].y).sum::<f64>() / 4.0;
            assert!(c.abs() < 1e-9);
        }
    }
}
