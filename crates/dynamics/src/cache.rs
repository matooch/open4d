//! The per-frame cache and scrubbing (docs/specs/DYNAMICS.md §5).

use std::collections::BTreeMap;

use forge_core::Scene;
use glam::{DMat4, DVec3};

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
    kinematic: Vec<BodyKey>,
    frames: Vec<Vec<DMat4>>,
    soft_keys: Vec<BodyKey>,
    soft_frames: Vec<Vec<Vec<DVec3>>>,
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
        let softs = crate::build::soft_of_scene(scene, mesh_of);
        Self::from_specs(
            scene,
            &settings,
            &specs,
            &crate::joints::links_of_scene(scene),
            &softs,
        )
    }

    pub(crate) fn from_specs(
        scene: &Scene,
        settings: &DynamicsSettings,
        specs: &[crate::sim::BodySpec],
        links: &crate::joints::Links,
        softs: &[crate::soft::SoftSpec],
    ) -> Result<DynamicsCache, DynamicsError> {
        let sim = Simulation::with_soft(settings, scene.settings().fps, specs, links, softs)?;
        let keys = sim.keys();
        let kinds = sim.kinds();
        let moving = kinds.iter().map(|k| *k == BodyKind::Dynamic).collect();
        let kinematic = keys
            .iter()
            .zip(kinds)
            .filter_map(|(key, kind)| (kind == BodyKind::Kinematic).then_some(*key))
            .collect();
        let first = sim.poses();
        let sim_soft_keys = sim.soft_keys();
        let sim_soft = sim.soft_points();
        Ok(DynamicsCache {
            sim,
            start: scene.settings().start,
            keys,
            moving,
            kinematic,
            frames: vec![first],
            soft_keys: sim_soft_keys,
            soft_frames: vec![sim_soft],
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
        self.advance_with(frame, |_, _| BTreeMap::new());
        let index = frame.saturating_sub(self.start).max(0) as usize;
        self.keys
            .iter()
            .copied()
            .zip(self.frames[index].iter().copied())
            .collect()
    }

    /// Steps missing frames with the animated goals for each kinematic body.
    pub fn advance_with(
        &mut self,
        frame: i64,
        mut goals: impl FnMut(i64, &[BodyKey]) -> BTreeMap<BodyKey, DMat4>,
    ) {
        let index = frame.saturating_sub(self.start).max(0) as usize;
        while self.frames.len() <= index {
            if self.enabled {
                if self.kinematic.is_empty() {
                    self.sim.step_frame();
                } else {
                    let frame = self.start.saturating_add(self.frames.len() as i64);
                    let targets = goals(frame, &self.kinematic);
                    self.sim.step_frame_with(&|key| targets.get(&key).copied());
                }
            }
            self.frames.push(self.sim.poses());
            self.soft_frames.push(self.sim.soft_points());
        }
    }

    /// The particles (world space, cm) of every soft body at `frame`, simulating up to it first.
    pub fn soft_at(&mut self, frame: i64) -> BTreeMap<BodyKey, Vec<DVec3>> {
        let _ = self.at(frame);
        let index = frame.saturating_sub(self.start).max(0) as usize;
        self.soft_keys
            .iter()
            .copied()
            .zip(self.soft_frames[index].iter().cloned())
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
