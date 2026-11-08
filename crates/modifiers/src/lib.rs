//! forge-modifiers: the deformer node types and their evaluation
//! (docs/specs/MODIFIERS.md). Deformers are pure functions on points in the
//! deformer's local space; `apply_deformer` handles the scene around them.

pub mod bend;
pub mod bulge;
pub mod cage;
pub mod camera;
pub mod collision;
pub mod common;
pub mod correction;
pub mod delta_mush;
pub mod explosion;
pub mod ffd;
pub mod formula;
pub mod helper;
pub mod jiggle;
pub mod melt;
pub mod shear;
pub mod spherify;
pub mod spline;
pub mod squash;
pub mod surface;
pub mod taper;
pub mod twist;
pub mod wind;
pub mod wrap;

use forge_core::{
    Category, EvalCtx, NodeId, NodeType, ParamDesc, Plugin, Registry, RegistryError, Scene, Unit,
    Value,
};
use forge_fields::FieldList;
use forge_geometry::PolyMesh;
use glam::{DMat4, DVec3};

use crate::bend::bend;
use crate::bulge::bulge;
use crate::common::{
    BendParams, BulgeParams, DEFAULT_SIZE, MODE_OPTIONS, ShearParams, SpherifyParams, SquashParams,
    TaperParams, TwistParams, WrapParams, read_bool, read_enum, read_float, read_int, read_string,
    read_vec3,
};
use crate::explosion::{BlastParams, Kind, blast};
use crate::ffd::{FfdParams, ffd};
use crate::formula::{Effect, FormulaParams, formula};
pub use crate::helper::Resolver;
use crate::melt::{MeltParams, melt};
use crate::shear::shear;
use crate::spherify::spherify;
use crate::squash::squash_stretch;
use crate::taper::taper;
use crate::twist::twist;
use crate::wind::{WindParams, wind};
use crate::wrap::wrap;

/// Type id of the Bend deformer (MODIFIERS.md §1).
pub const BEND: &str = "forge.bend";
/// Type id of the Twist deformer (MODIFIERS.md §2).
pub const TWIST: &str = "forge.twist";
/// Type id of the Taper deformer (MODIFIERS.md §3).
pub const TAPER: &str = "forge.taper";
/// Type id of the Bulge deformer (MODIFIERS.md §4).
pub const BULGE: &str = "forge.bulge";
/// Type id of the Shear deformer (MODIFIERS.md §5).
pub const SHEAR: &str = "forge.shear";
/// Type id of the Spherify deformer (MODIFIERS.md §6).
pub const SPHERIFY: &str = "forge.spherify";
/// Type id of the Squash & Stretch deformer (MODIFIERS.md §7.1).
pub const SQUASH_STRETCH: &str = "forge.squash_stretch";
/// Type id of the Wrap deformer (MODIFIERS.md §7.2).
pub const WRAP: &str = "forge.wrap";

/// Type id of the Explosion deformer (MODIFIERS.md §11).
pub const EXPLOSION: &str = "forge.explosion";
/// Type id of the Shatter deformer (MODIFIERS.md §11).
pub const SHATTER: &str = "forge.shatter";
/// Type id of the Wind deformer (MODIFIERS.md §12).
pub const WIND: &str = "forge.wind";

/// Type id of the FFD deformer (MODIFIERS.md §16).
pub const FFD: &str = "forge.ffd";
/// Type id of the Formula deformer (MODIFIERS.md §17).
pub const FORMULA: &str = "forge.formula";
/// Type id of the Melt deformer (MODIFIERS.md §18).
pub const MELT: &str = "forge.melt";

/// Type id of the Mesh (cage) deformer (MODIFIERS.md §14.1).
pub const MESH_DEFORMER: &str = "forge.mesh_deformer";
/// Type id of the Spline Wrap deformer (MODIFIERS.md §20.1).
pub const SPLINE_WRAP: &str = "forge.spline_wrap";
/// Type id of the Spline deformer (MODIFIERS.md §20.2).
pub const SPLINE_DEFORMER: &str = "forge.spline_deformer";
/// Type id of the Spline Rail deformer (MODIFIERS.md §20.3).
pub const SPLINE_RAIL: &str = "forge.spline_rail";
/// Type id of the Surface deformer (MODIFIERS.md §14.2).
pub const SURFACE_DEFORMER: &str = "forge.surface_deformer";
/// Type id of the Camera deformer (MODIFIERS.md §14.3).
pub const CAMERA_DEFORMER: &str = "forge.camera_deformer";
/// Type id of the Collision deformer (MODIFIERS.md §14.4).
pub const COLLISION: &str = "forge.collision";
/// Type id of the Jiggle deformer (MODIFIERS.md §14.5).
pub const JIGGLE: &str = "forge.jiggle";
/// Type id of the Delta Mush deformer (MODIFIERS.md §14.6).
pub const DELTA_MUSH: &str = "forge.delta_mush";
/// Type id of the Correction deformer (MODIFIERS.md §14.7).
pub const CORRECTION: &str = "forge.correction";

/// The deformer node types.
pub struct Deformers;

impl Deformers {
    /// Bend: the box and `strength`, `angle`, `keep_y_axis_length`.
    pub fn bend_type() -> NodeType {
        NodeType::new(BEND, "Bend", Category::Deformer)
            .param(box_param())
            .param(mode_param())
            .param(ParamDesc::float("strength", "Strength", 0.0).unit(Unit::Angle))
            .param(ParamDesc::float("angle", "Angle", 0.0).unit(Unit::Angle))
            .param(ParamDesc::bool(
                "keep_y_axis_length",
                "Keep Y-Axis Length",
                false,
            ))
            .param(fields_param())
            .param(enabled_param())
    }

    /// Twist: the box and the total `angle`.
    pub fn twist_type() -> NodeType {
        NodeType::new(TWIST, "Twist", Category::Deformer)
            .param(box_param())
            .param(mode_param())
            .param(ParamDesc::float("angle", "Angle", 0.0).unit(Unit::Angle))
            .param(fields_param())
            .param(enabled_param())
    }

    /// Taper: the box and `strength`, `curvature`, `fillet` (MODIFIERS.md §3).
    pub fn taper_type() -> NodeType {
        NodeType::new(TAPER, "Taper", Category::Deformer)
            .param(box_param())
            .param(mode_param())
            .param(strength_param())
            .param(curvature_param())
            .param(fillet_param())
            .param(fields_param())
            .param(enabled_param())
    }

