//! The force objects (DYNAMICS.md section 10): pure functions of position, velocity and time.

use forge_core::{Category, NodeId, NodeType, ParamDesc, Scene, Unit};
use forge_fields::FieldList;
use glam::{DMat4, DVec3};

use crate::params;

pub const GRAVITY: &str = "forge.force_gravity";
pub const WIND: &str = "forge.force_wind";
pub const ATTRACTOR: &str = "forge.force_attractor";
pub const DEFLECTOR: &str = "forge.force_deflector";
pub const FRICTION: &str = "forge.force_friction";
pub const TURBULENCE: &str = "forge.force_turbulence";
pub const ROTATION: &str = "forge.force_rotation";

/// Every force node type id, sorted.
pub const FORCE_TYPES: [&str; 7] = [
    ATTRACTOR, DEFLECTOR, FRICTION, GRAVITY, ROTATION, TURBULENCE, WIND,
];

const BIG: f64 = 1e18;

fn base(id: &str, label: &str) -> NodeType {
    NodeType::new(id, label, Category::Object)
        .param(
            ParamDesc::link_list(
                "emitters",
                "Emitters",
                &[Category::Object, Category::Generator],
            )
            .group("Force"),
        )
        .param(forge_fields::field_list_param("fields", "Fields").group("Falloff"))
}

fn dist(id: &str, label: &str, default: f64) -> ParamDesc {
    ParamDesc::float(id, label, default)
        .unit(Unit::Distance)
        .group("Force")
}

fn plain(id: &str, label: &str, default: f64) -> ParamDesc {
    ParamDesc::float(id, label, default).group("Force")
}

/// The seven force node types.
pub(crate) fn force_types() -> Vec<NodeType> {
    vec![
        base(GRAVITY, "Gravity")
            .param(dist("acceleration", "Acceleration", 981.0).range(-BIG, BIG)),
        base(WIND, "Wind")
            .param(dist("speed", "Speed", 100.0).range(-BIG, BIG))
            .param(plain("air_resistance", "Air Resistance", 1.0).range(0.0, BIG)),
        base(ATTRACTOR, "Attractor")
            .param(dist("strength", "Strength", 500.0).range(-BIG, BIG))
            .param(dist("radius", "Radius", 100.0).range(1e-6, BIG)),
        base(DEFLECTOR, "Deflector")
            .param(plain("bounce", "Bounce", 0.5).range(0.0, 1.0))
            .param(plain("friction", "Friction", 0.0).range(0.0, 1.0))
            .param(ParamDesc::bool("kill", "Kill", false).group("Force")),
        base(FRICTION, "Friction").param(plain("friction", "Friction", 1.0).range(0.0, BIG)),
        base(TURBULENCE, "Turbulence")
            .param(dist("strength", "Strength", 200.0).range(-BIG, BIG))
            .param(dist("scale", "Scale", 100.0).range(1e-6, BIG))
            .param(plain("speed", "Speed", 1.0).range(-BIG, BIG))
            .param(
                ParamDesc::int("seed", "Seed", 0)
                    .range(0, i64::from(u32::MAX))
                    .group("Force"),
            ),
        base(ROTATION, "Rotation").param(dist("strength", "Strength", 200.0).range(-BIG, BIG)),
    ]
}

/// One force with its world placement baked in.
#[derive(Debug, Clone, PartialEq)]
pub enum Force {
    Gravity {
        /// The acceleration vector.
        acceleration: DVec3,
    },
    Wind {
        /// The wind velocity.
        velocity: DVec3,
        /// Air resistance, 1/s.
        drag: f64,
    },
    Attractor {
        center: DVec3,
        strength: f64,
        radius: f64,
    },
    Friction {
        k: f64,
    },
    Turbulence {
        strength: f64,
        scale: f64,
        speed: f64,
        seed: u32,
    },
    Rotation {
        origin: DVec3,
        axis: DVec3,
        strength: f64,
    },
    Deflector {
        point: DVec3,
        /// Unit normal of the side particles stay on.
        normal: DVec3,
        bounce: f64,
        friction: f64,
        kill: bool,
    },
}

/// What a deflector does to a particle that crossed its plane.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Collision {
    Bounce { position: DVec3, velocity: DVec3 },
    Kill,
}

