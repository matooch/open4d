//! Capsule field (docs/specs/FIELDS.md F6).

use forge_core::{NodeType, ParamDesc, Unit};
use glam::DVec3;

use crate::node::{BIG, FIELD, Params, axis_direction, field_type, shape_params};
use crate::shape::{Direction, Shape, axial_radial, ratio};

pub const CAPSULE: &str = "forge.field.capsule";

/// Capsule in its own space: the segment from `-height / 2` to `+height / 2` on the axis, swept by
/// a radius running from `bottom_radius` to `top_radius`.
#[derive(Debug, Clone, PartialEq)]
pub struct Capsule {
    pub top_radius: f64,
    pub bottom_radius: f64,
    pub height: f64,
    pub direction: Direction,
}

impl Capsule {
    fn radius_at(&self, a: f64) -> f64 {
        let s = if self.height > 0.0 {
            (a / self.height + 0.5).clamp(0.0, 1.0)
        } else {
            0.5
        };
        self.bottom_radius + (self.top_radius - self.bottom_radius) * s
    }

    /// Normalised distance: 0 on the segment, 1 on the surface.
    pub fn distance(&self, p: DVec3) -> f64 {
        let (a, r) = axial_radial(p, self.direction);
        let beyond = (a.abs() - self.height / 2.0).max(0.0);
        ratio(r.hypot(beyond), self.radius_at(a))
    }
}

pub fn capsule_type() -> NodeType {
    let mut shape = vec![
        ParamDesc::float("top_radius", "Top Radius", 75.0)
            .range(0.0, BIG)
            .unit(Unit::Distance)
            .group(FIELD),
        ParamDesc::float("bottom_radius", "Bottom Radius", 75.0)
            .range(0.0, BIG)
            .unit(Unit::Distance)
            .group(FIELD),
        ParamDesc::float("height", "Height", 150.0)
            .range(0.0, BIG)
            .unit(Unit::Distance)
            .group(FIELD),
    ];
    shape.extend(shape_params());
    field_type(CAPSULE, "Capsule Field", shape, 0.5, true, true)
}

pub(crate) fn shape(p: &Params) -> Shape {
    Shape::Capsule(Capsule {
        top_radius: p.float("top_radius"),
        bottom_radius: p.float("bottom_radius"),
        height: p.float("height"),
        direction: axis_direction(&p.option("direction")),
    })
}
