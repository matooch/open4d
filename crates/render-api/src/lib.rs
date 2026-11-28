//! forge-render-api: the contract between the scene and the render engines.
//! Spec: docs/specs/RENDER.md.

/// The longest a GPU engine waits for one submission before giving up with an error
/// (RENDER.md R20.4, R21.7): a hung or pathologically slow device (e.g. a software adapter)
/// must fail the render, never freeze the caller.
pub const GPU_WAIT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(120);

pub mod camera;
pub mod engine;
pub mod files;
pub mod light;
pub mod output;
pub mod settings;
pub mod snapshot;

pub use camera::{Aperture, Camera, Projection};
pub use engine::{Control, RenderEngine, RenderError, RenderEvent};
pub use files::{
    FrameRange, Multipass, OutputFormat, OutputSettings, expand_path, frames, write_output,
};
pub use light::{
    AreaShape, Background, Environment, Falloff, Light, LightKind, Lighting, ShadowMode,
};
pub use output::{Aov, RenderOutput, RenderStats};
pub use settings::{HybridSettings, PathLighting, PixelFilter, PostSettings, RenderSettings};
pub use snapshot::{Instance, MaterialData, MeshData, Snapshot};
