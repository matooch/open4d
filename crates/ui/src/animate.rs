//! Animation editing logic without any UI: frame/time, record, autokey, rule 5, key
//! navigation and delete (docs/specs/ANIM.md §2).

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::sync::Arc;

use forge_anim::{scene_at, value_at};
use forge_core::transform::{POS, ROT, SCALE};
use forge_core::{
    Document, EditError, Key, NodeId, ParamKind, Scene, Track, Tx, Value, parse_target,
};

/// The 9 tracks one record writes, in order (docs/parity/keyframes.md §3): Position X/Y/Z,
/// Scale X/Y/Z, Rotation H/P/B.
pub const RECORD_TRACKS: [&str; 9] = [
    "transform.pos.x",
    "transform.pos.y",
    "transform.pos.z",
    "transform.scale.x",
    "transform.scale.y",
    "transform.scale.z",
    "transform.rot.x",
    "transform.rot.y",
    "transform.rot.z",
];

/// Seconds of `frame` at `fps`.
pub fn frame_time(frame: i64, fps: u32) -> f64 {
    frame as f64 / f64::from(fps)
}

/// Frames (possibly fractional) of `time` seconds at `fps`.
pub fn time_frame(time: f64, fps: u32) -> f64 {
    time * f64::from(fps)
}

/// What an edit needs to know about time and autokey.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AnimCtx {
    pub frame: i64,
    pub fps: u32,
    pub autokey: bool,
}

impl AnimCtx {
    pub fn time(&self) -> f64 {
        frame_time(self.frame, self.fps)
    }
}

/// Animation context plus the evaluated scene an edit starts from.
#[derive(Clone, Copy)]
pub struct AnimEdit<'a> {
    pub ctx: AnimCtx,
    pub eval: &'a Scene,
}

/// The evaluated scene, cached per (scene revision, node count, frame, fps).
#[derive(Default)]
pub struct Evaluated {
    cache: RefCell<Option<Cached>>,
}

struct Cached {
    key: (u64, usize, i64, u32),
    scene: Arc<Scene>,
}

impl Evaluated {
    /// `scene` animated to `frame`.
    pub fn get(&self, scene: &Scene, frame: i64) -> Arc<Scene> {
        let fps = scene.settings().fps;
        let key = (scene.revision(), scene.len(), frame, fps);
        let mut cache = self.cache.borrow_mut();
        if let Some(c) = cache.as_ref()
            && c.key == key
        {
            return c.scene.clone();
        }
        let evaluated = Arc::new(scene_at(scene, frame_time(frame, fps)));
        *cache = Some(Cached {
            key,
            scene: evaluated.clone(),
        });
        evaluated
    }

    pub fn clear(&self) {
        *self.cache.borrow_mut() = None;
    }
}

fn component_key(param: &str, axis: usize) -> String {
    format!("{param}.{}", ["x", "y", "z"][axis])
}

/// Same value for the purpose of "did this edit change it" (display round trips lose bits).
fn same(a: f64, b: f64) -> bool {
    (a - b).abs() <= 1e-9 * a.abs().max(b.abs()).max(1.0)
}

/// Index of the key on `frame` (within a millionth of a frame).
fn key_index(track: &Track, frame: i64, fps: u32) -> Option<usize> {
    track
        .keys
        .iter()
        .position(|k| (time_frame(k.time, fps) - frame as f64).abs() < 1e-6)
}

/// Writes (or updates) the Spline key at `ctx`'s frame; an existing key keeps its time
/// and interpolation.
fn write_key(
    tx: &mut Tx,
    id: NodeId,
    track_key: &str,
    value: f64,
    ctx: &AnimCtx,
) -> Result<(), EditError> {
    let existing = tx
        .scene()
        .node(id)
        .and_then(|n| n.tracks.get(track_key))
        .and_then(|t| key_index(t, ctx.frame, ctx.fps).map(|i| t.keys[i]));
    let key = match existing {
        Some(k) => Key { value, ..k },
        None => Key::new(ctx.time(), value),
    };
    tx.set_key(id, track_key, key)
}

