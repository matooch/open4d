//! OpenEXR reading (all layers) and multipart writing (docs/specs/IMAGE.md §2.1).

use std::collections::BTreeMap;
use std::io::Cursor;

use exr::prelude as ex;
use exr::prelude::{ReadChannels, ReadLayers, WritableImage};

use crate::buffer::Image;
use crate::colour::AlphaMode;
use crate::decode::Limits;
use crate::error::ImageError;

#[derive(Clone, Debug, PartialEq)]
pub struct ExrChannel {
    /// Short name inside its layer (`R`, `G`, `B`, `A`, `Z`, ...).
    pub name: String,
    /// `width * height` samples, row 0 at the top.
    pub samples: Vec<f32>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ExrLayer {
    pub name: Option<String>,
    pub width: usize,
    pub height: usize,
    pub channels: Vec<ExrChannel>,
}

impl ExrLayer {
    fn channel(&self, name: &str) -> Option<&ExrChannel> {
        self.channels.iter().find(|c| c.name == name)
    }

    /// RGBA (premultiplied, the OpenEXR convention) from `R G B [A]`, or grey from a single
    /// channel (`Y`, `Z`, else the first). `None` when the layer has no channels.
    pub fn to_image(&self) -> Option<Image> {
        self.to_image_info().map(|(i, _, _)| i)
    }

