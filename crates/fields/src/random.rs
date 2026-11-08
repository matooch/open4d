//! Random field (docs/specs/FIELDS.md F6): noise, random and sorted values.

use forge_core::{EvalCtx, NodeId, NodeType, ParamDesc, Scene, Unit, Value};
use forge_noise::{Noise, NoiseKind, hash_cell};
use glam::{DMat4, DVec3};

use crate::node::{BIG, FIELD, Params, field_type};
use crate::remap::Remap;
use crate::{Field, FieldError, FieldSample};

pub const RANDOM: &str = "forge.field.random";

/// Feature size in world units at Scale 100 % (NOISE.md N7: any size from 50 to 100 matches the
/// observed spread and decorrelation at the 75 unit grid).
const BASE_SCALE: f64 = 100.0;

/// What the field computes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RandomMode {
    Random,
    Sorted,
    Noise,
}

/// The space the sample position is taken in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RandomSpace {
    Global,
    Field,
    None,
}

/// The 31 noise names of the field (id, label, kind). A `None` kind is not implemented.
pub const NOISE_TYPES: [(&str, &str, Option<NoiseKind>); 31] = [
    ("box", "Box", Some(NoiseKind::Box)),
    (
        "blistered_turbulence",
        "Blistered Turbulence",
        Some(NoiseKind::BlisteredTurbulence),
    ),
    ("buya", "Buya", None),
    ("cell", "Cell", Some(NoiseKind::Cell)),
    ("cranal", "Cranal", None),
    ("dents", "Dents", None),
    (
        "displaced_turbulence",
        "Displaced Turbulence",
        Some(NoiseKind::DisplacedTurbulence),
    ),
    ("electric", "Electric", Some(NoiseKind::Electric)),
    ("fbm", "FBM", Some(NoiseKind::Fbm)),
    ("gaseous", "Gaseous", Some(NoiseKind::Gaseous)),
    ("hama", "Hama", None),
    ("luka", "Luka", None),
    ("mod_noise", "Mod. Noise", None),
    ("naki", "Naki", None),
    ("perlin", "Perlin", Some(NoiseKind::Perlin)),
    ("nutous", "Nutous", None),
    ("ober", "Ober", None),
    ("pezo", "Pezo", None),
    ("poxo", "Poxo", None),
    ("sema", "Sema", None),
    ("stupl", "Stupl", None),
    ("turbulence", "Turbulence", Some(NoiseKind::Turbulence)),
    ("vl_noise", "VL Noise", None),
    (
        "wavy_turbulence",
        "Wavy Turbulence",
        Some(NoiseKind::WavyTurbulence),
    ),
    ("cell_voronoi", "Cell Voronoi", None),
    (
        "displaced_voronoi",
        "Displaced Voronoi",
        Some(NoiseKind::DisplacedVoronoi),
    ),
    ("voronoi_1", "Voronoi 1", Some(NoiseKind::Voronoi1)),
    ("voronoi_2", "Voronoi 2", Some(NoiseKind::Voronoi2)),
    ("voronoi_3", "Voronoi 3", Some(NoiseKind::Voronoi3)),
    ("zada", "Zada", None),
    (
        "ridged_multifractal",
        "Ridged Multifractal",
        Some(NoiseKind::RidgedMultifractal),
    ),
];

/// The Random field.
#[derive(Debug, Clone, PartialEq)]
pub struct RandomField {
    pub mode: RandomMode,
    pub seed: u32,
    pub kind: NoiseKind,
    pub space: RandomSpace,
    /// Scale as a fraction (1 is 100 %).
    pub scale: f64,
    pub relative_scale: DVec3,
    /// Animation speed as a fraction.
    pub animation_speed: f64,
    pub remap: Remap,
    pub color: [f64; 3],
    /// World to field space.
    pub world_to_local: DMat4,
}