/// Writes a parameter value honouring animation (ANIM.md §2.4). Without `ctx` this is
/// `set_param`.
pub fn write_param(
    tx: &mut Tx,
    id: NodeId,
    param: &str,
    value: Value,
    ctx: Option<&AnimCtx>,
) -> Result<(), EditError> {
    let Some(ctx) = ctx else {
        return tx.set_param(id, param, value);
    };
    let node = tx.scene().node(id).ok_or(EditError::UnknownNode(id))?;
    let Some(desc) = tx.scene().node_type(id).and_then(|t| t.find_param(param)) else {
        return tx.set_param(id, param, value);
    };
    if !desc.animatable {
        return tx.set_param(id, param, value);
    }
    let value = desc.validate(value)?;
    let kind = desc.kind.clone();
    match (&kind, &value) {
        (ParamKind::Float { .. }, Value::Float(x)) => {
            let animated = node.tracks.contains_key(param);
            if ctx.autokey || animated {
                write_key(tx, id, param, *x, ctx)
            } else {
                tx.set_param(id, param, value)
            }
        }
        (ParamKind::Vec3 { .. }, Value::Vec3(new)) => {
            let stored = match node.params.get(param) {
                Some(Value::Vec3(v)) => *v,
                _ => [0.0; 3],
            };
            // the value shown now: the track's value where there is one
            let shown: [f64; 3] = std::array::from_fn(|k| {
                node.tracks
                    .get(&component_key(param, k))
                    .map_or(stored[k], |t| value_at(t, ctx.time()))
            });
            let has: [bool; 3] =
                std::array::from_fn(|k| node.tracks.contains_key(&component_key(param, k)));
            let mut base = stored;
            let mut touch_stored = false;
            let mut keys = Vec::new();
            for k in 0..3 {
                if ctx.autokey || (has[k] && !same(new[k], shown[k])) {
                    keys.push((component_key(param, k), new[k]));
                } else if !has[k] {
                    base[k] = new[k];
                    touch_stored = true;
                }
            }
            for (tk, x) in keys {
                write_key(tx, id, &tk, x, ctx)?;
            }
            if touch_stored {
                tx.set_param(id, param, Value::Vec3(base))?;
            }
            Ok(())
        }
        _ => tx.set_param(id, param, value),
    }
}

/// True when `anim` is set and `value` equals the evaluated value of `param` (nothing to
/// write for a gizmo or coordinate edit).
pub fn unchanged(anim: Option<&AnimEdit>, id: NodeId, param: &str, value: &Value) -> bool {
    let Some(a) = anim else { return false };
    match (a.eval.param(id, param), value) {
        (Some(Value::Vec3(old)), Value::Vec3(new)) => {
            old.iter().zip(new).all(|(o, n)| same(*o, *n))
        }
        _ => false,
    }
}

fn recordable(scene: &Scene, id: NodeId) -> bool {
    let Some(node) = scene.node(id) else {
        return false;
    };
    if node.is_tag() {
        return false;
    }
    scene.node_type(id).is_some_and(|ty| {
        [POS, ROT, SCALE]
            .iter()
            .all(|p| ty.find_param(p).is_some_and(|d| d.animatable))
    })
}

