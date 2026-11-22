//! Track evaluation (docs/specs/ANIM.md §1).

use forge_core::{Behavior, Interp, Key, KeyTangent, TangentMode, Track};

/// The reference evaluates at document times rounded to whole milliseconds
/// (ANIM.md §1.4, cases `two-keys`, `mixed-interp`).
pub const TICKS_PER_SECOND: f64 = 1000.0;

/// Number of bisection halvings used to find the curve parameter (ANIM.md §1.6).
const BISECTION_STEPS: u32 = 19;

/// Handle lengths live on a grid of this many seconds (ANIM.md §1.5, read-back of
/// `tangents.json`: every handle is a whole multiple of 4e-7 s).
pub const HANDLE_GRID: f64 = 4e-7;

/// Rounds a handle length (seconds) to the nearest multiple of [`HANDLE_GRID`].
fn quantize_handle(len: f64) -> f64 {
    (len / HANDLE_GRID).round() * HANDLE_GRID
}

/// Rounds a time in seconds to the evaluation resolution.
pub fn quantize_time(time: f64) -> f64 {
    (time * TICKS_PER_SECOND).round() / TICKS_PER_SECOND
}

/// The value of `track` at `time` (seconds). An empty track reads 0.
pub fn value_at(track: &Track, time: f64) -> f64 {
    let t = effective_time(&track.keys, time);
    eval_track(track, &resolve(&track.tangents), t)
}

/// Added to an Int track's value before it is floored: it absorbs the 2^-19 sawtooth of the
/// curve solver (ANIM.md section 1.8, `int-track`: 4.0 exactly at frame 10).
const INT_EPSILON: f64 = 1e-4;

/// The value of an Int parameter's track at `time`: the curve floored (ANIM.md section 1.8).
/// `exact` skips the millisecond rounding of document time.
pub fn int_at(track: &Track, time: f64, exact: bool) -> i64 {
    let v = if exact {
        value_at_exact(track, &[], time)
    } else {
        value_at(track, time)
    };
    (v + INT_EPSILON).floor() as i64
}

/// The value of a Bool, Enum or Link track at `time`: the key at or before `time` holds
/// (the first key before the first), whatever the keys' interpolation (ANIM.md section 1.8).
pub fn step_at(track: &Track, time: f64) -> f64 {
    let keys = &track.keys;
    let i = keys.partition_point(|k| k.time <= time);
    keys.get(i.saturating_sub(1)).map_or(0.0, |k| k.value)
}

/// The point array of a point-level track at `time` (ANIM.md section 1.8): constant outside
/// the keys, a step for Step keys, otherwise a linear blend of the two neighbouring arrays
/// (parity-pending: not observed). `None` for a track without point keys.
pub fn points_at(track: &Track, time: f64) -> Option<Vec<[f64; 3]>> {
    let keys = &track.keys;
    if track.points.is_empty() || track.points.len() != keys.len() {
        return None;
    }
    let j = keys.partition_point(|k| k.time <= time);
    if j == 0 {
        return Some(track.points[0].clone());
    }
    if j == keys.len() {
        return Some(track.points[j - 1].clone());
    }
    let (a, b) = (&keys[j - 1], &keys[j]);
    let (pa, pb) = (&track.points[j - 1], &track.points[j]);
    if a.interp == Interp::Step {
        return Some(pa.clone());
    }
    let f = (time - a.time) / (b.time - a.time);
    Some(
        pa.iter()
            .zip(pb)
            .map(|(p, q)| std::array::from_fn(|c| p[c] + (q[c] - p[c]) * f))
            .collect(),
    )
}

/// Tangents with the decision data the evaluator needs: the tie-breaking handle length of
/// each side when the file stored one (`None`: the automatic length on the grid).
#[derive(Clone, Copy, Default)]
struct Resolved {
    tangent: KeyTangent,
    grid: [Option<f64>; 2],
}

fn resolve(tangents: &[KeyTangent]) -> Vec<Resolved> {
    tangents
        .iter()
        .map(|t| Resolved {
            tangent: *t,
            grid: [None; 2],
        })
        .collect()
}

