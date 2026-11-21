//! Scene-level dynamics settings (docs/specs/DYNAMICS.md §2).

use forge_core::{NodeId, Scene};

use crate::nodes::{self, SETTINGS};

/// Fitted to the observed free fall (DYNAMICS.md §2.1), cm/s².
pub const DEFAULT_GRAVITY: f64 = 1000.0;
/// Fitted to the observed free fall (DYNAMICS.md §2.1).
pub const DEFAULT_STEPS: i64 = 2;

/// What the settings object holds (or its defaults).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DynamicsSettings {
    pub enabled: bool,
    /// Downward acceleration, cm/s².
    pub gravity: f64,
    pub time_scale: f64,
    pub steps_per_frame: u32,
}

impl Default for DynamicsSettings {
    fn default() -> Self {
        DynamicsSettings {
            enabled: true,
            gravity: DEFAULT_GRAVITY,
            time_scale: 1.0,
            steps_per_frame: DEFAULT_STEPS as u32,
        }
    }
}

impl DynamicsSettings {
    /// The first settings object in document order; defaults when there is none.
    pub fn of_scene(scene: &Scene) -> DynamicsSettings {
        let mut stack: Vec<NodeId> = scene.roots().iter().rev().copied().collect();
        while let Some(id) = stack.pop() {
            let Some(n) = scene.node(id) else { continue };
            if n.type_id == SETTINGS {
                return DynamicsSettings {
                    enabled: nodes::boolean(scene, id, "enabled"),
                    gravity: nodes::float(scene, id, "gravity"),
                    time_scale: nodes::float(scene, id, "time_scale"),
                    steps_per_frame: nodes::float(scene, id, "steps_per_frame").max(1.0) as u32,
                };
            }
            stack.extend(n.children.iter().rev());
        }
        DynamicsSettings::default()
    }

    /// Length of one solver step in seconds at `fps`.
    pub fn step_dt(&self, fps: u32) -> f64 {
        self.time_scale / (f64::from(fps) * f64::from(self.steps_per_frame))
    }
}
