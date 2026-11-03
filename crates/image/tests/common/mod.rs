#![allow(dead_code)]
use forge_image::{AlphaMode, Image};

/// Deterministic pseudo random numbers in 0..1 (LCG), so tests are reproducible.
pub struct Lcg(pub u64);
impl Lcg {
    pub fn next_f32(&mut self) -> f32 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((self.0 >> 40) as f32) / ((1u64 << 24) as f32)
    }
}

/// A smooth RGBA gradient (straight alpha) in linear values.
pub fn gradient(w: u32, h: u32) -> Image {
    Image::from_fn(w, h, AlphaMode::Straight, |x, y| {
        [
            x as f32 / (w - 1).max(1) as f32,
            y as f32 / (h - 1).max(1) as f32,
            0.25 + 0.5 * ((x + y) as f32 / (w + h - 2).max(1) as f32),
            1.0,
        ]
    })
}

pub fn psnr(a: &Image, b: &Image) -> f64 {
    assert_eq!((a.width, a.height), (b.width, b.height));
    let mut se = 0.0f64;
    let mut n = 0usize;
    for (pa, pb) in a.data.chunks(4).zip(b.data.chunks(4)) {
        for c in 0..3 {
            let d = (pa[c] - pb[c]) as f64;
            se += d * d;
            n += 1;
        }
    }
    let mse = se / n as f64;
    if mse == 0.0 {
        return f64::INFINITY;
    }
    10.0 * (1.0 / mse).log10()
}

pub fn max_abs_diff(a: &Image, b: &Image) -> f32 {
    assert_eq!((a.width, a.height), (b.width, b.height));
    a.data
        .iter()
        .zip(&b.data)
        .map(|(x, y)| (x - y).abs())
        .fold(0.0, f32::max)
}
