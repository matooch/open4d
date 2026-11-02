//! Shape values of the Linear, Spherical and Box fields (docs/specs/FIELDS.md F3).

use glam::DVec3;

use crate::capsule::Capsule;
use crate::cone::Cone;
use crate::cylinder::Cylinder;
use crate::radial::Radial;
use crate::torus::Torus;

/// A signed axis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Direction {
    pub axis: usize,
    pub negative: bool,
}

/// The shape of a field in its own space.
#[derive(Debug, Clone, PartialEq)]
pub enum Shape {
    Linear { length: f64, direction: Direction },
    Spherical { radius: f64 },
    Box { size: DVec3 },
    Cylinder(Cylinder),
    Cone(Cone),
    Torus(Torus),
    Capsule(Capsule),
    Radial(Radial),
}

/// The axial coordinate `a` (negated for a negative direction) and the distance `r` from the axis.
pub(crate) fn axial_radial(p: DVec3, direction: Direction) -> (f64, f64) {
    let a = if direction.negative {
        -p[direction.axis]
    } else {
        p[direction.axis]
    };
    let (u, v) = match direction.axis {
        0 => (p.y, p.z),
        1 => (p.x, p.z),
        _ => (p.x, p.y),
    };
    (a, u.hypot(v))
}

/// `|a| / b` with the zero cases defined: `0 / 0 = 0`, `a / 0 = inf`.
pub(crate) fn ratio(a: f64, b: f64) -> f64 {
    if b > 0.0 {
        a.abs() / b
    } else if a == 0.0 {
        0.0
    } else {
        f64::INFINITY
    }
}

impl Shape {
    /// Ramp position of `p` (field space): 1 in the core, 0 outside, in 0..1.
    pub fn ramp(&self, p: DVec3) -> f64 {
        match self {
            Shape::Linear { length, direction } => {
                let along = if direction.negative {
                    -p[direction.axis]
                } else {
                    p[direction.axis]
                };
                if *length > 0.0 {
                    (0.5 + along / (2.0 * length)).clamp(0.0, 1.0)
                } else if along > 0.0 {
                    1.0
                } else if along < 0.0 {
                    0.0
                } else {
                    0.5
                }
            }
            Shape::Spherical { radius } => {
                let d = if *radius > 0.0 {
                    p.length() / radius
                } else if p == DVec3::ZERO {
                    0.0
                } else {
                    f64::INFINITY
                };
                (1.0 - d).clamp(0.0, 1.0)
            }
            Shape::Box { size } => {
                let d = ratio(p.x, size.x)
                    .max(ratio(p.y, size.y))
                    .max(ratio(p.z, size.z));
                (1.0 - d).clamp(0.0, 1.0)
            }
            Shape::Radial(r) => r.ramp(p),
            _ => (1.0 - self.distance(p)).clamp(0.0, 1.0),
        }
    }

    /// Normalised distance of the Cylinder, Cone, Torus and Capsule (0 inside, 1 on the surface).
    fn distance(&self, p: DVec3) -> f64 {
        match self {
            Shape::Cylinder(s) => s.distance(p),
            Shape::Cone(s) => s.distance(p),
            Shape::Torus(s) => s.distance(p),
            Shape::Capsule(s) => s.distance(p),
            _ => 0.0,
        }
    }

    /// Whether `p` lies outside the shape (Clip to Shape). Linear has no outside.
    pub fn outside(&self, p: DVec3) -> bool {
        match self {
            Shape::Linear { .. } => false,
            Shape::Spherical { radius } => ratio(p.length(), *radius) > 1.0,
            Shape::Box { size } => {
                ratio(p.x, size.x)
                    .max(ratio(p.y, size.y))
                    .max(ratio(p.z, size.z))
                    > 1.0
            }
            Shape::Radial(r) => r.outside(p),
            _ => self.distance(p) > 1.0,
        }
    }
}
