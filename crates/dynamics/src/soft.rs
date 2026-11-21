//! Soft bodies: position-based dynamics on a mesh (docs/specs/DYNAMICS.md §8).
//!
//! Our own solver (rapier's soft body module is not used). The mesh points are particles of
//! unit mass; distance constraints on the polygon edges (structural), on the quad diagonals
//! (shear) and between the two far corners of faces that share an edge (flexion) hold the shape
//! together; an optional shape-matching goal (Keep Soft Body Shape) pulls the particles back to
//! the rest shape. Colliders of the rapier world push particles out, one way: soft bodies do not
//! push rigid bodies. Deterministic: fixed iteration counts and orders, no random numbers.

use std::collections::BTreeMap;

use glam::{DMat3, DQuat, DVec3};
use rapier3d_f64::prelude::*;

use crate::sim::{BodyKey, CM_TO_M, M_TO_CM};

/// Solver iterations per step.
pub const ITERATIONS: usize = 8;
/// Particles stay this far (cm) outside the surface of a collider.
pub const MARGIN: f64 = 0.5;
/// Strength of the shape-matching goal per step when Keep Soft Body Shape is on. Own design,
/// not observed (DYNAMICS.md §8).
pub const KEEP_SHAPE: f64 = 0.15;
/// Seconds⁻¹ per unit of damping: the rate at which the velocity along a constraint, and the
/// velocity relative to the body's mean, is removed. Fitted by hand (DYNAMICS.md §8.2).
pub const DAMPING_RATE: f64 = 150.0;

/// The tag values a soft body needs (fractions: 1.0 is 100 %).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SoftParams {
    pub structural: f64,
    pub structural_damping: f64,
    pub shear: f64,
    pub shear_damping: f64,
    pub flexion: f64,
    pub flexion_damping: f64,
    pub keep_shape: bool,
    pub friction: f64,
    pub bounce: f64,
    /// Linear damping of the tag (per second): the particles' velocity is divided by
    /// `1 + dt · damping` each step, as for a rigid body.
    pub linear_damping: f64,
}

/// A soft body to simulate: its points in world space (cm) and its polygons.
#[derive(Debug, Clone)]
pub struct SoftSpec {
    pub key: BodyKey,
    pub points: Vec<DVec3>,
    pub polys: Vec<Vec<u32>>,
    pub params: SoftParams,
}

/// `s / (1 + s)`: the share of a constraint's error removed per iteration for a stiffness `s`
/// (a fraction; 0 gives 0, 1 gives one half, 10 gives 0.91). Own design.
pub fn stiffness_factor(s: f64) -> f64 {
    let s = s.max(0.0);
    s / (1.0 + s)
}

#[derive(Debug, Clone, Copy)]
struct Link {
    a: u32,
    b: u32,
    rest: f64,
}

pub(crate) struct SoftBody {
    pub key: BodyKey,
    x: Vec<DVec3>,
    v: Vec<DVec3>,
    rest: Vec<DVec3>,
    structural: Vec<Link>,
    shear: Vec<Link>,
    flexion: Vec<Link>,
    params: SoftParams,
}

fn link(points: &[DVec3], a: u32, b: u32) -> Link {
    Link {
        a,
        b,
        rest: (points[a as usize] - points[b as usize]).length(),
    }
}

