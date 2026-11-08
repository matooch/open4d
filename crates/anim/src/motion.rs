//! Motion paths and onion frames: pure sampling (docs/specs/ANIM.md section 2b).

use forge_core::{NodeId, Scene};
use glam::DVec3;

/// Most frames one path samples; the step is raised to stay below it.
pub const MAX_PATH_SAMPLES: usize = 2000;
/// Most objects that get a path at once.
pub const MAX_PATH_NODES: usize = 16;
/// Most ghosts on one side of the current frame.
pub const MAX_ONION: u32 = 10;

/// A frame range sampled every `step` frames.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PathRange {
    pub start: i64,
    pub end: i64,
    pub step: i64,
}

impl PathRange {
    /// `None` unless `start <= end` and `step >= 1`. The step is raised until at most
    /// [`MAX_PATH_SAMPLES`] frames result.
    pub fn new(start: i64, end: i64, step: i64) -> Option<PathRange> {
        if start > end || step < 1 {
            return None;
        }
        let len = i128::from(end) - i128::from(start);
        let max = (MAX_PATH_SAMPLES as i128 - 1).max(1);
        let min_step = i64::try_from((len + max - 1) / max)
            .unwrap_or(i64::MAX)
            .max(1);
        Some(PathRange {
            start,
            end,
            step: step.max(min_step),
        })
    }

    /// `start, start + step, ...` up to `end`; `end` is appended when the walk misses it.
    pub fn frames(&self) -> Vec<i64> {
        let mut out = Vec::new();
        let mut f = self.start;
        while f <= self.end {
            out.push(f);
            match f.checked_add(self.step) {
                Some(n) => f = n,
                None => break,
            }
        }
        if out.last() != Some(&self.end) {
            out.push(self.end);
        }
        out
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PathSample {
    pub frame: i64,
    pub pos: DVec3,
}

/// A key time (in frames, may be fractional) and the object's position then.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KeyMark {
    pub frame: f64,
    pub pos: DVec3,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MotionPath {
    pub node: NodeId,
    pub samples: Vec<PathSample>,
    pub keys: Vec<KeyMark>,
}

fn origin(scene: &Scene, node: NodeId) -> Option<DVec3> {
    scene
        .world_matrix(node)
        .map(|m| m.transform_point3(DVec3::ZERO))
}

/// Key times (seconds) of the `transform.*` tracks of `node` and its ancestors.
fn key_times(scene: &Scene, node: NodeId) -> Vec<f64> {
    let mut times = Vec::new();
    let mut cur = Some(node);
    while let Some(id) = cur {
        let Some(n) = scene.node(id) else { break };
        for (key, track) in &n.tracks {
            if key.starts_with("transform.") {
                times.extend(track.keys.iter().map(|k| k.time));
            }
        }
        cur = n.parent;
    }
    times
}

/// The world-space paths of `nodes` over `range`. `eval(time)` is the scene animated to
/// `time` seconds; it is called once per sampled frame (and once per key time) for all
/// nodes. Tags and unknown nodes are skipped, at most [`MAX_PATH_NODES`] nodes are used, and
/// objects that never move get no path.
pub fn sample_paths(
    scene: &Scene,
    nodes: &[NodeId],
    range: &PathRange,
    eval: &dyn Fn(f64) -> Scene,
) -> Vec<MotionPath> {
    let nodes: Vec<NodeId> = nodes
        .iter()
        .copied()
        .filter(|id| scene.node(*id).is_some_and(|n| !n.is_tag()))
        .take(MAX_PATH_NODES)
        .collect();
    if nodes.is_empty() {
        return Vec::new();
    }
    let fps = f64::from(scene.settings().fps);
    let mut paths: Vec<MotionPath> = nodes
        .iter()
        .map(|&node| MotionPath {
            node,
            samples: Vec::new(),
            keys: Vec::new(),
        })
        .collect();
    for frame in range.frames() {
        let at = eval(frame as f64 / fps);
        for p in &mut paths {
            if let Some(pos) = origin(&at, p.node) {
                p.samples.push(PathSample { frame, pos });
            }
        }
    }
    // key marks: the distinct key frames of each node, evaluated at their exact times
    let (lo, hi) = (range.start as f64 - 1e-6, range.end as f64 + 1e-6);
    let mut per_node: Vec<Vec<f64>> = Vec::new();
    for p in &paths {
        let mut frames: Vec<f64> = key_times(scene, p.node)
            .into_iter()
            .map(|t| t * fps)
            .filter(|f| *f >= lo && *f <= hi)
            .collect();
        frames.sort_by(f64::total_cmp);
        frames.dedup_by(|a, b| (*a - *b).abs() < 1e-6);
        per_node.push(frames);
    }
    let mut all: Vec<f64> = per_node.iter().flatten().copied().collect();
    all.sort_by(f64::total_cmp);
    all.dedup_by(|a, b| (*a - *b).abs() < 1e-6);
    for f in all {
        let at = eval(f / fps);
        for (p, frames) in paths.iter_mut().zip(&per_node) {
            if frames.iter().any(|g| (g - f).abs() < 1e-6)
                && let Some(pos) = origin(&at, p.node)
            {
                p.keys.push(KeyMark { frame: f, pos });
            }
        }
    }
    paths.retain(|p| {
        p.samples.first().is_some_and(|first| {
            p.samples
                .iter()
                .any(|s| (s.pos - first.pos).length() >= 1e-9)
        })
    });
    paths
}

/// One ghost frame of the onion skin.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OnionFrame {
    pub frame: i64,
    /// 1 for the ghost nearest the current frame.
    pub rank: u32,
    /// After the current frame.
    pub forward: bool,
}

/// Ghost frames at `current -/+ k * step` (`k = 1..=count`, counts capped at
/// [`MAX_ONION`]) inside `[lo, hi]`, in ascending frame order.
pub fn onion_frames(
    current: i64,
    before: u32,
    after: u32,
    step: i64,
    lo: i64,
    hi: i64,
) -> Vec<OnionFrame> {
    let step = step.max(1);
    let mut out = Vec::new();
    for k in (1..=before.min(MAX_ONION)).rev() {
        let frame = current.saturating_sub(step.saturating_mul(i64::from(k)));
        if frame >= lo && frame <= hi {
            out.push(OnionFrame {
                frame,
                rank: k,
                forward: false,
            });
        }
    }
    for k in 1..=after.min(MAX_ONION) {
        let frame = current.saturating_add(step.saturating_mul(i64::from(k)));
        if frame >= lo && frame <= hi {
            out.push(OnionFrame {
                frame,
                rank: k,
                forward: true,
            });
        }
    }
    out
}
