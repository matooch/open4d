//! R25.2: the preview filter operates on linear beauty without modifying the raw frame.

use forge_image::{AlphaMode, Image};
use forge_render_rt::path_preview::{PrimaryGuide, denoise};

const FLAT: PrimaryGuide = PrimaryGuide::Hit {
    depth: 10.0,
    normal: [0.0, 1.0, 0.0],
};

#[test]
fn first_batch_denoises_hdr_without_feeding_back_into_raw() {
    let raw = Image::from_fn(3, 3, AlphaMode::Straight, |x, y| {
        if (x, y) == (1, 1) {
            [8.0, 4.0, 2.0, 0.75]
        } else {
            [0.0, 0.0, 0.0, 0.75]
        }
    });
    let guides = [FLAT; 9];
    let first = denoise(&raw, &guides, 1).unwrap();
    let later = denoise(&raw, &guides, 16).unwrap();
    assert_eq!(first.pixel(1, 1), [2.0, 1.0, 0.5, 0.75]);
    assert_eq!(later.pixel(1, 1), [6.5, 3.25, 1.625, 0.75]);
    assert_eq!(raw.pixel(1, 1), [8.0, 4.0, 2.0, 0.75]);
}

#[test]
fn guides_isolate_depth_normal_coverage_and_region_boundaries() {
    let raw = Image::from_fn(3, 2, AlphaMode::Straight, |x, y| {
        let a = if (x, y) == (2, 0) { 0.5 } else { 1.0 };
        [10.0 * (3 * y + x) as f32, 0.0, 0.0, a]
    });
    let guides = [
        PrimaryGuide::Hit {
            depth: 20.0,
            normal: [0.0, 1.0, 0.0],
        },
        FLAT,
        FLAT,
        PrimaryGuide::Hit {
            depth: 10.0,
            normal: [1.0, 0.0, 0.0],
        },
        PrimaryGuide::Miss,
        PrimaryGuide::Inactive,
    ];
    // The centre has a neighbour behind an edge of each kind. All must be excluded.
    assert_eq!(denoise(&raw, &guides, 1).unwrap(), raw);
}

#[test]
fn incomplete_batches_are_not_preview_frames() {
    let raw = Image::new(1, 1, AlphaMode::Straight);
    assert!(denoise(&raw, &[FLAT], 0).is_err());
    assert!(denoise(&raw, &[], 1).is_err());
}
