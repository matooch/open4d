//! forge-modifiers: the deformer node types and their evaluation
//! (docs/specs/MODIFIERS.md). Deformers are pure functions on points in the
//! deformer's local space; `apply_deformer` handles the scene around them.

pub mod bend;
pub mod bulge;
pub mod common;
pub mod shear;
pub mod spherify;
pub mod squash;
pub mod taper;
pub mod twist;
pub mod wrap;

use forge_core::{
    Category, EvalCtx, NodeId, NodeType, ParamDesc, Plugin, Registry, RegistryError, Scene, Unit,
    Value,
};
use forge_fields::FieldList;
use glam::{DMat4, DVec3};

use crate::bend::bend;
use crate::bulge::bulge;
use crate::common::{
    BendParams, BulgeParams, DEFAULT_SIZE, MODE_OPTIONS, ShearParams, SpherifyParams, SquashParams,
    TaperParams, TwistParams, WrapParams, read_bool, read_enum, read_float, read_vec3,
};
use crate::shear::shear;
use crate::spherify::spherify;
use crate::squash::squash_stretch;
use crate::taper::taper;
use crate::twist::twist;
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
    ParamDesc::link_list("fields", "Fields", &[Category::Field]).group("Falloff")
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
    let Some(Value::LinkList(links)) = scene.param(deformer, "fields") else {
        return Ok(None);
    };
    // A link to a deleted or non-field node is skipped, like an empty slot.
    let ids: Vec<NodeId> = links
        .iter()
        .copied()
        .filter(|f| {
            scene
                .node_type(*f)
                .is_some_and(|t| t.category == Category::Field)
        })
        .collect();
    let list = FieldList::from_nodes(scene, &ids).map_err(|_| ())?;
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
