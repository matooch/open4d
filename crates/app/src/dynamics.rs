//! Dynamics in scene evaluation: simulated bodies get their pose at the frame
//! (docs/specs/DYNAMICS.md §5.1).

use forge_core::{EvalCtx, NodeId, Scene};
use forge_dynamics::{DynamicsCache, RIGID_BODY, apply};
use forge_geometry::PolyMesh;

/// The simulation of one scene revision, kept between frames so playback steps forward and
/// scrubbing back reads the cache.
#[derive(Default)]
pub struct DynamicsState {
    key: Option<(u64, usize)>,
    cache: Option<DynamicsCache>,
}

fn has_bodies(scene: &Scene) -> bool {
    scene.registry().get(RIGID_BODY).is_some()
        && forge_dynamics::build::objects_in_order(scene)
            .into_iter()
            .any(|id| forge_dynamics::build::body_tag(scene, id).is_some())
}

impl DynamicsState {
    /// Moves the dynamic bodies of `working` (the scene animated to `frame`) to their
    /// simulated pose. `source` is the unanimated scene the simulation is keyed on.
    pub fn apply(&mut self, source: &Scene, frame: i64, working: &mut Scene) {
        let key = (source.revision(), source.len());
        if self.key != Some(key) {
            self.key = Some(key);
            self.cache = None;
            if has_bodies(source) {
                let ctx = EvalCtx {
                    time: 0.0,
                    fps: f64::from(source.settings().fps),
                    seed: 0,
                };
                let mesh_of = |id: NodeId| -> Option<PolyMesh> {
                    forge_generators::evaluate(source, id, &ctx).ok().flatten()
                };
                self.cache = DynamicsCache::build(source, &mesh_of).ok();
            }
        }
        let Some(cache) = self.cache.as_mut() else {
            return;
        };
        let poses = cache.moving_at(frame);
        working.apply_params(apply::overrides(working, &poses));
        working.apply_params(apply::soft_overrides(working, &cache.soft_at(frame)));
    }
}