impl Force {
    /// The acceleration at world position `p` with velocity `v` at time `t` (seconds). Zero for
    /// the deflector, which acts through [`Force::collide`].
    pub fn acceleration(&self, p: DVec3, v: DVec3, t: f64) -> DVec3 {
        match *self {
            Force::Gravity { acceleration } => acceleration,
            Force::Wind { velocity, drag } => (velocity - v) * drag,
            Force::Attractor {
                center,
                strength,
                radius,
            } => {
                let d = center - p;
                let len = d.length();
                if len == 0.0 {
                    return DVec3::ZERO;
                }
                let k = radius / len.max(radius);
                d / len * (strength * k * k)
            }
            Force::Friction { k } => -v * k,
            Force::Turbulence {
                strength,
                scale,
                speed,
                seed,
            } => {
                let q = p / scale;
                let at =
                    |i: u32| forge_noise::perlin(seed.wrapping_add(i), [q.x, q.y, q.z, speed * t]);
                DVec3::new(at(0), at(1), at(2)) * strength
            }
            Force::Rotation {
                origin,
                axis,
                strength,
            } => {
                let r = p - origin;
                let perp = r - axis * r.dot(axis);
                let len = perp.length();
                if len < 1e-12 {
                    DVec3::ZERO
                } else {
                    axis.cross(perp) / len * strength
                }
            }
            Force::Deflector { .. } => DVec3::ZERO,
        }
    }

    /// The deflector's response for a step from `p_old` to `p_new` with velocity `v` (already
    /// advanced); `None` when the particle did not cross the plane or this is another force.
    pub fn collide(&self, p_old: DVec3, p_new: DVec3, v: DVec3) -> Option<Collision> {
        let Force::Deflector {
            point,
            normal,
            bounce,
            friction,
            kill,
        } = *self
        else {
            return None;
        };
        let d_old = (p_old - point).dot(normal);
        let d_new = (p_new - point).dot(normal);
        if !(d_old >= 0.0 && d_new < 0.0) {
            return None;
        }
        if kill {
            return Some(Collision::Kill);
        }
        let position = p_new - normal * ((1.0 + bounce) * d_new);
        let vn = v.dot(normal);
        let tangent = (v - normal * vn) * (1.0 - friction);
        let velocity = tangent - normal * (bounce * vn);
        Some(Collision::Bounce { position, velocity })
    }
}

/// A force with its falloff and the emitters it acts on.
pub struct ForceInstance {
    pub force: Force,
    pub falloff: FieldList,
    /// Empty: every emitter.
    pub emitters: Vec<NodeId>,
    pub node: NodeId,
}

impl ForceInstance {
    pub fn acts_on(&self, emitter: NodeId) -> bool {
        self.emitters.is_empty() || self.emitters.contains(&emitter)
    }
}

fn axis_of(world: &DMat4, local: DVec3) -> DVec3 {
    let a = world.transform_vector3(local);
    if a.length_squared() > 0.0 {
        a.normalize()
    } else {
        local
    }
}

fn force_of(scene: &Scene, id: NodeId, type_id: &str, world: &DMat4) -> Option<Force> {
    let f = |k: &str| params::float(scene, id, k);
    Some(match type_id {
        GRAVITY => Force::Gravity {
            acceleration: axis_of(world, DVec3::NEG_Y) * f("acceleration"),
        },
        WIND => Force::Wind {
            velocity: axis_of(world, DVec3::Z) * f("speed"),
            drag: f("air_resistance"),
        },
        ATTRACTOR => Force::Attractor {
            center: world.transform_point3(DVec3::ZERO),
            strength: f("strength"),
            radius: f("radius"),
        },
        DEFLECTOR => Force::Deflector {
            point: world.transform_point3(DVec3::ZERO),
            normal: axis_of(world, DVec3::Y),
            bounce: f("bounce"),
            friction: f("friction"),
            kill: params::boolean(scene, id, "kill"),
        },
        FRICTION => Force::Friction { k: f("friction") },
        TURBULENCE => Force::Turbulence {
            strength: f("strength"),
            scale: f("scale"),
            speed: f("speed"),
            seed: params::int(scene, id, "seed").clamp(0, i64::from(u32::MAX)) as u32,
        },
        ROTATION => Force::Rotation {
            origin: world.transform_point3(DVec3::ZERO),
            axis: axis_of(world, DVec3::Y),
            strength: f("strength"),
        },
        _ => return None,
    })
}

/// The enabled forces of the scene that act on `emitter`, in document order, with the falloff
/// lists read (a field list that fails to build counts as empty).
pub fn forces_for(scene: &Scene, emitter: NodeId) -> Vec<ForceInstance> {
    let mut out = Vec::new();
    let mut stack: Vec<NodeId> = scene.roots().iter().rev().copied().collect();
    while let Some(id) = stack.pop() {
        let Some(n) = scene.node(id) else { continue };
        stack.extend(n.children.iter().rev());
        if !FORCE_TYPES.contains(&n.type_id.as_str()) || !forge_core::visibility::enabled(scene, id)
        {
            continue;
        }
        let Some(world) = scene.world_matrix(id) else {
            continue;
        };
        let Some(force) = force_of(scene, id, &n.type_id, &world) else {
            continue;
        };
        let inst = ForceInstance {
            force,
            falloff: FieldList::from_param(scene, id, "fields").unwrap_or_default(),
            emitters: params::links(scene, id, "emitters"),
            node: id,
        };
        if inst.acts_on(emitter) {
            out.push(inst);
        }
    }
    out
}
