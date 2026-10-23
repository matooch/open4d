//! forge-modifiers: the deformer node types and their evaluation
//! (docs/specs/MODIFIERS.md). Deformers are pure functions on points in the
//! deformer's local space; `apply_deformer` handles the scene around them.

pub mod bend;
pub mod common;
pub mod twist;

use forge_core::{
    Category, NodeId, NodeType, ParamDesc, Plugin, Registry, RegistryError, Scene, Unit,
};
use glam::{DMat4, DVec3};

use crate::bend::bend;
use crate::common::{
    BendParams, DEFAULT_SIZE, MODE_OPTIONS, TwistParams, read_bool, read_enum, read_float,
    read_vec3,
};
use crate::twist::twist;

/// Type id of the Bend deformer (MODIFIERS.md §1).
pub const BEND: &str = "forge.bend";
/// Type id of the Twist deformer (MODIFIERS.md §2).
pub const TWIST: &str = "forge.twist";

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
}

fn box_param() -> ParamDesc {
    ParamDesc::vec3("size", "Size", DEFAULT_SIZE).range(0.0, f64::MAX)
}

fn mode_param() -> ParamDesc {
    ParamDesc::enumeration("mode", "Mode", "limited", MODE_OPTIONS)
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
    }
    if to_target != DMat4::IDENTITY {
        for p in points.iter_mut() {
            *p = to_target.transform_point3(*p);
        }
    }
}
