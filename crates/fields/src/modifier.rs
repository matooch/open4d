//! Modifier layers of a field list (docs/specs/FIELDS.md F8): rows that transform the value
//! accumulated from the rows below them.

use std::collections::BTreeMap;

use forge_core::{Category, NodeType, ParamDesc, Unit, Value};

use crate::FieldError;
use crate::node::{BIG, Params};

pub const REMAP_LAYER: &str = "forge.field.remap_layer";
pub const INVERT_LAYER: &str = "forge.field.invert_layer";
pub const CLAMP_LAYER: &str = "forge.field.clamp_layer";
pub const COLORIZE_LAYER: &str = "forge.field.colorize_layer";
pub const CURVE_LAYER: &str = "forge.field.curve_layer";
pub const STEP_LAYER: &str = "forge.field.step_layer";

/// A modifier layer's function of the value below it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Modifier {
    /// `min + v (max - min)`.
    Remap { min: f64, max: f64 },
    /// `1 - v`.
    Invert,
    /// `clamp(v, minimum, maximum)` per flag.
    Clamp {
        minimum: f64,
        clamp_min: bool,
        maximum: f64,
        clamp_max: bool,
    },
    /// The value passes through (Colorize acts on the colour channel only; the default Curve
    /// layer is the identity).
    Identity,
}

impl Modifier {
    /// The sample a row holds above `below`: the modified value, the colour and direction of the
    /// rows below.
    pub(crate) fn sample_of(&self, below: &crate::FieldSample) -> crate::FieldSample {
        crate::FieldSample {
            value: self.apply(below.value),
            ..*below
        }
    }

    /// The modified value.
    pub fn apply(&self, v: f64) -> f64 {
        match *self {
            Modifier::Remap { min, max } => min + v * (max - min),
            Modifier::Invert => 1.0 - v,
            Modifier::Clamp {
                minimum,
                clamp_min,
                maximum,
                clamp_max,
            } => {
                let mut v = v;
                if clamp_min {
                    v = v.max(minimum);
                }
                if clamp_max {
                    v = v.min(maximum);
                }
                v
            }
            Modifier::Identity => v,
        }
    }
}

fn layer(id: &str, label: &str) -> NodeType {
    NodeType::new(id, label, Category::Field)
}

fn pct(id: &str, label: &str, default: f64) -> ParamDesc {
    ParamDesc::float(id, label, default)
        .range(-BIG, BIG)
        .unit(Unit::Percent)
}

pub fn remap_layer_type() -> NodeType {
    layer(REMAP_LAYER, "Remap Layer")
        .param(pct("min", "Min", 0.0))
        .param(pct("max", "Max", 1.0))
}

pub fn invert_layer_type() -> NodeType {
    layer(INVERT_LAYER, "Invert Layer")
}

pub fn clamp_layer_type() -> NodeType {
    layer(CLAMP_LAYER, "Clamp Layer")
        .param(ParamDesc::float("minimum", "Minimum", 0.0).range(-BIG, BIG))
        .param(ParamDesc::bool("clamp_min", "Clamp Min", true))
        .param(ParamDesc::float("maximum", "Maximum", 100.0).range(-BIG, BIG))
        .param(ParamDesc::bool("clamp_max", "Clamp Max", true))
}

pub fn colorize_layer_type() -> NodeType {
    layer(COLORIZE_LAYER, "Colorize Layer")
        .param(ParamDesc::enumeration(
            "source",
            "Source",
            "value",
            &[("value", "Value")],
        ))
        .param(ParamDesc::enumeration(
            "mode",
            "Mode",
            "monochrome",
            &[("monochrome", "Monochrome")],
        ))
        .param(ParamDesc::color(
            "base_color",
            "Base Color",
            [0.2, 0.4, 1.0],
        ))
        .param(pct("base_density", "Base Density", 0.5))
        .param(ParamDesc::bool("value_as_alpha", "Value as Alpha", false))
}

pub fn curve_layer_type() -> NodeType {
    layer(CURVE_LAYER, "Curve Layer")
        .param(pct("offset", "Offset", 0.0))
        .param(ParamDesc::enumeration(
            "before_curve",
            "Before Curve",
            "clamp",
            &[("clamp", "Clamp")],
        ))
        .param(ParamDesc::enumeration(
            "after_curve",
            "After Curve",
            "clamp",
            &[("clamp", "Clamp")],
        ))
        // The shape: the knots of the curve; the default is the straight ramp (0,0)-(1,1).
        .param(ParamDesc::vec3_list("shape_knots", "Shape"))
}

/// The Step layer (Contour Mode Step): pending, building it is `Unsupported` (F8).
pub fn step_layer_type() -> NodeType {
    layer(STEP_LAYER, "Step Layer")
        .param(ParamDesc::enumeration(
            "contour_mode",
            "Contour Mode",
            "step",
            &[("step", "Step")],
        ))
        .param(ParamDesc::int("steps", "Steps", 4).range(1, 1_000_000))
}

/// The registered modifier-layer types.
pub fn types() -> [NodeType; 6] {
    [
        remap_layer_type(),
        invert_layer_type(),
        clamp_layer_type(),
        colorize_layer_type(),
        curve_layer_type(),
        step_layer_type(),
    ]
}

/// The modifier of a node of type `type_id`; `None` if it is not a modifier-layer type.
pub(crate) fn build(
    type_id: &str,
    values: &BTreeMap<String, Value>,
) -> Option<Result<Modifier, FieldError>> {
    let ty = match type_id {
        REMAP_LAYER => remap_layer_type(),
        INVERT_LAYER => invert_layer_type(),
        CLAMP_LAYER => clamp_layer_type(),
        COLORIZE_LAYER => colorize_layer_type(),
        CURVE_LAYER => curve_layer_type(),
        STEP_LAYER => return Some(Err(FieldError::Unsupported("step layer"))),
        _ => return None,
    };
    let p = Params { ty: &ty, values };
    Some(match type_id {
        REMAP_LAYER => Ok(Modifier::Remap {
            min: p.float("min"),
            max: p.float("max"),
        }),
        INVERT_LAYER => Ok(Modifier::Invert),
        CLAMP_LAYER => Ok(Modifier::Clamp {
            minimum: p.float("minimum"),
            clamp_min: p.bool("clamp_min"),
            maximum: p.float("maximum"),
            clamp_max: p.bool("clamp_max"),
        }),
        COLORIZE_LAYER => Ok(Modifier::Identity),
        _ => match values.get("shape_knots") {
            // No stored shape, or the two knots (0,0) and (1,1): the straight ramp.
            None => Ok(Modifier::Identity),
            Some(Value::Vec3List(k))
                if k.is_empty()
                    || (k.len() == 2 && k[0][..2] == [0.0, 0.0] && k[1][..2] == [1.0, 1.0]) =>
            {
                Ok(Modifier::Identity)
            }
            Some(_) => Err(FieldError::Unsupported("curve layer shape")),
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modifiers_follow_the_observed_laws() {
        let remap = Modifier::Remap { min: 0.2, max: 0.8 };
        assert!((remap.apply(0.0) - 0.2).abs() < 1e-15);
        assert!((remap.apply(1.0) - 0.8).abs() < 1e-15);
        assert_eq!(Modifier::Invert.apply(0.25), 0.75);
        let clamp = Modifier::Clamp {
            minimum: 0.0,
            clamp_min: true,
            maximum: 100.0,
            clamp_max: true,
        };
        assert_eq!(clamp.apply(0.37), 0.37);
        assert_eq!(clamp.apply(-1.0), 0.0);
        assert_eq!(Modifier::Identity.apply(0.37), 0.37);
    }
}
