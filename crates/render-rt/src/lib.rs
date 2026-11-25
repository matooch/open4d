//! forge-render-rt: the Tier-1 real-time raster PBR engine (docs/specs/RENDER.md R21).
//!
//! A wgpu engine with a small frame graph, a deferred G-buffer, shadow maps per light and
//! environment lighting, shading through the same IR as the CPU and compute tracers.

mod engine;
pub mod framegraph;
pub mod post;
pub mod shader;
pub mod shadow;

pub use engine::RtEngine;
pub use framegraph::{FrameGraph, GraphError, Pass};

/// Requests the default adapter and device; `Err` carries the reason there is none.
pub fn request_default_device() -> Result<(wgpu::Device, wgpu::Queue), String> {
    let instance =
        wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle_from_env());
    let adapter =
        pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
            .map_err(|e| e.to_string())?;
    pollster::block_on(adapter.request_device(&Default::default())).map_err(|e| e.to_string())
}

/// Slots of the frame uniform (`array<vec4<f32>, 128>`), one number per entry; the shader
/// constants are generated from [`frame::ALL`] so the two sides cannot drift apart (R21.5).
pub mod frame {
    pub const W: usize = 0;
    pub const H: usize = 1;
    pub const RX0: usize = 2;
    pub const RY0: usize = 3;
    pub const RX1: usize = 4;
    pub const RY1: usize = 5;
    pub const SCALE: usize = 6;
    pub const LIGHT_COUNT: usize = 7;
    pub const SHADOW_COUNT: usize = 8;
    pub const TILE: usize = 9;
    pub const TILE_PX: usize = 10;
    pub const ATLAS_X: usize = 11;
    pub const ATLAS_Y: usize = 12;
    pub const PROJ_KIND: usize = 13;
    pub const FOCAL: usize = 14;
    pub const SENSOR: usize = 15;
    pub const FILM_OFF_X: usize = 16;
    pub const FILM_OFF_Y: usize = 17;
    pub const ORTHO_W: usize = 18;
    pub const NEAR: usize = 19;
    pub const FAR: usize = 20;
    /// Sixteen floats: camera to world, column-major.
    pub const CAM: usize = 24;
    /// Sixteen floats: world to camera, column-major.
    pub const VIEW: usize = 40;
    pub const CAM_POS: usize = 56;
    pub const BG_KIND: usize = 59;
    pub const BG_X: usize = 60;
    pub const BG_Y: usize = 61;
    pub const BG_Z: usize = 62;
    pub const LIGHTING_KIND: usize = 63;
    pub const LIGHTING_INTENSITY: usize = 64;
    pub const CONST: usize = 65;
    pub const SKY_ZEN: usize = 68;
    pub const SKY_HOR: usize = 71;
    pub const SKY_GND: usize = 74;
    pub const ENV_PRESENT: usize = 77;
    pub const ENV_ROT: usize = 78;
    pub const ENV_W: usize = 79;
    pub const ENV_H: usize = 80;
    /// Sixteen floats: the shadow matrix of the tile being rendered, column-major.
    pub const SHADOW_MAT: usize = 84;
    pub const EPS: usize = 100;

    /// Post: SSAO on (R22.2), its world radius and strength.
    pub const SSAO_ON: usize = 101;
    pub const SSAO_RADIUS: usize = 102;
    pub const SSAO_STRENGTH: usize = 103;
    /// Post: SSR on (R22.3) and its march steps.
    pub const SSR_ON: usize = 104;
    pub const SSR_STEPS: usize = 105;
    /// Post: the subpixel jitter of a TAA subframe, in pixels (R22.5).
    pub const JITTER_X: usize = 106;
    pub const JITTER_Y: usize = 107;

    pub const SLOTS: usize = 128;

    /// Every slot, in order: the shader constant name and its index.
    pub const ALL: &[(&str, usize)] = &[
        ("W", W),
        ("H", H),
        ("RX0", RX0),
        ("RY0", RY0),
        ("RX1", RX1),
        ("RY1", RY1),
        ("SCALE", SCALE),
        ("LIGHT_COUNT", LIGHT_COUNT),
        ("SHADOW_COUNT", SHADOW_COUNT),
        ("TILE", TILE),
        ("TILE_PX", TILE_PX),
        ("ATLAS_X", ATLAS_X),
        ("ATLAS_Y", ATLAS_Y),
        ("PROJ_KIND", PROJ_KIND),
        ("FOCAL", FOCAL),
        ("SENSOR", SENSOR),
        ("FILM_OFF_X", FILM_OFF_X),
        ("FILM_OFF_Y", FILM_OFF_Y),
        ("ORTHO_W", ORTHO_W),
        ("NEAR", NEAR),
        ("FAR", FAR),
        ("CAM", CAM),
        ("VIEW", VIEW),
        ("CAM_POS", CAM_POS),
        ("BG_KIND", BG_KIND),
        ("BG_X", BG_X),
        ("BG_Y", BG_Y),
        ("BG_Z", BG_Z),
        ("LIGHTING_KIND", LIGHTING_KIND),
        ("LIGHTING_INTENSITY", LIGHTING_INTENSITY),
        ("CONST", CONST),
        ("SKY_ZEN", SKY_ZEN),
        ("SKY_HOR", SKY_HOR),
        ("SKY_GND", SKY_GND),
        ("ENV_PRESENT", ENV_PRESENT),
        ("ENV_ROT", ENV_ROT),
        ("ENV_W", ENV_W),
        ("ENV_H", ENV_H),
        ("SHADOW_MAT", SHADOW_MAT),
        ("EPS", EPS),
        ("SSAO_ON", SSAO_ON),
        ("SSAO_RADIUS", SSAO_RADIUS),
        ("SSAO_STRENGTH", SSAO_STRENGTH),
        ("SSR_ON", SSR_ON),
        ("SSR_STEPS", SSR_STEPS),
        ("JITTER_X", JITTER_X),
        ("JITTER_Y", JITTER_Y),
    ];
}
