//! The emitter object and the birth of a particle (DYNAMICS.md sections 9.1-9.5).

use forge_clone_engine::SplitMix64;
use forge_core::{Category, NodeId, NodeType, ParamDesc, Scene, Unit};
use glam::{DMat4, DVec3};

use crate::cache::ParticleMode;
use crate::params;

pub const EMITTER: &str = "forge.particle_emitter";

const BIG: f64 = 1e18;

fn pct(id: &str, label: &str, default: f64, group: &str) -> ParamDesc {
    ParamDesc::float(id, label, default)
        .range(0.0, 100.0)
        .unit(Unit::Percent)
        .group(group)
}

/// The emitter node type: every parameter is a descriptor (O40's table plus the area and
/// spread parameters of the manual).
pub fn emitter_type() -> NodeType {
    const EMIT: &str = "Emitter";
    const PART: &str = "Particle";
    const SHAPE: &str = "Shape";
    NodeType::new(EMITTER, "Emitter", Category::Object)
        .param(
            ParamDesc::float("birthrate_editor", "Birthrate Editor", 10.0)
                .range(0.0, BIG)
                .group(EMIT),
        )
        .param(
            ParamDesc::float("birthrate_renderer", "Birthrate Renderer", 10.0)
                .range(0.0, BIG)
                .group(EMIT),
        )
        .param(pct("visibility_percent", "Visibility", 1.0, EMIT))
        .param(
            ParamDesc::int("seed", "Seed", 0)
                .range(0, i64::from(u32::MAX))
                .group(EMIT),
        )
        .param(ParamDesc::bool("relative_speed", "Relative Speed", false).group(EMIT))
        .param(
            ParamDesc::int("start_frame", "Start Emission", 0)
                .range(-1_000_000, 1_000_000)
                .group(EMIT),
        )
        .param(
            ParamDesc::int("stop_frame", "Stop Emission", 150)
                .range(-1_000_000, 1_000_000)
                .group(EMIT),
        )
        .param(
            ParamDesc::float("lifetime", "Lifetime", 3.0)
                .range(0.0, BIG)
                .unit(Unit::Time)
                .group(PART),
        )
        .param(pct(
            "lifetime_variation_percent",
            "Lifetime Variation",
            0.0,
            PART,
        ))
        .param(
            ParamDesc::float("speed", "Speed", 150.0)
                .range(0.0, BIG)
                .unit(Unit::Distance)
                .group(PART),
        )
        .param(pct("speed_variation_percent", "Speed Variation", 0.0, PART))
        .param(
            ParamDesc::float("rotation", "Rotation", 0.0)
                .unit(Unit::Angle)
                .group(PART),
        )
        .param(pct(
            "rotational_variation_percent",
            "Rotational Variation",
            1.0,
            PART,
        ))
        .param(
            ParamDesc::float("end_scale", "End Scale", 1.0)
                .range(0.0, BIG)
                .group(PART),
        )
        .param(pct("scale_variation_percent", "Scale Variation", 0.0, PART))
        .param(
            ParamDesc::enumeration(
                "shape",
                "Shape",
                "rectangle",
                &[("rectangle", "Rectangle"), ("circle", "Circle")],
            )
            .group(SHAPE),
        )
        .param(
            ParamDesc::float("size_x", "Horizontal Size", 50.0)
                .range(0.0, BIG)
                .unit(Unit::Distance)
                .group(SHAPE),
        )
        .param(
            ParamDesc::float("size_y", "Vertical Size", 50.0)
                .range(0.0, BIG)
                .unit(Unit::Distance)
                .group(SHAPE),
        )
        .param(
            ParamDesc::enumeration(
                "emitter_type",
                "Emitter Type",
                "angular",
                &[("angular", "Angular"), ("pyramid", "Pyramid")],
            )
            .group(SHAPE),
        )
        .param(
            ParamDesc::float("angle_h", "Horizontal Angle", 0.0)
                .range(0.0, 3.0)
                .unit(Unit::Angle)
                .group(SHAPE),
        )
        .param(
            ParamDesc::float("angle_v", "Vertical Angle", 0.0)
                .range(0.0, 3.0)
                .unit(Unit::Angle)
                .group(SHAPE),
        )
        .param(
            ParamDesc::int("substeps", "Substeps", 4)
                .range(1, 100)
                .group(EMIT),
        )
}

/// What the emitter node holds, with the birthrate of the chosen mode.
#[derive(Debug, Clone, PartialEq)]
pub struct EmitterSettings {
    pub rate: f64,
    pub seed: u32,
    pub start: i64,
    pub stop: i64,
    pub lifetime: f64,
    pub lifetime_variation: f64,
    pub speed: f64,
    pub speed_variation: f64,
    pub circle: bool,
    pub size: [f64; 2],
    pub pyramid: bool,
    pub angle: [f64; 2],
    pub rotation: f64,
    pub rotation_variation: f64,
    pub end_scale: f64,
    pub scale_variation: f64,
    pub substeps: u32,
}

