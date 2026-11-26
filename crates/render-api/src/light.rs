//! Lights and the environment (RENDER.md R8).

use std::sync::Arc;

use forge_image::Image;
use glam::DMat4;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AreaShape {
    Rectangle {
        width: f64,
        height: f64,
    },
    Disc {
        radius: f64,
    },
    Sphere {
        radius: f64,
    },
    /// Emitting geometry: not supported yet (`RenderError::Unsupported`).
    Object,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LightKind {
    Omni,
    /// Full cone angles in radians.
    Spot {
        inner: f64,
        outer: f64,
    },
    /// Angular diameter in radians (0 = a perfectly parallel light).
    Infinite {
        angle: f64,
    },
    Area {
        shape: AreaShape,
        two_sided: bool,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Falloff {
    None,
    Linear,
    InverseLinear,
    InverseSquare,
    /// Not supported yet (`RenderError::Unsupported`).
    Steps,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShadowMode {
    None,
    Hard,
    Soft,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Light {
    pub kind: LightKind,
    pub color: [f64; 3],
    /// 1.0 = 100 %.
    pub intensity: f64,
    /// Light to world, column-major. The light shines along local +Z.
    pub transform: [f64; 16],
    pub falloff: Falloff,
    /// Falloff radius ("radius decay").
    pub falloff_radius: f64,
    pub shadow: ShadowMode,
    pub shadow_density: f64,
    pub shadow_color: [f64; 3],
    /// Radius of the soft-shadow sphere of omni and spot lights.
    pub radius: f64,
    pub diffuse: bool,
    pub specular: bool,
    /// Whether an area light's own surface is seen by camera rays.
    pub visible: bool,
}

impl Light {
    /// A light with the O29 defaults.
    pub fn new(kind: LightKind, transform: DMat4) -> Light {
        Light {
            kind,
            color: [1.0; 3],
            intensity: 1.0,
            transform: transform.to_cols_array(),
            falloff: Falloff::None,
            falloff_radius: 500.0,
            shadow: ShadowMode::None,
            shadow_density: 1.0,
            shadow_color: [0.0; 3],
            radius: 0.0,
            diffuse: true,
            specular: true,
            visible: false,
        }
    }

    pub fn matrix(&self) -> DMat4 {
        DMat4::from_cols_array(&self.transform)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Lighting {
    None,
    /// Uniform radiance.
    Constant {
        color: [f64; 3],
        intensity: f64,
    },
    /// Gradient on the world +Y direction.
    Sky {
        zenith: [f64; 3],
        horizon: [f64; 3],
        ground: [f64; 3],
        intensity: f64,
    },
    /// Lat-long image (linear RGB), rotated about Y.
    Image {
        image: Arc<Image>,
        rotation: f64,
        intensity: f64,
    },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Background {
    Color([f64; 3]),
    /// Camera rays that miss show the environment lighting.
    Environment,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Environment {
    pub background: Background,
    pub lighting: Lighting,
}

impl Default for Environment {
    fn default() -> Self {
        Environment {
            background: Background::Color([0.0; 3]),
            lighting: Lighting::None,
        }
    }
}
