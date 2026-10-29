//! Track evaluation (docs/specs/ANIM.md §1).

use forge_core::{Interp, Key, Track};

/// The reference evaluates at document times rounded to whole milliseconds
/// (ANIM.md §1.4, cases `two-keys`, `mixed-interp`).
pub const TICKS_PER_SECOND: f64 = 1000.0;

/// Number of bisection halvings used to find the curve parameter (ANIM.md §1.6).
const BISECTION_STEPS: u32 = 19;

/// Rounds a time in seconds to the evaluation resolution.
pub fn quantize_time(time: f64) -> f64 {
    (time * TICKS_PER_SECOND).round() / TICKS_PER_SECOND
}

/// The value of `track` at `time` (seconds). An empty track reads 0.
pub fn value_at(track: &Track, time: f64) -> f64 {
    let keys = &track.keys;
    let (Some(first), Some(last)) = (keys.first(), keys.last()) else {
        return 0.0;
    };
    let t = effective_time(keys, time);
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
        Interp::Spline => spline_segment(keys, j - 1, t),
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
    /// Value per time unit; 0 at ends and local extremes.
    slope: f64,
}

fn tangent(keys: &[Key], i: usize) -> Tangent {
    let k = keys[i];
    if i == 0 {
        let len = (keys[1].time - k.time) / 3.0;
        return Tangent {
            left: len,
            right: len,
            slope: 0.0,
        };
    }
    if i + 1 == keys.len() {
        let len = (k.time - keys[i - 1].time) / 3.0;
        return Tangent {
            left: len,
            right: len,
            slope: 0.0,
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
        // Central difference, limited so neither handle leaves the value range of its
        // segment (no overshoot).
        let central = (n.value - p.value) / (n.time - p.time);
        let limit = (dl.abs() / len).min(dr.abs() / len);
        central.abs().min(limit).copysign(central)
    };
    Tangent {
        left: len,
        right: len,
        slope,
    }
}

fn bezier(p: [f64; 4], s: f64) -> f64 {
    let u = 1.0 - s;
    u * u * u * p[0] + 3.0 * u * u * s * p[1] + 3.0 * u * s * s * p[2] + s * s * s * p[3]
}

fn spline_segment(keys: &[Key], i: usize, t: f64) -> f64 {
    let (a, b) = (keys[i], keys[i + 1]);
    let (ta, tb) = (tangent(keys, i), tangent(keys, i + 1));
    let xs = [a.time, a.time + ta.right, b.time - tb.left, b.time];
    let ys = [
        a.value,
        a.value + ta.slope * ta.right,
        b.value - tb.slope * tb.left,
        b.value,
    ];
    // Bisection on the curve parameter; the answer is the middle of the final interval.
    // An exact tie (x(mid) == t, e.g. the mid-point of a symmetric curve) goes to the
    // upper half when mid >= 0.5 and to the lower half otherwise: the reference's handles
    // are a hair longer than a third of the span, which pushes ties away from the centre
    // (ANIM.md §1.6, cases `two-keys`, `four-keys`).
    let tie = 1e-12 * (b.time - a.time);
    let (mut lo, mut hi) = (0.0f64, 1.0f64);
    for _ in 0..BISECTION_STEPS {
        let mid = 0.5 * (lo + hi);
        let x = bezier(xs, mid);
        let up = if (x - t).abs() <= tie {
            mid >= 0.5
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
