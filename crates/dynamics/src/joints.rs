//! Connectors and springs (docs/specs/DYNAMICS.md §3).

use forge_core::{NodeId, Scene};
use glam::{DMat4, DVec3};

use crate::build::objects_in_order;
use crate::nodes::{self, CONNECTOR, SPRING};
use crate::sim::BodyKey;

/// Angular frequency squared (1/s²) a spring of stiffness 1 gives, whatever the mass: the
/// spring constant is `stiffness · OMEGA2 · m`. Fitted to the sag and the period of O41 `spring`
/// (docs/specs/DYNAMICS.md §3.2).
pub const OMEGA2_PER_STIFFNESS: f64 = 100.0;

/// One end of a connector or spring: a simulated body, or (when `key` is `None` or names no
/// body) a fixed point of the world with the pose the object has in the scene.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct End {
    pub key: Option<BodyKey>,
    /// World matrix (cm) at the start frame.
    pub world: DMat4,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JointKind {
    Hinge,
    BallSocket,
    Slider,
    Fixed,
}

#[derive(Debug, Clone, PartialEq)]
pub struct JointSpec {
    pub a: End,
    pub b: End,
    pub kind: JointKind,
    /// The joint's anchor in world space (cm).
    pub anchor: DVec3,
    /// The hinge axis or the slider direction in world space (unit).
    pub axis: DVec3,
    /// Whether the two linked bodies collide with each other.
    pub contacts: bool,
    /// Hinge limits in rapier's angle (counter-clockwise about the axis), relative to the start
    /// pose, radians.
    pub limit: Option<[f64; 2]>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SpringSpec {
    pub a: End,
    pub b: End,
    /// cm
    pub rest_length: f64,
    pub stiffness: f64,
    /// Fraction of critical damping.
    pub damping: f64,
}

/// Every joint and spring of a simulation.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Links {
    pub joints: Vec<JointSpec>,
    pub springs: Vec<SpringSpec>,
}

fn end(scene: &Scene, id: Option<NodeId>) -> End {
    match id {
        Some(id) => End {
            key: Some(BodyKey::Node(id)),
            world: scene.world_matrix(id).unwrap_or(DMat4::IDENTITY),
        },
        None => End {
            key: None,
            world: DMat4::IDENTITY,
        },
    }
}

/// The connectors and springs of the scene, in document order. A connector or spring with
/// neither object set is skipped; with one set, the other end is the world.
pub fn links_of_scene(scene: &Scene) -> Links {
    let mut out = Links::default();
    for id in objects_in_order(scene) {
        let Some(n) = scene.node(id) else { continue };
        let (a, b) = (
            nodes::link(scene, id, "object_a").filter(|o| scene.contains(*o)),
            nodes::link(scene, id, "object_b").filter(|o| scene.contains(*o)),
        );
        if a.is_none() && b.is_none() {
            continue;
        }
        match n.type_id.as_str() {
            CONNECTOR => {
                let Some(world) = scene.world_matrix(id) else {
                    continue;
                };
                let kind = match nodes::choice(scene, id, "type") {
                    "ball_socket" => JointKind::BallSocket,
                    "slider" => JointKind::Slider,
                    "fixed" => JointKind::Fixed,
                    _ => JointKind::Hinge,
                };
                let offset = DVec3::from_array(nodes::vec3(scene, id, "offset"));
                let axis_local = if kind == JointKind::Slider {
                    DVec3::X
                } else {
                    DVec3::Z
                };
                // Bank is positive clockwise about the axis, rapier's angle is the opposite way.
                let limit = nodes::boolean(scene, id, "angular_limit").then(|| {
                    let (from, to) = (
                        nodes::float(scene, id, "limit_from"),
                        nodes::float(scene, id, "limit_to"),
                    );
                    [-from.max(to), -from.min(to)]
                });
                out.joints.push(JointSpec {
                    a: end(scene, a),
                    b: end(scene, b),
                    kind,
                    anchor: world.transform_point3(offset),
                    axis: world.transform_vector3(axis_local).normalize_or_zero(),
                    contacts: !nodes::boolean(scene, id, "ignore_collisions"),
                    limit,
                });
            }
            SPRING => out.springs.push(SpringSpec {
                a: end(scene, a),
                b: end(scene, b),
                rest_length: nodes::float(scene, id, "rest_length"),
                stiffness: nodes::float(scene, id, "stiffness"),
                damping: nodes::float(scene, id, "damping"),
            }),
            _ => {}
        }
    }
    out
}