    /// Bulge: the same parameters as Taper (MODIFIERS.md §4).
    pub fn bulge_type() -> NodeType {
        NodeType::new(BULGE, "Bulge", Category::Deformer)
            .param(box_param())
            .param(mode_param())
            .param(strength_param())
            .param(curvature_param())
            .param(fillet_param())
            .param(fields_param())
            .param(enabled_param())
    }

    /// Shear: Taper's parameters plus the `angle` of the offset direction
    /// (MODIFIERS.md §5).
    pub fn shear_type() -> NodeType {
        NodeType::new(SHEAR, "Shear", Category::Deformer)
            .param(box_param())
            .param(mode_param())
            .param(strength_param())
            .param(ParamDesc::float("angle", "Angle", 0.0).unit(Unit::Angle))
            .param(curvature_param())
            .param(fillet_param())
            .param(fields_param())
            .param(enabled_param())
    }

    /// Spherify: `radius` and `strength`; no box and no mode (MODIFIERS.md §6).
    pub fn spherify_type() -> NodeType {
        NodeType::new(SPHERIFY, "Spherify", Category::Deformer)
            .param(ParamDesc::float("radius", "Radius", 200.0).range(0.0, f64::MAX))
            .param(
                ParamDesc::float("strength", "Strength", 50.0)
                    .unit(Unit::Percent)
                    .range(0.0, 100.0),
            )
            .param(fields_param())
            .param(enabled_param())
    }

    /// Squash & Stretch: its own `top` / `bottom`, no box (MODIFIERS.md §7.1).
    pub fn squash_stretch_type() -> NodeType {
        let pct = |id: &str, label: &str, default: f64| {
            ParamDesc::float(id, label, default).unit(Unit::Percent)
        };
        NodeType::new(SQUASH_STRETCH, "Squash & Stretch", Category::Deformer)
            .param(ParamDesc::float("top", "Top", 100.0).range(0.0, f64::MAX))
            .param(pct("center", "Center", 0.0).range(-100.0, 100.0))
            .param(ParamDesc::float("bottom", "Bottom", -100.0).range(-f64::MAX, 0.0))
            .param(pct("aspect", "Aspect", 100.0))
            .param(pct("factor", "Factor", 100.0).range(0.0, f64::MAX))
            .param(pct("expand", "Expand", 100.0).range(0.0, f64::MAX))
            .param(pct("smooth_start", "Smooth Start", 0.0).range(0.0, 100.0))
            .param(pct("smooth_end", "Smooth End", 0.0).range(0.0, 100.0))
            .param(pct("curvature", "Curvature", 100.0).range(0.0, 100.0))
            .param(ParamDesc::enumeration(
                "curvature_type",
                "Type",
                "square",
                &[
                    ("square", "Square"),
                    ("cubic", "Cubic"),
                    ("quartic", "Quartic"),
                    ("custom", "Custom"),
                    ("spline", "Spline"),
                ],
            ))
            .param(ParamDesc::float("power", "Power", 2.0).range(0.0, f64::MAX))
            .param(fields_param())
            .param(enabled_param())
    }

    /// Wrap: `width` / `height` / `radius`, the angle spans, `movement`, `scale_z`,
    /// `tension` (MODIFIERS.md §7.2).
    pub fn wrap_type() -> NodeType {
        let len = |id: &str, label: &str, default: f64| {
            ParamDesc::float(id, label, default).range(0.0, f64::MAX)
        };
        let ang = |id: &str, label: &str, deg: f64| {
            ParamDesc::float(id, label, deg.to_radians()).unit(Unit::Angle)
        };
        let lat = |id: &str, label: &str, deg: f64| {
            ang(id, label, deg).range((-90f64).to_radians(), 90f64.to_radians())
        };
        NodeType::new(WRAP, "Wrap", Category::Deformer)
            .param(len("width", "Width", 400.0))
            .param(len("height", "Height", 400.0))
            .param(len("radius", "Radius", 200.0))
            .param(ParamDesc::enumeration(
                "wrap",
                "Wrap",
                "cylindrical",
                &[("spherical", "Spherical"), ("cylindrical", "Cylindrical")],
            ))
            .param(ang("longitude_start", "Longitude Start", 180.0))
            .param(ang("longitude_end", "Longitude End", 360.0))
            .param(lat("latitude_start", "Latitude Start", -45.0))
            .param(lat("latitude_end", "Latitude End", 45.0))
            .param(ParamDesc::float("movement", "Movement", 0.0))
            .param(
                ParamDesc::float("scale_z", "Scale Z", 100.0)
                    .unit(Unit::Percent)
                    .range(0.0, 10000.0),
            )
            .param(
                ParamDesc::float("tension", "Tension", 100.0)
                    .unit(Unit::Percent)
                    .range(0.0, 100.0),
            )
            .param(fields_param())
            .param(enabled_param())
    }

    fn blast_type(id: &str, label: &str, with_speed: bool) -> NodeType {
        let pct = |id: &str, label: &str, default: f64| {
            ParamDesc::float(id, label, default).unit(Unit::Percent)
        };
        let mut ty = NodeType::new(id, label, Category::Deformer)
            .param(pct("strength", "Strength", 0.0).range(0.0, 100.0));
        if with_speed {
            ty = ty.param(ParamDesc::float("speed", "Speed", 100.0));
        }
        ty.param(
            ParamDesc::float("angle_speed", "Angle Speed", 100f64.to_radians()).unit(Unit::Angle),
        )
        .param(pct("end_size", "End Size", 0.0).range(0.0, 1000.0))
        .param(pct("randomness", "Randomness", 100.0).range(0.0, 100.0))
        .param(enabled_param())
    }

    /// Explosion: splits the polygons and flies them apart (MODIFIERS.md §11).
    pub fn explosion_type() -> NodeType {
        Self::blast_type(EXPLOSION, "Explosion", true)
    }

    /// Shatter: splits the polygons and tumbles them in place (MODIFIERS.md §11).
    pub fn shatter_type() -> NodeType {
        Self::blast_type(SHATTER, "Shatter", false)
    }

    /// Wind: a time-dependent noise wave along z (MODIFIERS.md §12).
    pub fn wind_type() -> NodeType {
        let len = |id: &str, label: &str, default: f64| {
            ParamDesc::float(id, label, default).range(0.0, f64::MAX)
        };
        NodeType::new(WIND, "Wind", Category::Deformer)
            .param(len("amplitude", "Amplitude", 50.0))
            .param(len("size", "Size", 100.0))
            .param(len("frequency", "Frequency", 1.0))
            .param(
                ParamDesc::float("turbulence", "Turbulence", 50.0)
                    .unit(Unit::Percent)
                    .range(0.0, 100.0),
            )
            .param(len("fx", "fx", 1.0))
            .param(len("fy", "fy", 1.0))
            .param(ParamDesc::bool("flag", "Flag", true))
            .param(fields_param())
            .param(enabled_param())
    }

