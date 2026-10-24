//! forge-modifiers: the deformer node types and their evaluation
//! (docs/specs/MODIFIERS.md). Deformers are pure functions on points in the
//! deformer's local space; `apply_deformer` handles the scene around them.

pub mod bend;
pub mod bulge;
pub mod common;
pub mod shear;
pub mod spherify;
pub mod taper;
pub mod twist;

use forge_core::{
    Category, NodeId, NodeType, ParamDesc, Plugin, Registry, RegistryError, Scene, Unit,
};
use glam::{DMat4, DVec3};

use crate::bend::bend;
use crate::bulge::bulge;
use crate::common::{
    BendParams, BulgeParams, DEFAULT_SIZE, MODE_OPTIONS, ShearParams, SpherifyParams, TaperParams,
    TwistParams, read_bool, read_enum, read_float, read_vec3,
};
use crate::shear::shear;
use crate::spherify::spherify;
use crate::taper::taper;
use crate::twist::twist;

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
            .param(enabled_param())
    }

    /// Twist: the box and the total `angle`.
    pub fn twist_type() -> NodeType {
        NodeType::new(TWIST, "Twist", Category::Deformer)
            .param(box_param())
            .param(mode_param())
            .param(ParamDesc::float("angle", "Angle", 0.0).unit(Unit::Angle))
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
            .param(enabled_param())
    }
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
}

/// Applies one deformer node to `points` expressed in `target`'s local space
/// (MODIFIERS.md §0): the points are taken into the deformer's space with the inverse
/// of the deformer's transform relative to `target`, deformed, and put back.
///
/// Does nothing for a disabled deformer, a missing node or an unknown type.
pub fn apply_deformer(scene: &Scene, deformer: NodeId, target: NodeId, points: &mut [DVec3]) {
    let (Some(node), Some(ty)) = (scene.node(deformer), scene.node_type(deformer)) else {
        return;
    };
    if ty.category != Category::Deformer || !is_enabled(ty, node) {
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
    }
    if to_target != DMat4::IDENTITY {
        for p in points.iter_mut() {
            *p = to_target.transform_point3(*p);
        }
    }
}
