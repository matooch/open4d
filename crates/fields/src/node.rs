//! Node types of the Linear, Spherical and Box fields and the reading of their parameters
//! (docs/specs/FIELDS.md F2).

use std::collections::BTreeMap;

use forge_core::{Category, NodeType, ParamDesc, Plugin, Registry, RegistryError, Unit, Value};
use glam::{DMat4, DVec3};

use crate::remap::{Contour, Remap};
use crate::shape::{Direction, Shape};
use crate::{FieldError, ShapeField};

pub const LINEAR: &str = "forge.field.linear";
pub const SPHERICAL: &str = "forge.field.spherical";
pub const BOX: &str = "forge.field.box";

const BIG: f64 = 1e20;
const STEPS_MAX: i64 = 2_147_483_647;
const FIELD: &str = "Field";
const REMAP: &str = "Remapping";

/// The Remapping tab parameters; `inner` and `mirror` are the per-type defaults.
fn remap_params(inner: f64, mirror: bool) -> Vec<ParamDesc> {
    let pct = |id: &str, label: &str, default: f64, lo: f64, hi: f64| {
        ParamDesc::float(id, label, default)
            .range(lo, hi)
            .unit(Unit::Percent)
            .group(REMAP)
    };
    vec![
        ParamDesc::bool("enable_remapping", "Enable Remapping", true).group(REMAP),
        pct("strength", "Strength", 1.0, -BIG, BIG),
        ParamDesc::bool("invert", "Invert", false).group(REMAP),
        pct("inner_offset", "Inner Offset", inner, 0.0, 1.0),
        pct("min", "Min", 0.0, -BIG, BIG),
        ParamDesc::bool("clamp_min", "Clamp Min", true).group(REMAP),
        pct("max", "Max", 1.0, -BIG, BIG),
        ParamDesc::bool("clamp_max", "Clamp Max", true).group(REMAP),
        pct("multiplier", "Multiplier", 1.0, -BIG, BIG),
        ParamDesc::enumeration(
            "contour_mode",
            "Contour Mode",
            "none",
            &[
                ("none", "None"),
                ("quadratic", "Quadratic"),
                ("step", "Step"),
                ("quantize", "Quantize"),
                ("curve", "Curve"),
            ],
        )
        .group(REMAP),
        pct("curve", "Curve", 0.0, -1.0, 1.0),
        ParamDesc::int("steps", "Steps", 1)
            .range(1, STEPS_MAX)
            .group(REMAP),
        ParamDesc::bool("mirror_graph", "Mirror Graph", mirror).group(REMAP),
        pct("spline_offset", "Spline Offset", 0.0, -BIG, BIG),
        pct("spline_range", "Spline Range", 1.0, -BIG, BIG),
    ]
}

fn field_type(id: &str, label: &str, shape: Vec<ParamDesc>, inner: f64, mirror: bool) -> NodeType {
    let mut ty = NodeType::new(id, label, Category::Field);
    for p in shape {
        ty = ty.param(p);
    }
    ty = ty.param(ParamDesc::bool("clip_to_shape", "Clip to Shape", false).group(FIELD));
    ty = ty.param(ParamDesc::color("color", "Color", [1.0; 3]).group(FIELD));
    for p in remap_params(inner, mirror) {
        ty = ty.param(p);
    }
    ty
}

pub fn linear_type() -> NodeType {
    field_type(
        LINEAR,
        "Linear Field",
        vec![
            ParamDesc::float("length", "Length", 100.0)
                .range(0.0, BIG)
                .unit(Unit::Distance)
                .group(FIELD),
            ParamDesc::enumeration(
                "direction",
                "Direction",
                "x_pos",
                &[
                    ("x_pos", "X+"),
                    ("x_neg", "X-"),
                    ("y_pos", "Y+"),
                    ("y_neg", "Y-"),
                    ("z_pos", "Z+"),
                    ("z_neg", "Z-"),
                ],
            )
            .group(FIELD),
        ],
        0.0,
        false,
    )
}

pub fn spherical_type() -> NodeType {
    field_type(
        SPHERICAL,
        "Spherical Field",
        vec![
            ParamDesc::float("radius", "Size", 100.0)
                .range(0.0, BIG)
                .unit(Unit::Distance)
                .group(FIELD),
        ],
        0.5,
        true,
    )
}