    /// FFD: a Bézier volume over a lattice (MODIFIERS.md §16).
    pub fn ffd_type() -> NodeType {
        let points = |id: &str, label: &str| ParamDesc::int(id, label, 3).range(2, 200);
        NodeType::new(FFD, "FFD", Category::Deformer)
            .param(ParamDesc::vec3("grid_size", "Grid Size", [300.0; 3]))
            .param(points("grid_points_x", "Grid Points X"))
            .param(points("grid_points_y", "Grid Points Y"))
            .param(points("grid_points_z", "Grid Points Z"))
            .param(ParamDesc::vec3_list("grid", "Lattice points").hidden())
            .param(fields_param())
            .param(enabled_param())
    }

    /// Formula: text formulas for the displacement and the point (MODIFIERS.md §17).
    pub fn formula_type() -> NodeType {
        NodeType::new(FORMULA, "Formula", Category::Deformer)
            .param(ParamDesc::vec3("size", "Size", [400.0; 3]).range(0.0, f64::MAX))
            .param(ParamDesc::enumeration(
                "effect",
                "Effect",
                "y_radial",
                &[
                    ("manual", "Manual"),
                    ("spherical", "Spherical"),
                    ("cylindrical", "Cylindrical"),
                    ("x_radial", "X Radial"),
                    ("y_radial", "Y Radial"),
                    ("z_radial", "Z Radial"),
                ],
            ))
            .param(ParamDesc::string(
                "formula_d",
                "d(u,v,x,y,z,t)",
                "Sin((u+t)*2.0*PI)*0.2",
            ))
            .param(ParamDesc::string("formula_x", "X(x,y,z,t)", "x"))
            .param(ParamDesc::string("formula_y", "Y(x,y,z,t)", "y"))
            .param(ParamDesc::string("formula_z", "Z(x,y,z,t)", "z"))
            .param(fields_param())
            .param(enabled_param())
    }

    /// Melt: spreads the points out and collapses their height (MODIFIERS.md §18).
    pub fn melt_type() -> NodeType {
        NodeType::new(MELT, "Melt", Category::Deformer)
            .param(percent("strength", "Strength", 25.0).range(0.0, 100.0))
            .param(ParamDesc::float("radius", "Radius", 100.0).range(0.0, f64::MAX))
            .param(percent("vertical_randomness", "Vertical Randomness", 100.0).range(0.0, 100.0))
            .param(percent("radial_randomness", "Radial Randomness", 100.0).range(0.0, 100.0))
            .param(percent("melted_size", "Melted Size", 400.0).range(100.0, 1000.0))
            .param(percent("noise_scale", "Noise Scale", 10.0).range(0.0, f64::MAX))
            .param(fields_param())
            .param(enabled_param())
    }

    /// Spline Wrap: lays the object along a linked spline (MODIFIERS.md §20.1).
    pub fn spline_wrap_type() -> NodeType {
        NodeType::new(SPLINE_WRAP, "Spline Wrap", Category::Deformer)
            .param(ParamDesc::link("spline", "Spline", SPLINE_OBJECTS))
            .param(ParamDesc::link("rail", "Rail", SPLINE_OBJECTS))
            .param(ParamDesc::enumeration(
                "axis",
                "Axis",
                "pos_x",
                &[
                    ("pos_x", "+X"),
                    ("neg_x", "-X"),
                    ("pos_y", "+Y"),
                    ("neg_y", "-Y"),
                    ("pos_z", "+Z"),
                    ("neg_z", "-Z"),
                ],
            ))
            .param(percent("strength", "Strength", 100.0).range(0.0, f64::MAX))
            .param(percent("offset", "Offset", 0.0))
            .param(percent("from", "From", 0.0))
            .param(percent("to", "To", 100.0))
            .param(ParamDesc::enumeration(
                "mode",
                "Mode",
                "fit_spline",
                &[("fit_spline", "Fit Spline"), ("keep_length", "Keep Length")],
            ))
            .param(ParamDesc::enumeration(
                "end_mode",
                "End Mode",
                "extend",
                &[("clamp", "Clamp"), ("extend", "Extend")],
            ))
            .param(ParamDesc::bool("scale_from_rail", "Scale from Rail", false))
            .param(percent("size_strength", "Size Strength", 100.0))
            .param(percent(
                "spline_size_strength",
                "Spline Size Strength",
                100.0,
            ))
            .param(ParamDesc::bool(
                "rotation_from_rail",
                "Rotation from Rail",
                true,
            ))
            .param(ParamDesc::vec3("up_vector", "Up Vector", [0.0, 0.0, 0.0]))
            .param(ParamDesc::float("banking", "Banking", 0.0).unit(Unit::Angle))
            .param(percent("rotation_strength", "Rotation Strength", 100.0))
            .param(percent(
                "spline_rotation_strength",
                "Spline Rotation Strength",
                100.0,
            ))
            .param(ParamDesc::bool(
                "fix_bounding_box",
                "Fix Bounding Box",
                false,
            ))
            .param(ParamDesc::vec3(
                "bounding_box_center",
                "Bounding Box Center",
                [0.0, 0.0, 0.0],
            ))
            .param(ParamDesc::vec3(
                "bounding_box_size",
                "Bounding Box Size",
                [200.0, 200.0, 200.0],
            ))
            .param(fields_param())
            .param(enabled_param())
    }

    /// Spline: follows the change from an original to a modifying spline (MODIFIERS.md §20.2).
    pub fn spline_deformer_type() -> NodeType {
        NodeType::new(SPLINE_DEFORMER, "Spline", Category::Deformer)
            .param(ParamDesc::link(
                "original_spline",
                "Original Spline",
                SPLINE_OBJECTS,
            ))
            .param(ParamDesc::link(
                "modifying_spline",
                "Modifying Spline",
                SPLINE_OBJECTS,
            ))
            .param(
                ParamDesc::int("b_spline_approximation", "B-Spline Approximation", 10)
                    .range(1, 100),
            )
            .param(
                ParamDesc::float("radius", "Radius", 30.0)
                    .range(0.0, f64::MAX)
                    .unit(Unit::Distance),
            )
            .param(ParamDesc::bool("use_length", "Use Length", false))
            .param(ParamDesc::bool("full_polygons", "Full Polygons", false))
            .param(fields_param())
            .param(enabled_param())
    }

