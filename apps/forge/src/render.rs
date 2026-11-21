//! `forge render` (docs/specs/CLI.md, RENDER.md R12): image sequences and passes.

use std::path::Path;

use forge_core::NodeId;
use forge_generators::is_consumed;
use forge_render_api::{
    Aov, Control, FrameRange, Multipass, OutputFormat, OutputSettings, RenderEngine,
    RenderSettings, frames, write_output,
};
use forge_render_bridge::{apply_scene_objects, settings_from_scene, snapshot_from_polys};
use forge_render_cpu::CpuEngine;
use serde_json::json;

use crate::{Fail, Out, Outcome, all_nodes, open, user};

#[derive(clap::Args, Debug)]
pub struct RenderArgs {
    pub scene: String,
    /// Output path template; tokens {frame}, {frame:N}, {pass}, {name}. The extension picks
    /// the format when it is a known one.
    #[arg(long)]
    pub out: String,
    /// Image width (default: the scene's stored render settings, else 320).
    #[arg(long)]
    pub width: Option<u32>,
    /// Image height (default: the scene's stored render settings, else 240).
    #[arg(long)]
    pub height: Option<u32>,
    /// Samples per pixel (default: the scene's stored render settings, else 16).
    #[arg(long)]
    pub samples: Option<u32>,
    /// Path vertices (default: the scene's stored render settings, else 5).
    #[arg(long)]
    pub max_depth: Option<u32>,
    /// Sampling seed (default: the scene's stored render settings, else 1).
    #[arg(long)]
    pub seed: Option<u64>,
    /// One frame (default 0, clamped to the scene range).
    #[arg(long, allow_negative_numbers = true)]
    pub frame: Option<i64>,
    /// First frame of a sequence (needs --end).
    #[arg(long, allow_negative_numbers = true)]
    pub start: Option<i64>,
    #[arg(long, allow_negative_numbers = true)]
    pub end: Option<i64>,
    #[arg(long, default_value_t = 1)]
    pub step: i64,
    /// Format when --out has no known extension: png, tif, jpg, hdr, exr.
    #[arg(long)]
    pub format: Option<String>,
    /// Bits per channel: 8, 16 or 32.
    #[arg(long)]
    pub depth: Option<u32>,
    /// Write the alpha channel.
    #[arg(long)]
    pub alpha: bool,
    /// Extra passes, comma separated: depth, normal, albedo, position, object_id, material_id,
    /// shadow, diffuse, specular, emission, or `all`.
    #[arg(long)]
    pub aov: Option<String>,
    /// One multipart EXR file with a layer per pass.
    #[arg(long)]
    pub multipart: bool,
}

fn parse_aovs(s: &str) -> Result<Vec<Aov>, Fail> {
    if s == "all" {
        return Ok(Aov::ALL.to_vec());
    }
    s.split(',')
        .filter(|p| !p.is_empty())
        .map(|p| Aov::parse(p.trim()).ok_or_else(|| user(format!("unknown pass {p:?}"))))
        .collect()
}

pub fn render(a: &RenderArgs) -> Outcome {
    let (mut app, _) = open(&a.scene)?;
    let (s_start, s_end) = {
        let st = app.doc.scene().settings();
        (st.start, st.end)
    };
    let range = match (a.start, a.end, a.frame) {
        (Some(s), Some(e), None) => FrameRange::Manual { start: s, end: e },
        (None, None, Some(f)) => FrameRange::Manual { start: f, end: f },
        (None, None, None) => FrameRange::Manual { start: 0, end: 0 },
        _ => {
            return Err(user(
                "use either --frame N or --start A --end B (both together)",
            ));
        }
    };
    let list = frames(range, a.step, 0, (s_start, s_end))
        .map_err(|e| user(e.to_string()))?
        .into_iter()
        .map(|f| f.clamp(s_start, s_end))
        .collect::<Vec<_>>();
    let ext = Path::new(&a.out)
        .extension()
        .and_then(|e| e.to_str())
        .and_then(OutputFormat::parse);
    let format = match (ext, a.format.as_deref()) {
        (Some(f), None) => f,
        (_, Some(s)) => {
            OutputFormat::parse(s).ok_or_else(|| user(format!("unknown format {s:?}")))?
        }
        (None, None) => OutputFormat::Png,
    };
    let depth_bits = a.depth.unwrap_or(match format {
        OutputFormat::Exr | OutputFormat::Hdr => 32,
        _ => 8,
    });
    let aovs = match &a.aov {
        Some(s) => parse_aovs(s)?,
        None => Vec::new(),
    };
    let name = Path::new(&a.scene)
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "scene".into());
    // stored render settings (a pure addition to the scene file), then the flags on top
    let base = settings_from_scene(app.doc.scene(), RenderSettings::default());
    let (width, height) = (
        a.width.unwrap_or(base.width),
        a.height.unwrap_or(base.height),
    );
    let out_settings = OutputSettings {
        width,
        height,
        path: a.out.clone(),
        name,
        format,
        depth_bits,
        alpha: a.alpha,
        straight_alpha: false,
        multipass: if a.multipart {
            Multipass::Multipart
        } else {
            Multipass::Separate
        },
        ..OutputSettings::default()
    };
    let mut settings = RenderSettings {
        width,
        height,
        samples: a.samples.unwrap_or(base.samples),
        max_depth: a.max_depth.unwrap_or(base.max_depth),
        seed: a.seed.unwrap_or(base.seed),
        aovs,
        ..base
    };
    settings.validate().map_err(|e| user(e.to_string()))?;
    let multiple = list.len() > 1;
    let engine = CpuEngine::new();
    let mut files = Vec::new();
    let mut warnings: Vec<String> = Vec::new();
    for f in &list {
        let scene = app.frame(*f);
        let ids: Vec<NodeId> = all_nodes(&scene)
            .into_iter()
            .filter(|id| !is_consumed(&scene, *id))
            .collect();
        let mut polys = Vec::new();
        for id in ids {
            match app.eval_mesh(id, *f) {
                Ok(m) => polys.push(m),
                Err(forge_app::AppError::NoGeometry(_)) => {}
                Err(e) => return Err(e.into()),
            }
        }
        let mut snap = snapshot_from_polys(&polys, width as f64 / height as f64);
        let applied = apply_scene_objects(&mut snap, &scene);
        if applied.scene_has_lights {
            // lights in the scene (even all switched off) replace the default light
            settings.default_light = false;
        }
        for w in &applied.warnings {
            eprintln!("warning: {w}");
            if !warnings.contains(w) {
                warnings.push(w.clone());
            }
        }
        let out = engine
            .render(&snap, &settings, &Control::new())
            .map_err(|e| user(e.to_string()))?;
        let written = write_output(&out, &out_settings, *f, multiple, Path::new("."))
            .map_err(|e| user(e.to_string()))?;
        for p in written {
            eprintln!("frame {f}: {}", p.display());
            files.push(p.display().to_string());
        }
    }
    Ok(Out {
        json: json!({"files": files, "frames": list, "width": width, "height": height,
            "samples": settings.samples, "format": format.extension(), "warnings": warnings}),
        text: format!("rendered {} frame(s), {} file(s)", list.len(), files.len()),
    })
}
