//! Motion paths and onion-skin ghosts: evaluation and the cache (docs/specs/ANIM.md
//! section 2b). The sampling maths are in `forge_anim::motion`; the viewport draws.

use std::sync::Arc;
use std::sync::mpsc::{Receiver, TryRecvError, channel};

use forge_anim::motion::{MotionPath, OnionFrame, PathRange, sample_paths};
use forge_core::{EvalCtx, NodeId, Scene};
use forge_geometry::PolyMesh;

use crate::animate::frame_time;
use crate::schedule;

/// What a cached result depends on in the scene: revision, node count and fps. The revision
/// rises on every edit, undo and redo.
pub fn scene_key(scene: &Scene) -> (u64, usize, u32) {
    (scene.revision(), scene.len(), scene.settings().fps)
}

/// The paths of `nodes` over `range`: the scene is animated like the viewport shows it
/// (tracks, then expression tags). Stored values are not touched.
pub fn compute_paths(scene: &Scene, nodes: &[NodeId], range: &PathRange) -> Vec<MotionPath> {
    let (tracks, tags, enabled) = schedule::animation_inputs(scene);
    let eval = |time: f64| schedule::animate(scene, &tracks, &tags, enabled, time).scene;
    sample_paths(scene, nodes, range, &eval)
}

/// One node's mesh at a ghost frame, world space.
#[derive(Debug, Clone, PartialEq)]
pub struct GhostMesh {
    pub node: NodeId,
    pub frame: i64,
    pub rank: u32,
    pub forward: bool,
    /// Ghosts on this side of the current frame (for the fade).
    pub of: u32,
    pub mesh: PolyMesh,
}

/// The meshes of `nodes` at each of `frames`. A node without geometry at a frame adds nothing.
pub fn compute_ghosts(scene: &Scene, nodes: &[NodeId], frames: &[OnionFrame]) -> Vec<GhostMesh> {
    let (tracks, tags, enabled) = schedule::animation_inputs(scene);
    let fps = scene.settings().fps;
    let side = |forward: bool| frames.iter().filter(|f| f.forward == forward).count() as u32;
    let mut out = Vec::new();
    for f in frames {
        let at = schedule::animate(scene, &tracks, &tags, enabled, frame_time(f.frame, fps)).scene;
        let ctx = EvalCtx::at_frame(f.frame, f64::from(fps));
        for &node in nodes {
            if let Some(mesh) = schedule::node_output(&at, node, &ctx).mesh {
                out.push(GhostMesh {
                    node,
                    frame: f.frame,
                    rank: f.rank,
                    forward: f.forward,
                    of: side(f.forward),
                    mesh,
                });
            }
        }
    }
    out
}

type Job<T> = Box<dyn FnOnce() -> T + Send>;

/// The latest finished result of a pure job and the key it was computed for. A job runs on
/// one worker thread (or inline when `synchronous`); a result is only ever used for the key
/// it was computed for, so timing never changes what is shown once it is ready.
pub struct AsyncCache<K, T> {
    ready: Option<(K, Arc<T>)>,
    running: Option<(K, Receiver<T>)>,
    queued: Option<(K, Job<T>)>,
    failed: Option<K>,
    /// Run jobs inline (tests, screenshots, headless).
    pub synchronous: bool,
}

impl<K, T> Default for AsyncCache<K, T> {
    fn default() -> Self {
        AsyncCache {
            ready: None,
            running: None,
            queued: None,
            failed: None,
            synchronous: false,
        }
    }
}

impl<K: PartialEq + Clone, T: Send + 'static> AsyncCache<K, T> {
    /// The result for `key`, or the previous (stale) result while the job for `key` runs.
    /// The flag is true while a result for `key` is still pending.
    pub fn get(
        &mut self,
        key: K,
        job: impl FnOnce() -> T + Send + 'static,
    ) -> (Option<Arc<T>>, bool) {
        if self.synchronous {
            if self.ready.as_ref().is_none_or(|(k, _)| *k != key) {
                self.ready = Some((key, Arc::new(job())));
            }
            return (self.ready.as_ref().map(|r| r.1.clone()), false);
        }
        self.poll(&key);
        if self.ready.as_ref().is_some_and(|(k, _)| *k == key) {
            self.queued = None;
            return (self.ready.as_ref().map(|r| r.1.clone()), false);
        }
        if self.failed.as_ref() == Some(&key) {
            return (self.ready.as_ref().map(|r| r.1.clone()), false);
        }
        if self.running.as_ref().is_some_and(|(k, _)| *k == key) {
            // already computing it
        } else if self.running.is_some() {
            self.queued = Some((key, Box::new(job)));
        } else {
            self.start(key, Box::new(job));
        }
        (self.ready.as_ref().map(|r| r.1.clone()), true)
    }

    /// Blocks until the job for the wanted key finished (tests).
    pub fn wait(&mut self, key: &K) {
        loop {
            self.poll(key);
            if self.running.is_none() && self.queued.is_none() {
                return;
            }
            std::thread::yield_now();
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
    }

    fn start(&mut self, key: K, job: Job<T>) {
        let (tx, rx) = channel();
        std::thread::spawn(move || {
            // the receiver is gone when the result is no longer wanted
            let _ = tx.send(job());
        });
        self.running = Some((key, rx));
    }

    fn poll(&mut self, wanted: &K) {
        if let Some((k, rx)) = self.running.take() {
            match rx.try_recv() {
                Ok(v) => {
                    // never replace a result for the wanted key with an older one
                    let keep =
                        self.ready.as_ref().is_some_and(|(r, _)| r == wanted) && k != *wanted;
                    if !keep {
                        self.ready = Some((k, Arc::new(v)));
                    }
                }
                Err(TryRecvError::Empty) => self.running = Some((k, rx)),
                Err(TryRecvError::Disconnected) => self.failed = Some(k),
            }
        }
        if self.running.is_none()
            && let Some((k, job)) = self.queued.take()
        {
            if self.ready.as_ref().is_some_and(|(r, _)| *r == k) || self.failed.as_ref() == Some(&k)
            {
                return;
            }
            self.start(k, job);
        }
    }
}
