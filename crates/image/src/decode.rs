//! Decoding: pure functions from bytes (the fuzz entry points, brief 182).

use std::io::Cursor;

use crate::buffer::Image;
use crate::colour::{AlphaMode, ColorSpace, srgb_to_linear};
use crate::error::ImageError;
use crate::exr_io::{decode_exr_image, exr_layer_count};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Format {
    Png,
    Jpeg,
    Tiff,
    Hdr,
    Exr,
}

impl Format {
    pub fn from_extension(ext: &str) -> Option<Format> {
        match ext.to_ascii_lowercase().as_str() {
            "png" => Some(Format::Png),
            "jpg" | "jpeg" => Some(Format::Jpeg),
            "tif" | "tiff" => Some(Format::Tiff),
            "hdr" => Some(Format::Hdr),
            "exr" => Some(Format::Exr),
            _ => None,
        }
    }

    pub(crate) fn image_format(self) -> Option<::image::ImageFormat> {
        match self {
            Format::Png => Some(::image::ImageFormat::Png),
            Format::Jpeg => Some(::image::ImageFormat::Jpeg),
            Format::Tiff => Some(::image::ImageFormat::Tiff),
            Format::Hdr => Some(::image::ImageFormat::Hdr),
            Format::Exr => None,
        }
    }
}

/// Sample depth of a file.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BitDepth {
    U8,
    U16,
    F16,
    F32,
}

impl BitDepth {
    pub fn is_float(self) -> bool {
        matches!(self, BitDepth::F16 | BitDepth::F32)
    }
}

/// Size limits checked before pixel memory is allocated.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    pub max_dimension: u32,
    pub max_pixels: u64,
}

impl Default for Limits {
    fn default() -> Self {
        Limits {
            max_dimension: 32768,
            max_pixels: 1 << 28,
        }
    }
}

impl Limits {
    pub(crate) fn check(&self, w: u64, h: u64) -> Result<(), ImageError> {
        if w == 0
            || h == 0
            || w > self.max_dimension as u64
            || h > self.max_dimension as u64
            || w * h > self.max_pixels
        {
            return Err(ImageError::TooLarge {
                width: w,
                height: h,
            });
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ImageInfo {
    pub format: Format,
    pub depth: BitDepth,
    /// Channels stored in the file (1 grey, 2 grey+alpha, 3 RGB, 4 RGBA; EXR: of the chosen layer).
    pub channels: u8,
    pub has_alpha: bool,
    /// EXR: number of layers in the file; 1 for the other formats.
    pub layers: usize,
}

/// Pixels as stored in the file (integers normalised to 0..1, not yet interpreted).
#[derive(Clone, Debug)]
pub struct Decoded {
    pub image: Image,
    pub info: ImageInfo,
}

impl Decoded {
    /// Reinterprets the stored alpha (for files whose alpha convention differs from the format
    /// default).
    pub fn with_source_alpha(mut self, mode: AlphaMode) -> Decoded {
        self.image.alpha = mode;
        self
    }

    /// Applies the colour rules of docs/specs/IMAGE.md §1: the result is linear (for `Srgb`
    /// integer files the transfer function is removed; float files are linear already) in the
    /// requested alpha mode. `Raw` returns the stored values untouched.
    pub fn into_working(self, tag: ColorSpace, alpha: AlphaMode) -> Image {
        let mut img = self.image;
        if tag == ColorSpace::Raw {
            return img;
        }
        img.set_alpha_mode(AlphaMode::Straight);
        if tag == ColorSpace::Srgb && !self.info.depth.is_float() {
            for p in img.data.as_chunks_mut::<4>().0.iter_mut() {
                p[0] = srgb_to_linear(p[0]);
                p[1] = srgb_to_linear(p[1]);
                p[2] = srgb_to_linear(p[2]);
            }
        }
        img.set_alpha_mode(alpha);
        img
    }
}

/// Recognises a format from its first bytes.
pub fn detect_format(bytes: &[u8]) -> Option<Format> {
    if bytes.starts_with(&[0x89, b'P', b'N', b'G']) {
        Some(Format::Png)
    } else if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        Some(Format::Jpeg)
    } else if bytes.starts_with(b"II*\0") || bytes.starts_with(b"MM\0*") {
        Some(Format::Tiff)
    } else if bytes.starts_with(b"#?") {
        Some(Format::Hdr)
    } else if bytes.starts_with(&[0x76, 0x2F, 0x31, 0x01]) {
        Some(Format::Exr)
    } else {
        None
    }
}

/// Decodes any supported image. Never panics; see `Limits`.
pub fn decode(bytes: &[u8], limits: &Limits) -> Result<Decoded, ImageError> {
    match detect_format(bytes).ok_or(ImageError::UnknownFormat)? {
        Format::Exr => {
            let layers = exr_layer_count(bytes, limits)?;
            let (image, channels, has_alpha) = decode_exr_image(bytes, limits)?;
            Ok(Decoded {
                image,
                info: ImageInfo {
                    format: Format::Exr,
                    depth: BitDepth::F32,
                    channels,
                    has_alpha,
                    layers,
                },
            })
        }
        f => decode_with_image_crate(bytes, f, limits),
    }
}

fn decode_with_image_crate(
    bytes: &[u8],
    format: Format,
    limits: &Limits,
) -> Result<Decoded, ImageError> {
    use ::image::{ColorType, ImageDecoder};
    let ifmt = format.image_format().expect("non-EXR format");
    let reader = ::image::ImageReader::with_format(Cursor::new(bytes), ifmt);
    let decoder = reader
        .into_decoder()
        .map_err(|e| ImageError::Decode(e.to_string()))?;
    let (w, h) = decoder.dimensions();
    limits.check(w as u64, h as u64)?;
    let mut lim = ::image::Limits::default();
    lim.max_image_width = Some(limits.max_dimension);
    lim.max_image_height = Some(limits.max_dimension);
    lim.max_alloc = Some(limits.max_pixels.saturating_mul(16).max(1 << 20));
    let mut decoder = decoder;
    decoder
        .set_limits(lim)
        .map_err(|e| ImageError::Decode(e.to_string()))?;
    let color = decoder.color_type();
    let dynimg = ::image::DynamicImage::from_decoder(decoder)
        .map_err(|e| ImageError::Decode(e.to_string()))?;
    let depth = match color {
        ColorType::L16 | ColorType::La16 | ColorType::Rgb16 | ColorType::Rgba16 => BitDepth::U16,
        ColorType::Rgb32F | ColorType::Rgba32F => BitDepth::F32,
        _ => BitDepth::U8,
    };
    let depth = if format == Format::Hdr {
        BitDepth::F32
    } else {
        depth
    };
    let rgba = dynimg.to_rgba32f();
    let (w, h) = (rgba.width(), rgba.height());
    Ok(Decoded {
        image: Image {
            width: w,
            height: h,
            data: rgba.into_raw(),
            alpha: AlphaMode::Straight,
        },
        info: ImageInfo {
            format,
            depth,
            channels: color.channel_count(),
            has_alpha: color.has_alpha(),
            layers: 1,
        },
    })
}