    /// Spline Rail: spreads the object between two Z splines (MODIFIERS.md §20.3).
    pub fn spline_rail_type() -> NodeType {
        NodeType::new(SPLINE_RAIL, "Spline Rail", Category::Deformer)
            .param(ParamDesc::link(
                "z_spline_left",
                "Z Spline Left",
                SPLINE_OBJECTS,
            ))
            .param(ParamDesc::link(
                "z_spline_right",
                "Z Spline Right",
                SPLINE_OBJECTS,
            ))
            .param(ParamDesc::link(
                "x_spline_top",
                "X Spline Top",
                SPLINE_OBJECTS,
            ))
            .param(ParamDesc::link(
                "x_spline_bottom",
                "X Spline Bottom",
                SPLINE_OBJECTS,
            ))
            .param(ParamDesc::link("reference", "Reference", HELPER_OBJECTS))
            .param(ParamDesc::enumeration(
                "mode",
                "Mode",
                "unlimited",
                &[
                    ("limited", "Limited"),
                    ("within_box", "Within Box"),
                    ("unlimited", "Unlimited"),
                ],
            ))
            .param(box_param())
            .param(percent("scale_before_start", "Scale Before Start", 100.0))
            .param(percent("scale_after_end", "Scale After End", 100.0))
            .param(fields_param())
            .param(enabled_param())
    }

    /// Mesh (cage): mean value coordinates of the linked cages (MODIFIERS.md §14.1).
    pub fn mesh_deformer_type() -> NodeType {
        NodeType::new(MESH_DEFORMER, "Mesh", Category::Deformer)
            .param(ParamDesc::link_list("cages", "Cages", HELPER_OBJECTS))
            .param(percent("strength", "Strength", 100.0))
            .param(ParamDesc::vec3_list("bind", "Bind"))
            .param(fields_param())
            .param(enabled_param())
    }

    /// Surface: follows a linked surface, projection type (MODIFIERS.md §14.2).
    pub fn surface_deformer_type() -> NodeType {
        NodeType::new(SURFACE_DEFORMER, "Surface", Category::Deformer)
            .param(ParamDesc::link("surface", "Surface", HELPER_OBJECTS))
            .param(percent("strength", "Strength", 100.0))
            .param(ParamDesc::bool("proportional", "Proportional", false))
            .param(ParamDesc::vec3_list("bind", "Bind"))
            .param(fields_param())
            .param(enabled_param())
    }

    /// Camera: a grid in screen space seen from a linked camera (MODIFIERS.md §14.3).
    pub fn camera_deformer_type() -> NodeType {
        NodeType::new(CAMERA_DEFORMER, "Camera", Category::Deformer)
            .param(ParamDesc::link("camera", "Camera", HELPER_OBJECTS))
            .param(percent("strength", "Strength", 100.0))
            .param(ParamDesc::int("grid_x", "Grid X", 32).range(1, 256))
            .param(ParamDesc::int("grid_y", "Grid Y", 32).range(1, 256))
            .param(ParamDesc::vec3_list("offsets", "Grid offsets"))
            .param(fields_param())
            .param(enabled_param())
    }

    /// Collision: pushes points out of the linked colliders (MODIFIERS.md §14.4).
    pub fn collision_type() -> NodeType {
        NodeType::new(COLLISION, "Collision", Category::Deformer)
            .param(ParamDesc::link_list("colliders", "Objects", HELPER_OBJECTS))
            .param(ParamDesc::float("offset", "Offset", 0.0).range(0.0, f64::MAX))
            .param(percent("strength", "Strength", 100.0))
            .param(fields_param())
            .param(enabled_param())
    }

    /// Jiggle: spring simulation over the frames (MODIFIERS.md §14.5).
    pub fn jiggle_type() -> NodeType {
        NodeType::new(JIGGLE, "Jiggle", Category::Deformer)
            .param(percent("strength", "Strength", 100.0).range(0.0, f64::MAX))
            .param(percent("stiffness", "Stiffness", 40.0).range(0.0, 100.0))
            .param(percent("structural", "Structural", 100.0).range(0.0, 100.0))
            .param(percent("drag", "Drag", 10.0).range(0.0, 100.0))
            .param(percent("motion_scale", "Motion Scale", 100.0).range(0.0, f64::MAX))
            .param(ParamDesc::int("springs", "Springs", 4).range(0, 100))
            .param(ParamDesc::int("iterations", "Iterations", 4).range(1, 1000))
            .param(ParamDesc::float("gravity", "Gravity", 0.0))
            .param(ParamDesc::bool("local", "Local", false))
            .param(ParamDesc::int("cache", "Cache", 20).range(1, 1000))
            .param(fields_param())
            .param(enabled_param())
    }

    /// Delta Mush: smoothing plus stored deltas (MODIFIERS.md §14.6).
    pub fn delta_mush_type() -> NodeType {
        NodeType::new(DELTA_MUSH, "Delta Mush", Category::Deformer)
            .param(percent("strength", "Strength", 100.0))
            .param(ParamDesc::int("iterations", "Iterations", 10).range(0, 1000))
            .param(ParamDesc::bool(
                "pin_border_points",
                "Pin Border Points",
                false,
            ))
            .param(ParamDesc::vec3_list("bind", "Bind"))
            .param(fields_param())
            .param(enabled_param())
    }

    /// Correction: stored point offsets after the deformers above it (MODIFIERS.md §14.7).
    pub fn correction_type() -> NodeType {
        NodeType::new(CORRECTION, "Correction", Category::Deformer)
            .param(ParamDesc::enumeration(
                "mapping",
                "Mapping",
                "nearest",
                &[("uv", "UV"), ("nearest", "Nearest"), ("normals", "Normals")],
            ))
            .param(percent("strength", "Strength", 100.0))
            .param(ParamDesc::vec3_list("reference", "Reference"))
            .param(ParamDesc::vec3_list("offsets", "Offsets"))
            .param(fields_param())
            .param(enabled_param())
    }
}

/// Objects a helper link may point to (cages, surfaces, cameras, colliders).
const HELPER_OBJECTS: &[Category] = &[Category::Object, Category::Generator];

/// Objects a spline link may point to (splines are generators).
const SPLINE_OBJECTS: &[Category] = &[Category::Generator];

fn percent(id: &str, label: &str, default: f64) -> ParamDesc {
    ParamDesc::float(id, label, default).unit(Unit::Percent)
}

fn box_param() -> ParamDesc {
    ParamDesc::vec3("size", "Size", DEFAULT_SIZE).range(0.0, f64::MAX)
}

