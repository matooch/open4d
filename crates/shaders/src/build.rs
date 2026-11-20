//! Building a [`Shader`] from a parameter map (SHADERS.md S10).

use std::collections::BTreeMap;
use std::sync::Arc;

use forge_core::Value;
use forge_image::Image;
use forge_noise::NoiseKind;

use crate::blend::BlendMode;
use crate::gradient::{Gradient, Interp};
use crate::params::{PREFIX, shader_types};
use crate::shader::*;

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum ShaderError {
    #[error("unknown shader type {0:?}")]
    UnknownType(String),
    #[error("unsupported: {0}")]
    Unsupported(String),
    #[error("{0}")]
    Inputs(String),
}

pub type Values = BTreeMap<String, Value>;

/// Defaults of a type merged with `overrides`.
struct V(Values);

impl V {
    fn new(type_id: &str, overrides: &Values) -> Result<V, ShaderError> {
        let t = shader_types()
            .into_iter()
            .find(|t| t.id == type_id)
            .ok_or_else(|| ShaderError::UnknownType(type_id.to_string()))?;
        let mut m: Values = t
            .params
            .iter()
            .map(|p| (p.id.clone(), p.default.clone()))
            .collect();
        for (k, v) in overrides {
            if m.contains_key(k) {
                m.insert(k.clone(), v.clone());
            }
        }
        Ok(V(m))
    }
    fn f(&self, id: &str) -> f64 {
        match self.0.get(id) {
            Some(Value::Float(v)) => *v,
            Some(Value::Int(v)) => *v as f64,
            _ => 0.0,
        }
    }
    fn i(&self, id: &str) -> i64 {
        self.f(id) as i64
    }
    fn b(&self, id: &str) -> bool {
        matches!(self.0.get(id), Some(Value::Bool(true)))
    }
    /// A colour parameter, decoded to linear (S1).
    fn c(&self, id: &str) -> [f64; 3] {
        match self.0.get(id) {
            Some(Value::Color(c)) => c.map(srgb_to_linear),
            _ => [0.0; 3],
        }
    }
    fn v3(&self, id: &str) -> [f64; 3] {
        match self.0.get(id) {
            Some(Value::Vec3(v)) => *v,
            _ => [0.0; 3],
        }
    }
    fn e(&self, id: &str) -> &str {
        match self.0.get(id) {
            Some(Value::Enum(s)) | Some(Value::String(s)) => s,
            _ => "",
        }
    }
    fn gradient(&self) -> Gradient {
        Gradient {
            knots: vec![(0.0, self.c("color_start")), (1.0, self.c("color_end"))],
            interp: match self.e("interpolation") {
                "none" => Interp::None,
                "linear" => Interp::Linear,
                _ => Interp::Cubic,
            },
        }
    }
}

fn noise_kind(id: &str) -> Result<NoiseKind, ShaderError> {
    Ok(match id {
        "noise" => NoiseKind::Perlin,
        "box" => NoiseKind::Box,
        "blistered_turbulence" => NoiseKind::BlisteredTurbulence,
        "cell_noise" => NoiseKind::Cell,
        "displaced_turbulence" => NoiseKind::DisplacedTurbulence,
        "electric" => NoiseKind::Electric,
        "fbm" => NoiseKind::Fbm,
        "gaseous" => NoiseKind::Gaseous,
        "turbulence" => NoiseKind::Turbulence,
        "wavy_turbulence" => NoiseKind::WavyTurbulence,
        "cell_voronoi" | "voronoi_1" => NoiseKind::Voronoi1,
        "displaced_voronoi" => NoiseKind::DisplacedVoronoi,
        "voronoi_2" => NoiseKind::Voronoi2,
        "voronoi_3" => NoiseKind::Voronoi3,
        other => {
            return Err(ShaderError::Unsupported(format!(
                "noise type {other} has no public definition"
            )));
        }
    })
}

/// Child shaders and the image a shader needs.
#[derive(Debug, Clone, Default)]
pub struct Inputs {
    pub shaders: Vec<Shader>,
    pub image: Option<Arc<Image>>,
}

fn first(inputs: &Inputs) -> Option<Box<Shader>> {
    inputs.shaders.first().cloned().map(Box::new)
}

