//! forge-rig: joints, joint tools, the weight tag, the skin deformer and auto weights
//! (docs/specs/RIG.md). Deformation is pure math on points; the scene enters through
//! `apply_skin` (called by `forge-modifiers`) and the tools in [`tools`].

pub mod auto_weights;
pub mod constraint;
pub mod display;
pub mod ik;
pub mod joint;
pub mod mirror;
pub mod paint;
pub mod pose_morph;
pub mod skin;
pub mod tags;
pub mod tools;
pub mod weights;

use forge_core::{Plugin, Registry, RegistryError};

pub use constraint::CONSTRAINT_TAG;
pub use ik::IK_TAG;
pub use joint::{Axis, JOINT};
pub use pose_morph::{POSE_MORPH_TAG, apply_pose_morph};
pub use skin::{SKIN, apply_skin};
pub use weights::{WEIGHT_TAG, WeightMap};

/// Node type id of the Protection tag.
pub const PROTECTION_TAG: &str = "forge.protection_tag";

/// True if `id` has an enabled Protection tag that locks `param`.
/// Recognised params: `transform.pos.x/y/z`.
pub fn is_protection_locked(
    scene: &forge_core::Scene,
    id: forge_core::NodeId,
    param: &str,
) -> bool {
    let node = match scene.node(id) {
        Some(n) => n,
        None => return false,
    };
    for tag in &node.tags {
        if !scene
            .node(*tag)
            .is_some_and(|n| n.type_id == PROTECTION_TAG)
        {
            continue;
        }
        if matches!(
            scene.param(*tag, "enabled"),
            Some(forge_core::Value::Bool(false))
        ) {
            continue;
        }
        if !matches!(scene.param(*tag, "position"), Some(forge_core::Value::Enum(s)) if s == "lock")
        {
            continue;
        }
        let axis_locked = match param {
            "transform.pos.x" => {
                matches!(
                    scene.param(*tag, "pos_x"),
                    Some(forge_core::Value::Bool(true))
                )
            }
            "transform.pos.y" => {
                matches!(
                    scene.param(*tag, "pos_y"),
                    Some(forge_core::Value::Bool(true))
                )
            }
            "transform.pos.z" => {
                matches!(
                    scene.param(*tag, "pos_z"),
                    Some(forge_core::Value::Bool(true))
                )
            }
            _ => false,
        };
        if axis_locked {
            return true;
        }
    }
    false
}

/// Registers the rig node types: joint, weight tag, skin deformer.
pub struct Rig;

impl Plugin for Rig {
    fn id(&self) -> &str {
        "forge.rig"
    }

    fn register(&self, registry: &mut Registry) -> Result<(), RegistryError> {
        registry.register(joint::joint_type())?;
        registry.register(weights::weight_tag_type())?;
        registry.register(skin::skin_type())?;
        registry.register(ik::ik_tag_type())?;
        registry.register(constraint::constraint_tag_type())?;
        registry.register(pose_morph::pose_morph_tag_type())?;
        Ok(())
    }
}
