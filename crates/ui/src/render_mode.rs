//! Camera and path tracer shared by the viewport and Picture Viewer.

use std::sync::OnceLock;

use forge_render_api::{
    Camera as RenderCamera, Control, Projection, RenderEngine, RenderError, RenderOutput,
    RenderSettings, Snapshot,
};
use forge_render_rt::PathTraceEngine;
use forge_viewport::camera::{Camera, Projection as ViewProjection};

/// The render camera of a viewport camera (R22.7): the same right-handed basis (both `look_at`
/// implementations put +Z forward and +Y up), the focal from the vertical field of view and the
/// orthographic width from the view height.
pub fn render_camera(camera: &Camera, width: u32, height: u32) -> RenderCamera {
    let w = f64::from(width.max(1));
    let h = f64::from(height.max(1));
    let projection = match camera.projection {
        ViewProjection::Perspective { fov_y } => Projection::Perspective {
            focal_mm: 36.0 * h / (2.0 * w * (fov_y / 2.0).tan()),
            sensor_mm: 36.0,
            film_offset: [0.0; 2],
        },
        ViewProjection::Orthographic { height } => Projection::Orthographic {
            width: height * (w / h),
        },
    };
    // the viewport's view space looks along -Z, the raster tier's camera space along +Z
    // (R21.2), so the camera-to-world transform flips Z
    RenderCamera::new(
        camera.view().inverse() * glam::DMat4::from_scale(glam::DVec3::new(1.0, 1.0, -1.0)),
        projection,
    )
}

#[derive(Default)]
pub(crate) struct StudioPathTracer(OnceLock<Result<PathTraceEngine, String>>);

impl RenderEngine for StudioPathTracer {
    fn id(&self) -> &str {
        "forge.render.pathtrace"
    }

    fn render(
        &self,
        snapshot: &Snapshot,
        settings: &RenderSettings,
        control: &Control,
    ) -> Result<RenderOutput, RenderError> {
        let engine = self
            .0
            .get_or_init(|| {
                let (device, queue) = forge_render_rt::request_rt_device()?;
                Ok(PathTraceEngine::with_device(device, queue))
            })
            .as_ref()
            .map_err(|error| RenderError::Unsupported(error.clone()))?;
        engine.render(snapshot, settings, control)
    }
}

/// Convert the linear beauty buffer to the display's sRGB encoding.
pub(crate) fn display_rgb(rgb: [f32; 3], gain: f32) -> egui::Color32 {
    let [r, g, b] = rgb.map(|v| forge_image::linear_to_srgb8(v * gain));
    egui::Color32::from_rgb(r, g, b)
}

/// Sample counts for complete, deterministic refinement passes.
pub(crate) fn progressive_samples(target: u32) -> impl Iterator<Item = u32> {
    std::iter::successors((target > 0).then_some(1u32), move |&samples| {
        (samples < target).then(|| samples.saturating_mul(4).min(target))
    })
}
