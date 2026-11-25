//! The tier-1 post chain (docs/specs/RENDER.md R22): tone mapping, bloom, the TAA subpixel
//! jitter and the subframe average.
//!
//! SSAO (R22.2) runs inside the lighting pass and SSR (R22.3) is a full-screen pass, because
//! both need the G-buffer before the reading; the effects here work on the linear beauty the
//! engine reads back, so they are plain Rust functions a test can call.

use forge_render_api::PostSettings;

/// Luminance of a linear RGB triple (Rec. 709 weights, as the R3 shadow rule uses).
pub fn luminance([r, g, b]: [f32; 3]) -> f32 {
    0.2126 * r + 0.7152 * g + 0.0722 * b
}

/// The ACES filmic fit (K. Narkowicz, 2016, public literature) with the exposure of R22.1
/// (R22.6): `x' = x (2.51 x + 0.03) / (x (2.43 x + 0.59) + 0.14)`, `x = exposure * linear`,
/// clamped to `[0, 1]`. The engine applies exactly this to the beauty RGB (never to the alpha
/// or an AOV), so a test can hold it against the pixels.
pub fn tonemap(x: f32, exposure: f32) -> f32 {
    let v = x.max(0.0) * exposure;
    ((v * (2.51 * v + 0.03)) / (v * (2.43 * v + 0.59) + 0.14)).clamp(0.0, 1.0)
}

/// Applies [`tonemap`] to the RGB of every pixel, leaving the alpha (coverage) alone.
pub fn tonemap_image(pixels: &mut [[f32; 4]], post: &PostSettings) {
    let exposure = post.exposure as f32;
    for p in pixels.iter_mut() {
        for c in &mut p[..3] {
            *c = tonemap(*c, exposure);
        }
    }
}

/// The Halton point of index `i` (1-based) in `base`, in `[0, 1)`.
fn halton(mut i: u32, base: u32) -> f32 {
    let mut f = 1.0;
    let mut r = 0.0;
    while i > 0 {
        f /= base as f32;
        r += f * (i % base) as f32;
        i /= base;
    }
    r
}

/// The subpixel jitter of subframe `i` (1-based, R22.5): the Halton(2,3) point centred on the
/// pixel, so the samples of every subframe stay inside one pixel.
pub fn halton_jitter(i: u32) -> [f32; 2] {
    [halton(i, 2) - 0.5, halton(i, 3) - 0.5]
}

/// Adds `frame` to `acc` with the weight `1/n` (R22.5): `n` subframes then hold their mean in
/// linear radiance. `acc` is created on the first call.
pub fn accumulate(acc: &mut Option<Vec<[f32; 4]>>, frame: &[[f32; 4]], n: usize) {
    let weight = 1.0 / n.max(1) as f32;
    match acc {
        Some(a) => {
            for (dst, src) in a.iter_mut().zip(frame) {
                for k in 0..4 {
                    dst[k] += src[k] * weight;
                }
            }
        }
        None => {
            *acc = Some(
                frame
                    .iter()
                    .map(|p| [p[0] * weight, p[1] * weight, p[2] * weight, p[3] * weight])
                    .collect(),
            );
        }
    }
}

/// The nine Gaussian taps of R22.4 (σ = 8 px), normalised.
fn bloom_weights() -> [f32; 9] {
    let sigma = 8.0f32;
    let mut w = [0.0f32; 9];
    for (i, v) in w.iter_mut().enumerate() {
        let d = i as f32 - 4.0;
        *v = (-(d * d) / (2.0 * sigma * sigma)).exp();
    }
    let sum: f32 = w.iter().sum();
    for v in &mut w {
        *v /= sum;
    }
    w
}