impl SoftBody {
    pub fn new(spec: &SoftSpec) -> SoftBody {
        let p = &spec.points;
        let mut edges: BTreeMap<(u32, u32), Vec<(usize, usize)>> = BTreeMap::new();
        let mut shear = Vec::new();
        for (fi, poly) in spec.polys.iter().enumerate() {
            let n = poly.len();
            for k in 0..n {
                let (a, b) = (poly[k], poly[(k + 1) % n]);
                if a == b {
                    continue;
                }
                edges.entry((a.min(b), a.max(b))).or_default().push((fi, k));
            }
            if n == 4 {
                let distinct = poly.iter().collect::<std::collections::BTreeSet<_>>().len() == 4;
                if distinct {
                    shear.push(link(p, poly[0], poly[2]));
                    shear.push(link(p, poly[1], poly[3]));
                }
            }
        }
        let structural: Vec<Link> = edges.keys().map(|(a, b)| link(p, *a, *b)).collect();
        // Flexion: the far corner of each of two faces that share an edge.
        let mut flexion = Vec::new();
        for uses in edges.values() {
            if let [(f0, k0), (f1, k1)] = uses[..] {
                let far = |f: usize, k: usize| {
                    let poly = &spec.polys[f];
                    poly[(k + 2) % poly.len()]
                };
                let (c, d) = (far(f0, k0), far(f1, k1));
                if c != d {
                    flexion.push(link(p, c.min(d), c.max(d)));
                }
            }
        }
        flexion.sort_by_key(|l| (l.a, l.b));
        flexion.dedup_by_key(|l| (l.a, l.b));
        SoftBody {
            key: spec.key,
            x: p.clone(),
            v: vec![DVec3::ZERO; p.len()],
            rest: p.clone(),
            structural,
            shear,
            flexion,
            params: spec.params,
        }
    }

    pub fn points(&self) -> &[DVec3] {
        &self.x
    }

    fn solve(&self, pred: &mut [DVec3], links: &[Link], stiffness: f64) {
        let k = stiffness_factor(stiffness);
        if k <= 0.0 {
            return;
        }
        for l in links {
            let (a, b) = (l.a as usize, l.b as usize);
            let d = pred[b] - pred[a];
            let len = d.length();
            if len < 1e-12 {
                continue;
            }
            let corr = d / len * ((len - l.rest) * 0.5 * k);
            pred[a] += corr;
            pred[b] -= corr;
        }
    }

    /// Moves `p` (the predicted position, from `from`) out of the colliders: a ray from the old
    /// position finds the surface it crossed (a thin plane cannot be tunnelled), a point query
    /// finds the solid it sank into. Returns the contact normal when it moved the particle.
    fn collide(&self, world: &PhysicsWorld, from: DVec3, p: &mut DVec3) -> Option<DVec3> {
        let margin = MARGIN * CM_TO_M;
        let (f, t) = (to_v(from), to_v(*p));
        let d = t - f;
        let len = d.length();
        let filter = QueryFilter::default();
        if len > 1e-12 {
            let ray = Ray::new(f, d / len);
            if let Some((_, hit)) = world.cast_ray_and_get_normal(&ray, len + margin, true, filter)
                && hit.time_of_impact > 1e-12
            {
                let mut n = hit.normal;
                if n.dot(d) > 0.0 {
                    n = -n; // face the side the particle came from
                }
                let n = n.normalize();
                let at = ray.point_at(hit.time_of_impact);
                let depth = margin - (t - at).dot(n);
                if depth > 0.0 {
                    *p = from_v(t + n * depth);
                    return Some(DVec3::new(n.x, n.y, n.z));
                }
            }
        }
        if let Some((_, proj)) = world.project_point(t, margin, true, filter)
            && proj.is_inside
        {
            let to_surface = proj.point - t;
            let dist = to_surface.length();
            let n = if dist > 1e-12 {
                to_surface / dist
            } else {
                Vector::Y
            };
            *p = from_v(proj.point + n * margin);
            return Some(DVec3::new(n.x, n.y, n.z));
        }
        None
    }

