//! Encoding the working-space image to files (docs/specs/IMAGE.md §3).

use std::io::Cursor;

use ::image::{DynamicImage, ImageBuffer, ImageFormat};

use crate::buffer::Image;
use crate::colour::{AlphaMode, ColorSpace, linear_to_srgb};
use crate::decode::{BitDepth, Format};
use crate::error::ImageError;
use crate::exr_io::{ExrChannel, ExrLayer, ExrPrecision, encode_exr_layers};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EncodeOptions {
    pub depth: BitDepth,
    /// Transfer function of integer files (`Srgb` encodes; `Linear`/`Raw` write as is).
    pub space: ColorSpace,
    /// JPEG quality 1..=100.
    pub jpeg_quality: u8,
}

impl Default for EncodeOptions {
    fn default() -> Self {
        EncodeOptions {
            depth: BitDepth::U8,
            space: ColorSpace::Srgb,
            jpeg_quality: 90,
        }
    }
}

fn unsupported(f: Format, d: BitDepth) -> ImageError {
    ImageError::Unsupported(format!("{f:?} cannot be written at {d:?}"))
}

fn quantise(v: f32, max: f32) -> f32 {
    if v.is_nan() {
        0.0
    } else {
        (v.clamp(0.0, 1.0) * max).round()
    }
}

/// Encodes colour+alpha as integer samples (`max` = 255 or 65535).
fn integer_samples(img: &Image, space: ColorSpace, max: f32, with_alpha: bool) -> Vec<f32> {
    let mut s = img.clone();
    s.set_alpha_mode(AlphaMode::Straight);
    let stride = if with_alpha { 4 } else { 3 };
    let mut out = Vec::with_capacity(s.data.len() / 4 * stride);
    for p in s.data.as_chunks::<4>().0.iter() {
        for &c in &p[..3] {
            let e = if space == ColorSpace::Srgb {
                linear_to_srgb(c)
            } else {
                c
            };
            out.push(quantise(e, max));
        }
        if with_alpha {
            out.push(quantise(p[3], max));
        }
    }
    out
}

fn write_dyn(img: DynamicImage, fmt: ImageFormat) -> Result<Vec<u8>, ImageError> {
    let mut out = Cursor::new(Vec::new());
    img.write_to(&mut out, fmt)
        .map_err(|e| ImageError::Encode(e.to_string()))?;
    Ok(out.into_inner())
}

fn buf<P: ::image::Pixel>(
    img: &Image,
    data: Vec<P::Subpixel>,
) -> Result<ImageBuffer<P, Vec<P::Subpixel>>, ImageError> {
    ImageBuffer::from_raw(img.width, img.height, data)
        .ok_or_else(|| ImageError::InvalidInput("pixel buffer does not match size".into()))
}

/// Encodes `img` (working space, any alpha mode) into `format`.
pub fn encode(img: &Image, format: Format, opts: &EncodeOptions) -> Result<Vec<u8>, ImageError> {
    if img.width == 0
        || img.height == 0
        || img.data.len() != img.width as usize * img.height as usize * 4
    {
        return Err(ImageError::InvalidInput(
            "empty or inconsistent image".into(),
        ));
    }
    let depth = opts.depth;
    match (format, depth) {
        (Format::Png | Format::Tiff, BitDepth::U8) => {
            let data = integer_samples(img, opts.space, 255.0, true)
                .into_iter()
                .map(|v| v as u8)
                .collect();
            let dynimg = DynamicImage::ImageRgba8(buf(img, data)?);
            write_dyn(
                dynimg,
                if format == Format::Png {
                    ImageFormat::Png
                } else {
                    ImageFormat::Tiff
                },
            )
        }
        (Format::Png | Format::Tiff, BitDepth::U16) => {
            let data = integer_samples(img, opts.space, 65535.0, true)
                .into_iter()
                .map(|v| v as u16)
                .collect();
            let dynimg = DynamicImage::ImageRgba16(buf(img, data)?);
            write_dyn(
                dynimg,
                if format == Format::Png {
                    ImageFormat::Png
                } else {
                    ImageFormat::Tiff
                },
            )
        }
        (Format::Jpeg, BitDepth::U8) => {
            let data: Vec<u8> = integer_samples(img, opts.space, 255.0, false)
                .into_iter()
                .map(|v| v as u8)
                .collect();
            let mut out = Vec::new();
            let q = opts.jpeg_quality.clamp(1, 100);
            use ::image::ImageEncoder;
            ::image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, q)
                .write_image(
                    &data,
                    img.width,
                    img.height,
                    ::image::ExtendedColorType::Rgb8,
                )
                .map_err(|e| ImageError::Encode(e.to_string()))?;
            Ok(out)
        }
        (Format::Hdr, BitDepth::F32) => {
            let mut s = img.clone();
            s.set_alpha_mode(AlphaMode::Straight);
            let data = s
                .data
                .as_chunks::<4>()
                .0
                .iter()
                .flat_map(|p| [p[0], p[1], p[2]])
                .collect();
            write_dyn(DynamicImage::ImageRgb32F(buf(img, data)?), ImageFormat::Hdr)
        }
        (Format::Exr, BitDepth::F16 | BitDepth::F32) => {
            let mut s = img.clone();
            s.set_alpha_mode(AlphaMode::Premultiplied);
            let n = s.data.len() / 4;
            let channels = ["R", "G", "B", "A"]
                .iter()
                .enumerate()
                .map(|(c, name)| ExrChannel {
                    name: name.to_string(),
                    samples: (0..n).map(|i| s.data[i * 4 + c]).collect(),
                })
                .collect();
            let layer = ExrLayer {
                name: None,
                width: img.width as usize,
                height: img.height as usize,
                channels,
            };
            let p = if depth == BitDepth::F16 {
                ExrPrecision::F16
            } else {
                ExrPrecision::F32
            };
            encode_exr_layers(&[layer], p)
        }
        _ => Err(unsupported(format, depth)),
    }
}