/// The value of a track at the resolved time `t`: the behaviours before the first and
/// after the last key (ANIM.md section 1.9), then the keys.
fn eval_track(track: &Track, tangs: &[Resolved], t: f64) -> f64 {
    let keys = &track.keys;
    let (Some(first), Some(last)) = (keys.first(), keys.last()) else {
        return 0.0;
    };
    let span = last.time - first.time;
    let delta = last.value - first.value;
    if t > last.time && span > 0.0 {
        match track.after {
            Behavior::Constant => {}
            Behavior::Continue => {
                return last.value + end_slope(keys, tangs, true) * (t - last.time);
            }
            Behavior::Repeat | Behavior::OffsetRepeat | Behavior::Oscillate => {
                // x in (0, ...]: whole cycles `k` done before the cycle t is in; at an exact
                // cycle boundary the value is the end of the cycle that just ended.
                let x = (t - first.time) / span;
                let k = x.ceil() - 1.0;
                let frac = x - k;
                let (u, shift) = match track.after {
                    Behavior::Repeat => (first.time + frac * span, 0.0),
                    Behavior::OffsetRepeat => (first.time + frac * span, k * delta),
                    _ if k % 2.0 == 0.0 => (first.time + frac * span, 0.0),
                    _ => (first.time + (1.0 - frac) * span, 0.0),
                };
                return eval(keys, tangs, u) + shift;
            }
        }
    }
    if t < first.time && span > 0.0 {
        match track.before {
            Behavior::Constant => {}
            Behavior::Continue => {
                return first.value + end_slope(keys, tangs, false) * (t - first.time);
            }
            Behavior::Repeat | Behavior::OffsetRepeat | Behavior::Oscillate => {
                // Backwards from the first key: the value at a cycle boundary is the end of
                // the cycle, `last` (`before-repeat` frame 0), and Offset Repeat shifts it down
                // by one delta per cycle passed plus one (`before-offset`: -100 at frame -30,
                // -200 at -60, -50 at -15 for keys 0 -> 0, 30 -> 100). Oscillate: `before-oscillate`.
                let x = (first.time - t) / span;
                let r = x - x.floor();
                let whole = x.floor() + 1.0;
                let back = r;
                let (u, shift) = match track.before {
                    Behavior::Repeat => (last.time - back * span, 0.0),
                    Behavior::OffsetRepeat => (last.time - back * span, -whole * delta),
                    _ => {
                        let k = x.ceil() - 1.0;
                        let frac = x - k;
                        if k % 2.0 == 0.0 {
                            (first.time + frac * span, 0.0)
                        } else {
                            (last.time - frac * span, 0.0)
                        }
                    }
                };
                return eval(keys, tangs, u) + shift;
            }
        }
    }
    eval(keys, tangs, t)
}

/// Slope (value per second) a Continue behaviour carries on with: that of the end key's
/// handle on the inside of the range.
fn end_slope(keys: &[Key], tangs: &[Resolved], after: bool) -> f64 {
    let i = if after { keys.len() - 1 } else { 0 };
    let t = tangent_with(keys, tangs, i);
    let (len, dv) = if after {
        (t.left, -t.left_dv)
    } else {
        (t.right, t.right_dv)
    };
    if len > 0.0 { dv / len } else { 0.0 }
}

/// Evaluates the keys at the already resolved time `t` (constant outside them).
fn eval(keys: &[Key], user: &[Resolved], t: f64) -> f64 {
    let (Some(first), Some(last)) = (keys.first(), keys.last()) else {
        return 0.0;
    };
    if t <= first.time {
        return first.value;
    }
    if t >= last.time {
        return last.value;
    }
    // First key at or after t (so not the first key); the segment is (keys[j-1], keys[j]].
    let j = keys.partition_point(|k| k.time < t);
    let (a, b) = (&keys[j - 1], &keys[j]);
    if t == b.time {
        return b.value;
    }
    match a.interp {
        Interp::Step => a.value,
        Interp::Linear => a.value + (b.value - a.value) * ((t - a.time) / (b.time - a.time)),
        Interp::Spline => spline_segment(keys, user, j - 1, t),
    }
}

/// `time` rounded to the evaluation resolution, except that a rounded time that falls
/// just before a key it rounds to counts as exactly on that key (`uneven`: frame 10 is
/// 0.333 s, the key is at 0.3333.. s, and the value is the key's). A rounded time just
/// after the key stays after it (`zigzag-uneven` frame 5, `before-after` frame 20).
fn effective_time(keys: &[Key], time: f64) -> f64 {
    let t = quantize_time(time);
    keys.iter()
        .find(|k| k.time > t && quantize_time(k.time) == t)
        .map_or(t, |k| k.time)
}