pub fn box_type() -> NodeType {
    field_type(
        BOX,
        "Box Field",
        vec![
            ParamDesc::vec3("size", "Size", [100.0; 3])
                .range(0.0, BIG)
                .unit(Unit::Distance)
                .group(FIELD),
        ],
        0.5,
        true,
    )
}

/// Registers the field node types.
pub struct FieldToolset;

impl Plugin for FieldToolset {
    fn id(&self) -> &str {
        "forge.fields"
    }

    fn register(&self, r: &mut Registry) -> Result<(), RegistryError> {
        r.register(linear_type())?;
        r.register(spherical_type())?;
        r.register(box_type())
    }
}

/// Typed parameter access with defaults from the descriptors.
struct Params<'a> {
    ty: &'a NodeType,
    values: &'a BTreeMap<String, Value>,
}

impl Params<'_> {
    fn get(&self, id: &str) -> Value {
        let desc = self
            .ty
            .find_param(id)
            .expect("a field reads its own params");
        self.values
            .get(id)
            .and_then(|v| desc.validate(v.clone()).ok())
            .unwrap_or_else(|| desc.default.clone())
    }

    fn float(&self, id: &str) -> f64 {
        match self.get(id) {
            Value::Float(f) => f,
            other => unreachable!("{id} is not a float: {other:?}"),
        }
    }

    fn bool(&self, id: &str) -> bool {
        matches!(self.get(id), Value::Bool(true))
    }

    fn vec3(&self, id: &str) -> DVec3 {
        match self.get(id) {
            Value::Vec3(v) => DVec3::from_array(v),
            other => unreachable!("{id} is not a vec3: {other:?}"),
        }
    }

    fn color(&self, id: &str) -> [f64; 3] {
        match self.get(id) {
            Value::Color(c) => c,
            other => unreachable!("{id} is not a colour: {other:?}"),
        }
    }

    fn option(&self, id: &str) -> String {
        match self.get(id) {
            Value::Enum(s) => s,
            other => unreachable!("{id} is not an enum: {other:?}"),
        }
    }

    fn remap(&self) -> Remap {
        let steps = match self.get("steps") {
            Value::Int(i) => i.clamp(1, STEPS_MAX) as u32,
            other => unreachable!("steps is not an int: {other:?}"),
        };
        Remap {
            enabled: self.bool("enable_remapping"),
            strength: self.float("strength"),
            invert: self.bool("invert"),
            inner_offset: self.float("inner_offset"),
            min: self.float("min"),
            clamp_min: self.bool("clamp_min"),
            max: self.float("max"),
            clamp_max: self.bool("clamp_max"),
            multiplier: self.float("multiplier"),
            contour: match self.option("contour_mode").as_str() {
                "quadratic" => Contour::Quadratic,
                "step" => Contour::Step,
                "quantize" => Contour::Quantize,
                "curve" => Contour::Curve,
                _ => Contour::None,
            },
            curve: self.float("curve"),
            steps,
        }
    }
}

/// The field of a node of type `type_id`; `None` if the type is not one of these fields.
pub(crate) fn shape_field(
    type_id: &str,
    values: &BTreeMap<String, Value>,
    world: DMat4,
) -> Option<Result<ShapeField, FieldError>> {
    let ty = match type_id {
        LINEAR => linear_type(),
        SPHERICAL => spherical_type(),
        BOX => box_type(),
        _ => return None,
    };
    let p = Params { ty: &ty, values };
    let shape = match type_id {
        LINEAR => {
            let dir = p.option("direction");
            Shape::Linear {
                length: p.float("length"),
                direction: Direction {
                    axis: match dir.as_bytes()[0] {
                        b'x' => 0,
                        b'y' => 1,
                        _ => 2,
                    },
                    negative: dir.ends_with("neg"),
                },
            }
        }
        SPHERICAL => Shape::Spherical {
            radius: p.float("radius"),
        },
        _ => Shape::Box {
            size: p.vec3("size"),
        },
    };
    Some(
        ShapeField::new(shape, p.remap(), p.bool("clip_to_shape"), world)
            .map(|f| f.with_color(p.color("color"))),
    )
}
