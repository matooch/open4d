//! The viewport's "Render" display mode: a real-time tier frame of the pane's own camera
//! (docs/specs/UI.md §6.3, docs/specs/RENDER.md R22.7).
//!
//! The frame is rendered synchronously and cached by (scene revision, camera, size, post), so a
//! static scene costs one render per change; a device that cannot be made is remembered, so the
//! panes fall back to a note instead of retrying every frame. A background worker with
//! latest-snapshot cancellation is the performance follow-up of brief 204.

use egui::{Color32, ColorImage, TextureHandle, TextureOptions};
use forge_core::Scene;
use forge_render_api::{
    Camera as RenderCamera, Control, PostSettings, Projection, RenderEngine, RenderSettings,
};
use forge_render_rt::RtEngine;
use forge_viewport::camera::{Camera, Projection as ViewProjection};

use crate::picture_viewer::prepare_render;

/// The render-rt frame a pane shows, and the cache key it was made for.
#[derive(Default)]
pub struct RenderView {
    engine: Option<Result<RtEngine, String>>,
    cache: Option<(Key, TextureHandle)>,
}

/// What a rendered pane depends on (R22.7): the scene revision, the shown frame, the pane's
/// camera, the pixel size and the post block. Any change re-renders.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Key {
    revision: u64,
    frame: i64,
    camera: [u64; 8],
    size: [u32; 2],
    post: [u64; 2],
}

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

fn camera_key(camera: &Camera) -> [u64; 8] {
    let (kind, a, b) = match camera.projection {
        ViewProjection::Perspective { fov_y } => (0u64, fov_y.to_bits(), 0),
        ViewProjection::Orthographic { height } => (1, height.to_bits(), 0),
    };
    [
        camera.target.x.to_bits(),
        camera.target.y.to_bits(),
        camera.target.z.to_bits(),
        camera.distance.to_bits(),
        camera.yaw.to_bits(),
        camera.pitch.to_bits(),
        kind,
        a ^ b,
    ]
}

fn post_key(post: PostSettings) -> [u64; 2] {
    let flags = u64::from(post.ssao)
        | u64::from(post.ssr) << 1
        | u64::from(post.bloom) << 2
        | u64::from(post.taa) << 3
        | u64::from(post.tonemap) << 4
        | u64::from(post.ssr_steps) << 8
        | u64::from(post.taa_samples) << 16;
    let values = post.ssao_radius.to_bits()
        ^ post.ssao_strength.to_bits().rotate_left(7)
        ^ post.bloom_threshold.to_bits().rotate_left(13)
        ^ post.bloom_strength.to_bits().rotate_left(23)
        ^ post.exposure.to_bits().rotate_left(31);
    [flags, values]
}

impl RenderView {
    /// The rendered pane image (RGBA8, opaque) of `scene` seen from `camera`, at `size` pixels.
    /// `Err` says why there is no image (usually no GPU adapter).
    pub fn frame(
        &mut self,
        scene: &Scene,
        camera: &Camera,
        frame: i64,
        size: [u32; 2],
        post: PostSettings,
    ) -> Result<ColorImage, String> {
        let (w, h) = (size[0].max(1), size[1].max(1));
        let engine = match self
            .engine
            .get_or_insert_with(|| RtEngine::new().map_err(|e| e.to_string()))
        {
            Ok(engine) => engine,
            Err(e) => return Err(e.clone()),
        };
        let mut settings = RenderSettings {
            width: w,
            height: h,
            samples: 1,
            post,
            ..RenderSettings::default()
        };
        let (mut snapshot, _warnings) =
            prepare_render(scene, frame, &mut settings, &Control::new())
                .map_err(|e| e.to_string())?;
        snapshot.camera = render_camera(camera, w, h);
        let out = engine
            .render(&snapshot, &settings, &Control::new())
            .map_err(|e| e.to_string())?;
        let mut pixels = Vec::with_capacity((w * h) as usize);
        for p in out.beauty.data.as_chunks::<4>().0 {
            let rgb = [
                forge_image::linear_to_srgb(p[0].clamp(0.0, 1.0)),
                forge_image::linear_to_srgb(p[1].clamp(0.0, 1.0)),
                forge_image::linear_to_srgb(p[2].clamp(0.0, 1.0)),
            ];
            let [r, g, b] = rgb.map(|v| (v * 255.0).round() as u8);
            pixels.push(Color32::from_rgb(r, g, b));
        }
        Ok(ColorImage::new([w as usize, h as usize], pixels))
    }

    /// [`frame`](Self::frame) as an egui texture, re-rendered only when the pane's inputs change.
    pub fn image(
        &mut self,
        ctx: &egui::Context,
        scene: &Scene,
        camera: &Camera,
        frame: i64,
        size: [u32; 2],
        post: PostSettings,
    ) -> Result<TextureHandle, String> {
        let size = size.map(|v| v.max(1));
        let key = Key {
            revision: scene.revision(),
            frame,
            camera: camera_key(camera),
            size,
            post: post_key(post),
        };
        if let Some((cached, texture)) = &self.cache
            && *cached == key
        {
            return Ok(texture.clone());
        }
        let image = self.frame(scene, camera, frame, size, post)?;
        let handle = match &mut self.cache {
            Some((_, texture)) => {
                texture.set(image, TextureOptions::LINEAR);
                texture.clone()
            }
            None => ctx.load_texture("render-mode", image, TextureOptions::LINEAR),
        };
        self.cache = Some((key, handle.clone()));
        Ok(handle)
    }

    /// Forgets the cached frame (a scene reload, a settings change).
    pub fn invalidate(&mut self) {
        self.cache = None;
    }
}
