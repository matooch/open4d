//! The per-frame cache and scrubbing (docs/specs/DYNAMICS.md §5).

use std::collections::BTreeMap;

use forge_core::Scene;
use glam::DMat4;

use crate::build::bodies_of_scene;
use crate::settings::DynamicsSettings;
use crate::sim::{BodyKey, BodyKind, DynamicsError, Simulation};

/// World matrices of every body for the frames simulated so far.
///
/// The live world sits at the last cached frame. Asking for a later frame steps it forward
/// frame by frame; asking for an earlier one reads the cache. Changing the scene means
/// building a new cache: the simulation then restarts from the start frame, so the same scene
/// always gives the same frames, however the timeline was scrubbed.
pub struct DynamicsCache {
    sim: Simulation,
    start: i64,
    keys: Vec<BodyKey>,
    moving: Vec<bool>,
    frames: Vec<Vec<DMat4>>,
    enabled: bool,
}

impl DynamicsCache {
    /// Builds the world from the scene at its start frame (`scene.settings().start`).
    pub fn build(
        scene: &Scene,
        mesh_of: &dyn Fn(forge_core::NodeId) -> Option<forge_geometry::PolyMesh>,
    ) -> Result<DynamicsCache, DynamicsError> {
        let settings = DynamicsSettings::of_scene(scene);
        let mut specs = bodies_of_scene(scene, mesh_of);
        specs.extend(crate::clones::clone_bodies(scene));
        Self::from_specs(scene, &settings, &specs)
    }

    pub(crate) fn from_specs(
        scene: &Scene,
        settings: &DynamicsSettings,
        specs: &[crate::sim::BodySpec],
    ) -> Result<DynamicsCache, DynamicsError> {
        let sim = Simulation::new(settings, scene.settings().fps, specs)?;
        let keys = sim.keys();
        let moving = sim
            .kinds()
            .iter()
            .map(|k| *k == BodyKind::Dynamic)
            .collect();
        let first = sim.poses();
        Ok(DynamicsCache {
            sim,
            start: scene.settings().start,
            keys,
            moving,
            frames: vec![first],
            enabled: settings.enabled,
        })
    }

    /// The simulated bodies.
    pub fn keys(&self) -> &[BodyKey] {
        &self.keys
    }

    /// Number of frames simulated so far (the start frame counts).
    pub fn cached_frames(&self) -> usize {
        self.frames.len()
    }

    /// World matrices at `frame` (frames before the start give the start pose), simulating up
    /// to it first if needed.
    pub fn at(&mut self, frame: i64) -> BTreeMap<BodyKey, DMat4> {
        let index = (frame - self.start).max(0) as usize;
        while self.frames.len() <= index {
            if self.enabled {
                self.sim.step_frame();
            }
            self.frames.push(self.sim.poses());
        }
        self.keys
            .iter()
            .copied()
            .zip(self.frames[index].iter().copied())
            .collect()
    }

    /// [`DynamicsCache::at`] for the bodies the simulation moves (dynamic ones): what a scene
    /// evaluation writes back.
    pub fn moving_at(&mut self, frame: i64) -> BTreeMap<BodyKey, DMat4> {
        let all = self.at(frame);
        self.keys
            .iter()
            .zip(&self.moving)
            .filter(|(_, m)| **m)
            .map(|(k, _)| (*k, all[k]))
            .collect()
    }

    /// The live world, for tests and the connector module.
    pub fn simulation(&self) -> &Simulation {
        &self.sim
    }
}
