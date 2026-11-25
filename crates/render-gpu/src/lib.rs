//! forge-render-gpu: the R9 path tracer as a wgpu compute pass (docs/specs/RENDER.md R20).

mod engine;
pub mod pack;
pub mod shader;

pub use engine::GpuEngine;

/// The deterministic RNG and sampling helpers (R20.2), for tests of the WGSL directly.
pub const RNG_WGSL: &str = include_str!("rng.wgsl");
/// The BVH walk, lights, surfaces and integrator (R20.1, R20.2).
pub const TRACE_WGSL: &str = include_str!("trace.wgsl");

/// Requests the default adapter and device; `Err` carries the reason there is none.
pub fn request_default_device() -> Result<(wgpu::Device, wgpu::Queue), String> {
    let instance =
        wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle_from_env());
    let adapter =
        pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
            .map_err(|e| e.to_string())?;
    pollster::block_on(adapter.request_device(&Default::default())).map_err(|e| e.to_string())
}

/// Slots of the frame uniform (`array<vec4<f32>, 96>`), one number per entry, generated into
/// the shader by [`shader::frame_constants`] so the two sides cannot drift apart.
pub mod frame {
    pub const W: usize = 0;
    pub const H: usize = 1;
    pub const RX0: usize = 2;
    pub const RY0: usize = 3;
    pub const RX1: usize = 4;
    pub const RY1: usize = 5;
    pub const TX: usize = 6;
    pub const TY: usize = 7;
    pub const SAMPLES: usize = 8;
    pub const MAX_DEPTH: usize = 9;
    pub const SEED_LO: usize = 10;
    pub const SEED_HI: usize = 11;
    pub const FILTER: usize = 12;
    pub const FILTER_WIDTH: usize = 13;
    pub const CLAMP: usize = 14;
    pub const EPS: usize = 15;
    pub const BG_X: usize = 16;
    pub const BG_Y: usize = 17;
    pub const BG_Z: usize = 18;
    pub const BG_KIND: usize = 19;
    pub const LIGHTING_KIND: usize = 20;
    pub const LIGHTING_INTENSITY: usize = 21;
    pub const CONST_X: usize = 22;
    pub const CONST_Y: usize = 23;
    pub const CONST_Z: usize = 24;
    pub const SKY_ZEN_X: usize = 25;
    pub const SKY_ZEN_Y: usize = 26;
    pub const SKY_ZEN_Z: usize = 27;
    pub const SKY_HOR_X: usize = 28;
    pub const SKY_HOR_Y: usize = 29;
    pub const SKY_HOR_Z: usize = 30;
    pub const SKY_GND_X: usize = 31;
    pub const SKY_GND_Y: usize = 32;
    pub const SKY_GND_Z: usize = 33;
    /// Sixteen floats: the camera-to-world matrix, column-major.
    pub const CAMERA: usize = 34;
    pub const PROJ_KIND: usize = 50;
    pub const FOCAL: usize = 51;
    pub const SENSOR: usize = 52;
    pub const ORTHO_WIDTH: usize = 53;
    pub const FILM_OFF_X: usize = 54;
    pub const FILM_OFF_Y: usize = 55;
    pub const APERTURE_PRESENT: usize = 56;
    pub const APERTURE_RADIUS: usize = 57;
    pub const APERTURE_FOCUS: usize = 58;
    pub const TILE_W: usize = 59;
    pub const TILE_H: usize = 60;
    pub const AOV_COUNT: usize = 61;
    pub const STRIDE: usize = 62;
    pub const SCALE: usize = 63;
    /// Ten AOV tags, `Aov` as a `u32` bitcast into the float slot.
    pub const AOV_TAG: usize = 64;

    pub const SLOTS: usize = 96;

    /// Every slot, in order: the shader constant names and values.
    pub const ALL: &[(&str, usize)] = &[
        ("W", W),
        ("H", H),
        ("RX0", RX0),
        ("RY0", RY0),
        ("RX1", RX1),
        ("RY1", RY1),
        ("TX", TX),
        ("TY", TY),
        ("SAMPLES", SAMPLES),
        ("MAX_DEPTH", MAX_DEPTH),
        ("SEED_LO", SEED_LO),
        ("SEED_HI", SEED_HI),
        ("FILTER", FILTER),
        ("FILTER_WIDTH", FILTER_WIDTH),
        ("CLAMP", CLAMP),
        ("EPS", EPS),
        ("BG_X", BG_X),
        ("BG_Y", BG_Y),
        ("BG_Z", BG_Z),
        ("BG_KIND", BG_KIND),
        ("LIGHTING_KIND", LIGHTING_KIND),
        ("LIGHTING_INTENSITY", LIGHTING_INTENSITY),
        ("CONST_X", CONST_X),
        ("CONST_Y", CONST_Y),
        ("CONST_Z", CONST_Z),
        ("SKY_ZEN_X", SKY_ZEN_X),
        ("SKY_ZEN_Y", SKY_ZEN_Y),
        ("SKY_ZEN_Z", SKY_ZEN_Z),
        ("SKY_HOR_X", SKY_HOR_X),
        ("SKY_HOR_Y", SKY_HOR_Y),
        ("SKY_HOR_Z", SKY_HOR_Z),
        ("SKY_GND_X", SKY_GND_X),
        ("SKY_GND_Y", SKY_GND_Y),
        ("SKY_GND_Z", SKY_GND_Z),
        ("CAMERA", CAMERA),
        ("PROJ_KIND", PROJ_KIND),
        ("FOCAL", FOCAL),
        ("SENSOR", SENSOR),
        ("ORTHO_WIDTH", ORTHO_WIDTH),
        ("FILM_OFF_X", FILM_OFF_X),
        ("FILM_OFF_Y", FILM_OFF_Y),
        ("APERTURE_PRESENT", APERTURE_PRESENT),
        ("APERTURE_RADIUS", APERTURE_RADIUS),
        ("APERTURE_FOCUS", APERTURE_FOCUS),
        ("TILE_W", TILE_W),
        ("TILE_H", TILE_H),
        ("AOV_COUNT", AOV_COUNT),
        ("STRIDE", STRIDE),
        ("SCALE", SCALE),
        ("AOV_TAG", AOV_TAG),
    ];
}

/// A pixel sample's AOV tag, the order the shader writes them in.
pub fn aov_tag(a: forge_render_api::Aov) -> u32 {
    use forge_render_api::Aov;
    match a {
        Aov::Depth => 0,
        Aov::Normal => 1,
        Aov::Albedo => 2,
        Aov::Position => 3,
        Aov::ObjectId => 4,
        Aov::MaterialId => 5,
        Aov::Shadow => 6,
        Aov::Diffuse => 7,
        Aov::Specular => 8,
        Aov::Emission => 9,
    }
}
