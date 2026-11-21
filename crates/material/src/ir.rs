//! The shading IR and its compiler (MATERIAL.md M3).

use crate::material::Material;

pub type NodeIx = usize;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MixMode {
    Normal,
    Add,
    Subtract,
    Multiply,
}

impl MixMode {
    pub fn parse(s: &str) -> MixMode {
        match s {
            "add" => MixMode::Add,
            "subtract" => MixMode::Subtract,
            "multiply" => MixMode::Multiply,
            _ => MixMode::Normal,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum IrNode {
    Color([f64; 3]),
    Texture(String),
    Scale {
        input: NodeIx,
        factor: f64,
    },
    Mix {
        base: NodeIx,
        top: NodeIx,
        mode: MixMode,
        strength: f64,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffuseModel {
    Lambertian,
    OrenNayar,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Diffuse {
    pub color: NodeIx,
    pub level: f64,
    pub falloff: f64,
    pub model: DiffuseModel,
    pub roughness: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Specular {
    pub kind: String,
    pub width: f64,
    pub falloff: f64,
    pub inner_width: f64,
    pub strength: f64,
    pub color: NodeIx,
    pub metal: bool,
    pub colored: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Transmission {
    pub color: NodeIx,
    pub ior: f64,
    pub absorption: [f64; 3],
    pub absorption_distance: f64,
    pub fresnel_reflectivity: f64,
    pub total_internal_reflection: bool,
    pub exit_reflections: bool,
    pub additive: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Fog {
    pub color: NodeIx,
    pub distance: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Alpha {
    pub texture: Option<NodeIx>,
    pub color: [f64; 3],
    pub delta: [f64; 3],
    pub invert: bool,
    pub soft: bool,
    pub image_alpha: bool,
    pub premultiplied: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Bump {
    pub texture: NodeIx,
    pub strength: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NormalMap {
    pub texture: NodeIx,
    pub strength: f64,
    pub method: String,
    pub flip: [bool; 3],
    pub swap_yz: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Displacement {
    pub texture: NodeIx,
    pub strength: f64,
    pub height: f64,
    pub kind: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Diffusion {
    pub brightness: f64,
    pub affect_luminance: bool,
    pub affect_specular: bool,
    pub affect_reflection: bool,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Outputs {
    pub diffuse: Option<Diffuse>,
    pub specular: Option<Specular>,
    pub emission: Option<NodeIx>,
    pub transmission: Option<Transmission>,
    pub environment: Option<NodeIx>,
    pub fog: Option<Fog>,
    pub alpha: Option<Alpha>,
    pub bump: Option<Bump>,
    pub normal: Option<NormalMap>,
    pub displacement: Option<Displacement>,
    pub diffusion: Option<Diffusion>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ShadingGraph {
    pub nodes: Vec<IrNode>,
    pub outputs: Outputs,
}

/// Where a texture is sampled.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct ShadeCtx {
    pub uvw: [f64; 3],
    pub position: [f64; 3],
    pub normal: [f64; 3],
    /// Direction from the surface point to the viewer (zero when unknown).
    pub view: [f64; 3],
    pub time: f64,
    /// Weight of the active vertex map at the point, when the host has one.
    pub vertex_weight: Option<f64>,
    /// Ambient occlusion at the point (1 = open), when the renderer supplies it.
    pub ao: Option<f64>,
}

/// The shader hook of the interpreter.
pub trait TextureSampler {
    fn sample(&self, name: &str, ctx: &ShadeCtx) -> [f64; 3];
}

/// A sampler that returns white for every texture.
pub struct White;
impl TextureSampler for White {
    fn sample(&self, _: &str, _: &ShadeCtx) -> [f64; 3] {
        [1.0; 3]
    }
}

/// `base + (op(base, top) - base) * strength` per component (M3 rule 2).
pub fn mix(base: [f64; 3], top: [f64; 3], mode: MixMode, strength: f64) -> [f64; 3] {
    std::array::from_fn(|i| {
        let op = match mode {
            MixMode::Normal => top[i],
            MixMode::Add => base[i] + top[i],
            MixMode::Subtract => base[i] - top[i],
            MixMode::Multiply => base[i] * top[i],
        };
        base[i] + (op - base[i]) * strength
    })
}

impl ShadingGraph {
    fn push(&mut self, n: IrNode) -> NodeIx {
        self.nodes.push(n);
        self.nodes.len() - 1
    }

    /// Rule 1: a colour channel with brightness and an optional texture slot.
    #[allow(clippy::too_many_arguments)]
    fn channel(
        &mut self,
        color: [f64; 3],
        brightness: f64,
        tex: &str,
        mode: &str,
        strength: f64,
    ) -> NodeIx {
        let c = self.push(IrNode::Color(color));
        let base = self.push(IrNode::Scale {
            input: c,
            factor: brightness,
        });
        if tex.is_empty() {
            return base;
        }
        let t = self.push(IrNode::Texture(tex.to_string()));
        let top = self.push(IrNode::Scale {
            input: t,
            factor: brightness,
        });
        self.push(IrNode::Mix {
            base,
            top,
            mode: MixMode::parse(mode),
            strength,
        })
    }

    fn texture(&mut self, tex: &str) -> Option<NodeIx> {
        (!tex.is_empty()).then(|| self.push(IrNode::Texture(tex.to_string())))
    }

    /// Evaluates node `ix` (M3 rule 8).
    pub fn eval(&self, ix: NodeIx, sampler: &dyn TextureSampler, ctx: &ShadeCtx) -> [f64; 3] {
        match &self.nodes[ix] {
            IrNode::Color(c) => *c,
            IrNode::Texture(name) => sampler.sample(name, ctx),
            IrNode::Scale { input, factor } => self.eval(*input, sampler, ctx).map(|v| v * factor),
            IrNode::Mix {
                base,
                top,
                mode,
                strength,
            } => mix(
                self.eval(*base, sampler, ctx),
                self.eval(*top, sampler, ctx),
                *mode,
                *strength,
            ),
        }
    }
}

/// Compiles a material (M3).
pub fn compile(m: &Material) -> ShadingGraph {
    let mut g = ShadingGraph::default();
    let ch = |g: &mut ShadingGraph, p: &str, c: &str, b: &str| {
        g.channel(
            m.c(c),
            m.f(b),
            m.e(&format!("{p}.texture")),
            m.e(&format!("{p}.mix_mode")),
            m.f(&format!("{p}.mix_strength")),
        )
    };
    if m.enabled("color") {
        let color = ch(&mut g, "color", "color.color", "color.brightness");
        g.outputs.diffuse = Some(Diffuse {
            color,
            level: m.f("color.diffuse_level"),
            falloff: m.f("color.diffuse_falloff"),
            model: if m.e("color.model") == "oren_nayar" {
                DiffuseModel::OrenNayar
            } else {
                DiffuseModel::Lambertian
            },
            roughness: m.f("color.roughness"),
        });
    }
    if m.enabled("diffusion") {
        g.outputs.diffusion = Some(Diffusion {
            brightness: m.f("diffusion.brightness"),
            affect_luminance: m.b("diffusion.affect_luminance"),
            affect_specular: m.b("diffusion.affect_specular"),
            affect_reflection: m.b("diffusion.affect_reflection"),
        });
    }
    if m.enabled("luminance") {
        g.outputs.emission = Some(ch(
            &mut g,
            "luminance",
            "luminance.color",
            "luminance.brightness",
        ));
    }
    if m.enabled("transparency") {
        let color = ch(
            &mut g,
            "transparency",
            "transparency.color",
            "transparency.brightness",
        );
        g.outputs.transmission = Some(Transmission {
            color,
            ior: m.f("transparency.refraction"),
            absorption: m.c("transparency.absorption_color"),
            absorption_distance: m.f("transparency.absorption_distance"),
            fresnel_reflectivity: m.f("transparency.fresnel_reflectivity"),
            total_internal_reflection: m.b("transparency.total_internal_reflection"),
            exit_reflections: m.b("transparency.exit_reflections"),
            additive: m.b("transparency.additive"),
        });
    }
    if m.enabled("reflectance") {
        let kind = m.e("reflectance.spec.type").to_string();
        let legacy = kind.ends_with("_legacy") && kind != "reflection_legacy";
        let global = if legacy {
            m.f("reflectance.global_specular")
        } else {
            m.f("reflectance.global_reflection")
        };
        let base = g.channel(
            m.c("reflectance.spec.color"),
            m.f("reflectance.spec.color_brightness"),
            m.e("reflectance.spec.color_texture"),
            m.e("reflectance.spec.color_mix_mode"),
            m.f("reflectance.spec.color_mix_strength"),
        );
        let color = g.push(IrNode::Scale {
            input: base,
            factor: m.f("reflectance.spec.mask_amount"),
        });
        g.outputs.specular = Some(Specular {
            kind,
            width: m.f("reflectance.spec.width"),
            falloff: m.f("reflectance.spec.falloff"),
            inner_width: m.f("reflectance.spec.inner_width"),
            strength: m.f("reflectance.spec.strength") * global,
            color,
            metal: m.e("reflectance.spec.attenuation") == "metal",
            colored: m.b("reflectance.spec.colored"),
        });
    }
    if m.enabled("environment") {
        g.outputs.environment = Some(ch(
            &mut g,
            "environment",
            "environment.color",
            "environment.brightness",
        ));
    }
    if m.enabled("fog") {
        let color = g.channel(m.c("fog.color"), m.f("fog.brightness"), "", "normal", 1.0);
        g.outputs.fog = Some(Fog {
            color,
            distance: m.f("fog.distance"),
        });
    }
    if m.enabled("alpha") {
        let texture = g.texture(m.e("alpha.texture"));
        g.outputs.alpha = Some(Alpha {
            texture,
            color: m.c("alpha.color"),
            delta: m.c("alpha.delta"),
            invert: m.b("alpha.invert"),
            soft: m.b("alpha.soft"),
            image_alpha: m.b("alpha.image_alpha"),
            premultiplied: m.b("alpha.premultiplied"),
        });
    }
    if m.enabled("bump")
        && let Some(texture) = g.texture(m.e("bump.texture"))
    {
        g.outputs.bump = Some(Bump {
            texture,
            strength: m.f("bump.strength"),
        });
    }
    if m.enabled("normal")
        && let Some(texture) = g.texture(m.e("normal.texture"))
    {
        g.outputs.normal = Some(NormalMap {
            texture,
            strength: m.f("normal.strength"),
            method: m.e("normal.method").to_string(),
            flip: [
                m.b("normal.flip_x"),
                m.b("normal.flip_y"),
                m.b("normal.flip_z"),
            ],
            swap_yz: m.b("normal.swap_yz"),
        });
    }
    if m.enabled("displacement")
        && let Some(texture) = g.texture(m.e("displacement.texture"))
    {
        g.outputs.displacement = Some(Displacement {
            texture,
            strength: m.f("displacement.strength"),
            height: m.f("displacement.height"),
            kind: m.e("displacement.type").to_string(),
        });
    }
    g
}