/// Bloom (R22.4): the bright pass over covered pixels, one separable 9-tap Gaussian
/// (σ = 8 px, edge clamp) in each direction, added back with `bloom_strength`.
pub fn bloom(pixels: &mut [[f32; 4]], width: u32, height: u32, post: &PostSettings) {
    let (w, h) = (width as usize, height as usize);
    if w == 0 || h == 0 || pixels.len() != w * h {
        return;
    }
    let threshold = post.bloom_threshold as f32;
    // bright pass: the part of a covered pixel's radiance above the threshold
    let mut bright = vec![[0.0f32; 3]; w * h];
    for (i, p) in pixels.iter().enumerate() {
        if p[3] <= 0.5 {
            continue;
        }
        let l = luminance([p[0], p[1], p[2]]);
        if l <= threshold {
            continue;
        }
        let k = (l - threshold) / l.max(1e-6);
        bright[i] = [p[0] * k, p[1] * k, p[2] * k];
    }
    let taps = bloom_weights();
    // horizontal, then vertical; the edges clamp to the nearest texel
    let mut tmp = vec![[0.0f32; 3]; w * h];
    for y in 0..h {
        for x in 0..w {
            let mut sum = [0.0f32; 3];
            for (k, weight) in taps.iter().enumerate() {
                let sx = (x as isize + k as isize - 4).clamp(0, w as isize - 1) as usize;
                let c = bright[y * w + sx];
                for ch in 0..3 {
                    sum[ch] += c[ch] * weight;
                }
            }
            tmp[y * w + x] = sum;
        }
    }
    let mut blurred = vec![[0.0f32; 3]; w * h];
    for y in 0..h {
        for x in 0..w {
            let mut sum = [0.0f32; 3];
            for (k, weight) in taps.iter().enumerate() {
                let sy = (y as isize + k as isize - 4).clamp(0, h as isize - 1) as usize;
                let c = tmp[sy * w + x];
                for ch in 0..3 {
                    sum[ch] += c[ch] * weight;
                }
            }
            blurred[y * w + x] = sum;
        }
    }
    let strength = post.bloom_strength as f32;
    for (p, b) in pixels.iter_mut().zip(&blurred) {
        for ch in 0..3 {
            p[ch] += b[ch] * strength;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tonemap_is_the_aces_fit() {
        assert_eq!(tonemap(0.0, 1.0), 0.0);
        // the middle of the fit: x = 1 maps to 0.8038..., x = 4 stays inside the display range
        assert!((tonemap(1.0, 1.0) - 2.54 / 3.16).abs() < 1e-6);
        assert!(tonemap(4.0, 1.0) < 1.0 && tonemap(4.0, 1.0) > 0.95);
        assert!(tonemap(1e6, 1.0) <= 1.0);
        // exposure multiplies before the curve
        assert_eq!(tonemap(1.0, 2.0), tonemap(2.0, 1.0));
        assert_eq!(tonemap(-1.0, 1.0), 0.0);
    }

    #[test]
    fn jitter_stays_inside_the_pixel_and_repeats() {
        let j: Vec<[f32; 2]> = (1..=8).map(halton_jitter).collect();
        for p in &j {
            assert!(p.iter().all(|v| (-0.5..0.5).contains(v)));
        }
        assert_ne!(j[0], j[1]);
        assert_eq!(j[0], halton_jitter(1));
        // distinct subpixels: no two of the first eight coincide
        for (i, a) in j.iter().enumerate() {
            for b in &j[i + 1..] {
                assert!(a != b);
            }
        }
    }

    #[test]
    fn accumulation_is_a_mean() {
        let a = [[1.0f32, 2.0, 3.0, 1.0], [4.0, 5.0, 6.0, 0.0]];
        let b = [[3.0f32, 4.0, 5.0, 0.0], [8.0, 9.0, 10.0, 2.0]];
        let mut acc = None;
        accumulate(&mut acc, &a, 2);
        accumulate(&mut acc, &b, 2);
        let mean = acc.unwrap();
        assert_eq!(mean[0], [2.0, 3.0, 4.0, 0.5]);
        assert_eq!(mean[1], [6.0, 7.0, 8.0, 1.0]);
    }

    #[test]
    fn bloom_spreads_above_the_threshold_only() {
        let (w, h) = (9u32, 9u32);
        let mut px = vec![[0.0f32; 4]; (w * h) as usize];
        px[4 * 9 + 4] = [4.0, 4.0, 4.0, 1.0];
        let post = PostSettings {
            bloom: true,
            bloom_strength: 0.25,
            ..PostSettings::default()
        };
        let mut out = px.clone();
        bloom(&mut out, w, h, &post);
        assert!(out[4 * 9 + 4][0] > 4.0 && out[4 * 9 + 4][0] < 4.5);
        assert!(out[4 * 9 + 5][0] > 0.0 && out[4 * 9 + 5][0] < 1.0);
        // the 9-tap kernel only reaches four pixels past the bright one
        assert!(out[0][0] < 0.01);
        // a dim pixel (below the threshold) does not bloom and is not bloomed
        let mut dim = vec![[0.0f32; 4]; (w * h) as usize];
        dim[4 * 9 + 4] = [0.5, 0.5, 0.5, 1.0];
        let mut out = dim.clone();
        bloom(&mut out, w, h, &post);
        assert_eq!(out, dim);
    }

    #[test]
    fn tonemap_leaves_the_alpha_alone() {
        let post = PostSettings {
            tonemap: true,
            ..PostSettings::default()
        };
        let mut px = vec![[4.0f32, 1.0, 0.5, 0.0], [0.1, 0.2, 0.3, 1.0]];
        tonemap_image(&mut px, &post);
        assert_eq!(px[0][3], 0.0);
        assert_eq!(px[1][3], 1.0);
        assert!(px[0][0] > 0.9 && px[0][0] <= 1.0);
    }
}
