mod common;
use common::*;
use forge_image::*;

fn lim() -> Limits {
    Limits::default()
}

/// An image whose linear values are exactly the sRGB-8 decode of each code.
fn srgb8_image(w: u32, h: u32) -> (Image, Vec<u8>) {
    let mut codes = Vec::new();
    let img = Image::from_fn(w, h, AlphaMode::Straight, |x, y| {
        let r = (x * 255 / (w - 1)) as u8;
        let g = (y * 255 / (h - 1)) as u8;
        let b = ((x * 7 + y * 13) % 256) as u8;
        let a = ((x + 3 * y) * 5 % 256) as u8;
        codes.extend_from_slice(&[r, g, b, a]);
        [
            srgb8_to_linear(r),
            srgb8_to_linear(g),
            srgb8_to_linear(b),
            a as f32 / 255.0,
        ]
    });
    (img, codes)
}

fn opts(depth: BitDepth, space: ColorSpace) -> EncodeOptions {
    EncodeOptions {
        depth,
        space,
        ..EncodeOptions::default()
    }
}

#[test]
fn png_and_tiff_8bit_srgb_round_trip_exact_on_codes() {
    let (img, codes) = srgb8_image(17, 9);
    for fmt in [Format::Png, Format::Tiff] {
        let bytes = encode(&img, fmt, &opts(BitDepth::U8, ColorSpace::Srgb)).unwrap();
        assert_eq!(detect_format(&bytes), Some(fmt));
        let dec = decode(&bytes, &lim()).unwrap();
        assert_eq!(dec.info.format, fmt);
        assert_eq!(dec.info.depth, BitDepth::U8);
        assert!(dec.info.has_alpha);
        // stored values are the codes / 255
        for (i, v) in dec.image.data.iter().enumerate() {
            assert_eq!((v * 255.0).round() as u8, codes[i], "{fmt:?} {i}");
        }
        let work = dec.into_working(ColorSpace::Srgb, AlphaMode::Straight);
        assert!(max_abs_diff(&img, &work) < 1e-6, "{fmt:?}");
    }
}

#[test]
fn png_and_tiff_linear_tag_is_exact() {
    let (img, _) = srgb8_image(8, 8);
    // quantise linear values to codes/255 first so the round trip is exact
    let q = Image::from_fn(8, 8, AlphaMode::Straight, |x, y| {
        let p = img.pixel(x, y);
        [
            (p[0] * 255.0).round() / 255.0,
            (p[1] * 255.0).round() / 255.0,
            (p[2] * 255.0).round() / 255.0,
            p[3],
        ]
    });
    for fmt in [Format::Png, Format::Tiff] {
        let bytes = encode(&q, fmt, &opts(BitDepth::U8, ColorSpace::Linear)).unwrap();
        let dec = decode(&bytes, &lim()).unwrap();
        let work = dec.into_working(ColorSpace::Linear, AlphaMode::Straight);
        assert_eq!(work.data, q.data, "{fmt:?}");
    }
}

#[test]
fn png_and_tiff_16bit_round_trip_exact() {
    let img = Image::from_fn(11, 6, AlphaMode::Straight, |x, y| {
        let k = (x * 6000 + y * 777) % 65536;
        [
            k as f32 / 65535.0,
            ((k * 3) % 65536) as f32 / 65535.0,
            ((k * 5) % 65536) as f32 / 65535.0,
            ((k + 1000) % 65536) as f32 / 65535.0,
        ]
    });
    for fmt in [Format::Png, Format::Tiff] {
        let bytes = encode(&img, fmt, &opts(BitDepth::U16, ColorSpace::Linear)).unwrap();
        let dec = decode(&bytes, &lim()).unwrap();
        assert_eq!(dec.info.depth, BitDepth::U16, "{fmt:?}");
        let work = dec.into_working(ColorSpace::Linear, AlphaMode::Straight);
        assert!(max_abs_diff(&img, &work) < 1e-7, "{fmt:?}");
        // sRGB encoded 16 bit: round trip within one 16-bit step (in encoded space)
        let bytes = encode(&img, fmt, &opts(BitDepth::U16, ColorSpace::Srgb)).unwrap();
        let work = decode(&bytes, &lim())
            .unwrap()
            .into_working(ColorSpace::Srgb, AlphaMode::Straight);
        assert!(max_abs_diff(&img, &work) < 1e-4, "{fmt:?}");
    }
}

