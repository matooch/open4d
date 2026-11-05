# IMAGE — image IO, texture cache, colour management (brief 130)

Sources: public format specifications (PNG, TIFF 6.0, JFIF/JPEG, Radiance RGBE "picture file"
format, OpenEXR technical introduction), IEC 61966-2-1 (sRGB) for the transfer function and
the Rec.709 primaries, ITU-R BT.709, the public docs of the `image` and `exr` crates, and
published texture-filtering literature (box-filter mip chains). No reference-app observation
is involved (brief 130 has none); colour-pipeline defaults follow the area design
(docs/roadmap/areas/J-materials.md, "linear workflow default").

Crate: `forge-image` (`crates/image`). It depends on `image` (PNG, JPEG, TIFF, Radiance HDR
decoders and encoders; MIT/Apache-2.0) and `exr` (OpenEXR, pure Rust; BSD-3-Clause). Both are
allowed by `deny.toml`; no exception is needed. It does not depend on any other forge crate:
file-reference params (CORE.md §6.5) hold plain path strings, so `TextureCache::get_ref` takes
the `&str` of such a param. No derived data is written to disk, so `forge-platform`'s cache
folder is not used (a later brief may put persistent mip caches there).

## 1. Working space and colour rules

- **Working space** is linear sRGB (Rec.709 primaries, D65 white). OCIO and ICC profiles are
  out of scope for v1: embedded ICC chunks are ignored.
- **sRGB transfer** (IEC 61966-2-1), `v` in 0..1:
  decode `lin = v / 12.92` for `v <= 0.04045`, else `((v + 0.055) / 1.055)^2.4`;
  encode `v = 12.92 lin` for `lin <= 0.0031308`, else `1.055 lin^(1/2.4) - 0.055`.
  Outside 0..1 the curve is extended oddly (`f(-x) = -f(x)`), so HDR values above 1 and small
  negative values survive a round trip. `decode(encode(x)) = x` to 1e-6 (f32).
  8-bit: `srgb8_to_linear(u8)` is a 256-entry table; `linear_to_srgb8(f32)` rounds to nearest and
  clamps, and `linear_to_srgb8(srgb8_to_linear(i)) = i` for all 256 codes.
- **Colour-space tag** per texture, `ColorSpace`:
  - `Srgb`: integer files (8/16-bit PNG, JPEG, TIFF) are sRGB-encoded; RGB is decoded to linear.
  - `Linear`: integer file values are already linear; normalised only (`v / 255`, `v / 65535`).
  - `Raw`: data (normal, roughness, masks): values are normalised only and never transformed or
    alpha-converted; the output alpha mode is the stored one.
  Float files (Radiance HDR, OpenEXR) are scene-linear by definition: the tag is ignored for
  them (always `Linear`). Alpha is never put through the transfer function.
  Param strings: `"srgb"`, `"linear"`, `"raw"` (`ColorSpace::parse` / `as_str`).
- **Alpha mode** `AlphaMode::{Straight, Premultiplied}`. Decoded alpha is `Straight` for PNG,
  JPEG, TIFF, HDR and `Premultiplied` for OpenEXR (the OpenEXR convention). `LoadOptions`
  `source_alpha` (default `None` = the format default) overrides how the file's alpha is read, and
  `alpha` (default `Straight`) is the mode of the returned pixels. Conversion order is always
  *to straight, transfer function, to the requested mode*, so the transfer function never sees
  premultiplied colour. Un-premultiplying a pixel with alpha 0 leaves RGB at 0.
- Images without alpha get alpha 1; greyscale (and grey+alpha) is expanded to R=G=B.

## 2. Pixel types and decoding

`Image { width, height, data: Vec<f32> (RGBA interleaved, row 0 = top), alpha }`. All pixels
are `f32` RGBA in memory, whatever the file depth.

`decode(bytes, &Limits) -> Result<Decoded, ImageError>` is a pure function of the bytes (the
entry point for fuzzing, brief 182): it never panics, never reads files and checks the declared
size against `Limits { max_dimension: 32768, max_pixels: 1 << 28 }` before allocating pixels.
`Decoded { image, info }` holds the values *as stored* (integers normalised to 0..1, not yet
interpreted) and `ImageInfo { format, depth, channels, has_alpha, layers }`.
`Decoded::into_working(tag, alpha) -> Image` applies §1.

`detect_format(bytes)` sniffs magic numbers (PNG `89 50 4E 47`, JPEG `FF D8 FF`, TIFF `II*\0` /
`MM\0*`, Radiance `#?`, EXR `76 2F 31 01`); `Format::from_extension` maps `png jpg jpeg tif
tiff hdr exr`. `decode` sniffs; an unknown signature is `ImageError::UnknownFormat`.

Bit depths read: PNG 8/16, JPEG 8, TIFF 8/16 (other TIFF depths are whatever the `tiff` crate
reads, untested), HDR 32-bit float (RGBE), EXR half, float and UINT channels (all read as `f32`; `ImageInfo.depth` is `F32`).
JPEG is lossy and has no alpha. Subsampled EXR channels, deep EXR data and tiled or mip-mapped
EXR levels beyond the largest level are not supported (`ImageError::Unsupported`; the largest
level is read).

### 2.1 OpenEXR layers