fn mode_param() -> ParamDesc {
    ParamDesc::enumeration("mode", "Mode", "limited", MODE_OPTIONS)
}

/// `strength` in percent as the app shows it (MODIFIERS.md §0); it has no range:
/// negative strength widens Taper (`taper/negative`) or pinches Bulge (`bulge/negative`).
fn strength_param() -> ParamDesc {
    ParamDesc::float("strength", "Strength", 0.0).unit(Unit::Percent)
}

/// `curvature` in percent: 100 is the pure quadratic profile, 0 the linear one.
fn curvature_param() -> ParamDesc {
    ParamDesc::float("curvature", "Curvature", 100.0)
        .unit(Unit::Percent)
        .range(0.0, f64::MAX)
}

/// The Fillet option: `smoothstep` instead of the curvature blend (MODIFIERS.md §3).
fn fillet_param() -> ParamDesc {
    ParamDesc::bool("fillet", "Fillet", false)
}

/// The deformer's field list (falloff, MODIFIERS.md §10): top to bottom, empty = full effect.
fn fields_param() -> ParamDesc {
    forge_fields::field_list_param("fields", "Fields").group("Falloff")
}

/// The object's green tick in the reference app, as a param so it survives
/// serialization (MODIFIERS.md §0).
fn enabled_param() -> ParamDesc {
    ParamDesc::bool("enabled", "Enabled", true)
}

impl Plugin for Deformers {
    fn id(&self) -> &str {
        "forge.deformers"
    }

    fn register(&self, r: &mut Registry) -> Result<(), RegistryError> {
        r.register(Deformers::bend_type())?;
        r.register(Deformers::twist_type())?;
        r.register(Deformers::taper_type())?;
        r.register(Deformers::bulge_type())?;
        r.register(Deformers::shear_type())?;
        r.register(Deformers::spherify_type())?;
        r.register(Deformers::squash_stretch_type())?;
        r.register(Deformers::wrap_type())?;
        r.register(Deformers::explosion_type())?;
        r.register(Deformers::shatter_type())?;
        r.register(Deformers::wind_type())?;
        r.register(Deformers::ffd_type())?;
        r.register(Deformers::formula_type())?;
        r.register(Deformers::melt_type())?;
        r.register(Deformers::spline_wrap_type())?;
        r.register(Deformers::spline_deformer_type())?;
        r.register(Deformers::spline_rail_type())?;
        r.register(Deformers::mesh_deformer_type())?;
        r.register(Deformers::surface_deformer_type())?;
        r.register(Deformers::camera_deformer_type())?;
        r.register(Deformers::collision_type())?;
        r.register(Deformers::jiggle_type())?;
        r.register(Deformers::delta_mush_type())?;
        r.register(Deformers::correction_type())?;
        Ok(())
    }
}

/// True for any deformer node in the scene.
pub fn is_deformer(scene: &Scene, id: NodeId) -> bool {
    scene
        .node_type(id)
        .is_some_and(|t| t.category == Category::Deformer)
}

/// A deformer's `enabled` param, defaulting to `true` (MODIFIERS.md §0).
fn is_enabled(ty: &NodeType, node: &forge_core::Node) -> bool {
    read_bool(ty, &node.params, "enabled")
}

/// What a deformer node does to a point list, read from its params.
enum Job {
    Bend(BendParams),
    Twist(TwistParams),
    Taper(TaperParams),
    Bulge(BulgeParams),
    Shear(ShearParams),
    Spherify(SpherifyParams),
    Squash(SquashParams),
    Wrap(WrapParams),
    Wind(WindParams),
    Ffd(FfdParams),
    Formula(FormulaParams),
    Melt(MeltParams),
}

