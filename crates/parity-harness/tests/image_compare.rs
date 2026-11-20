//! compare_image (docs/specs/RENDER.md R5).

use forge_parity_harness::{CompareOptions, ImageBuf, ImageError, compare_image};

fn solid(w: u32, h: u32, v: f32) -> ImageBuf {
    ImageBuf::from_fn(w, h, |_, _| [v, v, v])
}

fn square(w: u32, h: u32, x0: u32, y0: u32, s: u32, v: f32) -> ImageBuf {
    ImageBuf::from_fn(w, h, |x, y| {
        if x >= x0 && x < x0 + s && y >= y0 && y < y0 + s {
            [v; 3]
        } else {
            [0.0; 3]
        }
    })
}

#[test]
fn identical_images() {
    let a = square(16, 16, 4, 4, 6, 0.5);
    let r = compare_image(&a, &a, &CompareOptions::default()).unwrap();
    assert!(r.psnr.is_infinite());
    assert_eq!(r.silhouette_iou, 1.0);
    assert_eq!(r.silhouette_mismatch_px, 0);
    assert!((r.exposure_gain - 1.0).abs() < 1e-12);
}

#[test]
fn exposure_is_normalised_away() {
    let e = square(16, 16, 2, 2, 8, 0.6);
    let a = square(16, 16, 2, 2, 8, 0.3);
    let r = compare_image(&e, &a, &CompareOptions::default()).unwrap();
    assert!((r.exposure_gain - 2.0).abs() < 1e-9, "{}", r.exposure_gain);
    assert!(r.psnr > 100.0, "{}", r.psnr);
}

#[test]
fn psnr_of_a_known_error() {
    // Expected 0.5, actual 0.5 + 0.1 on every sample, no normalisation freedom wanted:
    // with the gain fixed at 1 the mse is 0.01 and the PSNR 20 dB.
    let e = solid(8, 8, 0.5);
    let a = solid(8, 8, 0.6);
    let o = CompareOptions {
        normalise_exposure: false,
        ..CompareOptions::default()
    };
    let r = compare_image(&e, &a, &o).unwrap();
    assert!((r.psnr - 20.0).abs() < 1e-3, "{}", r.psnr);
    assert_eq!(r.exposure_gain, 1.0);
}

#[test]
fn silhouette_shifted_by_one_pixel_is_within_tolerance() {
    let e = square(24, 24, 6, 6, 10, 0.8);
    let a = square(24, 24, 7, 6, 10, 0.8);
    let r = compare_image(&e, &a, &CompareOptions::default()).unwrap();
    assert!(r.silhouette_iou < 1.0);
    assert_eq!(r.silhouette_mismatch_px, 0);
}

#[test]
fn silhouette_shifted_by_three_pixels_is_not() {
    let e = square(24, 24, 6, 6, 10, 0.8);
    let a = square(24, 24, 9, 6, 10, 0.8);
    let r = compare_image(&e, &a, &CompareOptions::default()).unwrap();
    assert!(r.silhouette_mismatch_px > 0);
    // Expected columns 6 and 7 and actual columns 17 and 18 have no pixel of the other mask
    // within 1 px: 2 columns of 10 rows on each side.
    assert_eq!(r.silhouette_mismatch_px, 40);
}

#[test]
fn both_empty_is_a_match() {
    let e = solid(4, 4, 0.0);
    let r = compare_image(&e, &e, &CompareOptions::default()).unwrap();
    assert_eq!(r.silhouette_iou, 1.0);
    assert_eq!(r.expected_pixels, 0);
}

#[test]
fn alpha_plane_defines_the_silhouette() {
    let mut a = solid(8, 8, 0.0);
    // Dark but covered pixels: only the alpha plane says they belong to the object.
    a.alpha = Some(
        (0..64)
            .map(|i| if (i % 8) < 4 { 1.0 } else { 0.0 })
            .collect(),
    );
    let e = ImageBuf::from_fn(8, 8, |x, _| if x < 4 { [0.2; 3] } else { [0.0; 3] });
    let r = compare_image(&e, &a, &CompareOptions::default()).unwrap();
    assert_eq!(r.silhouette_iou, 1.0);
}

#[test]
fn size_mismatch_is_an_error() {
    let r = compare_image(
        &solid(4, 4, 0.0),
        &solid(4, 5, 0.0),
        &CompareOptions::default(),
    );
    assert!(matches!(r, Err(ImageError::Size { .. })));
}