#[test]
fn jpeg_round_trip_psnr() {
    let img = gradient(32, 32);
    let bytes = encode(&img, Format::Jpeg, &opts(BitDepth::U8, ColorSpace::Srgb)).unwrap();
    assert_eq!(detect_format(&bytes), Some(Format::Jpeg));
    let dec = decode(&bytes, &lim()).unwrap();
    assert!(!dec.info.has_alpha);
    let work = dec.into_working(ColorSpace::Srgb, AlphaMode::Straight);
    let p = psnr(&img, &work);
    assert!(p > 30.0, "psnr {p}");
    assert!(work.data.chunks(4).all(|px| px[3] == 1.0));
    // lower quality is worse but still valid
    let lo = EncodeOptions {
        jpeg_quality: 10,
        ..opts(BitDepth::U8, ColorSpace::Srgb)
    };
    let work_lo = decode(&encode(&img, Format::Jpeg, &lo).unwrap(), &lim())
        .unwrap()
        .into_working(ColorSpace::Srgb, AlphaMode::Straight);
    assert!(psnr(&img, &work_lo) < p);
}

#[test]
fn hdr_round_trip_within_rgbe_precision() {
    let mut rng = Lcg(7);
    let img = Image::from_fn(12, 5, AlphaMode::Straight, |_, _| {
        [
            rng.next_f32() * 100.0 + 0.01,
            rng.next_f32() * 3.0,
            rng.next_f32() * 0.05,
            1.0,
        ]
    });
    let bytes = encode(&img, Format::Hdr, &opts(BitDepth::F32, ColorSpace::Srgb)).unwrap();
    assert_eq!(detect_format(&bytes), Some(Format::Hdr));
    let dec = decode(&bytes, &lim()).unwrap();
    assert_eq!(dec.info.depth, BitDepth::F32);
    // the tag is ignored for float files: Srgb must not touch the values
    let work = dec.into_working(ColorSpace::Srgb, AlphaMode::Straight);
    for (a, b) in img.data.chunks(4).zip(work.data.chunks(4)) {
        let m = a[0].max(a[1]).max(a[2]);
        for c in 0..3 {
            assert!((a[c] - b[c]).abs() <= 0.01 * m, "{a:?} {b:?}");
        }
        assert_eq!(b[3], 1.0);
    }
}

#[test]
fn exr_round_trip_f32_exact_f16_close_and_premultiplied_disk() {
    let mut rng = Lcg(3);
    let img = Image::from_fn(9, 7, AlphaMode::Straight, |_, _| {
        [
            rng.next_f32() * 20.0,
            rng.next_f32(),
            rng.next_f32() * 0.01,
            0.25 + rng.next_f32() * 0.75,
        ]
    });
    let bytes = encode(&img, Format::Exr, &opts(BitDepth::F32, ColorSpace::Linear)).unwrap();
    assert_eq!(detect_format(&bytes), Some(Format::Exr));
    let dec = decode(&bytes, &lim()).unwrap();
    assert_eq!(dec.image.alpha, AlphaMode::Premultiplied);
    assert!(dec.info.has_alpha);
    let back = dec.into_working(ColorSpace::Srgb, AlphaMode::Straight);
    // un-premultiply after premultiply: tiny float error only
    for (a, b) in img.data.iter().zip(&back.data) {
        assert!((a - b).abs() <= 1e-5 * a.abs().max(1.0), "{a} {b}");
    }
    // premultiplied request returns the on-disk values exactly
    let mut pre = img.clone();
    pre.set_alpha_mode(AlphaMode::Premultiplied);
    let raw = decode(&bytes, &lim())
        .unwrap()
        .into_working(ColorSpace::Linear, AlphaMode::Premultiplied);
    assert_eq!(raw.data, pre.data);

    let b16 = encode(&img, Format::Exr, &opts(BitDepth::F16, ColorSpace::Linear)).unwrap();
    assert!(b16.len() < bytes.len() + 400);
    let back16 = decode(&b16, &lim())
        .unwrap()
        .into_working(ColorSpace::Linear, AlphaMode::Straight);
    for (a, b) in img.data.iter().zip(&back16.data) {
        assert!((a - b).abs() <= 2e-3 * a.abs().max(1e-3) + 1e-4, "{a} {b}");
    }
}

#[test]
fn grey_and_missing_alpha_expand() {
    // 8-bit grey PNG made by hand through the image crate
    let mut buf = Vec::new();
    let g = ::image::GrayImage::from_fn(4, 2, |x, _| ::image::Luma([(x * 80) as u8]));
    ::image::DynamicImage::ImageLuma8(g)
        .write_to(
            &mut std::io::Cursor::new(&mut buf),
            ::image::ImageFormat::Png,
        )
        .unwrap();
    let dec = decode(&buf, &lim()).unwrap();
    assert_eq!(dec.info.channels, 1);
    assert!(!dec.info.has_alpha);
    let w = dec.into_working(ColorSpace::Linear, AlphaMode::Straight);
    let p = w.pixel(3, 1);
    assert_eq!(p[0], p[1]);
    assert_eq!(p[1], p[2]);
    assert_eq!(p[3], 1.0);
    assert!((p[0] - 240.0 / 255.0).abs() < 1e-6);
}