/// Applies one deformer node to `points` expressed in `target`'s local space
/// (MODIFIERS.md §0): the points are taken into the deformer's space with the inverse
/// of the deformer's transform relative to `target`, deformed, and put back.
///
/// A non-empty `fields` list weights each point (MODIFIERS.md §10):
/// `p' = p + w·(deform(p) − p)` with `w` the list value at the point's world position
/// before the deformer. `ctx` (time, fps, seed; MODIFIERS.md §8) goes to the fields.
///
/// Does nothing for a disabled deformer, a missing node or an unknown type.
pub fn apply_deformer(
    scene: &Scene,
    deformer: NodeId,
    target: NodeId,
    points: &mut [DVec3],
    ctx: &EvalCtx,
) {
    let (Some(node), Some(ty)) = (scene.node(deformer), scene.node_type(deformer)) else {
        return;
    };
    if ty.category != Category::Deformer
        || !is_enabled(ty, node)
        || !forge_core::layers::deformers_on(scene, deformer)
    {
        return;
    }
    // Read the params before any point moves: an unknown type must not leave the mesh
    // in the deformer's space.
    let params = &node.params;
    let job = match node.type_id.as_str() {
        BEND => Job::Bend(BendParams {
            size: read_vec3(ty, params, "size"),
            mode: read_enum(ty, params, "mode"),
            strength: read_float(ty, params, "strength"),
            angle: read_float(ty, params, "angle"),
            keep_y_axis_length: read_bool(ty, params, "keep_y_axis_length"),
        }),
        TWIST => Job::Twist(TwistParams {
            size: read_vec3(ty, params, "size"),
            mode: read_enum(ty, params, "mode"),
            angle: read_float(ty, params, "angle"),
        }),
        TAPER => Job::Taper(TaperParams {
            size: read_vec3(ty, params, "size"),
            mode: read_enum(ty, params, "mode"),
            strength: read_float(ty, params, "strength"),
            curvature: read_float(ty, params, "curvature"),
            fillet: read_bool(ty, params, "fillet"),
        }),
        BULGE => Job::Bulge(BulgeParams {
            size: read_vec3(ty, params, "size"),
            mode: read_enum(ty, params, "mode"),
            strength: read_float(ty, params, "strength"),
            curvature: read_float(ty, params, "curvature"),
            fillet: read_bool(ty, params, "fillet"),
        }),
        SHEAR => Job::Shear(ShearParams {
            size: read_vec3(ty, params, "size"),
            mode: read_enum(ty, params, "mode"),
            strength: read_float(ty, params, "strength"),
            angle: read_float(ty, params, "angle"),
            curvature: read_float(ty, params, "curvature"),
            fillet: read_bool(ty, params, "fillet"),
        }),
        SPHERIFY => Job::Spherify(SpherifyParams {
            radius: read_float(ty, params, "radius"),
            strength: read_float(ty, params, "strength"),
        }),
        SQUASH_STRETCH => Job::Squash(SquashParams {
            top: read_float(ty, params, "top"),
            bottom: read_float(ty, params, "bottom"),
            center: read_float(ty, params, "center"),
            factor: read_float(ty, params, "factor"),
            expand: read_float(ty, params, "expand"),
            curvature: read_float(ty, params, "curvature"),
            curvature_type: read_enum(ty, params, "curvature_type"),
        }),
        WRAP => Job::Wrap(WrapParams {
            width: read_float(ty, params, "width"),
            height: read_float(ty, params, "height"),
            radius: read_float(ty, params, "radius"),
            wrap: read_enum(ty, params, "wrap"),
            longitude_start: read_float(ty, params, "longitude_start"),
            longitude_end: read_float(ty, params, "longitude_end"),
            latitude_start: read_float(ty, params, "latitude_start"),
            latitude_end: read_float(ty, params, "latitude_end"),
            movement: read_float(ty, params, "movement"),
            scale_z: read_float(ty, params, "scale_z"),
            tension: read_float(ty, params, "tension"),
        }),
        WIND => Job::Wind(WindParams {
            amplitude: read_float(ty, params, "amplitude"),
            size: read_float(ty, params, "size"),
            frequency: read_float(ty, params, "frequency"),
            turbulence: read_float(ty, params, "turbulence"),
            fx: read_float(ty, params, "fx"),
            fy: read_float(ty, params, "fy"),
            flag: read_bool(ty, params, "flag"),
        }),
        FFD => {
            let count = |id: &str| read_int(ty, params, id).clamp(2, 200) as usize;
            Job::Ffd(FfdParams {
                size: read_vec3(ty, params, "grid_size"),
                counts: [
                    count("grid_points_x"),
                    count("grid_points_y"),
                    count("grid_points_z"),
                ],
                grid: helper::vec3_list(scene, deformer, "grid"),
            })
        }
        FORMULA => Job::Formula(FormulaParams {
            size: read_vec3(ty, params, "size"),
            effect: Effect::from_id(&read_enum(ty, params, "effect")),
            d: read_string(ty, params, "formula_d"),
            x: read_string(ty, params, "formula_x"),
            y: read_string(ty, params, "formula_y"),
            z: read_string(ty, params, "formula_z"),
        }),
        MELT => Job::Melt(MeltParams {
            strength: read_float(ty, params, "strength"),
            radius: read_float(ty, params, "radius"),
            vertical_randomness: read_float(ty, params, "vertical_randomness"),
            radial_randomness: read_float(ty, params, "radial_randomness"),
            melted_size: read_float(ty, params, "melted_size"),
            noise_scale: read_float(ty, params, "noise_scale"),
        }),
        // Explosion and Shatter change the topology: `apply_deformer_mesh`.
        _ => return,
    };

    // `to_target` maps the deformer's space into `target`'s: world(deformer) with the
    // world of `target` taken out.
    let (Some(tw), Some(dw)) = (scene.world_matrix(target), scene.world_matrix(deformer)) else {
        return;
    };
    let to_target = tw.inverse() * dw;
    let to_deformer = to_target.inverse();
    if !to_deformer.is_finite() {
        return; // A degenerate transform (zero scale) would destroy the mesh.
    }
    // Falloff: the weights come from the points as they are now, in world space.
    let weights = match falloff(scene, deformer, tw, points, ctx) {
        Ok(w) => w,
        Err(()) => return, // A field that cannot be built leaves the points alone.
    };
    let before: Option<Vec<DVec3>> = weights.as_ref().map(|_| points.to_vec());
    if to_target != DMat4::IDENTITY {
        for p in points.iter_mut() {
            *p = to_deformer.transform_point3(*p);
        }
    }
    match job {
        Job::Bend(p) => bend(points, &p),
        Job::Twist(p) => twist(points, &p),
        Job::Taper(p) => taper(points, &p),
        Job::Bulge(p) => bulge(points, &p),
        Job::Shear(p) => shear(points, &p),
        Job::Spherify(p) => spherify(points, &p),
        Job::Squash(p) => squash_stretch(points, &p),
        Job::Wrap(p) => wrap(points, &p),
        Job::Wind(p) => wind(points, &p, ctx.time, ctx.seed),
        Job::Ffd(p) => ffd(points, &p),
        Job::Formula(p) => formula(points, &p, ctx.time),
        Job::Melt(p) => melt(points, &p, ctx.seed),
    }
    if to_target != DMat4::IDENTITY {
        for p in points.iter_mut() {
            *p = to_target.transform_point3(*p);
        }
    }
    if let (Some(w), Some(before)) = (weights, before) {
        for ((p, p0), w) in points.iter_mut().zip(before).zip(w) {
            *p = p0 + (*p - p0) * w;
        }
    }
}

/// [`apply_deformer`] for a whole mesh: Explosion and Shatter split the mesh into its
/// polygons (MODIFIERS.md §11), every other deformer moves the points only. The
/// helper-driven deformers (§14) see no linked objects here: see
/// [`apply_deformer_resolved`].
pub fn apply_deformer_mesh(
    scene: &Scene,
    deformer: NodeId,
    target: NodeId,
    mesh: &mut PolyMesh,
    ctx: &EvalCtx,
) {
    apply_deformer_resolved(scene, deformer, target, mesh, ctx, &|_| None)
}

/// [`apply_deformer_mesh`] with a `resolve` function that evaluates a linked helper object
/// (cage, surface, collider) to its local-space mesh; the helper-driven deformers of
/// MODIFIERS.md §14 need it.
pub fn apply_deformer_resolved(
    scene: &Scene,
    deformer: NodeId,
    target: NodeId,
    mesh: &mut PolyMesh,
    ctx: &EvalCtx,
    resolve: Resolver,
) {
    if scene
        .node(deformer)
        .is_some_and(|n| is_spline_type(&n.type_id))
    {
        return apply_spline(scene, deformer, target, mesh, ctx, resolve);
    }
    if scene
        .node(deformer)
        .is_some_and(|n| is_helper_type(&n.type_id))
    {
        return apply_helper(scene, deformer, target, mesh, ctx, resolve);
    }
    let kind = match scene.node(deformer).map(|n| n.type_id.as_str()) {
        Some(EXPLOSION) => Kind::Explosion,
        Some(SHATTER) => Kind::Shatter,
        _ => return apply_deformer(scene, deformer, target, &mut mesh.points, ctx),
    };
    let (Some(node), Some(ty)) = (scene.node(deformer), scene.node_type(deformer)) else {
        return;
    };
    if !is_enabled(ty, node) || !forge_core::layers::deformers_on(scene, deformer) {
        return;
    }
    let params = &node.params;
    let p = BlastParams {
        strength: read_float(ty, params, "strength"),
        speed: if kind == Kind::Explosion {
            read_float(ty, params, "speed")
        } else {
            0.0
        },
        angle_speed: read_float(ty, params, "angle_speed"),
        end_size: read_float(ty, params, "end_size"),
        randomness: read_float(ty, params, "randomness"),
    };
    let (Some(tw), Some(dw)) = (scene.world_matrix(target), scene.world_matrix(deformer)) else {
        return;
    };
    let to_target = tw.inverse() * dw;
    let to_deformer = to_target.inverse();
    if !to_deformer.is_finite() {
        return;
    }
    for q in mesh.points.iter_mut() {
        *q = to_deformer.transform_point3(*q);
    }
    blast(mesh, kind, &p, ctx.seed);
    for q in mesh.points.iter_mut() {
        *q = to_target.transform_point3(*q);
    }
}

