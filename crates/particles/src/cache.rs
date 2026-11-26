//! Stepping the particles and the per-frame cache (DYNAMICS.md sections 9.6, 9.7, 10.1).

use std::borrow::Cow;

use forge_core::{EvalCtx, NodeId, Scene};
use glam::{DMat4, DVec3};

use crate::MAX_PARTICLES;
use crate::emitter::{self, EMITTER, EmitterSettings};
use crate::force::{self, Collision, ForceInstance};
use crate::particles::Particles;

/// Which birthrate applies (DYNAMICS.md section 9.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ParticleMode {
    /// The viewport and every data-source consumer.
    #[default]
    Editor,
    /// The renderer's evaluation.
    Render,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ParticleError {
    #[error("node {0} is not a particle emitter")]
    NotAnEmitter(u64),
}

/// Particles of an emitter at every frame from its start frame, simulated on demand.
pub struct ParticleCache {
    scene: Scene,
    emitter: NodeId,
    mode: ParticleMode,
    base: EmitterSettings,
    animated: bool,
    fps: f64,
    frames: Vec<Particles>,
    emitted: u64,
}

impl ParticleCache {
    /// A cache for the emitter `emitter` of `scene` (the scene is copied: editing the original
    /// means building a new cache).
    pub fn build(
        scene: &Scene,
        emitter: NodeId,
        mode: ParticleMode,
    ) -> Result<ParticleCache, ParticleError> {
        if !scene.node(emitter).is_some_and(|n| n.type_id == EMITTER) {
            return Err(ParticleError::NotAnEmitter(emitter.0));
        }
        Ok(ParticleCache {
            scene: scene.clone(),
            emitter,
            mode,
            base: EmitterSettings::read(scene, emitter, mode),
            animated: !forge_anim::track_nodes(scene).is_empty(),
            fps: f64::from(scene.settings().fps),
            frames: Vec::new(),
            emitted: 0,
        })
    }

    /// The first emitting frame.
    pub fn start(&self) -> i64 {
        self.base.start
    }

    /// Number of frames simulated so far (the start frame counts).
    pub fn cached_frames(&self) -> usize {
        self.frames.len()
    }

    /// The particles at `frame`, simulating up to it first; frames before the start give none.
    pub fn at(&mut self, frame: i64) -> &Particles {
        static EMPTY: Particles = Particles {
            id: Vec::new(),
            position: Vec::new(),
            velocity: Vec::new(),
            age: Vec::new(),
            lifetime: Vec::new(),
            alive: Vec::new(),
            scale: Vec::new(),
            rotation: Vec::new(),
            birth_frame: Vec::new(),
            spin_axis: Vec::new(),
            spin_rate: Vec::new(),
            scale_end: Vec::new(),
        };
        if frame < self.base.start {
            return &EMPTY;
        }
        let index = (frame - self.base.start) as usize;
        while self.frames.len() <= index {
            let f = self.base.start + self.frames.len() as i64;
            let next = self.step(f);
            self.frames.push(next);
        }
        &self.frames[index]
    }

    fn step(&mut self, frame: i64) -> Particles {
        let snap: Cow<Scene> = if self.animated {
            Cow::Owned(forge_anim::scene_at(&self.scene, frame as f64 / self.fps))
        } else {
            Cow::Borrowed(&self.scene)
        };
        let settings = EmitterSettings::read(&snap, self.emitter, self.mode);
        let mut p = if frame > self.base.start {
            self.frames.last().cloned().unwrap_or_default()
        } else {
            Particles::default()
        };
        if frame > self.base.start {
            let forces = force::forces_for(&snap, self.emitter);
            integrate(&mut p, &forces, &settings, frame, self.fps);
        }
        // ages and deaths
        let fps = self.fps;
        for i in 0..p.len() {
            let n = (frame - p.birth_frame[i]) as f64;
            p.age[i] = n / fps;
            if n >= p.lifetime[i] * fps - 1e-9 {
                p.alive[i] = false;
            }
            let t = if p.lifetime[i] > 0.0 {
                (p.age[i] / p.lifetime[i]).min(1.0)
            } else {
                1.0
            };
            p.scale[i] = 1.0 + (p.scale_end[i] - 1.0) * t;
            p.rotation[i] = p.spin_rate[i] * p.age[i];
        }
        p.remove_dead();
        // births
        let (first, count) =
            emitter::births_in_frame(self.base.rate, self.base.start, self.base.stop, frame);
        debug_assert!(count == 0 || first == self.emitted);
        let world = snap.world_matrix(self.emitter).unwrap_or(DMat4::IDENTITY);
        for i in 0..count {
            if p.len() >= MAX_PARTICLES {
                break;
            }
            let n = first + i;
            let b = emitter::birth(&settings, n, &world);
            p.push(n, frame, &b);
        }
        self.emitted = first + count;
        p
    }
}

/// One frame of semi-implicit Euler (DYNAMICS.md section 10.1).
fn integrate(
    p: &mut Particles,
    forces: &[ForceInstance],
    s: &EmitterSettings,
    frame: i64,
    fps: f64,
) {
    let dt = 1.0 / (fps * f64::from(s.substeps));
    for step in 0..s.substeps {
        let t = (frame - 1) as f64 / fps + f64::from(step) * dt;
        let ctx = EvalCtx::at_frame(frame - 1, fps);
        let ctx = EvalCtx { time: t, ..ctx };
        let n = p.len();
        let old: Vec<DVec3> = p.position.clone();
        let mut acc = vec![DVec3::ZERO; n];
        for inst in forces {
            if matches!(inst.force, force::Force::Deflector { .. }) {
                continue;
            }
            let weights: Option<Vec<f64>> = (!inst.falloff.is_empty()).then(|| {
                inst.falloff
                    .sample_many(&p.position, &ctx)
                    .iter()
                    .map(|s| s.value)
                    .collect()
            });
            for i in 0..n {
                if !p.alive[i] {
                    continue;
                }
                let w = weights.as_ref().map_or(1.0, |w| w[i]);
                if w != 0.0 {
                    acc[i] += inst.force.acceleration(p.position[i], p.velocity[i], t) * w;
                }
            }
        }
        for (i, a) in acc.iter().enumerate() {
            if p.alive[i] {
                p.velocity[i] += *a * dt;
                p.position[i] += p.velocity[i] * dt;
            }
        }
        for inst in forces {
            if !matches!(inst.force, force::Force::Deflector { .. }) {
                continue;
            }
            for (i, from) in old.iter().enumerate() {
                if !p.alive[i] {
                    continue;
                }
                match inst.force.collide(*from, p.position[i], p.velocity[i]) {
                    Some(Collision::Bounce { position, velocity }) => {
                        p.position[i] = position;
                        p.velocity[i] = velocity;
                    }
                    Some(Collision::Kill) => p.alive[i] = false,
                    None => {}
                }
            }
        }
    }
}

/// The particles of `emitter` at `frame`, simulated from the start frame on every call.
pub fn particles_at(
    scene: &Scene,
    emitter: NodeId,
    frame: i64,
    mode: ParticleMode,
) -> Result<Particles, ParticleError> {
    let mut cache = ParticleCache::build(scene, emitter, mode)?;
    Ok(cache.at(frame).clone())
}