#[test]
fn raw_tag_never_transforms_or_converts_alpha() {
    let (img, _) = srgb8_image(6, 6);
    let bytes = encode(&img, Format::Png, &opts(BitDepth::U8, ColorSpace::Raw)).unwrap();
    let stored = decode(&bytes, &lim()).unwrap();
    let raw = stored
        .clone()
        .into_working(ColorSpace::Raw, AlphaMode::Premultiplied);
    assert_eq!(raw.data, stored.image.data);
    assert_eq!(raw.alpha, AlphaMode::Straight);
    // source_alpha override: reading a straight file as premultiplied then asking for straight
    let img2 = stored
        .clone()
        .with_source_alpha(AlphaMode::Premultiplied)
        .into_working(ColorSpace::Linear, AlphaMode::Straight);
    assert_ne!(img2.data, stored.image.data);
}

#[test]
fn unsupported_pairs_error() {
    let img = gradient(4, 4);
    assert!(matches!(
        encode(&img, Format::Png, &opts(BitDepth::F32, ColorSpace::Linear)),
        Err(ImageError::Unsupported(_))
    ));
    assert!(matches!(
        encode(&img, Format::Jpeg, &opts(BitDepth::U16, ColorSpace::Srgb)),
        Err(ImageError::Unsupported(_))
    ));
    assert!(matches!(
        encode(&img, Format::Exr, &opts(BitDepth::U8, ColorSpace::Linear)),
        Err(ImageError::Unsupported(_))
    ));
    assert!(matches!(
        encode(&img, Format::Tiff, &opts(BitDepth::F32, ColorSpace::Linear)),
        Err(ImageError::Unsupported(_))
    ));
}

#[test]
fn detection_and_extensions() {
    assert_eq!(detect_format(b"hello world"), None);
    assert_eq!(detect_format(&[]), None);
    assert!(matches!(
        decode(b"not an image at all", &lim()),
        Err(ImageError::UnknownFormat)
    ));
    assert_eq!(Format::from_extension("PNG"), Some(Format::Png));
    assert_eq!(Format::from_extension("jpeg"), Some(Format::Jpeg));
    assert_eq!(Format::from_extension("tif"), Some(Format::Tiff));
    assert_eq!(Format::from_extension("exr"), Some(Format::Exr));
    assert_eq!(Format::from_extension("hdr"), Some(Format::Hdr));
    assert_eq!(Format::from_extension("bmp"), None);
}

#[test]
fn limits_are_enforced_before_allocation() {
    let img = gradient(16, 16);
    let tight = Limits {
        max_dimension: 8,
        max_pixels: 1 << 20,
    };
    for (fmt, depth) in [
        (Format::Png, BitDepth::U8),
        (Format::Tiff, BitDepth::U8),
        (Format::Jpeg, BitDepth::U8),
        (Format::Hdr, BitDepth::F32),
        (Format::Exr, BitDepth::F32),
    ] {
        let bytes = encode(&img, fmt, &opts(depth, ColorSpace::Srgb)).unwrap();
        assert!(decode(&bytes, &tight).is_err(), "{fmt:?}");
    }
    let few = Limits {
        max_dimension: 1000,
        max_pixels: 100,
    };
    let bytes = encode(&img, Format::Png, &EncodeOptions::default()).unwrap();
    assert!(matches!(
        decode(&bytes, &few),
        Err(ImageError::TooLarge { .. })
    ));
}

#[test]
fn malformed_input_never_panics() {
    let img = gradient(12, 10);
    let mut rng = Lcg(99);
    for (fmt, depth) in [
        (Format::Png, BitDepth::U8),
        (Format::Tiff, BitDepth::U16),
        (Format::Jpeg, BitDepth::U8),
        (Format::Hdr, BitDepth::F32),
        (Format::Exr, BitDepth::F16),
    ] {
        let bytes = encode(&img, fmt, &opts(depth, ColorSpace::Srgb)).unwrap();
        // every truncation
        for n in 0..bytes.len() {
            let _ = decode(&bytes[..n], &lim());
        }
        // seeded byte flips
        for _ in 0..150 {
            let mut m = bytes.clone();
            for _ in 0..1 + (rng.next_f32() * 4.0) as usize {
                let i = (rng.next_f32() * m.len() as f32) as usize % m.len();
                m[i] = (rng.next_f32() * 256.0) as u8;
            }
            let _ = decode(&m, &lim());
            let _ = decode_exr_layers(&m, &lim());
        }
    }
    // garbage with valid signatures
    for sig in [
        &[0x89, b'P', b'N', b'G'][..],
        &[0xFF, 0xD8, 0xFF][..],
        b"II*\0",
        b"#?RADIANCE\n",
        &[0x76, 0x2F, 0x31, 0x01],
    ] {
        let mut v = sig.to_vec();
        v.extend((0..64).map(|_| (rng.next_f32() * 256.0) as u8));
        let _ = decode(&v, &lim());
    }
}
