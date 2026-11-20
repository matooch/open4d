//! Key reduction and baking (docs/specs/ANIM.md section 1.10).

use forge_core::{Interp, Key, NodeId, Scene, Track, Value};

use crate::{scene_at, value_at_exact};

/// Removes keys while the curve stays within `tolerance` of the original at every original
/// key time: greedily drops the interior key whose removal changes the curve least, until
/// the cheapest removal would exceed the tolerance. The end keys, the behaviours and the
/// interpolation of the remaining keys are kept; tangents of removed keys go with them.
pub fn reduce_keys(track: &Track, tolerance: f64) -> Track {
    let mut cur = track.clone();
    if cur.keys.len() <= 2 || !cur.points.is_empty() {
        return cur;
    }
    let times: Vec<(f64, f64)> = track.keys.iter().map(|k| (k.time, k.value)).collect();
    let error = |t: &Track| {
        times
            .iter()
            .map(|(time, v)| (value_at_exact(t, &[], *time) - v).abs())
            .fold(0.0, f64::max)
    };
    loop {
        if cur.keys.len() <= 2 {
            return cur;
        }
        let best = (1..cur.keys.len() - 1)
            .map(|i| {
                let mut c = cur.clone();
                c.remove_key(cur.keys[i].time);
                (error(&c), c)
            })
            .min_by(|a, b| a.0.total_cmp(&b.0));
        match best {
            Some((e, c)) if e <= tolerance => cur = c,
            _ => return cur,
        }
    }
}

/// Bakes the evaluated value of a Float parameter (or, with `component`, one component of a
/// Vec3 parameter) of node `id` into linear keys at `times` (seconds): the scene is
/// evaluated with [`scene_at`] at each time, so animation, layers and anything else that
/// `scene_at` applies are included. `None` when the parameter is missing or not numeric.
pub fn bake_param(
    scene: &Scene,
    id: NodeId,
    param: &str,
    component: Option<usize>,
    times: &[f64],
) -> Option<Track> {
    let mut keys = Vec::with_capacity(times.len());
    for &t in times {
        let at = scene_at(scene, t);
        let v = match (at.param(id, param)?, component) {
            (Value::Float(f), None) => *f,
            (Value::Int(i), None) => *i as f64,
            (Value::Vec3(c) | Value::Color(c), Some(a)) if a < 3 => c[a],
            _ => return None,
        };
        keys.push(Key::new(t, v).with_interp(Interp::Linear));
    }
    keys.dedup_by(|b, a| a.time == b.time);
    (!keys.is_empty()).then(|| Track::new(keys))
}
