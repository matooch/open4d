mod common;
use forge_image::*;

fn ref_decode(v: f64) -> f64 {
    if v <= 0.04045 {
        v / 12.92
    } else {
        ((v + 0.055) / 1.055).powf(2.4)
    }
}

#[test]
fn known_values() {
    assert_eq!(srgb_to_linear(0.0), 0.0);
    assert!((srgb_to_linear(1.0) - 1.0).abs() < 1e-7);
    assert!((srgb_to_linear(0.5) - 0.214_041_14).abs() < 1e-6);
    assert!((linear_to_srgb(0.214_041_14) - 0.5).abs() < 1e-6);
    // mid grey 18 %: sRGB code 118/255 is the usual rounding
    assert_eq!(linear_to_srgb8(0.18), 118);
}

#[test]
fn matches_reference_formula_and_is_continuous() {
    let mut worst = 0.0f64;
    for i in 0..=4096 {
        let v = i as f32 / 4096.0;
        worst = worst.max((srgb_to_linear(v) as f64 - ref_decode(v as f64)).abs());
    }
    assert!(worst < 1e-6, "{worst}");
    let below = srgb_to_linear(0.04045 - 1e-6);
    let above = srgb_to_linear(0.04045 + 1e-6);
    assert!((above - below).abs() < 1e-6);
    let below = linear_to_srgb(0.0031308 - 1e-8);
    let above = linear_to_srgb(0.0031308 + 1e-8);
    assert!((above - below).abs() < 1e-5);
}

#[test]
fn inverse_over_range_and_odd_extension() {
    for i in 0..=8192 {
        let v = i as f32 / 8192.0;
        assert!((linear_to_srgb(srgb_to_linear(v)) - v).abs() < 1e-6, "{v}");
        assert!((srgb_to_linear(linear_to_srgb(v)) - v).abs() < 1e-6, "{v}");
    }
    for v in [4.0f32, 16.5, -0.2, -3.0] {
        assert!((srgb_to_linear(linear_to_srgb(v)) - v).abs() < 1e-4 * v.abs().max(1.0));
    }
    assert_eq!(srgb_to_linear(-0.5), -srgb_to_linear(0.5));
}

#[test]
fn eight_bit_table_is_exact() {
    for i in 0..=255u8 {
        let t = srgb8_to_linear(i) as f64;
        assert!((t - ref_decode(i as f64 / 255.0)).abs() < 1e-7);
        assert_eq!(linear_to_srgb8(srgb8_to_linear(i)), i);
    }
    assert_eq!(linear_to_srgb8(-1.0), 0);
    assert_eq!(linear_to_srgb8(7.0), 255);
    assert_eq!(linear_to_srgb8(f32::NAN), 0);
}

#[test]
fn colour_space_strings() {
    for s in [ColorSpace::Srgb, ColorSpace::Linear, ColorSpace::Raw] {
        assert_eq!(ColorSpace::parse(s.as_str()), Some(s));
    }
    assert_eq!(ColorSpace::parse("SRGB"), Some(ColorSpace::Srgb));
    assert_eq!(ColorSpace::parse("aces"), None);
}

#[test]
fn premultiply_round_trip_and_zero_alpha() {
    let mut px = vec![0.5, 0.25, 1.0, 0.5, 0.7, 0.7, 0.7, 0.0];
    premultiply(&mut px);
    assert_eq!(&px[..4], &[0.25, 0.125, 0.5, 0.5]);
    unpremultiply(&mut px);
    assert_eq!(&px[..4], &[0.5, 0.25, 1.0, 0.5]);
    // alpha 0: colour is lost, stays finite (0)
    assert_eq!(&px[4..], &[0.0, 0.0, 0.0, 0.0]);
}

#[test]
fn alpha_mode_conversion_is_lossless_for_opaque_and_partial() {
    let img = common::gradient(5, 4);
    let mut p = img.clone();
    p.set_alpha_mode(AlphaMode::Premultiplied);
    assert_eq!(p.alpha, AlphaMode::Premultiplied);
    p.set_alpha_mode(AlphaMode::Straight);
    assert!(common::max_abs_diff(&img, &p) < 1e-6);
}
