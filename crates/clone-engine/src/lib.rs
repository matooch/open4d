//! forge-clone-engine: clone data, the cloner node type and its distribution modes
//! (Linear, Radial, Grid Array, Honeycomb). Spec: the clone-toolset spec in docs/specs (sections C1-C11).
//!
//! The crate is pure data and math: it does not evaluate scene children. The generators
//! crate evaluates the prototypes and expands the clones to geometry.

mod data;
mod delay;
mod effector;
mod formula;
mod fracture;
mod grid;
mod honeycomb;
mod linear;
mod matrix;
mod node;
mod plain;
mod radial;
mod random;
mod rng;
mod settings;
mod step;
mod target;
mod time;

pub use data::{CloneData, CloneItem};
pub use delay::{DELAY, Delay, DelayMode, WINDOW as DELAY_WINDOW, delay_type};
pub use effector::{Effector, EffectorCtx, apply_effectors, effector_from_node};
pub use formula::{DEFAULT_FORMULA, FORMULA, Formula, formula_type};
pub use fracture::{
    FRACTURE, FractureMode, fracture_settings, fracture_type, fragment_clones, polygon_islands,
};
pub use matrix::{MATRIX, matrix_settings, matrix_type};
pub use node::{CLONER, CloneToolset, cloner_type, settings_from_params};
pub use plain::{ColorMode, PLAIN, Plain, Space, TransformMode, plain_type};
pub use random::{
    NOISE_FEATURE, RANDOM, Random, RandomMode, effector_reads_time, random_type, uniform,
};
pub use rng::SplitMix64;
pub use settings::*;
pub use step::{STEP, Step, StepCurve, step_type};
pub use target::{TARGET, Target, TargetMode, target_type};
pub use time::{TIME, Time, time_type};

use forge_core::EvalCtx;
use forge_core::transform::Transform;
use glam::{DMat4, DVec3};

/// Upper bound of clones one cloner makes.
pub const MAX_CLONES: usize = 1_000_000;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CloneError {
    /// An observed or listed option whose rules are not derived (yet).
    #[error("not supported yet: {0}")]
    Unsupported(&'static str),
    #[error("more than {MAX_CLONES} clones")]
    TooMany,
}

/// Rotation matrix of `(h, p, b)` radians: `Ry(-h) · Rx(-p) · Rz(-b)` (TRANSFORM.md §3).
pub(crate) fn hpb(rot: DVec3) -> DMat4 {
    DMat4::from_mat3(
        Transform {
            pos: DVec3::ZERO,
            rot,
            scale: DVec3::ONE,
        }
        .rotation(),
    )
}

/// The clone data of the cloner's mode, with the Transform tab applied
/// (clone-toolset spec C3.1). `ctx` is the evaluation context (time-dependent parts arrive with
/// the effectors).
pub fn generate(settings: &Settings, _ctx: &EvalCtx) -> Result<CloneData, CloneError> {
    let items = match settings.mode {
        Mode::Object => return Err(CloneError::Unsupported("cloner object mode")),
        Mode::Linear => linear::generate(&settings.linear)?,
        Mode::Radial => radial::generate(&settings.radial)?,
        Mode::Grid => grid::generate(&settings.grid)?,
        Mode::Honeycomb => honeycomb::generate(&settings.honeycomb)?,
    };
    Ok(with_transform(items, &settings.transform))
}

/// Apply the common Transform tab to placements evaluated by the scene geometry layer.
pub fn with_transform(items: impl IntoIterator<Item = CloneItem>, t: &TransformTab) -> CloneData {
    let after = hpb(t.rotation) * DMat4::from_scale(t.scale);
    let before = DMat4::from_translation(t.position);
    let mut data = CloneData::default();
    for mut item in items {
        item.matrix = before * item.matrix * after;
        item.color = t.color;
        item.weight = t.weight;
        data.push(item);
    }
    data
}

impl CloneData {
    /// Chooses each clone's prototype among `prototypes` children (spec O1-O4):
    /// - Iterate repeats the children in order (`order-iterate`).
    /// - Random draws a child per clone from a generator seeded with `seed` (the reference's
    ///   sequence is not reproduced, `order-random`).
    /// - Blend maps the clone's place along the list, `i / (count - 1)`, to the child
    ///   `round(t (n - 1))`, halves up (`blend-order`: 2 children, 5 clones give 0 0 1 1 1).
    /// - Sort maps the clone weight to the child `round(w (n - 1))` clamped to the list
    ///   (`order-sort`: all weights 0 give child 0 for every clone).
    pub fn assign_children(&mut self, prototypes: usize, clones_mode: ClonesMode, seed: u64) {
        let n = self.child.len();
        let mut rng = SplitMix64::new(seed);
        for i in 0..n {
            let top = prototypes.saturating_sub(1) as f64;
            self.child[i] = if prototypes <= 1 {
                0
            } else {
                match clones_mode {
                    ClonesMode::Iterate => (i % prototypes) as u32,
                    ClonesMode::Random => {
                        ((rng.next_f64() * prototypes as f64) as usize).min(prototypes - 1) as u32
                    }
                    ClonesMode::Blend => {
                        let t = if n > 1 {
                            i as f64 / (n - 1) as f64
                        } else {
                            0.0
                        };
                        (t * top + 0.5).floor().clamp(0.0, top) as u32
                    }
                    ClonesMode::Sort => {
                        (self.weights[i] * top + 0.5).floor().clamp(0.0, top) as u32
                    }
                }
            };
        }
    }
}
