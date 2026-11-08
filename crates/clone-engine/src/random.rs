//! The Random effector (clone-toolset spec E11-E16): the Plain effector with a random,
//! gaussian, noise or turbulence value per clone and channel.

use forge_core::{NodeId, NodeType, ParamDesc, Scene, Unit};
use forge_fields::FieldList;
use forge_noise::{Noise, NoiseKind, octave_seed};
use glam::DVec3;

use crate::effector::{Effector, EffectorCtx};
use crate::node::Params;
use crate::plain::{ClonePoint, CommonDefaults, Plain, Weights, common_type, settings_from_params};
use crate::{CloneData, CloneError};

/// Type id of the Random effector.
pub const RANDOM: &str = "forge.effector.random";

const RND: &str = "Random";
const BIG: f64 = 1e20;
const SEED_MAX: i64 = 999_999_999;
/// Feature size of the noise in cm at Scale 100 % (spec E13).
pub const NOISE_FEATURE: f64 = 300.0;

/// The Random effector's node type: the Plain parameters with the Random defaults, plus
/// Random Mode, Synchronized, Indexed, Seed, Space, Animation Speed and the noise Scale.
pub fn random_type() -> NodeType {
    let defaults = CommonDefaults {
        minimum: -1.0,
        position: true,
        p: [50.0; 3],
        rotation: false,
        r: [0.0; 3],
        scale: false,
        color_mode: "off",
        uniform_scale: true,
        scale_value: 1.0,
        s: [1.0; 3],
    };
    common_type(RANDOM, "Random", &defaults)
        .param(
            ParamDesc::enumeration(
                "random_mode",
                "Random Mode",
                "random",
                &[
                    ("random", "Random"),
                    ("gaussian", "Gaussian"),
                    ("noise", "Noise"),
                    ("turbulence", "Turbulence"),
                    ("sorted", "Sorted"),
                ],
            )
            .group(RND),
        )
        .param(ParamDesc::bool("synchronized", "Synchronized", false).group(RND))
        .param(ParamDesc::bool("indexed", "Indexed", false).group(RND))
        .param(
            ParamDesc::int("seed", "Seed", 12345)
                .range(0, SEED_MAX)
                .group(RND),
        )
        .param(
            ParamDesc::enumeration(
                "space",
                "Space",
                "global",
                &[("global", "Global"), ("uv", "UV")],
            )
            .group(RND),
        )
        .param(
            ParamDesc::float("animation_speed", "Animation Speed", 1.0)
                .range(0.0, BIG)
                .unit(Unit::Percent)
                .group(RND),
        )
        .param(
            ParamDesc::float("noise_scale", "Scale", 1.0)
                .range(0.0, BIG)
                .unit(Unit::Percent)
                .group(RND),
        )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RandomMode {
    Random,
    Gaussian,
    Noise,
    Turbulence,
    Sorted,
}

/// The Random effector's settings: the Plain settings (its Minimum / Maximum are the range
/// of the random value) and the source of the value.
#[derive(Debug, Clone, PartialEq)]
pub struct Random {
    pub base: Plain,
    pub mode: RandomMode,
    pub synchronized: bool,
    pub indexed: bool,
    pub seed: u64,
    pub uv_space: bool,
    /// Fraction (1 = 100 %).
    pub animation_speed: f64,
    /// Fraction (1 = 100 %).
    pub noise_scale: f64,
}

impl Default for Random {
    /// The node defaults.
    fn default() -> Self {
        Random {
            base: Plain {
                minimum: -1.0,
                position: Some(DVec3::splat(50.0)),
                color_mode: crate::ColorMode::Off,
                uniform_scale: true,
                ..Plain::default()
            },
            mode: RandomMode::Random,
            synchronized: false,
            indexed: false,
            seed: 12345,
            uv_space: false,
            animation_speed: 1.0,
            noise_scale: 1.0,
        }
    }
}

/// The Random effector of a node; `None` if the node is disabled.
pub(crate) fn from_node(scene: &Scene, id: NodeId) -> Option<Random> {
    let node = scene.node(id)?;
    let ty = random_type();
    let base = settings_from_params(&ty, &node.params, scene.world_matrix(id)?)?;
    let p = Params {
        ty: &ty,
        values: &node.params,
    };
    Some(Random {
        base,
        mode: match p.option("random_mode").as_str() {
            "gaussian" => RandomMode::Gaussian,
            "noise" => RandomMode::Noise,
            "turbulence" => RandomMode::Turbulence,
            "sorted" => RandomMode::Sorted,
            _ => RandomMode::Random,
        },
        synchronized: p.bool("synchronized"),
        indexed: p.bool("indexed"),
        seed: p.int("seed").clamp(0, SEED_MAX) as u64,
        uv_space: p.option("space") == "uv",
        animation_speed: p.float("animation_speed"),
        noise_scale: p.float("noise_scale"),
    })
}

/// The finaliser of SplitMix64 (public domain) on `x`.
fn mix(x: u64) -> u64 {
    let mut z = x.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// The counter-based uniform draw `u(seed, key, k)` in `[0, 1)` (spec E12).
pub fn uniform(seed: u64, key: u64, k: u64) -> f64 {
    let x = seed
        .wrapping_mul(0x9E37_79B9_7F4A_7C15)
        .wrapping_add(key.wrapping_mul(0xD1B5_4A32_D192_ED03))
        .wrapping_add(k.wrapping_mul(0x94D0_49BB_1331_11EB));
    (mix(x) >> 11) as f64 / (1u64 << 53) as f64
}

/// The gaussian `r` (spec E12): `0.5 + g / 12` clamped, `g` a standard normal.
fn gaussian(seed: u64, key: u64, k: u64) -> f64 {
    let u1 = 1.0 - uniform(seed, key, k);
    let u2 = uniform(seed, key, k + 32);
    let g = (-2.0 * u1.ln()).sqrt() * (std::f64::consts::TAU * u2).cos();
    (0.5 + g / 12.0).clamp(0.0, 1.0)
}

/// Number of draws per clone: position 3, rotation 3, scale 3, weight 1.
const DRAWS: usize = 10;

impl Random {
    /// The three noises (position, rotation, scale and weight) at the evaluation time.
    fn noises(&self, time: f64) -> [Noise; 3] {
        let kind = if self.mode == RandomMode::Turbulence {
            NoiseKind::Turbulence
        } else {
            NoiseKind::Perlin
        };
        std::array::from_fn(|g| Noise {
            kind,
            seed: octave_seed(self.seed as u32, 1 + g as u32),
            scale: NOISE_FEATURE * self.noise_scale.max(1e-6),
            octaves: 1,
            time: time * self.animation_speed,
            ..Noise::default()
        })
    }

    /// The ten values `r` of one clone (spec E12): index `k` is the draw of the channel.
    fn draws(&self, noises: &[Noise; 3], c: &ClonePoint) -> [f64; DRAWS] {
        let key = if self.indexed {
            u64::from(c.index)
        } else {
            c.position as u64
        };
        let mut r = [0.0; DRAWS];
        match self.mode {
            RandomMode::Random => {
                for (k, r) in r.iter_mut().enumerate() {
                    *r = uniform(self.seed, key, k as u64);
                }
            }
            RandomMode::Gaussian => {
                for (k, r) in r.iter_mut().enumerate() {
                    *r = gaussian(self.seed, key, k as u64);
                }
            }
            RandomMode::Noise | RandomMode::Turbulence | RandomMode::Sorted => {
                let point = if self.uv_space {
                    c.uvw * 4.0 * NOISE_FEATURE * self.noise_scale.max(1e-6)
                } else {
                    c.world
                };
                for (k, r) in r.iter_mut().enumerate() {
                    let group = (k / 3).min(2);
                    let comp = (k % 3 + if k == 9 { 3 } else { 0 }) as f64;
                    let at = point + comp * DVec3::new(71.3, 37.7, 19.9);
                    let t = noises[group].time;
                    *r = (0.5 + 0.5 * noises[group].sample4(at, t)).clamp(0.0, 1.0);
                }
            }
        }
        if self.synchronized {
            r = [r[0]; DRAWS];
        }
        r
    }
}

impl Effector for Random {
    fn apply(
        &self,
        data: &mut CloneData,
        falloff: &FieldList,
        ctx: &EffectorCtx,
    ) -> Result<(), CloneError> {
        if self.mode == RandomMode::Sorted {
            return Err(CloneError::Unsupported("Random Mode Sorted"));
        }
        let noises = self.noises(ctx.eval.time);
        let (min, max) = (self.base.minimum, self.base.maximum);
        let uniform_scale = self.base.uniform_scale;
        self.base.apply_weighted(data, falloff, ctx, &mut |c| {
            let r = self.draws(&noises, c);
            let w = |k: usize| self.base.strength * c.value * (min + r[k] * (max - min));
            Weights {
                pos: DVec3::new(w(0), w(1), w(2)),
                rot: DVec3::new(w(3), w(4), w(5)),
                scale: if uniform_scale {
                    DVec3::splat(w(6))
                } else {
                    DVec3::new(w(6), w(7), w(8))
                },
                scalar: w(9),
            }
        })
    }
}

/// True if the effector node `id` changes with time: a Time, Formula or Delay effector, or a
/// Random effector in a noise or turbulence mode with a non-zero animation speed (CORE.md
/// section 5b).
pub fn effector_reads_time(scene: &Scene, id: NodeId) -> bool {
    let Some(node) = scene.node(id) else {
        return false;
    };
    match node.type_id.as_str() {
        crate::time::TIME | crate::formula::FORMULA | crate::delay::DELAY => true,
        RANDOM => from_node(scene, id).is_some_and(|r| {
            matches!(r.mode, RandomMode::Noise | RandomMode::Turbulence) && r.animation_speed != 0.0
        }),
        _ => false,
    }
}