/// Automatic tangent of a key: the handle length (same on both sides) and the slope.
#[derive(Clone, Copy)]
struct Tangent {
    /// Handle length in time units, left side (towards the previous key).
    left: f64,
    /// Handle length in time units, right side.
    right: f64,
    /// Value offset of the left handle end from the key (minus slope times handle length
    /// for an automatic tangent).
    left_dv: f64,
    /// Value offset of the right handle end from the key.
    right_dv: f64,
    /// Handle lengths used only to decide exact ties: the stored read-back where there is
    /// one, else the automatic length on the handle grid.
    grid_left: f64,
    grid_right: f64,
    /// Left / right side was replaced by a user tangent.
    edited: [bool; 2],
}

fn tangent(keys: &[Key], i: usize) -> Tangent {
    let k = keys[i];
    if i == 0 {
        let len = (keys[1].time - k.time) / 3.0;
        return Tangent {
            left: len,
            right: len,
            left_dv: 0.0,
            right_dv: 0.0,
            grid_left: quantize_handle(len),
            grid_right: quantize_handle(len),
            edited: [false; 2],
        };
    }
    if i + 1 == keys.len() {
        let len = (k.time - keys[i - 1].time) / 3.0;
        return Tangent {
            left: len,
            right: len,
            left_dv: 0.0,
            right_dv: 0.0,
            grid_left: quantize_handle(len),
            grid_right: quantize_handle(len),
            edited: [false; 2],
        };
    }
    let (p, n) = (keys[i - 1], keys[i + 1]);
    let (l, r) = (k.time - p.time, n.time - k.time);
    // Harmonic mean of the two spans, a third of it as the handle length.
    let len = 2.0 * l * r / (l + r) / 3.0;
    let (dl, dr) = (k.value - p.value, n.value - k.value);
    let slope = if dl * dr <= 0.0 {
        0.0
    } else {
        // Mean of the two segment slopes (ANIM.md §1.5; `mono-uneven`: 5.5 from 10 and 1),
        // limited so neither handle leaves the value range of its segment (no overshoot).
        let mean = 0.5 * (dl / l + dr / r);
        let limit = (dl.abs() / len).min(dr.abs() / len);
        mean.abs().min(limit).copysign(mean)
    };
    Tangent {
        left: len,
        right: len,
        left_dv: -(slope * len),
        right_dv: slope * len,
        grid_left: quantize_handle(len),
        grid_right: quantize_handle(len),
        edited: [false; 2],
    }
}

fn bezier(p: [f64; 4], s: f64) -> f64 {
    let u = 1.0 - s;
    u * u * u * p[0] + 3.0 * u * u * s * p[1] + 3.0 * u * s * s * p[2] + s * s * s * p[3]
}

/// A tangent set on a key by the user (ANIM.md §1.5a): the handle ends relative to the key,
/// as the reference reads them back (`tangents.json`; times in seconds here).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UserTangent {
    /// Left handle end: (time offset <= 0 in seconds, value offset).
    pub left: (f64, f64),
    /// Right handle end: (time offset >= 0 in seconds, value offset).
    pub right: (f64, f64),
}

/// Whether a read-back handle (`len`, `dv`) of key `i` on `side` (-1 left, 1 right) is
/// evaluated as the automatic handle (`auto_len`, `auto_dv`), ANIM.md §1.5a: it is
/// (a) the automatic handle to within 2 grid steps in time and 1e-6 relative in value,
/// (b) the display tangent the reference reads back on keys it has not edited: a third of
/// the span from the previous key and a third of that chord, mirrored on the two sides, or (c) of zero length (a key
/// that was never evaluated reads all zeros).
fn is_automatic(
    len: f64,
    dv: f64,
    auto_len: f64,
    auto_dv: f64,
    keys: &[Key],
    i: usize,
    side: i32,
) -> bool {
    let close = |a: f64, b: f64| (a - b).abs() <= 1e-6 * a.abs().max(b.abs()).max(1.0);
    if len <= 0.0 || ((len - auto_len).abs() <= 2.0 * HANDLE_GRID && close(dv, auto_dv)) {
        return true;
    }
    // Display tangent: a third of the span and of the chord from the previous key, mirrored.
    let Some(p) = i.checked_sub(1).map(|j| keys[j]) else {
        return false;
    };
    let (span, chord) = (keys[i].time - p.time, keys[i].value - p.value);
    (len - span / 3.0).abs() <= 2.0 * HANDLE_GRID && close(dv, f64::from(side) * chord / 3.0)
}

