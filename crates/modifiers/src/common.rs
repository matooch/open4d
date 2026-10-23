//! The pieces every deformer shares (docs/specs/MODIFIERS.md §0): the box, the Mode
//! rules and the parameter structs of the node types.

use std::collections::BTreeMap;

use forge_core::{NodeType, Value};
use glam::DVec3;

/// The `Mode` options every deformer offers (docs/parity/{bend,twist}.md).
pub const MODE_OPTIONS: &[(&str, &str)] = &[
    ("limited", "Limited"),
    ("within_box", "Within Box"),
    ("unlimited", "Unlimited"),
];

/// Default `size` of the box, in cm.
pub const DEFAULT_SIZE: [f64; 3] = [250.0, 250.0, 250.0];

/// What a point does with the deformation, decided by `Mode` from the point's y
/// position in the deformer box (MODIFIERS.md §0). The gate is y only: an object
/// wider than the box still deforms (`deformer-common/on-plane`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Placement {
    /// Below/above the box in `limited` and `within_box`: the point keeps its place.
    Unchanged,
    /// Inside the box, or anywhere in `unlimited` (then `t` can leave 0…1).
    Deform(f64),
    /// Above the box in `limited`: deform as at the top, then carry the point along
    /// the tangent of the end of the arc (the bend's part above the box).
    Carry,
}

/// `t = (y + h/2) / h` and the mode rules; `h` is `size_y`.
pub(crate) fn placement(mode: &str, y: f64, h: f64) -> Placement {
    if h <= 0.0 {
        return Placement::Unchanged;
    }
    let (lo, hi) = (-h / 2.0, h / 2.0);
    let t = (y - lo) / h;
    match mode {
        "unlimited" => Placement::Deform(t),
        "within_box" if y < lo || y > hi => Placement::Unchanged,
        "within_box" => Placement::Deform(t),
        // `limited`, the default, and any unknown option.
        _ if y < lo => Placement::Unchanged,
        _ if y > hi => Placement::Carry,
        _ => Placement::Deform(t),
    }
}

/// Parameters of `forge.bend`; `strength` and `angle` are **radians** (case files hold
/// degrees, the registry stores angles as radians).
#[derive(Debug, Clone, PartialEq)]
pub struct BendParams {
    pub size: DVec3,
    pub mode: String,
    pub strength: f64,
    pub angle: f64,
    pub keep_y_axis_length: bool,
}

impl Default for BendParams {
    fn default() -> Self {
        BendParams {
            size: DVec3::from(DEFAULT_SIZE),
            mode: "limited".to_string(),
            strength: 0.0,
            angle: 0.0,
            keep_y_axis_length: false,
        }
    }
}

/// Parameters of `forge.twist`; `angle` is in **radians**.
#[derive(Debug, Clone, PartialEq)]
pub struct TwistParams {
    pub size: DVec3,
    pub mode: String,
    pub angle: f64,
}

impl Default for TwistParams {
    fn default() -> Self {
        TwistParams {
            size: DVec3::from(DEFAULT_SIZE),
            mode: "limited".to_string(),
            angle: 0.0,
        }
    }
}

/// Reads a param of a node: the validated value, or the descriptor's default when the
/// node never got one (every deformer type declares the params it reads).
pub(crate) fn read(ty: &NodeType, params: &BTreeMap<String, Value>, id: &str) -> Value {
    let desc = ty
        .find_param(id)
        .unwrap_or_else(|| panic!("{} reads its own param {id:?}", ty.id));
    params
        .get(id)
        .and_then(|v| desc.validate(v.clone()).ok())
        .unwrap_or_else(|| desc.default.clone())
}

pub(crate) fn read_vec3(ty: &NodeType, params: &BTreeMap<String, Value>, id: &str) -> DVec3 {
    match read(ty, params, id) {
        Value::Vec3(v) => DVec3::from_array(v),
        other => unreachable!("{id} is not a vec3: {other:?}"),
    }
}

pub(crate) fn read_float(ty: &NodeType, params: &BTreeMap<String, Value>, id: &str) -> f64 {
    match read(ty, params, id) {
        Value::Float(f) => f,
        other => unreachable!("{id} is not a float: {other:?}"),
    }
}

pub(crate) fn read_bool(ty: &NodeType, params: &BTreeMap<String, Value>, id: &str) -> bool {
    matches!(read(ty, params, id), Value::Bool(true))
}

pub(crate) fn read_enum(ty: &NodeType, params: &BTreeMap<String, Value>, id: &str) -> String {
    match read(ty, params, id) {
        Value::Enum(s) => s,
        other => unreachable!("{id} is not an enum: {other:?}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn limited_leaves_the_points_below_the_box_alone() {
        assert_eq!(placement("limited", -300.0, 250.0), Placement::Unchanged);
        assert_eq!(placement("limited", -125.0, 250.0), Placement::Deform(0.0));
        assert_eq!(placement("limited", 125.0, 250.0), Placement::Deform(1.0));
        assert_eq!(placement("limited", 200.0, 250.0), Placement::Carry);
    }

    #[test]
    fn within_box_only_deforms_inside_and_unlimited_extrapolates() {
        assert_eq!(placement("within_box", 200.0, 250.0), Placement::Unchanged);
        assert_eq!(placement("within_box", 0.0, 250.0), Placement::Deform(0.5));
        assert_eq!(
            placement("unlimited", -200.0, 250.0),
            Placement::Deform(-0.3)
        );
        assert_eq!(placement("unlimited", 200.0, 250.0), Placement::Deform(1.3));
    }

    #[test]
    fn a_zero_height_box_deforms_nothing() {
        assert_eq!(placement("limited", 0.0, 0.0), Placement::Unchanged);
        assert_eq!(placement("unlimited", 10.0, 0.0), Placement::Unchanged);
    }
}
