//! Image comparison for the render parity tests (docs/specs/RENDER.md R5).

use std::fmt;

/// Linear RGB pixels (row 0 at the top) with an optional coverage plane.
#[derive(Debug, Clone, PartialEq)]
pub struct ImageBuf {
    pub width: u32,
    pub height: u32,
    /// `width * height * 3` samples.
    pub rgb: Vec<f32>,
    /// `width * height` coverage values; when present it defines the silhouette.
    pub alpha: Option<Vec<f32>>,
}

impl ImageBuf {
    pub fn from_fn(width: u32, height: u32, mut f: impl FnMut(u32, u32) -> [f32; 3]) -> ImageBuf {
        let mut rgb = Vec::with_capacity(width as usize * height as usize * 3);
        for y in 0..height {
            for x in 0..width {
                rgb.extend_from_slice(&f(x, y));
            }
        }
        ImageBuf {
            width,
            height,
            rgb,
            alpha: None,
        }
    }

    /// From interleaved RGBA floats (alpha becomes the coverage plane).
    pub fn from_rgba(width: u32, height: u32, rgba: &[f32]) -> ImageBuf {
        let n = width as usize * height as usize;
        let mut rgb = Vec::with_capacity(n * 3);
        let mut alpha = Vec::with_capacity(n);
        for p in rgba.as_chunks::<4>().0.iter().take(n) {
            rgb.extend_from_slice(&p[..3]);
            alpha.push(p[3]);
        }
        ImageBuf {
            width,
            height,
            rgb,
            alpha: Some(alpha),
        }
    }

    /// Whether pixel `i` belongs to the silhouette.
    fn covered(&self, i: usize, threshold: f32) -> bool {
        match &self.alpha {
            Some(a) => a[i] > threshold,
            None => self.rgb[i * 3..i * 3 + 3].iter().any(|&v| v > threshold),
        }
    }

    pub fn mask(&self, threshold: f32) -> Vec<bool> {
        (0..self.width as usize * self.height as usize)
            .map(|i| self.covered(i, threshold))
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ImageError {
    Size {
        expected: (u32, u32),
        actual: (u32, u32),
    },
    Data(String),
}

impl fmt::Display for ImageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ImageError::Size { expected, actual } => write!(
                f,
                "image size {}x{} differs from the expected {}x{}",
                actual.0, actual.1, expected.0, expected.1
            ),
            ImageError::Data(s) => f.write_str(s),
        }
    }
}

impl std::error::Error for ImageError {}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CompareOptions {
    /// Scale `actual` by the least-squares gain before the PSNR.
    pub normalise_exposure: bool,
    /// A pixel is in the silhouette when its alpha / largest component is above this.
    pub silhouette_threshold: f32,
    /// Tolerance of `silhouette_mismatch_px`, in pixels (Chebyshev).
    pub tolerance_px: u32,
}

impl Default for CompareOptions {
    fn default() -> Self {
        CompareOptions {
            normalise_exposure: true,
            silhouette_threshold: 0.0,
            tolerance_px: 1,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ImageReport {
    pub psnr: f64,
    pub exposure_gain: f64,
    pub silhouette_iou: f64,
    pub silhouette_mismatch_px: usize,
    pub expected_pixels: usize,
    pub actual_pixels: usize,
}

/// Pixels of `a` with no pixel of `b` within `tol` pixels (Chebyshev).
fn unmatched(a: &[bool], b: &[bool], w: usize, h: usize, tol: usize) -> usize {
    // Dilate b separably by tol, then count a & !dilated.
    let dil = |m: &[bool]| -> Vec<bool> {
        let mut horiz = vec![false; w * h];
        for y in 0..h {
            for x in 0..w {
                let lo = x.saturating_sub(tol);
                let hi = (x + tol).min(w - 1);
                horiz[y * w + x] = (lo..=hi).any(|xx| m[y * w + xx]);
            }
        }
        let mut out = vec![false; w * h];
        for y in 0..h {
            let lo = y.saturating_sub(tol);
            let hi = (y + tol).min(h - 1);
            for x in 0..w {
                out[y * w + x] = (lo..=hi).any(|yy| horiz[yy * w + x]);
            }
        }
        out
    };
    let d = dil(b);
    a.iter().zip(&d).filter(|(a, d)| **a && !**d).count()
}

/// Compares two images: PSNR after exposure normalisation, silhouette IoU and the number of
/// silhouette pixels that are further than the tolerance from the other silhouette.
pub fn compare_image(
    expected: &ImageBuf,
    actual: &ImageBuf,
    opts: &CompareOptions,
) -> Result<ImageReport, ImageError> {
    if (expected.width, expected.height) != (actual.width, actual.height) {
        return Err(ImageError::Size {
            expected: (expected.width, expected.height),
            actual: (actual.width, actual.height),
        });
    }
    let n = expected.width as usize * expected.height as usize;
    if expected.rgb.len() != n * 3 || actual.rgb.len() != n * 3 {
        return Err(ImageError::Data("rgb plane has the wrong length".into()));
    }
    let (mut sae, mut saa) = (0.0f64, 0.0f64);
    for (e, a) in expected.rgb.iter().zip(&actual.rgb) {
        sae += *a as f64 * *e as f64;
        saa += *a as f64 * *a as f64;
    }
    let gain = if !opts.normalise_exposure {
        1.0
    } else if saa > 0.0 {
        sae / saa
    } else {
        0.0
    };
    let mse = expected
        .rgb
        .iter()
        .zip(&actual.rgb)
        .map(|(e, a)| {
            let d = *e as f64 - *a as f64 * gain;
            d * d
        })
        .sum::<f64>()
        / (n * 3).max(1) as f64;
    let psnr = if mse <= 0.0 {
        f64::INFINITY
    } else {
        -10.0 * mse.log10()
    };
    let em = expected.mask(opts.silhouette_threshold);
    let am = actual.mask(opts.silhouette_threshold);
    let inter = em.iter().zip(&am).filter(|(a, b)| **a && **b).count();
    let union = em.iter().zip(&am).filter(|(a, b)| **a || **b).count();
    let (w, h) = (expected.width as usize, expected.height as usize);
    let tol = opts.tolerance_px as usize;
    let mismatch = if n == 0 {
        0
    } else {
        unmatched(&em, &am, w, h, tol) + unmatched(&am, &em, w, h, tol)
    };
    Ok(ImageReport {
        psnr,
        exposure_gain: gain,
        silhouette_iou: if union == 0 {
            1.0
        } else {
            inter as f64 / union as f64
        },
        silhouette_mismatch_px: mismatch,
        expected_pixels: em.iter().filter(|v| **v).count(),
        actual_pixels: am.iter().filter(|v| **v).count(),
    })
}
