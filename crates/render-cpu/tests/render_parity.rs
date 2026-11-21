//! Render parity at level C against tests/golden/render (docs/specs/RENDER.md R14).
//!
//! The reference renderer is not a path tracer, so only geometry is compared: the silhouette of
//! the objects must agree within 1 px. Brightness is reported, never asserted (the reference's
//! default light and lighting model are not observed).

mod common;

use common::golden::{case_names, load, root};
use forge_parity_harness::{CompareOptions, ImageBuf, compare_image};
use forge_render_api::*;
use forge_render_cpu::CpuEngine;

/// The reference silhouette: any channel above this linear value. Its 2 px wide cubic pixel
/// filter spreads a bright edge up to two pixels at a few percent of its value; this cuts the halo.
const REFERENCE_THRESHOLD: f32 = 0.05;

/// Cases whose silhouette cannot be compared, with the reason (the spec lists them too).
const UNSUPPORTED: &[(&str, &str)] = &[(
    "mat-metal",
    "silhouette not observable: with the colour channel off only 725 px of the light's reflection are above black",
)];

/// Cases that build but differ; asserted to still differ.
const KNOWN_MISMATCH: &[(&str, &str)] = &[
    (
        "area-soft",
        "the case file records light_type Infinite (rotation 0, travelling along +Z) although the form says Area: the plane stays dark in our render",
    ),
    (
        "mat-checker-uv",
        "the black squares are 0 in the reference, so its luminance silhouette is not the object's; the pattern grid is the same size but its phase differs (see the pattern test)",
    ),
    ("mat-checker-cubic", "as mat-checker-uv"),
];

/// Cases the form lists but whose renders were not observed (black): the folders must stay absent
/// until the observation is redone.
const NOT_OBSERVED: &[&str] = &["spot", "infinite"];

fn settings() -> RenderSettings {
    RenderSettings {
        filter: PixelFilter::Cubic,
        filter_width: 2.0,
        aovs: vec![Aov::Albedo],
        ..RenderSettings::new(320, 240, 16)
    }
}

fn render(name: &str) -> (forge_render_cpu_case::Case, RenderOutput) {
    let c = load(name);
    let out = CpuEngine::new()
        .render(&c.snapshot, &settings(), &Control::new())
        .unwrap();
    (c, out)
}

mod forge_render_cpu_case {
    pub use crate::common::golden::Case;
}

/// Silhouette mismatch beyond 1 px between the reference and our coverage.
fn silhouette(name: &str) -> (usize, f64, f64) {
    let (c, out) = render(name);
    let mut e = c.expected.clone();
    e.alpha = Some(
        e.mask(REFERENCE_THRESHOLD)
            .iter()
            .map(|b| if *b { 1.0 } else { 0.0 })
            .collect(),
    );
    let act = ImageBuf::from_rgba(320, 240, &out.beauty.data);
    let r = compare_image(
        &e,
        &act,
        &CompareOptions {
            silhouette_threshold: 0.5,
            ..CompareOptions::default()
        },
    )
    .unwrap();
    // Tone, for the log only.
    let tone = compare_image(&c.expected, &act, &CompareOptions::default()).unwrap();
    (r.silhouette_mismatch_px, r.silhouette_iou, tone.psnr)
}

#[test]
fn silhouettes_match_within_one_pixel() {
    let mut checked = 0;
    for name in case_names() {
        if UNSUPPORTED.iter().any(|(n, _)| *n == name) {
            continue;
        }
        let (mism, iou, psnr) = silhouette(&name);
        println!("{name:22} mismatch {mism:5} iou {iou:.4} psnr {psnr:.1} dB (not asserted)");
        if KNOWN_MISMATCH.iter().any(|(n, _)| *n == name) {
            assert!(
                mism > 0,
                "{name} now matches: remove it from KNOWN_MISMATCH"
            );
        } else {
            assert_eq!(mism, 0, "{name}: silhouette differs by more than 1 px");
            checked += 1;
        }
    }
    assert!(checked >= 14, "only {checked} cases were compared");
}

#[test]
fn checker_pattern_is_pinned() {
    // The albedo AOV shows the checker pattern independent of lighting. Compare its white
    // squares with the reference's lit squares: same grid, different phase (KNOWN_MISMATCH).
    for name in ["mat-checker-uv", "mat-checker-cubic"] {
        let (c, out) = render(name);
        let al = &out.aovs[&Aov::Albedo];
        let act = ImageBuf::from_fn(320, 240, |x, y| {
            let p = al.pixel(x, y);
            let v = if p[3] > 0.5 && p[0] > 0.1 { 1.0 } else { 0.0 };
            [v, v, v]
        });
        let r = compare_image(
            &c.expected,
            &act,
            &CompareOptions {
                silhouette_threshold: REFERENCE_THRESHOLD,
                ..CompareOptions::default()
            },
        )
        .unwrap();
        println!("{name} pattern mismatch {}", r.silhouette_mismatch_px);
        assert!(
            r.silhouette_mismatch_px > 500,
            "{name} pattern now matches: remove it from KNOWN_MISMATCH"
        );
    }
}

#[test]
fn unobserved_renders_stay_absent() {
    for n in NOT_OBSERVED {
        assert!(
            !root().join(n).exists(),
            "{n} now exists: write its test (observation needs redoing was the reason)"
        );
    }
}

#[test]
fn every_golden_case_is_listed_or_checked() {
    let names = case_names();
    assert!(names.len() >= 19, "{names:?}");
    for (n, why) in UNSUPPORTED.iter().chain(KNOWN_MISMATCH) {
        assert!(names.iter().any(|c| c == n), "{n} not in the golden folder");
        assert!(!why.is_empty());
    }
}
