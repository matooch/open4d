//! Tangent presets and handle operations (docs/specs/ANIM.md section 1.5b). Own design:
//! the O38 observations only fix how stored handles are evaluated.

use forge_core::{Handle, Key, KeyTangent, TangentMode};

/// A third of the span to the neighbour on `side` (-1 previous, 1 next), 0 past the ends.
fn third(keys: &[Key], i: usize, side: i32) -> f64 {
    let j = if side < 0 {
        i.checked_sub(1)
    } else {
        Some(i + 1).filter(|j| *j < keys.len())
    };
    j.map_or(0.0, |j| (keys[j].time - keys[i].time).abs() / 3.0)
}

fn user(dt: f64, dv: f64) -> Handle {
    Handle {
        mode: TangentMode::User,
        dt,
        dv,
    }
}

/// Flat handles (value offset 0) a third of the adjacent spans long on both sides: the ease
/// in and out preset.
pub fn ease(keys: &[Key], i: usize) -> KeyTangent {
    KeyTangent {
        left: user(-third(keys, i, -1), 0.0),
        right: user(third(keys, i, 1), 0.0),
        ..KeyTangent::default()
    }
}

/// Ease in: the left handle flat, the right side as `current`.
pub fn ease_in(keys: &[Key], i: usize, current: KeyTangent) -> KeyTangent {
    KeyTangent {
        left: ease(keys, i).left,
        ..current
    }
}

/// Ease out: the right handle flat, the left side as `current`.
pub fn ease_out(keys: &[Key], i: usize, current: KeyTangent) -> KeyTangent {
    KeyTangent {
        right: ease(keys, i).right,
        ..current
    }
}

/// Zero angle: both handles keep their length but become flat (`dv = 0`). Automatic sides
/// become user handles of the automatic length.
pub fn zero_angle(keys: &[Key], i: usize, current: KeyTangent) -> KeyTangent {
    let flat = |h: Handle, side: i32| {
        let len = if h.mode == TangentMode::User {
            h.dt.abs()
        } else {
            third(keys, i, side)
        };
        user(f64::from(side) * len, 0.0)
    };
    KeyTangent {
        left: flat(current.left, -1),
        right: flat(current.right, 1),
        ..current
    }
}

/// Zero length: both handle ends sit on the key.
pub fn zero_length(current: KeyTangent) -> KeyTangent {
    KeyTangent {
        left: user(0.0, 0.0),
        right: user(0.0, 0.0),
        ..current
    }
}

/// Moves the right handle end to `(dt, dv)` (seconds, value). With `broken` false the left
/// handle mirrors the right one (same slope, and the same length unless `weighted`);
/// with `broken` true the left side is untouched. Unweighted handles keep a third of the
/// span as their length.
pub fn set_right_handle(
    keys: &[Key],
    i: usize,
    current: KeyTangent,
    dt: f64,
    dv: f64,
) -> KeyTangent {
    let mut t = current;
    let len = if t.weighted {
        dt.max(0.0)
    } else {
        third(keys, i, 1)
    };
    let slope = if dt > 0.0 { dv / dt } else { 0.0 };
    t.right = user(len, slope * len);
    if !t.broken {
        let llen = if t.weighted { len } else { third(keys, i, -1) };
        t.left = user(-llen, -slope * llen);
    }
    t
}
