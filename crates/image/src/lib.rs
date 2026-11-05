//! Image IO, texture cache with mip chains, colour management (docs/specs/IMAGE.md).
//!
//! Working space is linear sRGB (Rec.709). Decoding is a pure function of bytes
//! ([`decode`], [`decode_exr_layers`]) so it can be fuzzed.

mod buffer;
mod cache;
mod colour;
mod decode;
mod encode;
mod error;
mod exr_io;
mod mip;

pub use buffer::Image;
pub use cache::{CacheStats, LoadOptions, Texture, TextureCache};
pub use colour::{
    AlphaMode, ColorSpace, linear_to_srgb, linear_to_srgb8, premultiply, srgb_to_linear,
    srgb8_to_linear, unpremultiply,
};
pub use decode::{BitDepth, Decoded, Format, ImageInfo, Limits, decode, detect_format};
pub use encode::{EncodeOptions, encode};
pub use error::ImageError;
pub use exr_io::{ExrChannel, ExrLayer, ExrPrecision, decode_exr_layers, encode_exr_layers};
pub use mip::MipChain;