    pub(crate) fn to_image_info(&self) -> Option<(Image, u8, bool)> {
        let (w, h) = (self.width, self.height);
        let mut img = Image::new(w as u32, h as u32, AlphaMode::Premultiplied);
        if let (Some(r), Some(g), Some(b)) =
            (self.channel("R"), self.channel("G"), self.channel("B"))
        {
            let a = self.channel("A");
            for i in 0..w * h {
                img.data[i * 4] = r.samples[i];
                img.data[i * 4 + 1] = g.samples[i];
                img.data[i * 4 + 2] = b.samples[i];
                img.data[i * 4 + 3] = a.map_or(1.0, |a| a.samples[i]);
            }
            return Some((img, 3 + a.is_some() as u8, a.is_some()));
        }
        let c = self
            .channel("Y")
            .or_else(|| self.channel("Z"))
            .or_else(|| self.channels.first())?;
        for i in 0..w * h {
            let v = c.samples[i];
            img.data[i * 4..i * 4 + 4].copy_from_slice(&[v, v, v, 1.0]);
        }
        Some((img, 1, false))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExrPrecision {
    F16,
    F32,
}

fn dec_err(e: impl std::fmt::Display) -> ImageError {
    ImageError::Decode(e.to_string())
}

fn check_headers(bytes: &[u8], limits: &Limits) -> Result<(), ImageError> {
    let meta =
        exr::meta::MetaData::read_from_buffered(Cursor::new(bytes), false).map_err(dec_err)?;
    let mut total = 0u64;
    for h in &meta.headers {
        if h.deep {
            return Err(ImageError::Unsupported("deep EXR data".into()));
        }
        let (w, hh) = (h.layer_size.width() as u64, h.layer_size.height() as u64);
        limits.check(w, hh)?;
        total = total.saturating_add(w * hh * h.channels.list.len() as u64);
        if total > limits.max_pixels.saturating_mul(4) {
            return Err(ImageError::TooLarge {
                width: w,
                height: hh,
            });
        }
    }
    Ok(())
}

/// Reads every layer of an EXR file (largest resolution level only). Layers of a
/// single-part file whose channel names are dotted (`diffuse.R`) are split by prefix.
pub fn decode_exr_layers(bytes: &[u8], limits: &Limits) -> Result<Vec<ExrLayer>, ImageError> {
    check_headers(bytes, limits)?;
    let image = ex::read()
        .no_deep_data()
        .largest_resolution_level()
        .all_channels()
        .all_layers()
        .all_attributes()
        .from_buffered(Cursor::new(bytes))
        .map_err(dec_err)?;
    let mut out = Vec::new();
    for layer in &image.layer_data {
        let (w, h) = (layer.size.width(), layer.size.height());
        let base = layer.attributes.layer_name.as_ref().map(|t| t.to_string());
        // group channels by prefix; BTreeMap keeps the order deterministic (None first)
        let mut groups: BTreeMap<Option<String>, Vec<ExrChannel>> = BTreeMap::new();
        for c in &layer.channel_data.list {
            if c.sampling != ex::Vec2(1, 1) {
                return Err(ImageError::Unsupported("subsampled EXR channel".into()));
            }
            let full = c.name.to_string();
            let (prefix, short) = match full.rsplit_once('.') {
                Some((p, s)) if !p.is_empty() && !s.is_empty() => (Some(p.to_string()), s),
                _ => (None, full.as_str()),
            };
            let name = match (&base, prefix) {
                (Some(b), Some(p)) => Some(format!("{b}.{p}")),
                (Some(b), None) => Some(b.clone()),
                (None, p) => p,
            };
            groups.entry(name).or_default().push(ExrChannel {
                name: short.to_string(),
                samples: c.sample_data.values_as_f32().collect(),
            });
        }
        if groups.is_empty() {
            out.push(ExrLayer {
                name: base,
                width: w,
                height: h,
                channels: vec![],
            });
        }
        for (name, channels) in groups {
            out.push(ExrLayer {
                name,
                width: w,
                height: h,
                channels,
            });
        }
    }
    Ok(out)
}

pub(crate) fn exr_layer_count(bytes: &[u8], limits: &Limits) -> Result<usize, ImageError> {
    Ok(decode_exr_layers(bytes, limits)?.len())
}

/// The image `decode` returns for an EXR file: the first layer that has colour or at least
/// one channel, preferring `rgba`, `beauty`, then the unnamed layer.
pub(crate) fn decode_exr_image(
    bytes: &[u8],
    limits: &Limits,
) -> Result<(Image, u8, bool), ImageError> {
    let layers = decode_exr_layers(bytes, limits)?;
    let rank = |l: &ExrLayer| match l.name.as_deref() {
        Some("rgba") => 0,
        Some("beauty") => 1,
        None => 2,
        _ => 3,
    };
    let mut order: Vec<&ExrLayer> = layers.iter().collect();
    order.sort_by_key(|l| rank(l)); // stable
    order
        .into_iter()
        .find_map(|l| l.to_image_info())
        .ok_or_else(|| ImageError::Decode("EXR file has no image channels".into()))
}

/// Writes a multipart EXR (one part per layer, zip compression). With more than one layer
/// every layer needs a unique non-empty name.
pub fn encode_exr_layers(
    layers: &[ExrLayer],
    precision: ExrPrecision,
) -> Result<Vec<u8>, ImageError> {
    let bad = |s: &str| ImageError::InvalidInput(s.to_string());
    if layers.is_empty() {
        return Err(bad("no layers"));
    }
    if layers.len() > 1 {
        let mut seen = std::collections::HashSet::new();
        for l in layers {
            match l.name.as_deref() {
                Some(n) if !n.is_empty() && seen.insert(n.to_string()) => {}
                _ => return Err(bad("layers need unique non-empty names")),
            }
        }
    }
    let mut parts = Vec::new();
    for l in layers {
        if l.width == 0 || l.height == 0 || l.channels.is_empty() {
            return Err(bad("empty layer"));
        }
        let mut chans: ex::SmallVec<[ex::AnyChannel<ex::FlatSamples>; 4]> = ex::SmallVec::new();
        for c in &l.channels {
            if c.samples.len() != l.width * l.height {
                return Err(bad("sample count does not match layer size"));
            }
            let name = ex::Text::new_or_none(&c.name).ok_or_else(|| bad("bad channel name"))?;
            let samples = match precision {
                ExrPrecision::F32 => ex::FlatSamples::F32(c.samples.clone()),
                ExrPrecision::F16 => {
                    ex::FlatSamples::F16(c.samples.iter().map(|&v| ex::f16::from_f32(v)).collect())
                }
            };
            chans.push(ex::AnyChannel::new(name, samples));
        }
        let attrs = match &l.name {
            Some(n) => ex::LayerAttributes::named(
                ex::Text::new_or_none(n).ok_or_else(|| bad("bad layer name"))?,
            ),
            None => ex::LayerAttributes::default(),
        };
        parts.push(ex::Layer::new(
            (l.width, l.height),
            attrs,
            ex::Encoding::SMALL_LOSSLESS,
            ex::AnyChannels::sort(chans),
        ));
    }
    let bounds = ex::IntegerBounds::from_dimensions((layers[0].width, layers[0].height));
    let image = ex::Image::from_layers(ex::ImageAttributes::new(bounds), parts);
    let mut out = Cursor::new(Vec::new());
    image
        .write()
        .to_buffered(&mut out)
        .map_err(|e| ImageError::Encode(e.to_string()))?;
    Ok(out.into_inner())
}
