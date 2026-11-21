//! The rapier world: bodies, colliders and fixed-step integration (docs/specs/DYNAMICS.md §4).
//!
//! The scene is in centimetres, rapier in metres; the conversion happens here and nowhere else.
//! Everything is deterministic: rapier's `enhanced-determinism` feature, a fixed step, bodies
//! inserted in a fixed order, no threads.

use forge_core::NodeId;
use forge_geometry::PolyMesh;
use glam::{DMat3, DMat4, DQuat, DVec3};
use rapier3d_f64::prelude::*;

use crate::settings::DynamicsSettings;

/// Scene units (cm) to rapier units (m).
pub const CM_TO_M: f64 = 0.01;
/// Rapier units to scene units.
pub const M_TO_CM: f64 = 100.0;

/// Which body a pose belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BodyKey {
    /// An object carrying a rigid body tag.
    Node(NodeId),
    /// One clone of a cloner that carries the tag (docs/specs/DYNAMICS.md §6).
    Clone { cloner: NodeId, index: u32 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BodyKind {
    /// A collider that never moves.
    Static,
    Dynamic,
    /// Follows poses handed to [`Simulation::step_frame_with`]; pushes dynamic bodies.
    Kinematic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShapeKind {
    Automatic,
    Box,
    Sphere,
    ConvexHull,
    StaticMesh,
}

/// A custom mass or a density (g/cm³).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MassSpec {
    Density(f64),
    /// kg
    Mass(f64),
}

/// Everything the simulation needs to know about one body.
#[derive(Debug, Clone)]
pub struct BodySpec {
    pub key: BodyKey,
    /// World matrix in cm at the start frame (scale kept: it scales the shape).
    pub world: DMat4,
    pub kind: BodyKind,
    pub shape: ShapeKind,
    /// The object's mesh in its own space, unscaled.
    pub mesh: PolyMesh,
    pub size_increment: f64,
    pub bounce: f64,
    pub friction: f64,
    pub mass: MassSpec,
    pub linear_damping: f64,
    pub angular_damping: f64,
    /// cm/s and rad/s (world space).
    pub linear_velocity: DVec3,
    pub angular_velocity: DVec3,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum DynamicsError {
    #[error("body {0:?} has no geometry to build a collider from")]
    NoGeometry(BodyKey),
    #[error("body {0:?}: shape {1} is not supported for this body")]
    UnsupportedShape(BodyKey, &'static str),
    #[error("{0}")]
    Joint(String),
}

pub(crate) struct Live {
    pub key: BodyKey,
    pub handle: RigidBodyHandle,
    pub kind: BodyKind,
    pub scale: DVec3,
}

/// One simulated world.
pub struct Simulation {
    pub(crate) world: PhysicsWorld,
    pub(crate) bodies: Vec<Live>,
    steps: u32,
    gravity_cm: f64,
}

fn v(p: DVec3) -> Vector {
    Vector::new(p.x, p.y, p.z)
}

fn dv(p: Vector) -> DVec3 {
    DVec3::new(p.x, p.y, p.z)
}

/// Splits a world matrix into translation, rotation and the lengths of its columns.
pub fn decompose(m: &DMat4) -> (DVec3, DQuat, DVec3) {
    let c = DMat3::from_mat4(*m);
    let scale = DVec3::new(c.x_axis.length(), c.y_axis.length(), c.z_axis.length());
    let r = DMat3::from_cols(
        c.x_axis / scale.x.max(1e-12),
        c.y_axis / scale.y.max(1e-12),
        c.z_axis / scale.z.max(1e-12),
    );
    (m.w_axis.truncate(), DQuat::from_mat3(&r).normalize(), scale)
}

fn pose_of(t_cm: DVec3, q: DQuat) -> Pose {
    Pose::from_parts(v(t_cm * CM_TO_M), Rotation::from_xyzw(q.x, q.y, q.z, q.w))
}

fn bounds(points: &[DVec3]) -> (DVec3, DVec3) {
    let mut lo = DVec3::splat(f64::MAX);
    let mut hi = DVec3::splat(f64::MIN);
    for p in points {
        lo = lo.min(*p);
        hi = hi.max(*p);
    }
    (lo, hi)
}

fn collider_for(spec: &BodySpec, scale: DVec3) -> Result<ColliderBuilder, DynamicsError> {
    let pts: Vec<DVec3> = spec.mesh.points.iter().map(|p| *p * scale).collect();
    if pts.is_empty() {
        return Err(DynamicsError::NoGeometry(spec.key));
    }
    let (lo, hi) = bounds(&pts);
    let center = (lo + hi) * 0.5;
    let half = (hi - lo) * 0.5 + DVec3::splat(spec.size_increment);
    let shape = match (spec.shape, spec.kind) {
        (ShapeKind::Automatic, BodyKind::Static) | (ShapeKind::StaticMesh, _) => {
            ShapeKind::StaticMesh
        }
        (ShapeKind::Automatic, _) => ShapeKind::ConvexHull,
        (s, _) => s,
    };
    let metres = |p: &DVec3| v(*p * CM_TO_M);
    let boxed = |c: DVec3, h: DVec3| {
        ColliderBuilder::cuboid(
            (h.x * CM_TO_M).max(1e-6),
            (h.y * CM_TO_M).max(1e-6),
            (h.z * CM_TO_M).max(1e-6),
        )
        .translation(v(c * CM_TO_M))
    };
    Ok(match shape {
        ShapeKind::Box | ShapeKind::Automatic => boxed(center, half),
        ShapeKind::Sphere => {
            let r = (half.x + half.y + half.z) / 3.0;
            ColliderBuilder::ball(r * CM_TO_M).translation(v(center * CM_TO_M))
        }
        ShapeKind::ConvexHull => {
            let points: Vec<Vector> = pts.iter().map(metres).collect();
            ColliderBuilder::convex_hull(&points).unwrap_or_else(|| boxed(center, half))
        }
        ShapeKind::StaticMesh => {
            let points: Vec<Vector> = pts.iter().map(metres).collect();
            let mut tris: Vec<[u32; 3]> = Vec::new();
            for poly in &spec.mesh.polys {
                for k in 1..poly.len().saturating_sub(1) {
                    tris.push([poly[0], poly[k], poly[k + 1]]);
                }
            }
            if tris.is_empty() {
                boxed(center, half)
            } else {
                ColliderBuilder::trimesh(points, tris).unwrap_or_else(|_| boxed(center, half))
            }
        }
    })
}

impl Simulation {
    /// A world holding `specs` in the given order (the order fixes handle numbering and so the
    /// result); `fps` is the document frame rate.
    pub fn new(
        settings: &DynamicsSettings,
        fps: u32,
        specs: &[BodySpec],
    ) -> Result<Simulation, DynamicsError> {
        let mut world = PhysicsWorld::default();
        let dt = settings.step_dt(fps);
        world.gravity = Vector::new(0.0, -settings.gravity * CM_TO_M, 0.0);
        world.integration_parameters.dt = dt;
        let mut bodies = Vec::with_capacity(specs.len());
        for spec in specs {
            let (t, q, scale) = decompose(&spec.world);
            let mut rb = match spec.kind {
                BodyKind::Static => RigidBodyBuilder::fixed(),
                BodyKind::Dynamic => RigidBodyBuilder::dynamic(),
                BodyKind::Kinematic => RigidBodyBuilder::kinematic_position_based(),
            }
            .pose(pose_of(t, q))
            .linear_damping(spec.linear_damping)
            .angular_damping(spec.angular_damping);
            if spec.kind == BodyKind::Dynamic {
                rb = rb
                    .linvel(v(spec.linear_velocity * CM_TO_M))
                    .angvel(v(spec.angular_velocity));
            }
            let handle = world.bodies.insert(rb);
            let mut col = collider_for(spec, scale)?
                .friction(spec.friction)
                .restitution(spec.bounce)
                .friction_combine_rule(CoefficientCombineRule::Max)
                .restitution_combine_rule(CoefficientCombineRule::Multiply);
            col = match spec.mass {
                MassSpec::Density(d) => col.density(d * 1000.0),
                MassSpec::Mass(m) => col.mass(m),
            };
            let (bodies_set, colliders) = (&mut world.bodies, &mut world.colliders);
            colliders.insert_with_parent(col, handle, bodies_set);
            bodies.push(Live {
                key: spec.key,
                handle,
                kind: spec.kind,
                scale,
            });
        }
        Ok(Simulation {
            world,
            bodies,
            steps: settings.steps_per_frame,
            gravity_cm: settings.gravity,
        })
    }

    /// The bodies, in insertion order.
    pub fn keys(&self) -> Vec<BodyKey> {
        self.bodies.iter().map(|b| b.key).collect()
    }

    /// The kind of every body, in insertion order.
    pub fn kinds(&self) -> Vec<BodyKind> {
        self.bodies.iter().map(|b| b.kind).collect()
    }

    /// Gravity in cm/s².
    pub fn gravity(&self) -> f64 {
        self.gravity_cm
    }

    /// World matrices (cm, scale kept) of every body, in insertion order.
    pub fn poses(&self) -> Vec<DMat4> {
        self.bodies
            .iter()
            .map(|b| {
                let rb = &self.world.bodies[b.handle];
                let t = dv(rb.translation()) * M_TO_CM;
                let q = rb.rotation();
                let rot = DQuat::from_xyzw(q.x, q.y, q.z, q.w);
                DMat4::from_scale_rotation_translation(b.scale, rot, t)
            })
            .collect()
    }

    /// Linear velocities in cm/s, in insertion order.
    pub fn velocities(&self) -> Vec<DVec3> {
        self.bodies
            .iter()
            .map(|b| dv(self.world.bodies[b.handle].linvel()) * M_TO_CM)
            .collect()
    }

    /// One document frame: `steps_per_frame` solver steps.
    pub fn step_frame(&mut self) {
        self.step_frame_with(&|_| None);
    }

    /// One document frame; kinematic bodies move to the matrix `target` returns for their key
    /// (linearly over the frame's steps), others keep their last pose.
    pub fn step_frame_with(&mut self, target: &dyn Fn(BodyKey) -> Option<DMat4>) {
        let goals: Vec<Option<(DVec3, DQuat, DVec3, DQuat)>> = self
            .bodies
            .iter()
            .map(|b| {
                if b.kind != BodyKind::Kinematic {
                    return None;
                }
                let m = target(b.key)?;
                let (t, q, _) = decompose(&m);
                let rb = &self.world.bodies[b.handle];
                let q0 = rb.rotation();
                Some((
                    dv(rb.translation()) * M_TO_CM,
                    DQuat::from_xyzw(q0.x, q0.y, q0.z, q0.w),
                    t,
                    q,
                ))
            })
            .collect();
        for s in 1..=self.steps {
            let a = f64::from(s) / f64::from(self.steps);
            for (b, g) in self.bodies.iter().zip(&goals) {
                if let Some((t0, q0, t1, q1)) = g {
                    let t = t0.lerp(*t1, a);
                    let q = q0.slerp(*q1, a);
                    self.world.bodies[b.handle].set_next_kinematic_position(pose_of(t, q));
                }
            }
            self.world.step();
        }
    }
}
