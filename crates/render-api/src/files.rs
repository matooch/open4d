//! Render settings of the output, frame ranges, path tokens and files (RENDER.md R12).

use std::path::{Path, PathBuf};

use forge_image::{
    AlphaMode, BitDepth, ColorSpace, EncodeOptions, ExrChannel, ExrLayer, ExrPrecision, Format,
    Image, encode, encode_exr_layers,
};

use crate::engine::RenderError;
use crate::output::{Aov, RenderOutput};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    Png,
    Tiff,
    Jpeg,
    Hdr,
    Exr,
}

impl OutputFormat {
    pub fn extension(self) -> &'static str {
        match self {
            OutputFormat::Png => "png",
            OutputFormat::Tiff => "tif",
            OutputFormat::Jpeg => "jpg",
            OutputFormat::Hdr => "hdr",
            OutputFormat::Exr => "exr",
        }
    }

    pub fn parse(s: &str) -> Option<OutputFormat> {
        match s.to_ascii_lowercase().as_str() {
            "png" => Some(OutputFormat::Png),
            "tif" | "tiff" => Some(OutputFormat::Tiff),
            "jpg" | "jpeg" => Some(OutputFormat::Jpeg),
            "hdr" => Some(OutputFormat::Hdr),
            "exr" => Some(OutputFormat::Exr),
            _ => None,
        }
    }

    fn image_format(self) -> Format {
        match self {
            OutputFormat::Png => Format::Png,
            OutputFormat::Tiff => Format::Tiff,
            OutputFormat::Jpeg => Format::Jpeg,
            OutputFormat::Hdr => Format::Hdr,
            OutputFormat::Exr => Format::Exr,
        }
    }

    /// The pixel depth of `bits` (8, 16 or 32) in this format, or an error for a pairing the
    /// format cannot store.
    fn depth(self, bits: u32) -> Result<BitDepth, RenderError> {
        let err = || {
            Err(RenderError::Output(format!(
                "{} cannot be written at {bits} bits per channel",
                self.extension()
            )))
        };
        match (self, bits) {
            (OutputFormat::Png | OutputFormat::Tiff, 8) | (OutputFormat::Jpeg, 8) => {
                Ok(BitDepth::U8)
            }
            (OutputFormat::Png | OutputFormat::Tiff, 16) => Ok(BitDepth::U16),
            (OutputFormat::Exr, 16) => Ok(BitDepth::F16),
            (OutputFormat::Exr | OutputFormat::Hdr, 32) => Ok(BitDepth::F32),
            _ => err(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameRange {
    Current,
    /// The scene's frame range.
    All,
    Manual {
        start: i64,
        end: i64,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Multipass {
    /// One file per pass.
    Separate,
    /// One multipart EXR file with a layer per pass.
    Multipart,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OutputSettings {
    pub width: u32,
    pub height: u32,
    pub range: FrameRange,
    pub step: i64,
    pub frame_rate: f64,
    /// Path template with `{frame}`, `{frame:N}`, `{pass}`, `{name}` tokens.
    pub path: String,
    /// Replaces `{name}` (the scene name).
    pub name: String,
    pub format: OutputFormat,
    pub depth_bits: u32,
    pub alpha: bool,
    pub straight_alpha: bool,
    pub multipass: Multipass,
}

impl Default for OutputSettings {
    /// The O29 defaults: 320 x 240 at 30 fps, the current frame, step 1, 8 bit, no alpha,
    /// file name `Name0000.TIF`.
    fn default() -> Self {
        OutputSettings {
            width: 320,
            height: 240,
            range: FrameRange::Current,
            step: 1,
            frame_rate: 30.0,
            path: "{name}{frame:4}".to_string(),
            name: "Name".to_string(),
            format: OutputFormat::Tiff,
            depth_bits: 8,
            alpha: false,
            straight_alpha: false,
            multipass: Multipass::Separate,
        }
    }
}

/// The frames to render (R12).
pub fn frames(
    range: FrameRange,
    step: i64,
    current: i64,
    scene_range: (i64, i64),
) -> Result<Vec<i64>, RenderError> {
    if step < 1 {
        return Err(RenderError::Settings(
            "frame step must be at least 1".into(),
        ));
    }
    let (a, b) = match range {
        FrameRange::Current => return Ok(vec![current]),
        FrameRange::All => scene_range,
        FrameRange::Manual { start, end } => (start, end),
    };
    if b < a {
        return Err(RenderError::Settings(format!(
            "the frame range {a}..{b} is empty"
        )));
    }
    Ok((a..=b).step_by(step as usize).collect())
}

/// Expands the tokens of `template` and adds the extension of `format`.
///
/// `multiple_frames` adds `{frame:4}` to a template without a frame token; an AOV (`pass` is not
/// `beauty`) without a `{pass}` token gets `_<pass>` appended to the stem.
pub fn expand_path(
    template: &str,
    frame: i64,
    pass: &str,
    name: &str,
    multiple_frames: bool,
    format: OutputFormat,
) -> String {
    let ext = format!(".{}", format.extension());
    let mut t = template.to_string();
    if let Some(stripped) = t.strip_suffix(&ext) {
        t = stripped.to_string();
    }
    if multiple_frames && !t.contains("{frame") {
        t.push_str("{frame:4}");
    }
    if pass != "beauty" && !t.contains("{pass}") {
        t.push_str("_{pass}");
    }
    let mut out = String::new();
    let mut rest = t.as_str();
    while let Some(i) = rest.find('{') {
        out.push_str(&rest[..i]);
        let tail = &rest[i..];
        let Some(j) = tail.find('}') else {
            out.push_str(tail);
            rest = "";
            break;
        };
        let token = &tail[1..j];
        match token {
            "frame" => out.push_str(&frame.to_string()),
            "pass" => out.push_str(pass),
            "name" => out.push_str(name),
            _ => match token
                .strip_prefix("frame:")
                .and_then(|n| n.parse::<usize>().ok())
            {
                Some(n) => out.push_str(&format!("{frame:0n$}")),
                None => out.push_str(&tail[..=j]),
            },
        }
        rest = &tail[j + 1..];
    }
    out.push_str(rest);
    out.push_str(&ext);
    out
}

fn prepare_colour(img: &Image, settings: &OutputSettings) -> Image {
    let mut im = img.clone();
    im.alpha = AlphaMode::Straight;
    if !settings.alpha {
        for p in im.data.as_chunks_mut::<4>().0.iter_mut() {
            p[3] = 1.0;
        }
    } else if !settings.straight_alpha {
        for p in im.data.as_chunks_mut::<4>().0.iter_mut() {
            let a = p[3];
            p[0] *= a;
            p[1] *= a;
            p[2] *= a;
        }
    }
    im
}

fn write_file(path: &Path, bytes: &[u8]) -> Result<(), RenderError> {
    if let Some(dir) = path.parent()
        && !dir.as_os_str().is_empty()
    {
        std::fs::create_dir_all(dir)
            .map_err(|e| RenderError::Output(format!("{}: {e}", dir.display())))?;
    }
    std::fs::write(path, bytes).map_err(|e| RenderError::Output(format!("{}: {e}", path.display())))
}

fn exr_layer(name: &str, img: &Image, channels: &[&str], src: &[usize]) -> ExrLayer {
    let n = img.data.len() / 4;
    ExrLayer {
        name: Some(name.to_string()),
        width: img.width as usize,
        height: img.height as usize,
        channels: channels
            .iter()
            .zip(src)
            .map(|(c, &s)| ExrChannel {
                name: c.to_string(),
                samples: (0..n).map(|i| img.data[i * 4 + s]).collect(),
            })
            .collect(),
    }
}

/// Writes the beauty and the AOVs of one frame; returns the files in the order written.
/// `base_dir` is joined to a relative template.
pub fn write_output(
    out: &RenderOutput,
    settings: &OutputSettings,
    frame: i64,
    multiple_frames: bool,
    base_dir: &Path,
) -> Result<Vec<PathBuf>, RenderError> {
    let depth = settings.format.depth(settings.depth_bits)?;
    let place = |pass: &str| -> PathBuf {
        base_dir.join(expand_path(
            &settings.path,
            frame,
            pass,
            &settings.name,
            multiple_frames,
            settings.format,
        ))
    };
    let mut written = Vec::new();
    if settings.multipass == Multipass::Multipart {
        if settings.format != OutputFormat::Exr {
            return Err(RenderError::Output(
                "multipart output needs the exr format".into(),
            ));
        }
        let beauty = prepare_colour(&out.beauty, settings);
        let mut layers = vec![exr_layer(
            "beauty",
            &beauty,
            &["R", "G", "B", "A"],
            &[0, 1, 2, 3],
        )];
        for (aov, img) in &out.aovs {
            layers.push(match aov {
                Aov::Depth => exr_layer(aov.name(), img, &["Z"], &[0]),
                Aov::ObjectId | Aov::MaterialId => exr_layer(aov.name(), img, &["Y"], &[0]),
                _ => exr_layer(aov.name(), img, &["R", "G", "B"], &[0, 1, 2]),
            });
        }
        let p = if depth == BitDepth::F16 {
            ExrPrecision::F16
        } else {
            ExrPrecision::F32
        };
        let bytes =
            encode_exr_layers(&layers, p).map_err(|e| RenderError::Output(e.to_string()))?;
        let path = place("beauty");
        write_file(&path, &bytes)?;
        written.push(path);
        return Ok(written);
    }
    let opts = |data: bool| EncodeOptions {
        depth,
        space: if data {
            ColorSpace::Linear
        } else {
            ColorSpace::Srgb
        },
        jpeg_quality: 90,
    };
    let fmt = settings.format.image_format();
    let beauty = prepare_colour(&out.beauty, settings);
    let bytes =
        encode(&beauty, fmt, &opts(false)).map_err(|e| RenderError::Output(e.to_string()))?;
    let path = place("beauty");
    write_file(&path, &bytes)?;
    written.push(path);
    for (aov, img) in &out.aovs {
        let mut im = img.clone();
        im.alpha = AlphaMode::Straight;
        for p in im.data.as_chunks_mut::<4>().0.iter_mut() {
            p[3] = 1.0;
        }
        let bytes = encode(&im, fmt, &opts(aov.is_data()))
            .map_err(|e| RenderError::Output(e.to_string()))?;
        let path = place(aov.name());
        write_file(&path, &bytes)?;
        written.push(path);
    }
    Ok(written)
}
