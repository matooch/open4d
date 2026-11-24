//! Shared by the WGSL tests: the case list of every shader family (REALTIME.md §2.9).
#![allow(dead_code)]

use forge_noise::NoiseKind;
use forge_shaders::shader::*;

pub struct Case {
    pub name: String,
    pub shader: Shader,
    /// The absolute bound (per channel) this case is asserted against.
    pub bound: f64,
}

/// Default bound of REALTIME.md §2.9 (f32).
pub const DEFAULT_BOUND: f64 = 1e-4;

pub fn case(name: &str, shader: Shader, bound: f64) -> Case {
    Case {
        name: name.to_string(),
        shader,
        bound,
    }
}

pub fn noise_shader(kind: NoiseKind) -> NoiseShader {
    NoiseShader {
        kind,
        seed: 7,
        octaves: 4,
        space: NoiseSpace::Texture,
        global_scale: 1.0,
        relative_scale: [1.0; 3],
        animation_speed: 0.0,
        absolute: false,
        low_clip: 0.0,
        high_clip: 1.0,
        brightness: 0.0,
        contrast: 0.0,
        color_1: [0.0; 3],
        color_2: [1.0; 3],
    }
}

pub fn noise_cases() -> Vec<Case> {
    let mut cases = Vec::new();
    for kind in NoiseKind::ALL {
        // position space: lattice coordinates up to about |q| = 20 with 4D time
        let n = NoiseShader {
            space: NoiseSpace::Object,
            animation_speed: 0.5,
            ..noise_shader(kind)
        };
        cases.push(case(&format!("{kind:?}"), Shader::Noise(n), DEFAULT_BOUND));
    }
    let base = |kind| NoiseShader {
        space: NoiseSpace::World,
        animation_speed: 0.25,
        ..noise_shader(kind)
    };
    for (label, n) in [
        (
            "Fbm-8-octaves",
            NoiseShader {
                octaves: 8,
                ..base(NoiseKind::Fbm)
            },
        ),
        (
            "Turbulence-8-octaves",
            NoiseShader {
                octaves: 8,
                ..base(NoiseKind::Turbulence)
            },
        ),
        (
            "Ridged-8-octaves",
            NoiseShader {
                octaves: 8,
                ..base(NoiseKind::RidgedMultifractal)
            },
        ),
        (
            "Perlin-uv-absolute",
            NoiseShader {
                space: NoiseSpace::Uv,
                absolute: true,
                ..base(NoiseKind::Perlin)
            },
        ),
        (
            "Simplex-clip-contrast",
            NoiseShader {
                low_clip: 0.2,
                high_clip: 0.8,
                brightness: 0.1,
                contrast: 0.5,
                color_1: [0.1, 0.2, 0.3],
                color_2: [0.9, 0.7, 0.4],
                ..base(NoiseKind::Simplex)
            },
        ),
        (
            "Value-relative-scale",
            NoiseShader {
                relative_scale: [0.5, 2.0, 1.5],
                global_scale: 3.0,
                seed: 123_456,
                ..base(NoiseKind::Value)
            },
        ),
        (
            "Voronoi2-seeded",
            NoiseShader {
                seed: 99,
                ..base(NoiseKind::Voronoi2)
            },
        ),
    ] {
        cases.push(case(label, Shader::Noise(n), DEFAULT_BOUND));
    }
    cases
}