/// SplitMix64 finalizer (public domain), as in forge-noise.
fn mix(mut z: u64) -> u64 {
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
}

/// A hash mapped to `[0, 1)`.
fn unit_from(h: u64) -> f64 {
    (h >> 11) as f64 / (1u64 << 53) as f64
}

impl RandomField {
    /// The sample position in the space of the field.
    fn position(&self, p_world: DVec3) -> DVec3 {
        match self.space {
            RandomSpace::Global => p_world,
            RandomSpace::Field => self.world_to_local.transform_point3(p_world),
            RandomSpace::None => DVec3::ZERO,
        }
    }

    /// The uniform random value at a position: a hash of the seed and the position bits.
    fn hashed(&self, q: DVec3) -> f64 {
        // `+ 0.0` maps -0.0 to 0.0 so equal positions hash equally.
        let bits = |v: f64| (v + 0.0).to_bits() as i64;
        unit_from(hash_cell::<3>(
            self.seed,
            &[bits(q.x), bits(q.y), bits(q.z)],
        ))
    }

    /// The noise value in `[0, 1]` at a position of the field's space.
    fn noise(&self, q: DVec3, time: f64) -> f64 {
        let size = BASE_SCALE * self.scale;
        // The noise origin is moved off the lattice (a seed-derived offset in lattice units) so
        // that the grid origin is not a lattice point.
        let off = DVec3::new(
            unit_from(mix(u64::from(self.seed) ^ 0xa1)),
            unit_from(mix(u64::from(self.seed) ^ 0xb2)),
            unit_from(mix(u64::from(self.seed) ^ 0xc3)),
        );
        let n = Noise {
            kind: self.kind,
            seed: self.seed,
            scale: size,
            ..Noise::default()
        };
        0.5 + 0.5 * n.sample4(q / self.relative_scale + off * size, time)
    }

    /// The raw (pre-remapping) value at a world position.
    fn raw(&self, p_world: DVec3, ctx: &EvalCtx) -> f64 {
        let q = self.position(p_world);
        match self.mode {
            RandomMode::Noise => self.noise(q, ctx.time * self.animation_speed),
            RandomMode::Random | RandomMode::Sorted => self.hashed(q),
        }
    }
}

impl Field for RandomField {
    fn sample(&self, p_world: DVec3, ctx: &EvalCtx) -> FieldSample {
        FieldSample {
            color: self.color,
            ..FieldSample::from_value(self.remap.apply(self.raw(p_world, ctx)))
        }
    }

    fn sample_many(&self, points: &[DVec3], ctx: &EvalCtx) -> Vec<FieldSample> {
        let mut raw: Vec<f64> = points.iter().map(|p| self.raw(*p, ctx)).collect();
        if self.mode == RandomMode::Sorted {
            raw.sort_by(f64::total_cmp);
        }
        raw.into_iter()
            .map(|t| FieldSample {
                color: self.color,
                ..FieldSample::from_value(self.remap.apply(t))
            })
            .collect()
    }
}