/// The tangent of key `i`: each side automatic, on the chord, or the user's stored handle.
fn tangent_with(keys: &[Key], tangs: &[Resolved], i: usize) -> Tangent {
    let mut t = tangent(keys, i);
    let Some(r) = tangs.get(i) else { return t };
    let k = keys[i];
    for (side, h) in [(0usize, r.tangent.left), (1, r.tangent.right)] {
        if let Some(g) = r.grid[side].filter(|g| *g > 0.0) {
            if side == 0 {
                t.grid_left = g;
            } else {
                t.grid_right = g;
            }
        }
        // the neighbour this side's handle points to (none past the ends)
        let neighbour = if side == 0 {
            i.checked_sub(1).map(|j| keys[j])
        } else {
            keys.get(i + 1).copied()
        };
        let (len, dv, edited) = match h.mode {
            TangentMode::Auto => continue,
            TangentMode::User => (h.dt.abs(), h.dv, true),
            TangentMode::Chord => {
                let Some(n) = neighbour else { continue };
                // On the line to the neighbour with the handle length on the grid.
                let span = (n.time - k.time).abs();
                let len = quantize_handle(span / 3.0);
                (
                    len,
                    (n.value - k.value) / (n.time - k.time).abs() * len,
                    true,
                )
            }
        };
        if side == 0 {
            t.left = len;
            t.left_dv = dv;
            t.edited[0] = edited;
            if h.mode == TangentMode::Chord {
                t.grid_left = len;
            }
        } else {
            t.right = len;
            t.right_dv = dv;
            t.edited[1] = edited;
            if h.mode == TangentMode::Chord {
                t.grid_right = len;
            }
        }
    }
    t
}

/// The legacy read-back form: `user[i]` as stored by the reference's tangent read-back,
/// each side classified automatic or user by [`is_automatic`] (ANIM.md section 1.5a).
fn resolve_legacy(keys: &[Key], user: &[Option<UserTangent>]) -> Vec<Resolved> {
    (0..keys.len())
        .map(|i| {
            let Some(Some(u)) = user.get(i) else {
                return Resolved::default();
            };
            let t = tangent(keys, i);
            let (ll, rl) = (-u.left.0, u.right.0);
            let side = |len: f64, dv: f64, auto_len: f64, auto_dv: f64, s: i32| {
                // The neighbour this side's handle points to: a handle end outside the value
                // range of its segment overshoots and is evaluated automatically
                // (ANIM.md §1.5a, `broken` / `broken-2`; parity-pending).
                let neighbour =
                    if s < 0 { i.checked_sub(1) } else { Some(i + 1) }.and_then(|j| keys.get(j));
                let overshoots = neighbour.is_some_and(|n| {
                    let end = keys[i].value + dv;
                    let (lo, hi) = (n.value.min(keys[i].value), n.value.max(keys[i].value));
                    let eps = 1e-9 * hi.abs().max(lo.abs()).max(1.0);
                    end < lo - eps || end > hi + eps
                });
                if overshoots || is_automatic(len, dv, auto_len, auto_dv, keys, i, s) {
                    forge_core::Handle::default()
                } else {
                    forge_core::Handle {
                        mode: TangentMode::User,
                        dt: if s < 0 { -len } else { len },
                        dv,
                    }
                }
            };
            Resolved {
                tangent: KeyTangent {
                    left: side(ll, u.left.1, t.left, t.left_dv, -1),
                    right: side(rl, u.right.1, t.right, t.right_dv, 1),
                    ..KeyTangent::default()
                },
                grid: [(ll > 0.0).then_some(ll), (rl > 0.0).then_some(rl)],
            }
        })
        .collect()
}

/// The value of `track` at exactly `time` seconds (no millisecond rounding), with `user[i]`
/// the user tangent of key `i` as the reference reads it back (a missing or `None` entry is
/// automatic; the track's own `tangents` are used for entries that are absent). ANIM.md
/// section 1.5a: this is how the O38 observations were sampled.
pub fn value_at_exact(track: &Track, user: &[Option<UserTangent>], time: f64) -> f64 {
    let tangs = if user.is_empty() {
        resolve(&track.tangents)
    } else {
        resolve_legacy(&track.keys, user)
    };
    eval_track(track, &tangs, time)
}

