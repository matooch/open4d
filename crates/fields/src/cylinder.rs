//! Cylinder field (docs/specs/FIELDS.md F6).

use forge_core::{NodeType, ParamDesc, Unit};

use crate::node::{BIG, FIELD, Params, axis_direction, field_type, shape_params};
use crate::shape::{Direction, Shape, axial_radial, ratio};

pub const CYLINDER: &str = "forge.field.cylinder";

/// Cylinder in its own space.
#[derive(Debug, Clone, PartialEq)]
pub struct Cylinder {
    pub height: f64,
    pub radius: f64,
    pub direction: Direction,
}

impl Cylinder {
    /// Normalised distance: 0 on the axis, 1 on the surface.
    pub fn distance(&self, p: glam::DVec3) -> f64 {
        let (a, r) = axial_radial(p, self.direction);
        ratio(r, self.radius).max(ratio(a, self.height))
    }
}

pub fn cylinder_type() -> NodeType {
    let mut shape = vec![
        ParamDesc::float("height", "Height", 100.0)
            .range(0.0, BIG)
            .unit(Unit::Distance)
            .group(FIELD),
        ParamDesc::float("radius", "Radius", 75.0)
            .range(0.0, BIG)
            .unit(Unit::Distance)
            .group(FIELD),
    ];
    shape.extend(shape_params());
    field_type(CYLINDER, "Cylinder Field", shape, 0.5, true, true)
}

pub(crate) fn shape(p: &Params) -> Shape {
    Shape::Cylinder(Cylinder {
        height: p.float("height"),
        radius: p.float("radius"),
        direction: axis_direction(&p.option("direction")),
    })
}
