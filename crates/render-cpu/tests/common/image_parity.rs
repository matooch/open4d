//! Shared measurement policy for O30, O43, and O57.

use std::path::Path;

use forge_parity_harness::{CompareOptions, ImageBuf, ImageReport, compare_image};
use forge_render_api::{Aov, PixelFilter, RenderSettings};

pub const REFERENCE_THRESHOLD: f32 = 0.05;

pub fn settings((w, h): (u32, u32), aovs: Vec<Aov>) -> RenderSettings {
    RenderSettings {
        filter: PixelFilter::Cubic,
        filter_width: 2.0,
        aovs,
        ..RenderSettings::new(w, h, 16)
    }
}

pub fn silhouette(expected: &ImageBuf, actual: &ImageBuf) -> ImageReport {
    let mut mask = expected.clone();
    mask.alpha = Some(
        expected
            .mask(REFERENCE_THRESHOLD)
            .into_iter()
            .map(|on| if on { 1.0 } else { 0.0 })
            .collect(),
    );
    compare_image(
        &mask,
        actual,
        &CompareOptions {
            silhouette_threshold: 0.5,
            ..CompareOptions::default()
        },
    )
    .unwrap()
}

pub fn psnr(case: &Path, expected: &ImageBuf, actual: &ImageBuf) -> f64 {
    let doc: toml::Table = std::fs::read_to_string(case.join("case.toml"))
        .unwrap()
        .parse()
        .unwrap();
    let tone = compare_image(expected, actual, &CompareOptions::default()).unwrap();
    if let Some(min) = doc.get("tolerance").and_then(|t| t.get("psnr")) {
        let min = min
            .as_float()
            .or_else(|| min.as_integer().map(|x| x as f64))
            .filter(|x| x.is_finite())
            .expect("PSNR threshold must be finite");
        assert!(
            tone.psnr >= min,
            "{}: PSNR {} dB below {min} dB",
            case.display(),
            tone.psnr
        );
    }
    tone.psnr
}
