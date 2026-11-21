//! forge-shaders: shaders sampled at a point. Spec: docs/specs/SHADERS.md.

pub mod blend;
pub mod build;
pub mod gradient;
pub mod host;
pub mod params;
pub mod shader;

pub use blend::BlendMode;
pub use build::{Inputs, ShaderError, Values, build};
pub use gradient::{Gradient, Interp};
pub use host::{ImageLoader, build_node, no_images};
pub use params::{ShadersPlugin, shader_types};
pub use shader::{Shader, ShaderSet, srgb_to_linear};
