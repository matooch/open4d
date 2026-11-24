//! Render parity at level C against tests/golden/render (docs/specs/RENDER.md R14).
//!
//! The reference renderer is not a path tracer, so only geometry is compared: the silhouette of
//! the objects must agree within 1 px. Brightness is reported, never asserted (the reference's
//! default light and lighting model are not observed).

mod common;

use common::golden::{case_names, load, root};
use common::image_parity::{self, REFERENCE_THRESHOLD};
use forge_parity_harness::{CompareOptions, ImageBuf, compare_image};
use forge_render_api::*;
use forge_render_cpu::CpuEngine;

/// Cases whose silhouette cannot be compared, with the reason (the spec lists them too).
const UNSUPPORTED: &[(&str, &str)] = &[(
    "mat-metal",
    "silhouette not observable: with the colour channel off only 725 px of the light's reflection are above black",
)];

/// Cases that build but differ; asserted to still differ.
const KNOWN_MISMATCH: &[(&str, &str)] = &[
    (
        "area-soft",
        "the case file records light_type Infinite, but area-soft-2 shows the light was an Area \
         (Rectangle) light: its render is byte-identical. Our Area light lights the plane \
         differently (see area-soft-2)",
    ),
    (
        "area-soft-2",
        "Area light (Rectangle, read back before rendering) at (300, 400, -300) aimed at the origin: \
         1500 px of the silhouette differ; the reference's area-light orientation or size rule is \
         not derived yet (RENDER.md, lights)",
    ),
    (
        "mat-checker-uv",
        "the black squares are 0 in the reference, so its luminance silhouette is not the object's; the pattern grid is the same size but its phase differs (see the pattern test)",
    ),
    ("mat-checker-cubic", "as mat-checker-uv"),
    (
        "checker-plane",
        "as mat-checker-uv: the reference's black squares are 0, so its luminance silhouette \
         loses edge pixels (1 px differs); the pattern phase is checked by the pattern test",
    ),
];

/// Cases the form lists but whose renders were not observed (black): the folders must stay absent
/// until the observation is redone.
const NOT_OBSERVED: &[&str] = &["spot", "infinite"];

fn render(name: &str) -> (forge_render_cpu_case::Case, RenderOutput) {
    let c = load(name);
    let out = CpuEngine::new()
        .render(
            &c.snapshot,
            &image_parity::settings((320, 240), vec![Aov::Albedo]),
            &Control::new(),
        )
        .unwrap();
    (c, out)
}

mod forge_render_cpu_case {
    pub use crate::common::golden::Case;
}

/// Silhouette mismatch beyond 1 px between the reference and our coverage.
fn silhouette(name: &str) -> (usize, f64, f64) {
    let (c, out) = render(name);
    let act = ImageBuf::from_rgba(320, 240, &out.beauty.data);
    let r = image_parity::silhouette(&c.expected, &act);
    let tone = image_parity::psnr(&root().join(name), &c.expected, &act);
    (r.silhouette_mismatch_px, r.silhouette_iou, tone)
}

#[test]
fn silhouettes_match_within_one_pixel() {
    let mut checked = 0;
    for name in case_names() {
        let (mism, iou, psnr) = silhouette(&name);
        println!("{name:22} mismatch {mism:5} iou {iou:.4} psnr {psnr:.1} dB");
        if let Some((_, why)) = UNSUPPORTED.iter().find(|(n, _)| *n == name) {
            println!("  silhouette unsupported: {why}");
        } else if KNOWN_MISMATCH.iter().any(|(n, _)| *n == name) {
            assert!(
                mism > 0,
                "{name} now matches: remove it from KNOWN_MISMATCH"
            );
        } else {
            assert_eq!(mism, 0, "{name}: silhouette differs by more than 1 px");
            checked += 1;
        }
    }
    assert_eq!(checked, 17);
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
    assert_eq!(names.len(), 23, "{names:?}");
    for n in &names {
        assert!(
            root().join(n).join("render.png").is_file(),
            "{n}: missing render"
        );
    }
    for (n, why) in UNSUPPORTED.iter().chain(KNOWN_MISMATCH) {
        assert!(names.iter().any(|c| c == n), "{n} not in the golden folder");
        assert!(!why.is_empty());
    }
}