fn is_spline_type(id: &str) -> bool {
    matches!(id, SPLINE_WRAP | SPLINE_DEFORMER | SPLINE_RAIL)
}

/// The polyline of the spline linked by `deformer`'s `key`, in `dw_inv`'s space (the
/// deformer's local space).
fn linked_polyline(
    scene: &Scene,
    deformer: NodeId,
    key: &str,
    dw_inv: DMat4,
    resolve: Resolver,
) -> Option<spline::Polyline> {
    let id = helper::link(scene, deformer, key)?;
    let mesh = resolve(id)?;
    let to_deformer = dw_inv * scene.world_matrix(id)?;
    spline::Polyline::from_mesh(&mesh, |p| to_deformer.transform_point3(p))
}

/// Runs Spline Wrap, Spline or Spline Rail on `mesh` (points in `target` space), working in
/// the deformer's local space (MODIFIERS.md §20).
fn apply_spline(
    scene: &Scene,
    deformer: NodeId,
    target: NodeId,
    mesh: &mut PolyMesh,
    ctx: &EvalCtx,
    resolve: Resolver,
) {
    let (Some(node), Some(ty)) = (scene.node(deformer), scene.node_type(deformer)) else {
        return;
    };
    if !is_enabled(ty, node) || !forge_core::layers::deformers_on(scene, deformer) {
        return;
    }
    let params = &node.params;
    let (Some(tw), Some(dw)) = (scene.world_matrix(target), scene.world_matrix(deformer)) else {
        return;
    };
    let (Some(dw_inv), Some(tw_inv)) = (helper::inverse(dw), helper::inverse(tw)) else {
        return;
    };
    let to_deformer = dw_inv * tw;
    let Some(back) = helper::inverse(to_deformer) else {
        return;
    };
    let before = mesh.points.clone();
    let mut pts: Vec<DVec3> = before
        .iter()
        .map(|p| to_deformer.transform_point3(*p))
        .collect();
    let line = |key: &str| linked_polyline(scene, deformer, key, dw_inv, resolve);
    match node.type_id.as_str() {
        SPLINE_WRAP => {
            let Some(path) = line("spline") else {
                return;
            };
            let axis = match read_enum(ty, params, "axis").as_str() {
                "neg_x" => (0, true),
                "pos_y" => (1, false),
                "neg_y" => (1, true),
                "pos_z" => (2, false),
                "neg_z" => (2, true),
                _ => (0, false),
            };
            let pct = |id: &str| read_float(ty, params, id) / 100.0;
            let fixed_box = read_bool(ty, params, "fix_bounding_box").then(|| {
                (
                    read_vec3(ty, params, "bounding_box_center"),
                    read_vec3(ty, params, "bounding_box_size"),
                )
            });
            let p = spline::WrapParams {
                axis: spline::WrapAxis {
                    index: axis.0,
                    negative: axis.1,
                },
                strength: pct("strength"),
                offset: pct("offset"),
                from: pct("from"),
                to: pct("to"),
                keep_length: read_enum(ty, params, "mode") == "keep_length",
                extend: read_enum(ty, params, "end_mode") != "clamp",
                rotate: read_bool(ty, params, "rotation_from_rail"),
                fixed_box,
            };
            spline::spline_wrap(&mut pts, &path, &p);
        }
        SPLINE_DEFORMER => {
            let (Some(original), Some(modifying)) =
                (line("original_spline"), line("modifying_spline"))
            else {
                return;
            };
            let p = spline::SplineDeformParams {
                radius: read_float(ty, params, "radius"),
            };
            spline::spline_deform(&mut pts, &original, &modifying, &p);
        }
        _ => {
            let (left, right) = (line("z_spline_left"), line("z_spline_right"));
            let mode = match read_enum(ty, params, "mode").as_str() {
                "limited" => spline::RailMode::Limited,
                "within_box" => spline::RailMode::WithinBox,
                _ => spline::RailMode::Unlimited,
            };
            let p = spline::RailParams {
                mode,
                size: read_vec3(ty, params, "size"),
                scale_before: read_float(ty, params, "scale_before_start") / 100.0,
                scale_after: read_float(ty, params, "scale_after_end") / 100.0,
            };
            spline::spline_rail(&mut pts, left.as_ref(), right.as_ref(), &p);
        }
    }
    let weights = falloff(scene, deformer, tw, &before, ctx).ok().flatten();
    for (i, (q, p0)) in pts.iter().zip(&before).enumerate() {
        let new = back.transform_point3(*q);
        let w = weights.as_ref().map_or(1.0, |w| w[i]);
        mesh.points[i] = *p0 + (new - *p0) * w;
    }
    let _ = tw_inv;
}

fn is_helper_type(id: &str) -> bool {
    matches!(
        id,
        MESH_DEFORMER
            | SURFACE_DEFORMER
            | CAMERA_DEFORMER
            | COLLISION
            | JIGGLE
            | DELTA_MUSH
            | CORRECTION
    )
}

/// The world points and triangles of the helper object linked by `node`.
fn helper_mesh(
    scene: &Scene,
    node: NodeId,
    resolve: Resolver,
) -> Option<(Vec<DVec3>, Vec<[u32; 3]>)> {
    let mesh = resolve(node)?;
    let world = helper::world_points(scene, node, &mesh)?;
    Some((world, helper::triangles(&mesh)))
}

