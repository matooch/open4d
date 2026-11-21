//! Deterministic random numbers and sampling helpers (RENDER.md R9).

use std::f64::consts::{PI, TAU};

use glam::DVec3;

pub fn splitmix(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = x;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// PCG32.
#[derive(Debug, Clone)]
pub struct Rng {
    state: u64,
    inc: u64,
}

impl Rng {
    pub fn new(seed: u64, stream: u64) -> Rng {
        let mut r = Rng {
            state: 0,
            inc: (stream << 1) | 1,
        };
        r.next_u32();
        r.state = r.state.wrapping_add(seed);
        r.next_u32();
        r
    }

    pub fn next_u32(&mut self) -> u32 {
        let old = self.state;
        self.state = old
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(self.inc);
        let xor = (((old >> 18) ^ old) >> 27) as u32;
        let rot = (old >> 59) as u32;
        xor.rotate_right(rot)
    }

    /// Uniform in [0, 1).
    pub fn f(&mut self) -> f64 {
        let hi = self.next_u32() as u64;
        let lo = self.next_u32() as u64;
        (((hi << 21) ^ lo) & ((1 << 53) - 1)) as f64 / (1u64 << 53) as f64
    }

    pub fn f2(&mut self) -> (f64, f64) {
        (self.f(), self.f())
    }
}

/// Orthonormal basis around `n`.
pub fn basis(n: DVec3) -> (DVec3, DVec3) {
    let s = if n.z >= 0.0 { 1.0 } else { -1.0 };
    let a = -1.0 / (s + n.z);
    let b = n.x * n.y * a;
    (
        DVec3::new(1.0 + s * n.x * n.x * a, s * b, -s * n.x),
        DVec3::new(b, s + n.y * n.y * a, -n.y),
    )
}

pub fn to_world(n: DVec3, l: DVec3) -> DVec3 {
    let (t, b) = basis(n);
    t * l.x + b * l.y + n * l.z
}

pub fn cosine_hemisphere(u: (f64, f64)) -> DVec3 {
    let r = u.0.sqrt();
    let phi = TAU * u.1;
    DVec3::new(r * phi.cos(), r * phi.sin(), (1.0 - u.0).max(0.0).sqrt())
}

pub fn uniform_sphere(u: (f64, f64)) -> DVec3 {
    let z = 1.0 - 2.0 * u.0;
    let r = (1.0 - z * z).max(0.0).sqrt();
    let phi = TAU * u.1;
    DVec3::new(r * phi.cos(), r * phi.sin(), z)
}

pub fn uniform_ball(rng: &mut Rng) -> DVec3 {
    loop {
        let p = DVec3::new(
            rng.f() * 2.0 - 1.0,
            rng.f() * 2.0 - 1.0,
            rng.f() * 2.0 - 1.0,
        );
        if p.length_squared() <= 1.0 {
            return p;
        }
    }
}

/// Uniform direction in the cone about `axis` with half angle `half` (radians).
pub fn uniform_cone(axis: DVec3, half: f64, u: (f64, f64)) -> DVec3 {
    let cos_t = 1.0 - u.0 * (1.0 - half.cos());
    let sin_t = (1.0 - cos_t * cos_t).max(0.0).sqrt();
    let phi = TAU * u.1;
    to_world(
        axis,
        DVec3::new(sin_t * phi.cos(), sin_t * phi.sin(), cos_t),
    )
}

pub fn power_heuristic(a: f64, b: f64) -> f64 {
    let (a2, b2) = (a * a, b * b);
    if a2 + b2 <= 0.0 { 0.0 } else { a2 / (a2 + b2) }
}

pub const INV_4PI: f64 = 1.0 / (4.0 * PI);

pub fn lum(c: DVec3) -> f64 {
    c.dot(DVec3::new(0.2126, 0.7152, 0.0722))
}
