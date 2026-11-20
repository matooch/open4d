//! Key editing on tracks: move, scale, delete, interpolation, paste (docs/specs/ANIM.md
//! section 2b). Pure functions on a `Track`; the commands wrap them in transactions. Keys are
//! addressed by time (seconds), matched to within a nanosecond.

use forge_core::{Interp, Key, KeyTangent, Track};

const EPS: f64 = 1e-9;

fn selected(times: &[f64], t: f64) -> bool {
    times.iter().any(|s| (s - t).abs() < EPS)
}

/// One key with its side data, so a rebuild keeps tangents and point arrays attached.
struct Item {
    key: Key,
    tangent: Option<KeyTangent>,
    points: Option<Vec<[f64; 3]>>,
    moved: bool,
}

fn items(track: &Track, moved: impl Fn(&Key) -> bool) -> Vec<Item> {
    track
        .keys
        .iter()
        .enumerate()
        .map(|(i, k)| Item {
            key: *k,
            tangent: track.tangents.get(i).copied(),
            points: track.points.get(i).cloned(),
            moved: moved(k),
        })
        .collect()
}

/// Sorts by time; of two keys at the same time the moved one wins.
fn rebuild(track: &Track, mut v: Vec<Item>) -> Track {
    v.sort_by(|a, b| a.key.time.total_cmp(&b.key.time));
    let mut out: Vec<Item> = Vec::with_capacity(v.len());
    for it in v {
        match out.last_mut() {
            Some(last) if (last.key.time - it.key.time).abs() < EPS => {
                if it.moved || !last.moved {
                    *last = it;
                }
            }
            _ => out.push(it),
        }
    }
    let mut t = track.clone();
    t.keys = out.iter().map(|i| i.key).collect();
    t.tangents = if track.tangents.is_empty() {
        Vec::new()
    } else {
        out.iter().map(|i| i.tangent.unwrap_or_default()).collect()
    };
    t.points = if track.points.is_empty() {
        Vec::new()
    } else {
        out.into_iter().filter_map(|i| i.points).collect()
    };
    t
}

/// Moves the keys at `times` by `dt` seconds and `dv` value units. With `ripple` every key at
/// or after the earliest selected one moves in time (the later keys keep their value).
/// A key landing on another key replaces it. `None` when no key is selected.
pub fn move_keys(track: &Track, times: &[f64], dt: f64, dv: f64, ripple: bool) -> Option<Track> {
    let first = track
        .keys
        .iter()
        .map(|k| k.time)
        .find(|t| selected(times, *t))?;
    let mut v = items(track, |k| selected(times, k.time));
    for it in &mut v {
        let sel = it.moved;
        if sel || (ripple && it.key.time >= first - EPS) {
            it.key.time += dt;
            it.moved = true;
        }
        if sel {
            it.key.value += dv;
        }
    }
    Some(rebuild(track, v))
}

/// Scales the time of the keys at `times` about `pivot` (seconds) by `factor` (> 0; a
/// negative factor mirrors them). Handles of user tangents scale with the keys' spacing.
pub fn scale_keys(track: &Track, times: &[f64], pivot: f64, factor: f64) -> Option<Track> {
    if !factor.is_finite() || factor == 0.0 || !track.keys.iter().any(|k| selected(times, k.time)) {
        return None;
    }
    let mut v = items(track, |k| selected(times, k.time));
    for it in &mut v {
        if it.moved {
            it.key.time = pivot + (it.key.time - pivot) * factor;
            if let Some(t) = &mut it.tangent {
                let f = factor.abs();
                t.left.dt *= f;
                t.right.dt *= f;
                if factor < 0.0 {
                    // mirrored in time: a handle that pointed right now points left, with the
                    // same value offset
                    let (l, r) = (t.left, t.right);
                    t.left = forge_core::Handle { dt: -r.dt, ..r };
                    t.right = forge_core::Handle { dt: -l.dt, ..l };
                }
            }
        }
    }
    Some(rebuild(track, v))
}

/// Removes the keys at `times`. `None` when the track would be empty or no key matches
/// (delete the track itself in that case).
pub fn delete_keys(track: &Track, times: &[f64]) -> Option<Track> {
    let mut t = track.clone();
    let mut any = false;
    for time in times {
        let found = t
            .keys
            .iter()
            .find(|k| (k.time - time).abs() < EPS)
            .map(|k| k.time);
        if let Some(f) = found {
            any |= t.remove_key(f);
        }
    }
    (any && !t.keys.is_empty()).then_some(t)
}

/// Sets the interpolation of the keys at `times`.
pub fn set_interp(track: &Track, times: &[f64], interp: Interp) -> Option<Track> {
    let mut t = track.clone();
    let mut any = false;
    for k in &mut t.keys {
        if selected(times, k.time) {
            k.interp = interp;
            any = true;
        }
    }
    any.then_some(t)
}

/// A key to paste: time (seconds), value, interpolation, tangent.
pub type PastedKey = (f64, f64, Interp, Option<KeyTangent>);

/// Pastes keys: each `(time, value, interp, tangent)` is inserted, replacing a key at the same
/// time. Point tracks are not pasted into.
pub fn paste_keys(track: &Track, keys: &[PastedKey]) -> Option<Track> {
    if keys.is_empty() || !track.points.is_empty() {
        return None;
    }
    let mut t = track.clone();
    for (time, value, interp, tangent) in keys {
        let existing = t.keys.iter().position(|k| (k.time - time).abs() < EPS);
        t.set_key(Key::new(*time, *value).with_interp(*interp));
        if let Some(tg) = tangent {
            let i = existing.unwrap_or_else(|| {
                t.keys
                    .iter()
                    .position(|k| (k.time - time).abs() < EPS)
                    .unwrap_or(0)
            });
            t.set_tangent(i, *tg);
        } else if let Some(i) = t.keys.iter().position(|k| (k.time - time).abs() < EPS)
            && !t.tangents.is_empty()
        {
            t.tangents[i] = KeyTangent::default();
        }
    }
    Some(t)
}