/// Builds the shader of `type_id` (`forge.shader.<name>`) from `values` over the type's defaults.
pub fn build(type_id: &str, values: &Values, inputs: &Inputs) -> Result<Shader, ShaderError> {
    let v = V::new(type_id, values)?;
    let name = type_id.strip_prefix(PREFIX).unwrap_or(type_id);
    let unsup = |s: &str| Err(ShaderError::Unsupported(s.to_string()));
    Ok(match name {
        "color" => Shader::Color {
            color: v.c("color"),
            brightness: v.f("brightness"),
        },
        "gradient" => Shader::Gradient(GradientShader {
            kind: match v.e("type") {
                "v" => GradientKind::V,
                "diagonal" => GradientKind::Diagonal,
                "circular" => GradientKind::Circular,
                "radial" => GradientKind::Radial,
                "linear_3d" => GradientKind::Linear3d,
                "spherical_3d" => GradientKind::Spherical3d,
                "cylindrical_3d" => GradientKind::Cylindrical3d,
                _ => GradientKind::U,
            },
            gradient: v.gradient(),
            cycle: v.b("cycle"),
            start: v.v3("start"),
            end: v.v3("end"),
            radius: v.f("radius"),
            turbulence: v.f("turbulence"),
            octaves: v.i("octaves") as u32,
            scale: v.f("scale"),
            seed: v.i("seed") as u32,
        }),
        "noise" => Shader::Noise(NoiseShader {
            kind: noise_kind(v.e("noise"))?,
            seed: v.i("seed") as u32,
            octaves: v.i("octaves") as u32,
            space: match v.e("space") {
                "object" => NoiseSpace::Object,
                "world" => NoiseSpace::World,
                "uv" => NoiseSpace::Uv,
                _ => NoiseSpace::Texture,
            },
            global_scale: v.f("global_scale"),
            relative_scale: v.v3("relative_scale"),
            animation_speed: v.f("animation_speed"),
            absolute: v.b("absolute"),
            low_clip: v.f("low_clip"),
            high_clip: v.f("high_clip"),
            brightness: v.f("brightness"),
            contrast: v.f("contrast"),
            color_1: v.c("color_1"),
            color_2: v.c("color_2"),
        }),
        "fresnel" => Shader::Fresnel {
            ior: v.f("ior"),
            invert: v.b("invert"),
            gradient: Gradient {
                knots: vec![(0.0, [0.0; 3]), (1.0, [1.0; 3])],
                interp: Interp::Linear,
            },
        },
        "falloff" => Shader::Falloff {
            invert: v.b("invert"),
            gradient: v.gradient(),
        },
        "layer" => {
            let mode = BlendMode::parse(v.e("blend_mode")).unwrap_or(BlendMode::Normal);
            let mut it = inputs.shaders.iter().cloned();
            let base = it.next().map(Box::new);
            let layers = it
                .map(|shader| LayerItem {
                    shader,
                    mode,
                    opacity: v.f("opacity"),
                    enabled: true,
                })
                .collect();
            Shader::Layer { base, layers }
        }
        "colorizer" => Shader::Colorizer {
            input: match v.e("input") {
                "red" => ColorizeInput::Red,
                "green" => ColorizeInput::Green,
                "blue" => ColorizeInput::Blue,
                _ => ColorizeInput::Luminance,
            },
            cycle: v.b("cycle"),
            gradient: v.gradient(),
            texture: first(inputs),
        },
        "bitmap" => {
            let image = inputs
                .image
                .clone()
                .ok_or_else(|| ShaderError::Inputs("bitmap needs a decoded image".into()))?;
            Shader::Bitmap(Bitmap {
                image,
                repeat: v.b("repeat"),
                interpolate: v.b("interpolate"),
            })
        }
        "checkerboard" => Shader::Checkerboard {
            color_1: v.c("color_1"),
            color_2: v.c("color_2"),
            u_frequency: v.f("u_frequency"),
            v_frequency: v.f("v_frequency"),
        },
        "tiles" => {
            if v.e("pattern") != "squares" {
                return unsup("tile pattern other than Squares (only Squares was observed)");
            }
            if v.f("rotate") != 0.0 {
                return unsup("tile rotation");
            }
            let g = v.f("global_scale");
            Shader::Tiles(Tiles {
                grout_color: v.c("grout_color"),
                colors: [
                    v.c("tiles_color_1"),
                    v.c("tiles_color_2"),
                    v.c("tiles_color_3"),
                ],
                grout_width: v.f("grout_width"),
                bevel_width: v.f("bevel_width"),
                bevel: v.b("bevel"),
                randomize: v.b("randomize_color"),
                pitch: [0.1 / (g * v.f("u_scale")), 0.1 / (g * v.f("v_scale"))],
                seed: v.i("seed") as u32,
            })
        }
        "brick" => {
            return unsup("brick: the observed pattern is noise-perturbed and was not derived");
        }
        "field" => {
            return unsup("field shader: needs a field evaluated at the sample point (not wired)");
        }
        "fusion" => {
            if inputs.shaders.len() < 2 {
                return Err(ShaderError::Inputs(
                    "fusion needs a base and a blend shader".into(),
                ));
            }
            Shader::Fusion {
                base: Box::new(inputs.shaders[0].clone()),
                blend: Box::new(inputs.shaders[1].clone()),
                mode: BlendMode::parse(v.e("blend_mode")).unwrap_or(BlendMode::Normal),
                opacity: v.f("opacity"),
            }
        }
        "filter" => Shader::Filter {
            texture: first(inputs),
            hue: v.f("hue") / std::f64::consts::TAU,
            saturation: v.f("saturation"),
            lightness: v.f("lightness"),
            contrast: v.f("contrast"),
            gamma: v.f("gamma"),
            invert: v.b("invert"),
        },
        "normalizer" => Shader::Normalizer {
            texture: first(inputs),
            low: v.f("low"),
            high: v.f("high"),
        },
        "vertex_map" => Shader::VertexMap {
            low: v.c("color_low"),
            high: v.c("color_high"),
        },
        "ao" => Shader::Ao {
            low: v.c("color_low"),
            high: v.c("color_high"),
        },
        _ => return Err(ShaderError::UnknownType(type_id.to_string())),
    })
}
