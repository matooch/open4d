//! Radial field (docs/specs/FIELDS.md F6): an angular range around an axis.

use std::f64::consts::TAU;

use forge_core::{NodeType, ParamDesc, Unit};
use glam::DVec3;

use crate::node::{BIG, FIELD, Params, field_type};
use crate::shape::Shape;

pub const RADIAL: &str = "forge.field.radial";

/// A range this close to a full turn is a full turn (degrees converted to radians round).
const FULL: f64 = TAU - 1e-9;

/// Radial field in its own space. Angles are in radians.
#[derive(Debug, Clone, PartialEq)]
pub struct Radial {
    pub start_angle: f64,
    pub start_transition: f64,
    pub end_angle: f64,
    pub end_transition: f64,
    pub iterations: u32,
    pub offset: f64,
    /// 0 = X, 1 = Y, 2 = Z.
    pub axis: usize,
}

impl Radial {
    /// The angle of `p` about the axis, right-handed, in `[-pi, pi]`.
    fn theta(&self, p: DVec3) -> f64 {
        // `+ 0.0` turns -0.0 into 0.0 so the half-turn ray has one angle (pi).
        let (u, v) = match self.axis {
            0 => (p.y, p.z),
            1 => (p.x, -p.z),
            _ => (p.x, p.y),
        };
        (v + 0.0).atan2(u + 0.0)
    }

    /// The position inside the repeated range, `[0, TAU)`, and the range span.
    fn phase(&self, p: DVec3) -> (f64, f64) {
        let phi = (self.theta(p) * f64::from(self.iterations) - self.offset - self.start_angle)
            .rem_euclid(TAU);
        (phi, self.end_angle - self.start_angle)
    }

    /// Ramp position: 1 inside the range, ramps through the transitions, 0 outside.
    pub fn ramp(&self, p: DVec3) -> f64 {
        let (phi, span) = self.phase(p);
        if span >= FULL {
            return 1.0;
        }
        if span <= 0.0 || phi >= span {
            return 0.0;
        }
        let mut t: f64 = 1.0;
        if self.start_transition > 0.0 && phi < self.start_transition {
            t = t.min(phi / self.start_transition);
        }
        if self.end_transition > 0.0 && phi > span - self.end_transition {
            t = t.min((span - phi) / self.end_transition);
        }
        t.clamp(0.0, 1.0)
    }

    /// Whether `p` lies outside the angular range.
    pub fn outside(&self, p: DVec3) -> bool {
        let (phi, span) = self.phase(p);
        span < FULL && (span <= 0.0 || phi >= span)
    }
}

pub fn radial_type() -> NodeType {
    let angle = |id: &str, label: &str, default: f64, lo: f64, hi: f64| {
        ParamDesc::float(id, label, default)
            .range(lo, hi)
            .unit(Unit::Angle)
            .group(FIELD)
    };
    let shape = vec![
        angle("start_angle", "Start Angle", 0.0, -BIG, BIG),
        angle("start_transition", "Start Transition", 0.0, 0.0, TAU),
        angle("end_angle", "End Angle", TAU, -BIG, BIG),
        angle("end_transition", "End Transition", 0.0, 0.0, TAU),
        ParamDesc::int("iterations", "Iterations", 1)
            .range(1, i64::MAX)
            .group(FIELD),
        angle("offset", "Offset", 0.0, -BIG, BIG),
        ParamDesc::enumeration("axis", "Axis", "y", &[("x", "X"), ("y", "Y"), ("z", "Z")])
            .group(FIELD),
    ];
    field_type(RADIAL, "Radial Field", shape, 0.0, true, true)
}

pub(crate) fn shape(p: &Params) -> Shape {
    let iterations = match p.get("iterations") {
        forge_core::Value::Int(i) => i.clamp(1, i64::from(u32::MAX)) as u32,
        other => unreachable!("iterations is not an int: {other:?}"),
    };
    Shape::Radial(Radial {
        start_angle: p.float("start_angle"),
        start_transition: p.float("start_transition"),
        end_angle: p.float("end_angle"),
        end_transition: p.float("end_transition"),
        iterations,
        offset: p.float("offset"),
        axis: match p.option("axis").as_str() {
            "x" => 0,
            "y" => 1,
            _ => 2,
        },
    })
}