/// Command Record: keys the current evaluated transform of `ids` on the 9 tracks at the
/// current frame, one undo step. Returns the number of keys written.
pub fn record_keys(doc: &mut Document, ids: &[NodeId], ctx: &AnimCtx) -> Result<usize, String> {
    let eval = scene_at(doc.scene(), ctx.time());
    let targets: Vec<NodeId> = ids
        .iter()
        .copied()
        .filter(|id| recordable(doc.scene(), *id))
        .collect();
    if targets.is_empty() {
        return Err("Record: select an object with a transform first".into());
    }
    doc.transact("Record", |tx| {
        let mut n = 0;
        for id in &targets {
            for track_key in RECORD_TRACKS {
                let (param, axis) = track_key
                    .rsplit_once('.')
                    .expect("record keys have an axis");
                let axis = ["x", "y", "z"]
                    .iter()
                    .position(|a| *a == axis)
                    .expect("record keys end in x, y or z");
                let Some(Value::Vec3(v)) = eval.param(*id, param) else {
                    continue;
                };
                write_key(tx, *id, track_key, v[axis], ctx)?;
                n += 1;
            }
        }
        Ok(n)
    })
    .map_err(|e| e.to_string())
}

/// Command Delete Keys: removes the keys on the current frame from every track of `ids`,
/// one undo step. Returns how many keys went.
pub fn delete_keys(doc: &mut Document, ids: &[NodeId], ctx: &AnimCtx) -> Result<usize, String> {
    let mut found: Vec<(NodeId, String, f64)> = Vec::new();
    for id in ids {
        let Some(node) = doc.scene().node(*id) else {
            continue;
        };
        for (tk, track) in &node.tracks {
            if let Some(i) = key_index(track, ctx.frame, ctx.fps) {
                found.push((*id, tk.clone(), track.keys[i].time));
            }
        }
    }
    let n = found.len();
    doc.transact("Delete Keys", |tx| {
        found
            .iter()
            .try_for_each(|(id, tk, time)| tx.remove_key(*id, tk, *time))
    })
    .map_err(|e| e.to_string())?;
    Ok(n)
}

/// Frames (fractional where a key is off the grid) holding a key of any track of `ids`,
/// sorted and de-duplicated.
pub fn key_frames(scene: &Scene, ids: &[NodeId]) -> Vec<f64> {
    let fps = scene.settings().fps;
    let mut frames: Vec<f64> = ids
        .iter()
        .filter_map(|id| scene.node(*id))
        .flat_map(|n| n.tracks.values())
        .flat_map(|t| t.keys.iter().map(|k| time_frame(k.time, fps)))
        .collect();
    frames.sort_by(f64::total_cmp);
    frames.dedup_by(|a, b| (*a - *b).abs() < 1e-6);
    frames
}

/// The frame of the closest key before `current` (rounded down).
pub fn prev_key(frames: &[f64], current: i64) -> Option<i64> {
    frames
        .iter()
        .rev()
        .find(|f| **f < current as f64 - 1e-6)
        .map(|f| (f + 1e-6).floor() as i64)
}

/// The frame of the closest key after `current` (rounded up).
pub fn next_key(frames: &[f64], current: i64) -> Option<i64> {
    frames
        .iter()
        .find(|f| **f > current as f64 + 1e-6)
        .map(|f| (f - 1e-6).ceil() as i64)
}

/// Marker state of an animatable parameter row (ANIM.md §2.5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum KeyState {
    /// Has a track but no key on the current frame (hollow marker).
    Animated,
    /// Has a key on the current frame (filled marker).
    Keyed,
}

/// Marker state per parameter id of node `id` at `frame`; parameters without a track are
/// absent.
pub fn key_states(scene: &Scene, id: NodeId, frame: i64) -> BTreeMap<String, KeyState> {
    let mut out = BTreeMap::new();
    let (Some(node), Some(ty)) = (scene.node(id), scene.node_type(id)) else {
        return out;
    };
    let fps = scene.settings().fps;
    for (tk, track) in &node.tracks {
        let Ok(target) = parse_target(tk, |p| ty.find_param(p)) else {
            continue;
        };
        let param = match target {
            forge_core::Target::Float(p) | forge_core::Target::Component(p, _) => p,
        };
        let state = if key_index(track, frame, fps).is_some() {
            KeyState::Keyed
        } else {
            KeyState::Animated
        };
        let slot = out.entry(param).or_insert(state);
        *slot = (*slot).max(state);
    }
    out
}