impl EmitterSettings {
    pub fn read(scene: &Scene, id: NodeId, mode: ParticleMode) -> EmitterSettings {
        let f = |k: &str| params::float(scene, id, k);
        EmitterSettings {
            rate: match mode {
                ParticleMode::Editor => f("birthrate_editor"),
                ParticleMode::Render => f("birthrate_renderer"),
            },
            seed: params::int(scene, id, "seed").clamp(0, i64::from(u32::MAX)) as u32,
            start: params::int(scene, id, "start_frame"),
            stop: params::int(scene, id, "stop_frame"),
            lifetime: f("lifetime"),
            lifetime_variation: f("lifetime_variation_percent"),
            speed: f("speed"),
            speed_variation: f("speed_variation_percent"),
            circle: params::choice(scene, id, "shape") == "circle",
            size: [f("size_x"), f("size_y")],
            pyramid: params::choice(scene, id, "emitter_type") == "pyramid",
            angle: [f("angle_h"), f("angle_v")],
            rotation: f("rotation"),
            rotation_variation: f("rotational_variation_percent"),
            end_scale: f("end_scale"),
            scale_variation: f("scale_variation_percent"),
            substeps: params::int(scene, id, "substeps").clamp(1, 100) as u32,
        }
    }
}

/// `(first emission index, count)` born in `frame` (DYNAMICS.md section 9.5):
/// `floor(R (k + 1) + 1e-9) - floor(R k + 1e-9)` for `start <= frame < stop`, `k = frame - start`.
pub fn births_in_frame(rate: f64, start: i64, stop: i64, frame: i64) -> (u64, u64) {
    if rate <= 0.0 || frame < start || frame >= stop {
        return (total_before(rate, start, stop, frame), 0);
    }
    let k = (frame - start) as f64;
    let before = (rate * k + 1e-9).floor() as u64;
    let after = (rate * (k + 1.0) + 1e-9).floor() as u64;
    (before, after - before)
}

fn total_before(rate: f64, start: i64, stop: i64, frame: i64) -> u64 {
    if rate <= 0.0 || frame <= start {
        return 0;
    }
    let k = (frame.min(stop) - start) as f64;
    (rate * k + 1e-9).floor() as u64
}

/// A particle at birth, world space.
#[derive(Debug, Clone, PartialEq)]
pub struct Newborn {
    pub position: DVec3,
    pub velocity: DVec3,
    pub lifetime: f64,
    pub spin_axis: DVec3,
    pub spin_rate: f64,
    pub scale_end: f64,
}

fn disc(t0: f64, t1: f64) -> (f64, f64) {
    let r = t0.sqrt();
    let phi = std::f64::consts::TAU * t1;
    (r * phi.cos(), r * phi.sin())
}

/// Particle number `n` (DYNAMICS.md sections 9.2-9.4): its own stream, all draws always taken.
pub fn birth(s: &EmitterSettings, n: u64, world: &DMat4) -> Newborn {
    let mut rng = SplitMix64::new(forge_noise::hash_cell(s.seed, &[n as i64]));
    let mut t = [0.0; 10];
    for v in &mut t {
        *v = rng.next_f64();
    }
    let signed = |x: f64| 2.0 * x - 1.0;
    let (bx, by) = if s.circle {
        disc(t[0], t[1])
    } else {
        (signed(t[0]), signed(t[1]))
    };
    // The unit disc has radius 1 and the square half-extent 1: both scale by size / 2.
    let local = DVec3::new(bx * s.size[0] * 0.5, by * s.size[1] * 0.5, 0.0);
    let (u, v) = if s.pyramid {
        (signed(t[2]), signed(t[3]))
    } else {
        disc(t[2], t[3])
    };
    let dir = DVec3::new(
        u * (s.angle[0] * 0.5).tan(),
        v * (s.angle[1] * 0.5).tan(),
        1.0,
    )
    .normalize();
    let world_dir = world.transform_vector3(dir);
    let world_dir = if world_dir.length_squared() > 0.0 {
        world_dir.normalize()
    } else {
        dir
    };
    let vary = |base: f64, var: f64, draw: f64| (base * (1.0 + var * signed(draw))).max(0.0);
    let speed = vary(s.speed, s.speed_variation, t[4]);
    let lifetime = vary(s.lifetime, s.lifetime_variation, t[5]);
    let z = signed(t[6]);
    let ring = (1.0 - z * z).max(0.0).sqrt();
    let phi = std::f64::consts::TAU * t[7];
    Newborn {
        position: world.transform_point3(local),
        velocity: world_dir * speed,
        lifetime,
        spin_axis: DVec3::new(ring * phi.cos(), ring * phi.sin(), z),
        spin_rate: s.rotation * (1.0 + s.rotation_variation * signed(t[8])),
        scale_end: (s.end_scale * (1.0 + s.scale_variation * signed(t[9]))).max(0.0),
    }
}
