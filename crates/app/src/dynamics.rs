//! Dynamics in scene evaluation: simulated bodies get their pose at the frame
//! (docs/specs/DYNAMICS.md §5.1).

use std::collections::BTreeMap;

use forge_core::{EvalCtx, NodeId, Scene, Value};
use forge_dynamics::{BodyKey, DynamicsCache, RIGID_BODY, apply, archive};
use forge_geometry::PolyMesh;
use glam::DMat4;

use crate::schedule;

pub use forge_dynamics::DynamicsSettings;

/// The simulation of one scene revision, kept between frames so playback steps forward and
/// scrubbing back reads the cache.
#[derive(Default)]
pub struct DynamicsState {
    key: Option<(u64, usize)>,
    cache: Option<DynamicsCache>,
    archives: Vec<NodeId>,
    inputs: (Vec<NodeId>, Vec<NodeId>, bool),
}

fn has_bodies(scene: &Scene) -> bool {
    scene.registry().get(RIGID_BODY).is_some()
        && forge_dynamics::build::objects_in_order(scene)
            .into_iter()
            .any(|id| {
                forge_dynamics::build::body_tag(scene, id)
                    .is_some_and(|tag| forge_dynamics::build::kind_of(scene, tag).is_some())
            })
}

impl DynamicsState {
    pub fn active(&mut self, scene: &Scene) -> bool {
        self.prepare(scene);
        self.cache.is_some() || !self.archives.is_empty()
    }

    fn prepare(&mut self, source: &Scene) {
        let key = (source.revision(), source.len());
        if self.key == Some(key) {
            return;
        }
        self.key = Some(key);
        self.archives = forge_core::depgraph::all_nodes(source)
            .into_iter()
            .filter(|id| {
                source.node(*id).is_some_and(|n| n.type_id == archive::TYPE)
                    && archive::enabled(source, *id)
                    && source
                        .node(*id)
                        .and_then(|n| n.host)
                        .and_then(|host| archive::tag(source, host))
                        == Some(*id)
            })
            .collect();
        self.cache = None;
        if has_bodies(source) {
            self.inputs = schedule::animation_inputs(source);
            let ctx = EvalCtx::at_frame(source.settings().start, f64::from(source.settings().fps));
            let initial = schedule::animate(
                source,
                &self.inputs.0,
                &self.inputs.1,
                self.inputs.2,
                ctx.time,
            )
            .scene;
            let mesh_of = |id: NodeId| -> Option<PolyMesh> {
                forge_generators::evaluate(&initial, id, &ctx)
                    .ok()
                    .flatten()
            };
            self.cache = DynamicsCache::build(&initial, &mesh_of).ok();
        }
    }

    /// Moves the dynamic bodies of `working` (the scene animated to `frame`) to their
    /// simulated or baked pose. Returns the parameters touched by this stage.
    pub fn apply(
        &mut self,
        source: &Scene,
        frame: i64,
        working: &mut Scene,
    ) -> Vec<(NodeId, String)> {
        self.prepare(source);
        let mut touched = Vec::new();
        if let Some(cache) = &mut self.cache {
            cache.advance_with(frame, |frame, keys| {
                kinematic_targets(source, &self.inputs, frame, keys)
            });
            let poses = cache.moving_at(frame);
            let values = apply::overrides(working, &poses);
            write(working, &mut touched, values);
            let values = apply::soft_overrides(working, &cache.soft_at(frame));
            write(working, &mut touched, values);
        }
        for id in &self.archives {
            if let Some(values) = archive::playback(working, *id, frame) {
                write(working, &mut touched, values);
            }
        }
        touched
    }
}

fn write(
    working: &mut Scene,
    touched: &mut Vec<(NodeId, String)>,
    values: Vec<(NodeId, String, Value)>,
) {
    touched.extend(values.iter().map(|(id, name, _)| (*id, name.clone())));
    working.apply_params(values);
}

fn kinematic_targets(
    source: &Scene,
    inputs: &(Vec<NodeId>, Vec<NodeId>, bool),
    frame: i64,
    keys: &[BodyKey],
) -> BTreeMap<BodyKey, DMat4> {
    let ctx = EvalCtx::at_frame(frame, f64::from(source.settings().fps));
    let scene = schedule::animate(source, &inputs.0, &inputs.1, inputs.2, ctx.time).scene;
    let mut cloners = BTreeMap::new();
    keys.iter()
        .filter_map(|key| {
            let world = match *key {
                BodyKey::Node(id) => scene.world_matrix(id)?,
                BodyKey::Clone { cloner, index } => {
                    let (world, clones) = cloners
                        .entry(cloner)
                        .or_insert_with(|| {
                            let world = scene.world_matrix(cloner)?;
                            let clones = forge_generators::clone_data(&scene, cloner, &ctx)
                                .ok()
                                .flatten()?;
                            Some((world, clones))
                        })
                        .as_ref()?;
                    *world * *clones.matrices.get(index as usize)?
                }
            };
            Some((*key, world))
        })
        .collect()
}
