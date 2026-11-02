//! Node types of the Linear, Spherical and Box fields and the reading of their parameters
//! (docs/specs/FIELDS.md F2).

use std::collections::BTreeMap;

use forge_core::{Category, NodeType, ParamDesc, Plugin, Registry, RegistryError, Unit, Value};
use glam::{DMat4, DVec3};

use crate::remap::{Contour, Remap};
use crate::shape::{Direction, Shape};
use crate::{
    Field, FieldError, ShapeField, capsule, capsule_type, cone, cone_type, cylinder, cylinder_type,
    radial, radial_type, random, random_type, torus, torus_type,
};

pub const LINEAR: &str = "forge.field.linear";
pub const SPHERICAL: &str = "forge.field.spherical";
pub const BOX: &str = "forge.field.box";

pub(crate) const BIG: f64 = 1e20;
const STEPS_MAX: i64 = 2_147_483_647;
pub(crate) const FIELD: &str = "Field";
const REMAP: &str = "Remapping";
const LIST: &str = "List Row";

/// Id of the list-row parameter: the strength of this field's layer in a field list
/// (FIELDS.md F1; O24 `layer_strength_percent`).
pub const LAYER_STRENGTH: &str = "layer_strength";

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

/// The Direction parameter with the six signed axes (default Y+).
pub(crate) fn shape_params() -> Vec<ParamDesc> {
    vec![
        ParamDesc::enumeration(
            "direction",
            "Direction",
            "y_pos",
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
    ]
}

/// The signed axis of a direction option id (`x_pos`, `z_neg`, ...).
pub(crate) fn axis_direction(id: &str) -> Direction {
    Direction {
        axis: match id.as_bytes()[0] {
            b'x' => 0,
            b'y' => 1,
            _ => 2,
        },
        negative: id.ends_with("neg"),
    }
}

pub(crate) fn field_type(
    id: &str,
    label: &str,
    shape: Vec<ParamDesc>,
    inner: f64,
    mirror: bool,
    clip: bool,
) -> NodeType {
    let mut ty = NodeType::new(id, label, Category::Field);
    for p in shape {
        ty = ty.param(p);
    }
    if clip {
        ty = ty.param(ParamDesc::bool("clip_to_shape", "Clip to Shape", false).group(FIELD));
    }
    ty = ty.param(ParamDesc::color("color", "Color", [1.0; 3]).group(FIELD));
    for p in remap_params(inner, mirror) {
        ty = ty.param(p);
    }
    ty = ty.param(
        ParamDesc::float(LAYER_STRENGTH, "Layer Strength", 1.0)
            .range(-BIG, BIG)
            .unit(Unit::Percent)
            .group(LIST),
    );
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
        true,
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
        r.register(box_type())?;
        r.register(cylinder_type())?;
        r.register(cone_type())?;
        r.register(torus_type())?;
        r.register(capsule_type())?;
        r.register(radial_type())?;
        r.register(random_type())
    }
}

/// Typed parameter access with defaults from the descriptors.
pub(crate) struct Params<'a> {
    pub(crate) ty: &'a NodeType,
    pub(crate) values: &'a BTreeMap<String, Value>,
}

impl Params<'_> {
    pub(crate) fn get(&self, id: &str) -> Value {
        let desc = self
            .ty
            .find_param(id)
            .expect("a field reads its own params");
        self.values
            .get(id)
            .and_then(|v| desc.validate(v.clone()).ok())
            .unwrap_or_else(|| desc.default.clone())
    }

    pub(crate) fn float(&self, id: &str) -> f64 {
        match self.get(id) {
            Value::Float(f) => f,
            other => unreachable!("{id} is not a float: {other:?}"),
        }
    }

    pub(crate) fn bool(&self, id: &str) -> bool {
        matches!(self.get(id), Value::Bool(true))
    }

    pub(crate) fn vec3(&self, id: &str) -> DVec3 {
        match self.get(id) {
            Value::Vec3(v) => DVec3::from_array(v),
            other => unreachable!("{id} is not a vec3: {other:?}"),
        }
    }

    pub(crate) fn color(&self, id: &str) -> [f64; 3] {
        match self.get(id) {
            Value::Color(c) => c,
            other => unreachable!("{id} is not a colour: {other:?}"),
        }
    }

    pub(crate) fn option(&self, id: &str) -> String {
        match self.get(id) {
            Value::Enum(s) => s,
            other => unreachable!("{id} is not an enum: {other:?}"),
        }
    }

    pub(crate) fn remap(&self) -> Remap {
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

/// The field of a node of type `type_id`; `None` if the type is not a field type of this crate.
pub(crate) fn build_field(
    type_id: &str,
    values: &BTreeMap<String, Value>,
    world: DMat4,
) -> Option<Result<Box<dyn Field>, FieldError>> {
    let ty = match type_id {
        LINEAR => linear_type(),
        SPHERICAL => spherical_type(),
        BOX => box_type(),
        cylinder::CYLINDER => cylinder_type(),
        cone::CONE => cone_type(),
        torus::TORUS => torus_type(),
        capsule::CAPSULE => capsule_type(),
        radial::RADIAL => radial_type(),
        random::RANDOM => random_type(),
        _ => return None,
    };
    let p = Params { ty: &ty, values };
    let shape = match type_id {
        LINEAR => {
            let dir = p.option("direction");
            Shape::Linear {
                length: p.float("length"),
                direction: axis_direction(&dir),
            }
        }
        SPHERICAL => Shape::Spherical {
            radius: p.float("radius"),
        },
        BOX => Shape::Box {
            size: p.vec3("size"),
        },
        cylinder::CYLINDER => cylinder::shape(&p),
        cone::CONE => cone::shape(&p),
        torus::TORUS => torus::shape(&p),
        capsule::CAPSULE => capsule::shape(&p),
        radial::RADIAL => radial::shape(&p),
        _ => {
            return Some(
                random::field(&p, p.remap(), world)
                    .and_then(|f| p.remap().check().map(|()| f))
                    .map(|f| Box::new(f) as Box<dyn Field>),
            );
        }
    };
    Some(
        ShapeField::new(shape, p.remap(), p.bool("clip_to_shape"), world)
            .map(|f| Box::new(f.with_color(p.color("color"))) as Box<dyn Field>),
    )
}
