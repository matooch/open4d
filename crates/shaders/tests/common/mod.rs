//! Shared by the WGSL tests: the case list of every shader family (REALTIME.md §2.9).
#![allow(dead_code)]

use forge_noise::NoiseKind;
use forge_shaders::shader::*;
use forge_shaders::{Gradient, Interp};

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

fn ramp(interp: Interp) -> Gradient {
    Gradient {
        knots: vec![
            (0.1, [0.9, 0.1, 0.1]),
            (0.4, [0.2, 0.8, 0.3]),
            (0.45, [0.1, 0.2, 0.9]),
            (0.9, [1.0, 1.0, 0.0]),
        ],
        interp,
    }
}

fn gradient_shader(kind: GradientKind) -> GradientShader {
    GradientShader {
        kind,
        gradient: ramp(Interp::Cubic),
        cycle: false,
        start: [-1.0, 0.5, -0.5],
        end: [1.5, -1.0, 1.0],
        radius: 1.5,
        turbulence: 0.0,
        octaves: 4,
        scale: 1.0,
        seed: 11,
    }
}

pub fn gradient_cases() -> Vec<Case> {
    let mut cases = Vec::new();
    for kind in [
        GradientKind::U,
        GradientKind::V,
        GradientKind::Diagonal,
        GradientKind::Circular,
        GradientKind::Radial,
        GradientKind::Linear3d,
        GradientKind::Spherical3d,
        GradientKind::Cylindrical3d,
    ] {
        cases.push(case(
            &format!("{kind:?}"),
            Shader::Gradient(gradient_shader(kind)),
            DEFAULT_BOUND,
        ));
        cases.push(case(
            &format!("{kind:?}-cycle-linear-turbulence"),
            Shader::Gradient(GradientShader {
                cycle: true,
                gradient: ramp(Interp::Linear),
                turbulence: 0.3,
                scale: 4.0,
                ..gradient_shader(kind)
            }),
            DEFAULT_BOUND,
        ));
    }
    cases.push(case(
        "U-step-ramp",
        Shader::Gradient(GradientShader {
            gradient: ramp(Interp::None),
            ..gradient_shader(GradientKind::U)
        }),
        DEFAULT_BOUND,
    ));
    cases.push(case(
        "V-one-knot",
        Shader::Gradient(GradientShader {
            gradient: Gradient {
                knots: vec![(0.5, [0.3, 0.6, 0.9])],
                interp: Interp::Linear,
            },
            ..gradient_shader(GradientKind::V)
        }),
        DEFAULT_BOUND,
    ));
    cases.push(case(
        "Linear3d-degenerate-axis",
        Shader::Gradient(GradientShader {
            end: [-1.0, 0.5, -0.5],
            ..gradient_shader(GradientKind::Linear3d)
        }),
        DEFAULT_BOUND,
    ));
    cases
}

/// Color, Checkerboard, Fresnel and Falloff.
pub fn surface_cases() -> Vec<Case> {
    let g = Gradient::default();
    vec![
        case(
            "Color",
            Shader::Color {
                color: [0.2, 0.5, 0.7],
                brightness: 1.5,
            },
            DEFAULT_BOUND,
        ),
        case(
            "Checkerboard",
            Shader::Checkerboard {
                color_1: [0.9, 0.1, 0.2],
                color_2: [0.1, 0.8, 0.4],
                u_frequency: 10.0,
                v_frequency: 6.5,
            },
            DEFAULT_BOUND,
        ),
        case(
            "Fresnel",
            Shader::Fresnel {
                ior: 1.5,
                invert: false,
                gradient: g.clone(),
            },
            DEFAULT_BOUND,
        ),
        case(
            "Fresnel-invert-ramp",
            Shader::Fresnel {
                ior: 2.4,
                invert: true,
                gradient: ramp(Interp::Linear),
            },
            DEFAULT_BOUND,
        ),
        case(
            "Falloff",
            Shader::Falloff {
                invert: false,
                gradient: g,
            },
            DEFAULT_BOUND,
        ),
        case(
            "Falloff-invert-ramp",
            Shader::Falloff {
                invert: true,
                gradient: ramp(Interp::Cubic),
            },
            DEFAULT_BOUND,
        ),
    ]
}

pub fn tiles_cases() -> Vec<Case> {
    let t = |bevel, randomize, pitch: [f64; 2], seed, grout_width, bevel_width| {
        Shader::Tiles(Tiles {
            grout_color: [0.1, 0.1, 0.12],
            colors: [[0.9, 0.2, 0.2], [0.2, 0.9, 0.2], [0.2, 0.2, 0.9]],
            grout_width,
            bevel_width,
            bevel,
            randomize,
            pitch,
            seed,
        })
    };
    vec![
        case(
            "Tiles-bevel",
            t(true, false, [0.1, 0.1], 0, 0.2, 0.5),
            DEFAULT_BOUND,
        ),
        case(
            "Tiles-no-bevel",
            t(false, false, [0.25, 0.125], 0, 0.3, 0.5),
            DEFAULT_BOUND,
        ),
        case(
            "Tiles-randomize",
            t(true, true, [0.1, 0.2], 12345, 0.1, 0.4),
            DEFAULT_BOUND,
        ),
        case(
            "Tiles-randomize-seed0",
            t(false, true, [0.05, 0.05], 0, 0.15, 0.0),
            DEFAULT_BOUND,
        ),
    ]
}
