//! forge-render-api: the contract between the scene and the render engines.
//! Spec: docs/specs/RENDER.md.

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
pub use settings::{PixelFilter, PostSettings, RenderSettings};
pub use snapshot::{Instance, MaterialData, MeshData, Snapshot};
