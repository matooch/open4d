//! Viewport lights and shadow cameras (docs/specs/VIEWPORT.md §8.1-§8.2). Pure maths: the
//! renderer packs these into a uniform buffer. Directions and positions are world space.

use glam::dcamera::rh::{proj::directx, view::look_at_mat4};
use glam::{DMat4, DVec3};

use crate::camera::Camera;

/// Lights the shader evaluates, and shadow-map layers (one per shadow-casting light).
pub const MAX_LIGHTS: usize = 8;
/// Constant light every lit fragment receives.
pub const AMBIENT: f32 = 0.25;
/// Intensity of the headlight (`AMBIENT + HEADLIGHT_INTENSITY = 1`).
pub const HEADLIGHT_INTENSITY: f32 = 0.75;
/// Scale of the shadow frustum round the scene's bounding sphere.
const SHADOW_MARGIN: f64 = 1.05;
/// Upper bound of a spot's shadow field of view, radians.
const MAX_SPOT_FOV: f64 = 2.9;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LightKind {
    #[default]
    Directional,
    Point,
    Spot,
}

/// One light (VIEWPORT.md §8.1). `Copy`, so `Settings` stays `Copy`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Light {
    pub kind: LightKind,
    /// Directional and spot: the direction the light travels.
    pub direction: DVec3,
    /// Point and spot: world position.
    pub position: DVec3,
    pub color: [f32; 3],
    pub intensity: f32,
    /// Point and spot: 0 = no falloff, else `(1 - min(d / range, 1))^2`.
    pub range: f64,
    /// Spot half-angles in radians.
    pub inner: f64,
    pub outer: f64,
    pub cast_shadow: bool,
}

impl Default for Light {
    fn default() -> Self {
        Light {
            kind: LightKind::Directional,
            direction: DVec3::NEG_Y,
            position: DVec3::ZERO,
            color: [1.0; 3],
            intensity: 1.0,
            range: 0.0,
            inner: 0.0,
            outer: 0.0,
            cast_shadow: false,
        }
    }
}

impl Light {
    /// A light travelling along `direction` (need not be unit length).
    pub fn directional(direction: DVec3, intensity: f32) -> Light {
        Light {
            kind: LightKind::Directional,
            direction,
            intensity,
            ..Light::default()
        }
    }

    pub fn point(position: DVec3, intensity: f32) -> Light {
        Light {
            kind: LightKind::Point,
            position,
            intensity,
            ..Light::default()
        }
    }

    /// A cone from `position` along `direction`; `inner <= outer` are half-angles in radians.
    pub fn spot(
        position: DVec3,
        direction: DVec3,
        inner: f64,
        outer: f64,
        intensity: f32,
    ) -> Light {
        Light {
            kind: LightKind::Spot,
            position,
            direction,
            inner: inner.min(outer),
            outer,
            intensity,
            ..Light::default()
        }
    }

    pub fn with_shadow(mut self, cast: bool) -> Light {
        self.cast_shadow = cast;
        self
    }

    pub fn with_color(mut self, color: [f32; 3]) -> Light {
        self.color = color;
        self
    }

    pub fn with_range(mut self, range: f64) -> Light {
        self.range = range;
        self
    }

    /// The unit travel direction; a zero vector falls back to `-Y`.
    pub fn travel(&self) -> DVec3 {
        let d = self.direction;
        if d.length() > 1e-12 {
            d.normalize()
        } else {
            DVec3::NEG_Y
        }
    }
}

/// Up to [`MAX_LIGHTS`] caller-supplied lights, fixed size so `Settings` stays `Copy`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SceneLights {
    pub lights: [Light; MAX_LIGHTS],
    pub count: usize,
}

impl SceneLights {
    /// The first [`MAX_LIGHTS`] of `lights`; the rest are ignored.
    pub fn new(lights: &[Light]) -> SceneLights {
        let mut out = SceneLights {
            lights: [Light::default(); MAX_LIGHTS],
            count: lights.len().min(MAX_LIGHTS),
        };
        out.lights[..out.count].copy_from_slice(&lights[..out.count]);
        out
    }

    pub fn as_slice(&self) -> &[Light] {
        &self.lights[..self.count]
    }
}

impl Default for SceneLights {
    fn default() -> Self {
        SceneLights::new(&[])
    }
}

/// Where the light comes from (VIEWPORT.md §8.1).
// Scene lights are stored inline so that `Settings` stays `Copy` (every caller builds it by
// struct update); the size is irrelevant for a per-frame settings value.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum Lighting {
    /// One directional light along the view direction: the classic viewport look.
    #[default]
    Headlight,
    /// Three world-anchored directional lights; the key light casts a shadow.
    Rig,
    /// Caller-supplied lights (scene light objects).
    Scene(SceneLights),
}

/// The headlight: travels along `forward_world()`, no shadow.
pub fn headlight(cam: &Camera) -> Light {
    Light::directional(cam.forward_world(), HEADLIGHT_INTENSITY)
}

/// The default rig (our design, parity-pending G8). Directions given as "towards the light".
pub fn default_rig() -> [Light; 3] {
    let toward = |x: f64, y: f64, z: f64, intensity: f32| {
        Light::directional(-DVec3::new(x, y, z).normalize(), intensity)
    };
    [
        toward(0.45, 0.85, 0.55, 0.50).with_shadow(true),
        toward(-0.70, 0.30, -0.40, 0.20),
        toward(0.00, -0.20, -1.00, 0.10),
    ]
}

/// The lights to draw with, at most [`MAX_LIGHTS`].
pub fn resolve(lighting: &Lighting, cam: &Camera) -> Vec<Light> {
    match lighting {
        Lighting::Headlight => vec![headlight(cam)],
        Lighting::Rig => default_rig().to_vec(),
        Lighting::Scene(s) => s.as_slice().to_vec(),
    }
}

/// Bounding sphere (centre, radius) of a box.
pub fn bounds_sphere(bounds: (DVec3, DVec3)) -> (DVec3, f64) {
    let (lo, hi) = bounds;
    ((lo + hi) * 0.5, ((hi - lo).length() * 0.5).max(1e-6))
}

/// World to light clip space of `light`'s shadow map, covering `bounds`; `None` when the light
/// casts no shadow or has no shadow camera (point lights; VIEWPORT.md §8.2).
pub fn shadow_matrix(light: &Light, bounds: (DVec3, DVec3)) -> Option<DMat4> {
    if !light.cast_shadow {
        return None;
    }
    let (c, r) = bounds_sphere(bounds);
    let d = light.travel();
    let up = if d.y.abs() > 0.99 { DVec3::X } else { DVec3::Y };
    match light.kind {
        LightKind::Directional => {
            let e = r * SHADOW_MARGIN;
            let view = look_at_mat4(c - d * e, c, up);
            Some(directx::orthographic(-e, e, -e, e, 0.0, 2.0 * e) * view)
        }
        LightKind::Spot => {
            let far = (c - light.position).length() + r;
            // as far as the bounds allow: perspective depth is finest near the near plane, so
            // a tiny near plane would make the fixed NDC bias swallow the scene
            let near = ((c - light.position).length() - r)
                .max(far * 0.02)
                .max(1e-6);
            let fov = (2.0 * light.outer + 0.1).clamp(0.05, MAX_SPOT_FOV);
            let view = look_at_mat4(light.position, light.position + d, up);
            Some(directx::perspective(fov, 1.0, near, far) * view)
        }
        LightKind::Point => None,
    }
}