/// The Bezier control values of segment `i` and the curve parameter solved for time `t`.
fn spline_solve(keys: &[Key], user: &[Resolved], i: usize, t: f64) -> ([f64; 4], [f64; 4], f64) {
    let (a, b) = (keys[i], keys[i + 1]);
    let (ta, tb) = (tangent_with(keys, user, i), tangent_with(keys, user, i + 1));
    // In a segment with a user-edited handle, the automatic handles sit on the handle grid
    // (ANIM.md §1.5a; `weighted-long`, `weighted-short`: 9.999996 frames at key 0).
    let on_grid = ta.edited != [false; 2] || tb.edited != [false; 2];
    let (ra, lb) = if on_grid {
        (
            if ta.edited[1] {
                ta.right
            } else {
                ta.grid_right
            },
            if tb.edited[0] { tb.left } else { tb.grid_left },
        )
    } else {
        (ta.right, tb.left)
    };
    let xs = [a.time, a.time + ra, b.time - lb, b.time];
    let ys = [
        a.value,
        a.value + ta.right_dv,
        b.value + tb.left_dv,
        b.value,
    ];
    // The same time polynomial with the handle lengths on the reference's grid; it only
    // decides exact ties (below).
    let grid_xs = [
        a.time,
        a.time + ta.grid_right,
        b.time - tb.grid_left,
        b.time,
    ];
    // Bisection on the curve parameter; the answer is the middle of the final interval.
    // `x(mid) < t` goes up. When `x(mid)` equals `t` to rounding, the stored handles
    // decide: their lengths are on a grid of 4e-7 s, a hair off a third of the span, and
    // `x(s) = span·s − 3·d·s(1−s)(1−2s)` moves the tie below or above `t` by their error
    // `d` (`fps-24`, `dense-quarter`: 1 s span, shorter handle, ties below the centre go
    // up and above it go down; `four-keys`: longer handle, the other way round). A tie
    // that is exact on the grid too (the centre of a symmetric curve) goes up,
    // except for a segment with both handles collapsed (RIG.md §13).
    let collapsed = ra == 0.0 && lb == 0.0;
    let tie = 1e-12 * (b.time - a.time);
    let (mut lo, mut hi) = (0.0f64, 1.0f64);
    for _ in 0..BISECTION_STEPS {
        let mid = 0.5 * (lo + hi);
        let x = bezier(xs, mid);
        let up = if !collapsed && (x - t).abs() <= tie {
            let xg = bezier(grid_xs, mid);
            if (xg - t).abs() <= tie { true } else { xg < t }
        } else {
            x < t
        };
        if up {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    (xs, ys, 0.5 * (lo + hi))
}

fn spline_segment(keys: &[Key], user: &[Resolved], i: usize, t: f64) -> f64 {
    let (_, ys, s) = spline_solve(keys, user, i, t);
    bezier(ys, s)
}

fn bezier_slope(p: [f64; 4], s: f64) -> f64 {
    let u = 1.0 - s;
    3.0 * (u * u * (p[1] - p[0]) + 2.0 * u * s * (p[2] - p[1]) + s * s * (p[3] - p[2]))
}

/// The speed `dv/dt` of `track` at exactly `time` seconds (value units per second), the
/// analytic derivative of the segment (ANIM.md section 1.5b): 0 for Step and outside the
/// keys (constant behaviours), the chord slope for Linear, `(dy/ds) / (dx/ds)` for a spline
/// segment. Behaviours that repeat the keys are not unfolded (the speed is that of the
/// segment in the key range, or 0 outside it).
pub fn speed_at(track: &Track, time: f64) -> f64 {
    let keys = &track.keys;
    let tangs = resolve(&track.tangents);
    let j = keys.partition_point(|k| k.time < time);
    if j == 0 || j >= keys.len() {
        return 0.0;
    }
    let (a, b) = (&keys[j - 1], &keys[j]);
    match a.interp {
        Interp::Step => 0.0,
        Interp::Linear => (b.value - a.value) / (b.time - a.time),
        Interp::Spline => {
            let (xs, ys, s) = spline_solve(keys, &tangs, j - 1, time);
            let dx = bezier_slope(xs, s);
            if dx.abs() < 1e-300 {
                0.0
            } else {
                bezier_slope(ys, s) / dx
            }
        }
    }
}

/// The evaluated handles of key `i` as `((left dt, left dv), (right dt, right dv))` in seconds
/// and value units relative to the key (left `dt` is not positive): the automatic ease, the
/// chord or the stored user handle, exactly as the curve uses them (for drawing and editing).
pub fn handles(track: &Track, i: usize) -> Option<((f64, f64), (f64, f64))> {
    let keys = &track.keys;
    if i >= keys.len() || keys.len() < 2 {
        return None;
    }
    let t = tangent_with(keys, &resolve(&track.tangents), i);
    Some(((-t.left, t.left_dv), (t.right, t.right_dv)))
}
