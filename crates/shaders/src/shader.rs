//! The shader enum and its sampling rules (SHADERS.md S1-S9).

use std::collections::BTreeMap;
use std::sync::Arc;

use forge_image::Image;
use forge_material::{ShadeCtx, TextureSampler};
use forge_noise::{Noise, NoiseKind};
use glam::DVec3;

use crate::blend::BlendMode;
use crate::gradient::Gradient;

/// IEC 61966-2-1 decode, applied to shader colour parameters (S1).
pub fn srgb_to_linear(v: f64) -> f64 {
    if v <= 0.04045 {
        v / 12.92
    } else {
        ((v + 0.055) / 1.055).powf(2.4)
    }
}

fn smooth(x: f64) -> f64 {
    let x = x.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

fn luminance(c: [f64; 3]) -> f64 {
    0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2]
}

fn lerp3(a: [f64; 3], b: [f64; 3], t: f64) -> [f64; 3] {
    std::array::from_fn(|i| a[i] + (b[i] - a[i]) * t)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GradientKind {
    U,
    V,
    Diagonal,
    Circular,
    Radial,
    Linear3d,
    Spherical3d,
    Cylindrical3d,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoiseSpace {
    Texture,
    Object,
    World,
    Uv,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NoiseShader {
    pub kind: NoiseKind,
    pub seed: u32,
    pub octaves: u32,
    pub space: NoiseSpace,
    /// Fraction (1.0 = 100 %).
    pub global_scale: f64,
    pub relative_scale: [f64; 3],
    pub animation_speed: f64,
    pub absolute: bool,
    pub low_clip: f64,
    pub high_clip: f64,
    pub brightness: f64,
    pub contrast: f64,
    pub color_1: [f64; 3],
    pub color_2: [f64; 3],
}

impl NoiseShader {
    /// The noise value in 0..1 before the output stage.
    pub fn raw(&self, ctx: &ShadeCtx) -> f64 {
        let p = match self.space {
            NoiseSpace::Texture | NoiseSpace::Uv => DVec3::from(ctx.uvw),
            NoiseSpace::Object | NoiseSpace::World => DVec3::from(ctx.position),
        };
        let p = p / DVec3::from(self.relative_scale);
        let n = Noise {
            kind: self.kind,
            seed: self.seed,
            scale: (0.1 * self.global_scale).max(1e-9),
            octaves: self.octaves.max(1),
            time: self.animation_speed * ctx.time,
            ..Noise::default()
        };
        let v = n.sample(p);
        if self.absolute {
            v.abs()
        } else {
            0.5 + 0.5 * v
        }
    }

    fn value(&self, ctx: &ShadeCtx) -> f64 {
        let mut v = self.raw(ctx);
        let span = (self.high_clip - self.low_clip).max(1e-9);
        v = ((v - self.low_clip) / span).clamp(0.0, 1.0);
        v += self.brightness;
        v = (v - 0.5) * (1.0 + self.contrast) + 0.5;
        v.clamp(0.0, 1.0)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GradientShader {
    pub kind: GradientKind,
    pub gradient: Gradient,
    pub cycle: bool,
    pub start: [f64; 3],
    pub end: [f64; 3],
    pub radius: f64,
    /// Fraction; turbulence displaces the ramp parameter with noise.
    pub turbulence: f64,
    pub octaves: u32,
    pub scale: f64,
    pub seed: u32,
}

impl GradientShader {
    fn t(&self, ctx: &ShadeCtx) -> f64 {
        let [u, v, _] = ctx.uvw;
        let p = DVec3::from(ctx.position);
        let mut t = match self.kind {
            GradientKind::U => u,
            // v grows upwards in texture space while the ramp starts at the top (gradient-v).
            GradientKind::V => 1.0 - v,
            GradientKind::Diagonal => (u + 1.0 - v) * 0.5,
            // the corner of the unit square is the end of the ramp (gradient-circular)
            GradientKind::Circular => (u - 0.5).hypot(v - 0.5) * std::f64::consts::SQRT_2,
            GradientKind::Radial => (v - 0.5).atan2(u - 0.5) / std::f64::consts::TAU + 0.5,
            GradientKind::Linear3d => {
                let a = DVec3::from(self.start);
                let d = DVec3::from(self.end) - a;
                let l2 = d.length_squared();
                if l2 > 0.0 { (p - a).dot(d) / l2 } else { 0.0 }
            }
            GradientKind::Spherical3d => {
                (p - DVec3::from(self.start)).length() / self.radius.max(1e-9)
            }
            GradientKind::Cylindrical3d => {
                let q = p - DVec3::from(self.start);
                q.x.hypot(q.z) / self.radius.max(1e-9)
            }
        };
        if self.turbulence != 0.0 {
            let n = Noise {
                kind: NoiseKind::Fbm,
                seed: self.seed,
                scale: (0.1 * self.scale).max(1e-9),
                octaves: self.octaves.max(1),
                ..Noise::default()
            };
            let q = if matches!(
                self.kind,
                GradientKind::U
                    | GradientKind::V
                    | GradientKind::Diagonal
                    | GradientKind::Circular
                    | GradientKind::Radial
            ) {
                DVec3::from(ctx.uvw)
            } else {
                p
            };
            t += self.turbulence * n.sample(q);
        }
        if self.cycle {
            t - t.floor()
        } else {
            t.clamp(0.0, 1.0)
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct LayerItem {
    pub shader: Shader,
    pub mode: BlendMode,
    pub opacity: f64,
    pub enabled: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorizeInput {
    Luminance,
    Red,
    Green,
    Blue,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Tiles {
    pub grout_color: [f64; 3],
    pub colors: [[f64; 3]; 3],
    pub grout_width: f64,
    pub bevel_width: f64,
    pub bevel: bool,
    pub randomize: bool,
    /// Tiles across one UV unit is `1 / pitch`.
    pub pitch: [f64; 2],
    pub seed: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Bitmap {
    pub image: Arc<Image>,
    pub repeat: bool,
    pub interpolate: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Shader {
    Color {
        color: [f64; 3],
        brightness: f64,
    },
    Gradient(GradientShader),
    Noise(NoiseShader),
    Fresnel {
        ior: f64,
        invert: bool,
        gradient: Gradient,
    },
    Falloff {
        invert: bool,
        gradient: Gradient,
    },
    Layer {
        base: Option<Box<Shader>>,
        layers: Vec<LayerItem>,
    },
    Colorizer {
        input: ColorizeInput,
        cycle: bool,
        gradient: Gradient,
        texture: Option<Box<Shader>>,
    },
    Bitmap(Bitmap),
    Checkerboard {
        color_1: [f64; 3],
        color_2: [f64; 3],
        u_frequency: f64,
        v_frequency: f64,
    },
    Tiles(Tiles),
    Fusion {
        base: Box<Shader>,
        blend: Box<Shader>,
        mode: BlendMode,
        opacity: f64,
    },
    Filter {
        texture: Option<Box<Shader>>,
        hue: f64,
        saturation: f64,
        lightness: f64,
        contrast: f64,
        gamma: f64,
        invert: bool,
    },
    Normalizer {
        texture: Option<Box<Shader>>,
        low: f64,
        high: f64,
    },
    VertexMap {
        low: [f64; 3],
        high: [f64; 3],
    },
    Ao {
        low: [f64; 3],
        high: [f64; 3],
    },
}

fn rgb_to_hsl(c: [f64; 3]) -> [f64; 3] {
    let (mx, mn) = (c[0].max(c[1]).max(c[2]), c[0].min(c[1]).min(c[2]));
    let l = (mx + mn) * 0.5;
    if mx == mn {
        return [0.0, 0.0, l];
    }
    let d = mx - mn;
    let s = if l > 0.5 {
        d / (2.0 - mx - mn)
    } else {
        d / (mx + mn)
    };
    let h = if mx == c[0] {
        (c[1] - c[2]) / d + if c[1] < c[2] { 6.0 } else { 0.0 }
    } else if mx == c[1] {
        (c[2] - c[0]) / d + 2.0
    } else {
        (c[0] - c[1]) / d + 4.0
    };
    [h / 6.0, s, l]
}

fn hsl_to_rgb(h: [f64; 3]) -> [f64; 3] {
    let [h, s, l] = h;
    if s == 0.0 {
        return [l; 3];
    }
    let q = if l < 0.5 {
        l * (1.0 + s)
    } else {
        l + s - l * s
    };
    let p = 2.0 * l - q;
    let f = |mut t: f64| {
        t -= t.floor();
        if t < 1.0 / 6.0 {
            p + (q - p) * 6.0 * t
        } else if t < 0.5 {
            q
        } else if t < 2.0 / 3.0 {
            p + (q - p) * (2.0 / 3.0 - t) * 6.0
        } else {
            p
        }
    };
    [f(h + 1.0 / 3.0), f(h), f(h - 1.0 / 3.0)]
}

fn cell_hash(seed: u32, i: i64, j: i64) -> u64 {
    let mut x = (u64::from(seed) << 32)
        ^ (i as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ (j as u64).wrapping_mul(0xC2B2_AE3D_27D4_EB4F);
    x ^= x >> 33;
    x = x.wrapping_mul(0xFF51_AFD7_ED55_8CCD);
    x ^= x >> 33;
    x
}

impl Tiles {
    fn sample(&self, ctx: &ShadeCtx) -> [f64; 3] {
        let mut p: f64 = 1.0;
        let mut cell = [0i64; 2];
        for ((cell_a, &pitch), &uv) in cell.iter_mut().zip(&self.pitch).zip(&ctx.uvw) {
            let x = uv / pitch;
            let c = x.floor();
            *cell_a = c as i64;
            let f = x - c;
            let half = pitch * 0.5;
            let d = f.min(1.0 - f) * pitch;
            let g = self.grout_width * half;
            let b = self.bevel_width * (half - g);
            p = p.min(if self.bevel && b > 0.0 {
                smooth((d - g) / b)
            } else if d < g {
                0.0
            } else {
                1.0
            });
        }
        let tile = if self.randomize {
            self.colors[(cell_hash(self.seed, cell[0], cell[1]) % 3) as usize]
        } else if (cell[0] + cell[1]).rem_euclid(2) == 0 {
            self.colors[0]
        } else {
            self.colors[1]
        };
        lerp3(self.grout_color, tile, p)
    }
}

impl Bitmap {
    fn texel(&self, x: i64, y: i64) -> [f64; 3] {
        let (w, h) = (i64::from(self.image.width), i64::from(self.image.height));
        let (x, y) = if self.repeat {
            (x.rem_euclid(w), y.rem_euclid(h))
        } else {
            (x.clamp(0, w - 1), y.clamp(0, h - 1))
        };
        let p = self.image.pixel(x as u32, y as u32);
        [f64::from(p[0]), f64::from(p[1]), f64::from(p[2])]
    }

    fn sample(&self, ctx: &ShadeCtx) -> [f64; 3] {
        let (w, h) = (f64::from(self.image.width), f64::from(self.image.height));
        let x = ctx.uvw[0] * w;
        let y = (1.0 - ctx.uvw[1]) * h;
        if !self.interpolate {
            return self.texel(x.floor() as i64, y.floor() as i64);
        }
        let (fx, fy) = (x - 0.5, y - 0.5);
        let (x0, y0) = (fx.floor(), fy.floor());
        let (tx, ty) = (fx - x0, fy - y0);
        let (x0, y0) = (x0 as i64, y0 as i64);
        let top = lerp3(self.texel(x0, y0), self.texel(x0 + 1, y0), tx);
        let bot = lerp3(self.texel(x0, y0 + 1), self.texel(x0 + 1, y0 + 1), tx);
        lerp3(top, bot, ty)
    }
}

impl Shader {
    /// The shader's colour at `ctx` (linear RGB).
    pub fn sample(&self, ctx: &ShadeCtx) -> [f64; 3] {
        let sub = |s: &Option<Box<Shader>>| s.as_ref().map_or([0.0; 3], |s| s.sample(ctx));
        match self {
            Shader::Color { color, brightness } => color.map(|c| c * brightness),
            Shader::Gradient(g) => g.gradient.eval(g.t(ctx)),
            Shader::Noise(n) => lerp3(n.color_1, n.color_2, n.value(ctx)),
            Shader::Fresnel {
                ior,
                invert,
                gradient,
            } => {
                let n = DVec3::from(ctx.normal);
                let v = DVec3::from(ctx.view);
                let f = if n.length_squared() == 0.0 || v.length_squared() == 0.0 {
                    // no geometry in the context: the observed UV-grid fallback (S6)
                    smooth(ctx.uvw[1])
                } else {
                    let cos = n.normalize().dot(v.normalize()).abs();
                    let f0 = ((ior - 1.0) / (ior + 1.0)).powi(2);
                    f0 + (1.0 - f0) * (1.0 - cos).powi(5)
                };
                gradient.eval(if *invert { 1.0 - f } else { f })
            }
            Shader::Falloff { invert, gradient } => {
                let n = DVec3::from(ctx.normal);
                let v = DVec3::from(ctx.view);
                let t = if n.length_squared() == 0.0 || v.length_squared() == 0.0 {
                    0.0
                } else {
                    1.0 - n.normalize().dot(v.normalize()).abs()
                };
                gradient.eval(if *invert { 1.0 - t } else { t })
            }
            Shader::Layer { base, layers } => {
                let mut acc = if base.is_some() { sub(base) } else { [0.0; 3] };
                for l in layers.iter().filter(|l| l.enabled) {
                    acc = l.mode.apply(acc, l.shader.sample(ctx), l.opacity);
                }
                acc
            }
            Shader::Colorizer {
                input,
                cycle,
                gradient,
                texture,
            } => {
                let c = sub(texture);
                let mut t = match input {
                    ColorizeInput::Luminance => luminance(c),
                    ColorizeInput::Red => c[0],
                    ColorizeInput::Green => c[1],
                    ColorizeInput::Blue => c[2],
                };
                t = if *cycle {
                    t - t.floor()
                } else {
                    t.clamp(0.0, 1.0)
                };
                gradient.eval(t)
            }
            Shader::Bitmap(b) => b.sample(ctx),
            Shader::Checkerboard {
                color_1,
                color_2,
                u_frequency,
                v_frequency,
            } => {
                // cells are half a period wide: `f` periods of two cells per UV unit
                let i = (ctx.uvw[0] * 2.0 * u_frequency).floor() as i64;
                let j = (ctx.uvw[1] * 2.0 * v_frequency).floor() as i64;
                if (i + j).rem_euclid(2) == 0 {
                    *color_1
                } else {
                    *color_2
                }
            }
            Shader::Tiles(t) => t.sample(ctx),
            Shader::Fusion {
                base,
                blend,
                mode,
                opacity,
            } => mode.apply(base.sample(ctx), blend.sample(ctx), *opacity),
            Shader::Filter {
                texture,
                hue,
                saturation,
                lightness,
                contrast,
                gamma,
                invert,
            } => {
                let mut hsl = rgb_to_hsl(sub(texture));
                hsl[0] += hue;
                hsl[1] = (hsl[1] * (1.0 + saturation)).clamp(0.0, 1.0);
                let mut c = hsl_to_rgb(hsl);
                for v in &mut c {
                    *v += lightness;
                    *v = (*v - 0.5) * (1.0 + contrast) + 0.5;
                    *v = v.max(0.0).powf(1.0 / gamma.max(1e-9));
                    if *invert {
                        *v = 1.0 - *v;
                    }
                }
                c
            }
            Shader::Normalizer { texture, low, high } => {
                let span = (high - low).max(1e-9);
                sub(texture).map(|v| ((v - low) / span).clamp(0.0, 1.0))
            }
            Shader::VertexMap { low, high } => lerp3(
                *low,
                *high,
                ctx.vertex_weight.unwrap_or(0.0).clamp(0.0, 1.0),
            ),
            Shader::Ao { low, high } => lerp3(*low, *high, ctx.ao.unwrap_or(1.0).clamp(0.0, 1.0)),
        }
    }
}

/// Named shaders of a document; implements the IR's texture hook (S9).
#[derive(Debug, Clone, Default)]
pub struct ShaderSet {
    pub shaders: BTreeMap<String, Shader>,
}

impl TextureSampler for ShaderSet {
    fn sample(&self, name: &str, ctx: &ShadeCtx) -> [f64; 3] {
        self.shaders.get(name).map_or([1.0; 3], |s| s.sample(ctx))
    }
}
