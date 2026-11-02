//! Torus field (docs/specs/FIELDS.md F6).

use forge_core::{NodeType, ParamDesc, Unit};
use glam::DVec3;

use crate::node::{BIG, FIELD, Params, axis_direction, field_type};
use crate::shape::{Direction, Shape, axial_radial, ratio};

pub const TORUS: &str = "forge.field.torus";

/// Torus in its own space: a ring of `radius` around the axis, tube radius `thickness`.
#[derive(Debug, Clone, PartialEq)]
pub struct Torus {
    pub radius: f64,
    pub thickness: f64,
    pub direction: Direction,
}

impl Torus {
    /// Normalised distance: 0 on the ring, 1 on the tube surface.
    pub fn distance(&self, p: DVec3) -> f64 {
        let (a, r) = axial_radial(p, self.direction);
        ratio((r - self.radius).hypot(a), self.thickness)
    }
}

pub fn torus_type() -> NodeType {
    let shape = vec![
        ParamDesc::float("radius", "Radius", 100.0)
            .range(0.0, BIG)
            .unit(Unit::Distance)
            .group(FIELD),
        ParamDesc::float("thickness", "Thickness", 35.0)
            .range(0.0, BIG)
            .unit(Unit::Distance)
            .group(FIELD),
        ParamDesc::enumeration(
            "direction",
            "Direction",
            "y_pos",
            &[("x_pos", "X+"), ("y_pos", "Y+"), ("z_pos", "Z+")],
        )
        .group(FIELD),
    ];
    field_type(TORUS, "Torus Field", shape, 0.5, true, true)
}

pub(crate) fn shape(p: &Params) -> Shape {
    Shape::Torus(Torus {
        radius: p.float("radius"),
        thickness: p.float("thickness"),
        direction: axis_direction(&p.option("direction")),
    })
}