/// Runs one of the deformers of MODIFIERS.md §14 on `mesh` (points in `target` space).
fn apply_helper(
    scene: &Scene,
    deformer: NodeId,
    target: NodeId,
    mesh: &mut PolyMesh,
    ctx: &EvalCtx,
    resolve: Resolver,
) {
    let (Some(node), Some(ty)) = (scene.node(deformer), scene.node_type(deformer)) else {
        return;
    };
    if !is_enabled(ty, node) || !forge_core::layers::deformers_on(scene, deformer) {
        return;
    }
    let params = &node.params;
    let Some(tw) = scene.world_matrix(target) else {
        return;
    };
    let Some(tw_inv) = helper::inverse(tw) else {
        return;
    };
    let strength = read_float(ty, params, "strength") / 100.0;
    let before = mesh.points.clone();
    // Deformers working in the target's own space; the others in world space.
    let local = matches!(node.type_id.as_str(), DELTA_MUSH | CORRECTION);
    let mut pts: Vec<DVec3> = if local {
        before.clone()
    } else {
        before.iter().map(|p| tw.transform_point3(*p)).collect()
    };
    match node.type_id.as_str() {
        MESH_DEFORMER => {
            for cage in helper::link_list(scene, deformer, "cages") {
                let Some((current, tris)) = helper_mesh(scene, cage, resolve) else {
                    continue;
                };
                let stored = helper::vec3_list(scene, deformer, "bind");
                let bind = if stored.is_empty() {
                    current.clone()
                } else {
                    stored
                };
                cage::cage_deform(&mut pts, &bind, &current, &tris, strength);
            }
        }
        SURFACE_DEFORMER => {
            if let Some((current, tris)) = helper::link(scene, deformer, "surface")
                .and_then(|s| helper_mesh(scene, s, resolve))
            {
                let stored = helper::vec3_list(scene, deformer, "bind");
                let bind = if stored.is_empty() {
                    current.clone()
                } else {
                    stored
                };
                let proportional = read_bool(ty, params, "proportional");
                surface::surface_deform(&mut pts, &bind, &current, &tris, strength, proportional);
            }
        }
        CAMERA_DEFORMER => {
            if let Some(cam) =
                helper::link(scene, deformer, "camera").and_then(|c| scene.world_matrix(c))
            {
                let grid = |id: &str| match scene.param(deformer, id) {
                    Some(Value::Int(i)) => (*i).max(1) as usize,
                    _ => 32,
                };
                let offsets = helper::vec3_list(scene, deformer, "offsets");
                camera::camera_deform(
                    &mut pts,
                    cam,
                    &offsets,
                    (grid("grid_x"), grid("grid_y")),
                    strength,
                );
            }
        }
        COLLISION => {
            let offset = read_float(ty, params, "offset");
            for c in helper::link_list(scene, deformer, "colliders") {
                if let Some((cp, tris)) = helper_mesh(scene, c, resolve) {
                    collision::collide(&mut pts, &cp, &tris, offset, strength);
                }
            }
        }
        JIGGLE => {
            let int = |id: &str| match scene.param(deformer, id) {
                Some(Value::Int(i)) => (*i).max(0) as usize,
                _ => 0,
            };
            let p = jiggle::JiggleParams {
                strength: read_float(ty, params, "strength"),
                stiffness: read_float(ty, params, "stiffness"),
                structural: read_float(ty, params, "structural"),
                drag: read_float(ty, params, "drag"),
                motion_scale: read_float(ty, params, "motion_scale"),
                springs: int("springs"),
                iterations: int("iterations").max(1),
                gravity: read_float(ty, params, "gravity"),
                local: read_bool(ty, params, "local"),
                fps: ctx.fps,
            };
            let frame = (ctx.time * ctx.fps).round() as i64;
            let start = (frame - int("cache") as i64).max(0);
            let worlds: Vec<DMat4> = (start..=frame)
                .map(|k| {
                    if k == frame {
                        tw
                    } else {
                        forge_anim::scene_at(scene, k as f64 / ctx.fps)
                            .world_matrix(target)
                            .unwrap_or(tw)
                    }
                })
                .collect();
            // `pts` are world points of this frame; the simulation wants target space.
            let nb = helper::neighbours(mesh);
            let out = jiggle::jiggle(&before, &nb, &worlds, &p);
            pts = out.iter().map(|q| tw.transform_point3(*q)).collect();
        }
        DELTA_MUSH => {
            let stored = helper::vec3_list(scene, deformer, "bind");
            let bind = if stored.is_empty() {
                before.clone()
            } else {
                stored
            };
            let int = match scene.param(deformer, "iterations") {
                Some(Value::Int(i)) => (*i).max(0) as usize,
                _ => 10,
            };
            let p = delta_mush::DeltaMushParams {
                strength,
                iterations: int,
                pin_border: read_bool(ty, params, "pin_border_points"),
            };
            let mut m = PolyMesh {
                points: pts.clone(),
                polys: mesh.polys.clone(),
            };
            delta_mush::delta_mush(&mut m, &bind, &p);
            pts = m.points;
        }
        CORRECTION => {
            let mapping = match read_enum(ty, params, "mapping").as_str() {
                "uv" => correction::Mapping::Uv,
                _ => correction::Mapping::Nearest,
            };
            let reference = helper::vec3_list(scene, deformer, "reference");
            let offsets = helper::vec3_list(scene, deformer, "offsets");
            correction::correct(&mut pts, &reference, &offsets, mapping, strength);
        }
        _ => return,
    }
    let weights = falloff(scene, deformer, tw, &before, ctx).ok().flatten();
    for (i, (q, p0)) in pts.iter().zip(&before).enumerate() {
        let new = if local {
            *q
        } else {
            tw_inv.transform_point3(*q)
        };
        let w = weights.as_ref().map_or(1.0, |w| w[i]);
        mesh.points[i] = *p0 + (new - *p0) * w;
    }
}

/// The weights of `points` (in `target`'s space, `target_world` its world matrix) from the
/// deformer's `fields` list: `Ok(None)` for an empty list (full effect), `Err` when a field
/// cannot be built.
fn falloff(
    scene: &Scene,
    deformer: NodeId,
    target_world: DMat4,
    points: &[DVec3],
    ctx: &EvalCtx,
) -> Result<Option<Vec<f64>>, ()> {
    let list = FieldList::from_param(scene, deformer, "fields").map_err(|_| ())?;
    if list.is_empty() {
        return Ok(None);
    }
    let world: Vec<DVec3> = points
        .iter()
        .map(|p| target_world.transform_point3(*p))
        .collect();
    Ok(Some(
        list.sample_many(&world, ctx)
            .into_iter()
            .map(|s| s.value)
            .collect(),
    ))
}
