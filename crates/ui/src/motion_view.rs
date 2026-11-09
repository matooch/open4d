//! Motion paths and onion skin in the editor: what to draw for the selected objects, cached
//! and computed in the background (docs/specs/ANIM.md section 2b, VIEWPORT.md section 12).

use std::collections::BTreeSet;

use forge_anim::motion::{MotionPath, PathRange, onion_frames};
use forge_app::motion::{AsyncCache, compute_ghosts, compute_paths, scene_key};
use forge_command::MotionDisplay;
use forge_core::{NodeId, Scene};
use forge_viewport::motion::{GhostDraw, MotionDraw, PathDraw, ghost_geometry};
use glam::DVec3;

type SceneKey = (u64, usize, u32);
type PathKey = (SceneKey, Vec<NodeId>, (i64, i64, i64));
type GhostKey = (SceneKey, Vec<NodeId>, (i64, u32, u32, i64));

/// The caches of the two overlays. Results depend only on their key (scene revision, nodes,
/// frame range or current frame, settings), never on what was drawn before.
#[derive(Default)]
pub struct MotionState {
    paths: AsyncCache<PathKey, Vec<MotionPath>>,
    ghosts: AsyncCache<GhostKey, Vec<GhostDraw>>,
}

impl MotionState {
    /// Evaluate inline instead of on a worker thread (tests and screenshots).
    pub fn synchronous() -> MotionState {
        let mut m = MotionState::default();
        m.paths.synchronous = true;
        m.ghosts.synchronous = true;
        m
    }

    pub fn set_synchronous(&mut self, on: bool) {
        self.paths.synchronous = on;
        self.ghosts.synchronous = on;
    }

    /// What to draw for `selected` at `current`, and whether a result is still being computed
    /// (the caller repaints). `scene` is the stored scene; `shown` the one at `current`.
    pub fn draw(
        &mut self,
        scene: &Scene,
        shown: &Scene,
        selected: &BTreeSet<NodeId>,
        display: &MotionDisplay,
        current: i64,
    ) -> (MotionDraw, bool) {
        let nodes: Vec<NodeId> = selected.iter().copied().collect();
        let skey = scene_key(scene);
        let settings = scene.settings();
        let mut out = MotionDraw::default();
        let mut pending = false;
        if display.paths && !nodes.is_empty() {
            let (start, end) = display.range.unwrap_or((settings.start, settings.end));
            if let Some(range) = PathRange::new(start, end, display.step) {
                let key = (skey, nodes.clone(), (range.start, range.end, range.step));
                let (sc, ns) = (scene.clone(), nodes.clone());
                let (paths, p) = self.paths.get(key, move || compute_paths(&sc, &ns, &range));
                pending |= p;
                // a stale result may name nodes that are no longer selected
                out.paths = paths
                    .iter()
                    .flat_map(|v| v.iter())
                    .filter(|m| nodes.contains(&m.node))
                    .map(|m| path_draw(m, shown, current, range))
                    .collect();
            }
        }
        if display.onion && !nodes.is_empty() {
            let frames = onion_frames(
                current,
                display.before,
                display.after,
                display.onion_step,
                settings.start,
                settings.end,
            );
            if !frames.is_empty() {
                let key = (
                    skey,
                    nodes.clone(),
                    (current, display.before, display.after, display.onion_step),
                );
                let (sc, ns) = (scene.clone(), nodes);
                let (ghosts, p) = self.ghosts.get(key, move || {
                    compute_ghosts(&sc, &ns, &frames)
                        .into_iter()
                        .map(|g| GhostDraw {
                            forward: g.forward,
                            rank: g.rank,
                            of: g.of,
                            lines: ghost_geometry(&g.mesh),
                        })
                        .collect()
                });
                pending |= p;
                out.ghosts = ghosts.iter().flat_map(|v| v.iter().cloned()).collect();
            }
        }
        (out, pending)
    }
}

fn path_draw(m: &MotionPath, shown: &Scene, current: i64, range: PathRange) -> PathDraw {
    let current = (current >= range.start && current <= range.end)
        .then(|| {
            shown
                .world_matrix(m.node)
                .map(|w| (current as f64, w.transform_point3(DVec3::ZERO)))
        })
        .flatten();
    PathDraw {
        samples: m.samples.iter().map(|s| (s.frame as f64, s.pos)).collect(),
        keys: m.keys.iter().map(|k| k.pos).collect(),
        current,
    }
}
