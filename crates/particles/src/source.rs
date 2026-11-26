//! Particles as a point source for the clone toolset and volumes (DYNAMICS.md section 11).

use forge_core::{NodeId, Scene};
use glam::DVec3;

use crate::cache::{ParticleCache, ParticleMode};
use crate::emitter::EMITTER;

/// One live particle, world space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ParticlePoint {
    pub id: u64,
    pub position: DVec3,
    pub velocity: DVec3,
    /// Seconds since birth.
    pub age: f64,
    pub scale: f64,
}

/// Anything that yields live points per frame: the interface the Tracer, the cloner's Object
/// mode and the volume builder read.
pub trait PointSource {
    /// The live points at `frame`, ordered by id.
    fn points_at(&mut self, frame: i64) -> Vec<ParticlePoint>;
}

/// True when `id` is a particle emitter.
pub fn is_emitter(scene: &Scene, id: NodeId) -> bool {
    scene.node(id).is_some_and(|n| n.type_id == EMITTER)
}

/// The point source of one emitter (one cache: asking for many frames steps once).
pub struct EmitterPoints {
    cache: ParticleCache,
}

impl EmitterPoints {
    /// `None` when `node` is not an emitter. Uses the editor birthrate.
    pub fn new(scene: &Scene, node: NodeId) -> Option<EmitterPoints> {
        ParticleCache::build(scene, node, ParticleMode::Editor)
            .ok()
            .map(|cache| EmitterPoints { cache })
    }
}

impl PointSource for EmitterPoints {
    fn points_at(&mut self, frame: i64) -> Vec<ParticlePoint> {
        let p = self.cache.at(frame);
        (0..p.len())
            .filter(|i| p.alive[*i])
            .map(|i| ParticlePoint {
                id: p.id[i],
                position: p.position[i],
                velocity: p.velocity[i],
                age: p.age[i],
                scale: p.scale[i],
            })
            .collect()
    }
}
