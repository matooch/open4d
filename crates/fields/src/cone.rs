//! Cone field (docs/specs/FIELDS.md F6).

use forge_core::{NodeType, ParamDesc, Unit};
use glam::DVec3;

use crate::node::{BIG, FIELD, Params, axis_direction, field_type, shape_params};
use crate::shape::{Direction, Shape, axial_radial, ratio};

pub const CONE: &str = "forge.field.cone";

/// Cone in its own space: base (full radius) at `-height / 2` on the axis, apex at `+height / 2`.
#[derive(Debug, Clone, PartialEq)]
pub struct Cone {
    pub height: f64,
    pub radius: f64,
    pub direction: Direction,
}

impl Cone {
    /// The cone radius at axial position `a`: full below the base, 0 above the apex.
    fn radius_at(&self, a: f64) -> f64 {
        if self.height > 0.0 {
            self.radius * ((self.height / 2.0 - a) / self.height).clamp(0.0, 1.0)
        } else if a < 0.0 {
            self.radius
        } else if a > 0.0 {
            0.0
        } else {
            self.radius / 2.0
        }
    }

    /// Normalised distance: 0 on the axis, 1 on the surface.
    pub fn distance(&self, p: DVec3) -> f64 {
        let (a, r) = axial_radial(p, self.direction);
        ratio(r, self.radius_at(a)).max(ratio(a, self.height))
    }
}

pub fn cone_type() -> NodeType {
    let mut shape = vec![
        ParamDesc::float("height", "Height", 100.0)
            .range(0.0, BIG)
            .unit(Unit::Distance)
            .group(FIELD),
        ParamDesc::float("radius", "Radius", 100.0)
            .range(0.0, BIG)
            .unit(Unit::Distance)
            .group(FIELD),
    ];
    shape.extend(shape_params());
    field_type(CONE, "Cone Field", shape, 0.5, true, true)
}

pub(crate) fn shape(p: &Params) -> Shape {
    Shape::Cone(Cone {
        height: p.float("height"),
        radius: p.float("radius"),
        direction: axis_direction(&p.option("direction")),
    })
}