    /// One solver step of `dt` seconds under `gravity` (cm/s², downward).
    pub fn step(&mut self, dt: f64, gravity: f64, world: &PhysicsWorld) {
        let n = self.x.len();
        let p = self.params;
        let mut pred = Vec::with_capacity(n);
        for i in 0..n {
            // Semi-implicit Euler: over a free fall of n steps it falls (1 + 1/n) times the
            // exact distance (DYNAMICS.md §8).
            self.v[i].y -= gravity * dt;
            self.v[i] /= 1.0 + dt * p.linear_damping;
            pred.push(self.x[i] + self.v[i] * dt);
        }
        let mut contact: Vec<Option<DVec3>> = vec![None; n];
        for _ in 0..ITERATIONS {
            self.solve(&mut pred, &self.structural, p.structural);
            self.solve(&mut pred, &self.shear, p.shear);
            self.solve(&mut pred, &self.flexion, p.flexion);
            for i in 0..n {
                if let Some(nrm) = self.collide(world, self.x[i], &mut pred[i]) {
                    contact[i] = Some(nrm);
                }
            }
        }
        if p.keep_shape {
            self.shape_match(&mut pred);
            for i in 0..n {
                if let Some(nrm) = self.collide(world, self.x[i], &mut pred[i]) {
                    contact[i] = Some(nrm);
                }
            }
        }
        for i in 0..n {
            let mut v = (pred[i] - self.x[i]) / dt;
            if let Some(nrm) = contact[i] {
                let vt = v - nrm * v.dot(nrm);
                let before = self.v[i].dot(nrm);
                let bounced = if before < 0.0 {
                    -before * p.bounce
                } else {
                    0.0
                };
                let friction = 1.0 - p.friction.clamp(0.0, 1.0);
                v = vt * friction + nrm * bounced;
            }
            self.v[i] = v;
        }
        // Internal damping: the motion relative to the body's own mean velocity decays.
        let f = 1.0 - (-p.structural_damping * DAMPING_RATE * dt).exp();
        let mean = self.v.iter().copied().sum::<DVec3>() / n as f64;
        for v in &mut self.v {
            *v -= (*v - mean) * f;
        }
        for (links, damping) in [
            (&self.structural, p.structural_damping),
            (&self.shear, p.shear_damping),
            (&self.flexion, p.flexion_damping),
        ] {
            damp(&mut self.v, links, &pred, damping, dt);
        }
        self.x = pred;
    }

    fn shape_match(&self, pred: &mut [DVec3]) {
        let n = pred.len() as f64;
        let c = pred.iter().copied().sum::<DVec3>() / n;
        let c0 = self.rest.iter().copied().sum::<DVec3>() / n;
        let mut a = DMat3::ZERO;
        for (p, r) in pred.iter().zip(&self.rest) {
            let (pp, rr) = (*p - c, *r - c0);
            a += DMat3::from_cols(pp * rr.x, pp * rr.y, pp * rr.z);
        }
        let q = extract_rotation(&a);
        let rot = DMat3::from_quat(q);
        for (p, r) in pred.iter_mut().zip(&self.rest) {
            let goal = c + rot * (*r - c0);
            *p += (goal - *p) * KEEP_SHAPE;
        }
    }
}

fn damp(v: &mut [DVec3], links: &[Link], pred: &[DVec3], damping: f64, dt: f64) {
    let f = 1.0 - (-damping * DAMPING_RATE * dt).exp();
    if f <= 0.0 {
        return;
    }
    for l in links {
        let (a, b) = (l.a as usize, l.b as usize);
        let d = pred[b] - pred[a];
        let len = d.length();
        if len < 1e-12 {
            continue;
        }
        let dir = d / len;
        let rel = (v[b] - v[a]).dot(dir);
        let dv = dir * (rel * f * 0.5);
        v[a] += dv;
        v[b] -= dv;
    }
}

/// The rotation closest to the matrix `a` (Müller et al., "A robust method to extract the
/// rotational part of deformations", 2016): iterated from the identity.
fn extract_rotation(a: &DMat3) -> DQuat {
    let mut q = DQuat::IDENTITY;
    for _ in 0..16 {
        let r = DMat3::from_quat(q);
        let omega =
            (r.x_axis.cross(a.x_axis) + r.y_axis.cross(a.y_axis) + r.z_axis.cross(a.z_axis))
                / (r.x_axis.dot(a.x_axis) + r.y_axis.dot(a.y_axis) + r.z_axis.dot(a.z_axis))
                    .abs()
                    .max(1e-9);
        let w = omega.length();
        if w < 1e-9 {
            break;
        }
        q = (DQuat::from_axis_angle(omega / w, w) * q).normalize();
    }
    q
}

fn to_v(p: DVec3) -> Vector {
    Vector::new(p.x, p.y, p.z) * CM_TO_M
}

fn from_v(p: Vector) -> DVec3 {
    DVec3::new(p.x, p.y, p.z) * M_TO_CM
}
