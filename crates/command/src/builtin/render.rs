//! render.settings (docs/specs/SCENE-OBJECTS.md S6): the document's stored render settings.

use forge_core::{NodeId, RenderConfig, RenderFilter};
use forge_generators::scene_objects::{CAMERA, Lens, camera_data};
use glam::DVec3;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};

use super::spec;
use crate::context::CommandContext;
use crate::error::CommandError;
use crate::registry::CommandRegistry;
use crate::state::CameraSet;
use crate::state::ProjectionArg;

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct SettingsArgs {
    /// Image width in pixels.
    width: Option<u32>,
    /// Image height in pixels.
    height: Option<u32>,
    /// Samples per pixel.
    samples: Option<u32>,
    /// Maximum path vertices.
    max_depth: Option<u32>,
    /// Sampling seed.
    seed: Option<u64>,
    /// Light a scene without lights with the default light.
    default_light: Option<bool>,
    /// `box`, `triangle`, `gaussian` or `cubic`.
    filter: Option<String>,
    /// Full filter width in pixels.
    filter_width: Option<f64>,
    /// true: remove the stored settings (renders use the caller's defaults again).
    reset: Option<bool>,
}

fn json_of(c: Option<&RenderConfig>) -> Value {
    match c {
        None => json!({"stored": false}),
        Some(c) => json!({"stored": true, "width": c.width, "height": c.height,
            "samples": c.samples, "max_depth": c.max_depth, "seed": c.seed,
            "default_light": c.default_light, "filter": c.filter.id(),
            "filter_width": c.filter_width}),
    }
}

fn settings(ctx: &mut dyn CommandContext, a: SettingsArgs) -> Result<Value, CommandError> {
    let touched = a.width.is_some()
        || a.height.is_some()
        || a.samples.is_some()
        || a.max_depth.is_some()
        || a.seed.is_some()
        || a.default_light.is_some()
        || a.filter.is_some()
        || a.filter_width.is_some();
    if a.reset == Some(true) {
        if touched {
            return Err(CommandError::InvalidArgs(
                "render.settings: reset cannot be combined with values".into(),
            ));
        }
        ctx.doc_mut()
            .transact("Reset Render Settings", |tx| tx.set_render_config(None))?;
    } else if touched {
        let mut c = ctx
            .doc()
            .scene()
            .render_config()
            .cloned()
            .unwrap_or_default();
        c.width = a.width.unwrap_or(c.width);
        c.height = a.height.unwrap_or(c.height);
        c.samples = a.samples.unwrap_or(c.samples);
        c.max_depth = a.max_depth.unwrap_or(c.max_depth);
        c.seed = a.seed.unwrap_or(c.seed);
        c.default_light = a.default_light.unwrap_or(c.default_light);
        c.filter_width = a.filter_width.unwrap_or(c.filter_width);
        if let Some(f) = &a.filter {
            c.filter = RenderFilter::parse(f).ok_or_else(|| {
                CommandError::InvalidArgs(format!(
                    "unknown filter {f:?} (box, triangle, gaussian, cubic)"
                ))
            })?;
        }
        ctx.doc_mut()
            .transact_merged("Render Settings", "render-settings", |tx| {
                tx.set_render_config(Some(c))
            })?;
    }
    Ok(json_of(ctx.doc().scene().render_config()))
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct LookArgs {
    /// A camera object (default: the selected camera, else the first camera of the scene).
    node: Option<u64>,
    /// Viewport width / height (default: the render settings' aspect, else 16:9).
    aspect: Option<f64>,
}

/// Points the viewport camera along a camera object with its lens (docs/specs/SCENE-OBJECTS.md
/// S7). The editor viewport is left-handed: display space mirrors Z. Film offset and bank
/// cannot be shown by the orbit camera and are dropped.
fn look_through(ctx: &mut dyn CommandContext, a: LookArgs) -> Result<Value, CommandError> {
    let scene = ctx.doc().scene();
    let node = match a.node {
        Some(n) => NodeId(n),
        None => {
            let selected = ctx
                .selection()
                .nodes
                .iter()
                .copied()
                .find(|n| scene.node(*n).is_some_and(|x| x.type_id == CAMERA));
            let first = scene
                .roots()
                .iter()
                .flat_map(|r| std::iter::once(*r).chain(scene.descendants(*r)))
                .find(|n| scene.node(*n).is_some_and(|x| x.type_id == CAMERA));
            selected.or(first).ok_or_else(|| {
                CommandError::Failed("view.look_through_camera: the scene has no camera".into())
            })?
        }
    };
    let cam = camera_data(scene, node)
        .ok_or_else(|| CommandError::Failed(format!("node {} is not a camera", node.0)))?;
    let aspect = a
        .aspect
        .or_else(|| {
            scene
                .render_config()
                .map(|c| f64::from(c.width) / f64::from(c.height))
        })
        .unwrap_or(16.0 / 9.0);
    if !(aspect.is_finite() && aspect > 0.0) {
        return Err(CommandError::InvalidArgs(
            "aspect must be greater than 0".into(),
        ));
    }
    let eye = cam.world.w_axis.truncate();
    let fwd = cam.world.transform_vector3(DVec3::Z).normalize();
    let distance = cam.focus_distance.max(1.0);
    let mirrored = fwd * DVec3::new(1.0, 1.0, -1.0);
    let mut set = CameraSet {
        target: Some((eye + fwd * distance).to_array()),
        distance: Some(distance),
        yaw: Some((-mirrored.x).atan2(-mirrored.z).to_degrees()),
        pitch: Some((-mirrored.y).clamp(-1.0, 1.0).asin().to_degrees()),
        ..CameraSet::default()
    };
    match cam.lens {
        Lens::Perspective {
            focal_mm,
            sensor_mm,
            ..
        } => {
            let fov_h = 2.0 * (sensor_mm / (2.0 * focal_mm)).atan();
            let fov_y = 2.0 * ((fov_h * 0.5).tan() / aspect).atan();
            set.projection = Some(ProjectionArg::Perspective);
            set.fov = Some(fov_y.to_degrees().clamp(1.0, 169.9));
        }
        Lens::Orthographic { width } => {
            set.projection = Some(ProjectionArg::Orthographic);
            set.height = Some(width / aspect);
        }
    }
    ctx.set_camera(set);
    Ok(json!({"camera": node.0}))
}

pub(super) fn register(r: &mut CommandRegistry) -> Result<(), CommandError> {
    r.register_fn(
        spec(
            "render.settings",
            "Render Settings",
            true,
            || json!({"width": 640, "height": 360, "samples": 32}),
        ),
        settings,
    )?;
    r.register_fn(
        spec(
            "view.look_through_camera",
            "Look Through Camera",
            false,
            || json!({"node": 2}),
        ),
        look_through,
    )
}
