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

impl Placement {
    /// The `t` a point deforms with, or `None` when it keeps its place. `Carry` is
    /// `t = 1`: the deformers that only scale or offset (Taper, Bulge, Shear) hold the
    /// value they have at the top of the box above it (MODIFIERS.md §0).
    pub(crate) fn t(self) -> Option<f64> {
        match self {
            Placement::Unchanged => None,
            Placement::Deform(t) => Some(t),
            Placement::Carry => Some(1.0),
        }
    }
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

/// `smoothstep(x) = 3x² − 2x³`, the profile of the Fillet option (MODIFIERS.md §3, §4).
pub(crate) fn smoothstep(x: f64) -> f64 {
    x * x * (3.0 - 2.0 * x)
}

/// Taper's and Shear's curve over the box (MODIFIERS.md §3, §5): `curvature` (a
/// fraction, 1 = 100 %) blends the linear profile with the quadratic one, `fillet`
/// replaces the blend with `smoothstep`.
pub(crate) fn taper_curve(t: f64, curvature: f64, fillet: bool) -> f64 {
    if fillet {
        smoothstep(t)
    } else {
        curvature * t * t + (1.0 - curvature) * t
    }
}

/// Bulge's curve (MODIFIERS.md §4), centred on the box: `u` is −1 at the bottom and +1
/// at the top, and `g(±1) = 0`, so the bulge vanishes at the ends of the box.
pub(crate) fn bulge_curve(u: f64, curvature: f64, fillet: bool) -> f64 {
    let (u2, a) = (u * u, u.abs());
    if fillet {
        smoothstep(1.0 - a)
    } else {
        curvature * (1.0 - u2) + (1.0 - curvature) * (1.0 - a)
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

/// Parameters of `forge.taper` (MODIFIERS.md §3). `strength` and `curvature` are
/// **percent as the app shows them** (50 = 50 %), so `curvature` defaults to 100.
#[derive(Debug, Clone, PartialEq)]
pub struct TaperParams {
    pub size: DVec3,
    pub mode: String,
    pub strength: f64,
    pub curvature: f64,
    pub fillet: bool,
}

impl Default for TaperParams {
    fn default() -> Self {
        TaperParams {
            size: DVec3::from(DEFAULT_SIZE),
            mode: "limited".to_string(),
            strength: 0.0,
            curvature: 100.0,
            fillet: false,
        }
    }
}

/// Parameters of `forge.bulge` (MODIFIERS.md §4): the same as Taper's.
#[derive(Debug, Clone, PartialEq)]
pub struct BulgeParams {
    pub size: DVec3,
    pub mode: String,
    pub strength: f64,
    pub curvature: f64,
    pub fillet: bool,
}

impl Default for BulgeParams {
    fn default() -> Self {
        BulgeParams {
            size: DVec3::from(DEFAULT_SIZE),
            mode: "limited".to_string(),
            strength: 0.0,
            curvature: 100.0,
            fillet: false,
        }
    }
}

/// Parameters of `forge.shear` (MODIFIERS.md §5); `angle` is in **radians** and turns
/// the offset direction about Y.
#[derive(Debug, Clone, PartialEq)]
pub struct ShearParams {
    pub size: DVec3,
    pub mode: String,
    pub strength: f64,
    pub angle: f64,
    pub curvature: f64,
    pub fillet: bool,
}

impl Default for ShearParams {
    fn default() -> Self {
        ShearParams {
            size: DVec3::from(DEFAULT_SIZE),
            mode: "limited".to_string(),
            strength: 0.0,
            angle: 0.0,
            curvature: 100.0,
            fillet: false,
        }
    }
}

/// Parameters of `forge.spherify` (MODIFIERS.md §6): no box and no mode — every point
/// is treated. `strength` is percent as the app shows it.
#[derive(Debug, Clone, PartialEq)]
pub struct SpherifyParams {
    pub radius: f64,
    pub strength: f64,
}

impl Default for SpherifyParams {
    fn default() -> Self {
        SpherifyParams {
            radius: 200.0,
            strength: 50.0,
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

    #[test]
    fn carry_is_the_value_at_the_top_of_the_box() {
        assert_eq!(Placement::Carry.t(), Some(1.0));
        assert_eq!(Placement::Deform(0.25).t(), Some(0.25));
        assert_eq!(Placement::Unchanged.t(), None);
    }

    fn close(a: f64, b: f64) {
        assert!((a - b).abs() < 1e-12, "{a} != {b}");
    }

    #[test]
    fn the_curvature_blend_runs_from_linear_to_quadratic() {
        close(taper_curve(0.5, 0.0, false), 0.5);
        close(taper_curve(0.5, 1.0, false), 0.25);
        // `taper/curvature` (c = 1/2) and `taper/fillet`.
        close(taper_curve(0.1, 0.5, false), 0.055);
        close(taper_curve(0.1, 1.0, true), 0.028);
        close(taper_curve(1.0, 0.5, false), 1.0);
        close(taper_curve(1.0, 0.5, true), 1.0);
    }

    #[test]
    fn the_bulge_curve_vanishes_at_the_box_ends_and_is_symmetric() {
        close(bulge_curve(1.0, 1.0, false), 0.0);
        close(bulge_curve(-1.0, 1.0, false), 0.0);
        close(bulge_curve(0.0, 1.0, false), 1.0);
        close(bulge_curve(0.8, 1.0, false), bulge_curve(-0.8, 1.0, false));
        // `bulge/strength`, `bulge/curvature` and `bulge/fillet` at u = −0.8.
        close(bulge_curve(-0.8, 1.0, false), 0.36);
        close(bulge_curve(-0.8, 0.5, false), 0.28);
        close(bulge_curve(-0.8, 1.0, true), 0.104);
        // `bulge/mode-unlimited` extrapolates u to −1.6.
        close(bulge_curve(-1.6, 1.0, false), -1.56);
    }
}