`decode_exr_layers(bytes, &Limits) -> Vec<ExrLayer>`; `ExrLayer { name: Option<String>, width,
height, channels: Vec<ExrChannel { name, samples: Vec<f32> }> }`. Multi-part files give one
layer per part. A single-part file whose channel names are dotted (`diffuse.R`, the legacy
layer convention) is split into one layer per prefix; channels without a dot form the unnamed
layer. Channel names inside a layer are the short ones (`R`, `G`, `B`, `A`, `Z`).
`ExrLayer::to_image()` builds RGBA from `R G B [A]`, or from a single channel (`Y`, `Z`, or the
first one) replicated to grey; layers with neither return `None`.
`decode` of an EXR file returns the first layer that `to_image` accepts, preferring the layer
named `rgba`/`beauty`/unnamed.

`encode_exr_layers(layers, ExrPrecision::{F16, F32}) -> Vec<u8>` writes a multipart file (one
part per layer; all layers need unique non-empty names when there is more than one; zip
compression, lossless). This is what multi-pass render output (brief 139) will use for AOVs.
F32 round-trips bit-exactly, F16 to half precision (relative 2^-11).

## 3. Encoding

`encode(&Image, Format, &EncodeOptions) -> Result<Vec<u8>, ImageError>` with
`EncodeOptions { depth: BitDepth, space: ColorSpace, jpeg_quality: u8 (default 90) }`.
The input image is in the working space (linear). Rules:
- `space = Srgb` (default for integer formats) encodes RGB with the sRGB curve; `Linear` and
  `Raw` write the values unchanged. Alpha is written straight (un-premultiplied) and never
  encoded. Float formats ignore `space` and write linear.
- PNG: `U8`/`U16`, RGBA. TIFF: `U8`/`U16`, RGBA. JPEG: `U8` RGB (alpha dropped), quality
  1..100. HDR: RGB RGBE (alpha dropped; ~8-bit mantissa, relative error below 1 %). EXR:
  `F16`/`F32` RGBA (premultiplied on disk per convention).
- Unsupported pairs (for example PNG with `F32`, TIFF float, JPEG 16-bit) are
  `ImageError::Unsupported`, never a silent downgrade.
Lossless pairs round-trip exactly on the quantised values: 8-bit PNG/TIFF encode(decode(x)) = x
for every code, 16-bit likewise. JPEG is tested with a PSNR bound.

## 4. Mip chains

`MipChain::build(&Image) -> MipChain` (levels down to 1x1, `max(1, floor(n/2))` per axis). Each
destination pixel is the *area-weighted box average* of the source pixels it covers (source
interval `[x*sw/dw, (x+1)*sw/dw)`, fractional overlaps as weights), so odd sizes keep the
mean exactly. Filtering is done on **linear, premultiplied** RGBA, so energy is preserved:
the mean of premultiplied RGBA is the same at every level (tolerance 1e-5 relative), and straight-alpha
textures do not bleed colour from transparent pixels. Levels are converted back to the
image's alpha mode. Mips are never built from sRGB-encoded data: `TextureCache` only builds
them from working-space images. `Raw` textures use the same box filter (no re-normalisation).

## 5. Texture cache

`TextureCache::new(budget_bytes)`; `get(path, &LoadOptions) -> Result<Arc<Texture>, ImageError>`,
`get_ref(param_value: &str, &LoadOptions) -> Result<Option<Arc<Texture>>, ImageError>` (empty
string = no texture = `Ok(None)`), `stats() -> CacheStats`, `clear()`, `set_budget`.
`Texture { levels: Vec<Image>, tag, info, path }` (level 0 is the full image; one level when
`mips` is off).

- **Key**: resolved path (canonicalised when the file exists, else made absolute lexically) and
  the load options (`tag`, `alpha`, `source_alpha`, `mips`); the entry also stores the file's
  modification time and size (its *stamp*). The same file loaded with a different tag is a
  different entry. A lookup hits only when path, options and stamp all match.
- **Invalidation**: each `get` stats the file. If the stamp (mtime, size) differs from the cached
  entry for the same path+options, the stale entry is dropped (`invalidations += 1`) and the file
  is decoded again. A file that disappeared is an error (`ImageError::Io`) and its entries
  are dropped.
- **Budget / LRU**: the cost of an entry is the bytes of all its levels (`16 * pixels`). After an
  insert, least-recently-used entries are evicted (`evictions += 1`) until the total is within the
  budget. A texture larger than the whole budget is returned but not cached. `get` of an
  entry marks it most recently used. Eviction does not invalidate handles: callers hold `Arc`s.
- **Threads**: the cache is `Send + Sync` (one mutex around the index; decoding runs outside the
  lock). Two threads asking for the same missing key may both decode; one result is kept
  (documented, harmless).
- **Stats**: `hits`, `misses`, `evictions`, `invalidations`, `bytes`, `entries`.
- External references: node types store the path in a `ParamDesc::file_path` param (CORE.md
  §6.5), absolute after load; the cache is the consumer of that string.

## 6. Tests

`crates/image/tests/`: `colour.rs` (transfer accuracy, inverse, table, alpha), `formats.rs`
(round trip per format and depth, PSNR for JPEG, RGBE tolerance, detection, malformed input never
panics, limits), `exr_layers.rs` (multipart write/read, legacy dotted layers, F16 precision),
`mips.rs` (box filter values, odd sizes, energy, alpha), `cache.rs` (hit, miss, key by options,
invalidate on change, LRU eviction, oversize, threads, `get_ref`). All images are generated in
the tests (tiny, temp dirs); no binary fixtures are committed.

Not covered (not in v1): ICC / OCIO, other primaries, animated formats, deep EXR, DDS/KTX,
persistent on-disk mip caches.
