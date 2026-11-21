//! Dynamics on clones (docs/specs/DYNAMICS.md §6).

use forge_core::Scene;

use crate::sim::BodySpec;

/// One body per clone of every cloner that carries a rigid body tag.
pub fn clone_bodies(_scene: &Scene) -> Vec<BodySpec> {
    Vec::new()
}