pub fn random_type() -> NodeType {
    let pct = |id: &str, label: &str, default: f64, lo: f64, hi: f64| {
        ParamDesc::float(id, label, default)
            .range(lo, hi)
            .unit(Unit::Percent)
            .group(FIELD)
    };
    let noise_options: Vec<(&str, &str)> = NOISE_TYPES.iter().map(|(i, l, _)| (*i, *l)).collect();
    let shape = vec![
        ParamDesc::enumeration(
            "random_mode",
            "Random Mode",
            "noise",
            &[
                ("random", "Random"),
                ("sorted", "Sorted"),
                ("noise", "Noise"),
            ],
        )
        .group(FIELD),
        ParamDesc::int("seed", "Seed", 123_456)
            .range(0, i64::MAX)
            .group(FIELD),
        ParamDesc::enumeration("noise_type", "Noise Type", "perlin", &noise_options).group(FIELD),
        ParamDesc::enumeration(
            "space",
            "Space",
            "field",
            &[("global", "Global"), ("field", "Field"), ("none", "None")],
        )
        .group(FIELD),
        pct("scale", "Scale", 1.0, 0.0001, BIG),
        ParamDesc::vec3("relative_scale", "Relative Scale", [1.0; 3])
            .range(0.0001, BIG)
            .unit(Unit::Percent)
            .group(FIELD),
        pct("animation_speed", "Animation Speed", 0.0, -BIG, BIG),
        ParamDesc::bool(
            "randomized_sample_positions",
            "Randomized Sample Positions",
            false,
        )
        .group(FIELD),
        ParamDesc::int("sample_position_seed", "Sample Position Seed", 123_456)
            .range(1, 2_147_483_647)
            .group(FIELD),
        pct("color_shift", "Color Shift", 0.1, 0.0, BIG),
        ParamDesc::enumeration(
            "color_alpha",
            "Color Alpha",
            "none",
            &[("none", "None"), ("strength", "Strength")],
        )
        .group(FIELD),
    ];
    field_type(RANDOM, "Random Field", shape, 0.0, false, false)
}

pub(crate) fn field(p: &Params, remap: Remap, world: DMat4) -> Result<RandomField, FieldError> {
    let mode = match p.option("random_mode").as_str() {
        "random" => RandomMode::Random,
        "sorted" => RandomMode::Sorted,
        _ => RandomMode::Noise,
    };
    // The noise type matters in Noise mode only.
    let kind = NOISE_TYPES
        .iter()
        .find(|(id, _, _)| *id == p.option("noise_type"))
        .and_then(|(_, _, k)| *k);
    let kind = match (mode, kind) {
        (_, Some(k)) => k,
        (RandomMode::Noise, None) => return Err(FieldError::Unsupported("noise type")),
        (_, None) => NoiseKind::Perlin,
    };
    let seed = match p.get("seed") {
        Value::Int(i) => (i as u64 ^ (i as u64 >> 32)) as u32,
        other => unreachable!("seed is not an int: {other:?}"),
    };
    Ok(RandomField {
        mode,
        seed,
        kind,
        space: match p.option("space").as_str() {
            "global" => RandomSpace::Global,
            "none" => RandomSpace::None,
            _ => RandomSpace::Field,
        },
        scale: p.float("scale"),
        relative_scale: p.vec3("relative_scale"),
        animation_speed: p.float("animation_speed"),
        remap,
        color: p.color("color"),
        world_to_local: world.inverse(),
    })
}

/// True if the field node `id` changes with time: a Random field in a noise mode with a non-zero
/// animation speed, a Formula field that names `t`, or a Group holding one (CORE.md section 5b).
pub fn reads_time(scene: &Scene, id: NodeId) -> bool {
    reads_time_at(scene, id, 0)
}

fn reads_time_at(scene: &Scene, id: NodeId, depth: usize) -> bool {
    let Some(node) = scene.node(id) else {
        return false;
    };
    match node.type_id.as_str() {
        RANDOM => {
            let ty = random_type();
            let p = Params {
                ty: &ty,
                values: &node.params,
            };
            !matches!(p.option("random_mode").as_str(), "random" | "sorted")
                && p.float("animation_speed") != 0.0
        }
        crate::FORMULA => {
            let ty = crate::formula_type();
            let p = Params {
                ty: &ty,
                values: &node.params,
            };
            let Value::String(text) = p.get("formula") else {
                return false;
            };
            p.float("time_delta") != 0.0 && crate::formula::mentions_time(&text)
        }
        crate::GROUP => match node.params.get(crate::FIELD_LIST) {
            Some(Value::LinkRows(rows)) if depth < 16 => rows
                .iter()
                .any(|r| r.enabled && reads_time_at(scene, r.target, depth + 1)),
            _ => false,
        },
        _ => false,
    }
}
