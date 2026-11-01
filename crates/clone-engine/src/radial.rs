//! Radial mode (clone-toolset spec C5).

use glam::{DMat4, DVec3, DVec4};

use crate::settings::{Plane, Radial};
use crate::{CloneError, CloneItem, MAX_CLONES, SplitMix64};

/// Position and axes of the clone at angle `a` on `plane` (C5 table).
fn frame(plane: Plane, radius: f64, align: bool, a: f64) -> DMat4 {
    let (s, c) = a.sin_cos();
    let (pos, x, y, z) = match plane {
        Plane::Xz => (
            DVec3::new(radius * s, 0.0, radius * c),
            DVec3::new(c, 0.0, -s),
            DVec3::Y,
            DVec3::new(s, 0.0, c),
        ),
        Plane::Zy => (
            DVec3::new(0.0, radius * c, radius * s),
            DVec3::X,
            DVec3::new(0.0, c, s),
            DVec3::new(0.0, -s, c),
        ),
        Plane::Xy => (
            DVec3::new(radius * s, radius * c, 0.0),
            DVec3::new(c, -s, 0.0),
            DVec3::new(s, c, 0.0),
            DVec3::Z,
        ),
    };
    if align {
        DMat4::from_cols(x.extend(0.0), y.extend(0.0), z.extend(0.0), pos.extend(1.0))
    } else {
        DMat4::from_cols(DVec4::X, DVec4::Y, DVec4::Z, pos.extend(1.0))
    }
}

pub(crate) fn generate(s: &Radial) -> Result<Vec<CloneItem>, CloneError> {
    if s.count > MAX_CLONES {
        return Err(CloneError::TooMany);
    }
    let n = s.count;
    let step = if n > 0 {
        (s.end_angle - s.start_angle) / n as f64
    } else {
        0.0
    };
    let mut rng = SplitMix64::new(s.offset_seed);
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let jitter = s.offset_variation * step * rng.next_signed() / 2.0;
        let a = s.start_angle + s.offset + i as f64 * step + jitter;
        out.push(CloneItem::at(
            frame(s.plane, s.radius, s.align, a),
            DVec3::new(i as f64 / n as f64, 0.0, 0.0),
        ));
    }
    Ok(out)
}
