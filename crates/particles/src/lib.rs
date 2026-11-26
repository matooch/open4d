//! Particles: the standard emitter, the force objects and the particle point source
//! (docs/specs/DYNAMICS.md sections 9-11). Own design (DECISIONS D20), tested statistically.

mod cache;
mod emitter;
mod force;
mod params;
mod particles;
mod source;

use forge_core::{Plugin, Registry, RegistryError};

pub use cache::{ParticleCache, ParticleError, ParticleMode, particles_at};
pub use emitter::{EMITTER, EmitterSettings, Newborn, births_in_frame, emitter_type};
pub use force::{
    ATTRACTOR, Collision, DEFLECTOR, FORCE_TYPES, FRICTION, Force, ForceInstance, GRAVITY,
    ROTATION, TURBULENCE, WIND, forces_for,
};
pub use particles::Particles;
pub use source::{EmitterPoints, ParticlePoint, PointSource, is_emitter};

/// The cap on live particles (births past it are dropped, deterministically).
pub const MAX_PARTICLES: usize = 1_000_000;

/// Installs the emitter and the force node types.
pub struct ParticleToolset;

impl Plugin for ParticleToolset {
    fn id(&self) -> &str {
        "forge.particle-toolset"
    }

    fn register(&self, r: &mut Registry) -> Result<(), RegistryError> {
        r.register(emitter_type())?;
        for t in force::force_types() {
            r.register(t)?;
        }
        Ok(())
    }
}
