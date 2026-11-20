//! Track evaluation (docs/specs/ANIM.md §1).

use forge_core::{Interp, Key, Track};

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
    eval(&track.keys, &[], t)
}

/// Evaluates at the already resolved time `t`.
fn eval(keys: &[Key], user: &[Option<UserTangent>], t: f64) -> f64 {
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

/// The tangent of key `i`: the automatic one, each side replaced by the user's where that
/// side differs from the automatic handle.
fn tangent_with(keys: &[Key], user: &[Option<UserTangent>], i: usize) -> Tangent {
    let mut t = tangent(keys, i);
    if let Some(Some(u)) = user.get(i) {
        let (ll, rl) = (-u.left.0, u.right.0);
        if ll > 0.0 {
            t.grid_left = ll;
        }
        if rl > 0.0 {
            t.grid_right = rl;
        }
        if !is_automatic(ll, u.left.1, t.left, t.left_dv, keys, i, -1) {
            t.left = ll;
            t.left_dv = u.left.1;
            t.edited[0] = true;
        }
        if !is_automatic(rl, u.right.1, t.right, t.right_dv, keys, i, 1) {
            t.right = rl;
            t.right_dv = u.right.1;
            t.edited[1] = true;
        }
    }
    t
}

/// The value of `track` at exactly `time` seconds (no millisecond rounding), with `user[i]`
/// the user tangent of key `i` (a missing or `None` entry is automatic). ANIM.md §1.5a:
/// this is how the O38 observations were sampled.
pub fn value_at_exact(track: &Track, user: &[Option<UserTangent>], time: f64) -> f64 {
    eval(&track.keys, user, time)
}

fn spline_segment(keys: &[Key], user: &[Option<UserTangent>], i: usize, t: f64) -> f64 {
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
    // that is exact on the grid too (the centre of a symmetric curve) goes up.
    let tie = 1e-12 * (b.time - a.time);
    let (mut lo, mut hi) = (0.0f64, 1.0f64);
    for _ in 0..BISECTION_STEPS {
        let mid = 0.5 * (lo + hi);
        let x = bezier(xs, mid);
        let up = if (x - t).abs() <= tie {
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
    bezier(ys, 0.5 * (lo + hi))
}
