//! forge-rig: joints, joint tools, the weight tag, the skin deformer and auto weights
//! (docs/specs/RIG.md). Deformation is pure math on points; the scene enters through
//! `apply_skin` (called by `forge-modifiers`) and the tools in [`tools`].

pub mod auto_weights;
pub mod display;
pub mod joint;
pub mod mirror;
pub mod paint;
pub mod skin;
pub mod tools;
pub mod weights;

use forge_core::{Plugin, Registry, RegistryError};

pub use joint::{Axis, JOINT};
pub use skin::{SKIN, apply_skin};
pub use weights::{WEIGHT_TAG, WeightMap};

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
        Ok(())
    }
}
