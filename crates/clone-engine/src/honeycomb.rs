//! Honeycomb mode (clone-toolset spec C7).

use glam::{DMat4, DVec3};

use crate::settings::{Honeycomb, HoneycombForm, HoneycombMode, OffsetDirection, Orientation};
use crate::{CloneError, CloneItem, MAX_CLONES, SplitMix64};

pub(crate) fn generate(s: &Honeycomb) -> Result<Vec<CloneItem>, CloneError> {
    if s.form != HoneycombForm::Square {
        return Err(CloneError::Unsupported(
            "cloner honeycomb circle and spline forms",
        ));
    }
    // Lines, half-cells along each line, spacing between lines, full spacing along.
    let (lines, along, line_size, along_size) = match s.offset_direction {
        OffsetDirection::Height => (s.count_width, s.count_height, s.size_width, s.size_height),
        OffsetDirection::Width => (s.count_height, s.count_width, s.size_height, s.size_width),
    };
    if lines.saturating_mul(along.div_ceil(2)) > MAX_CLONES {
        return Err(CloneError::TooMany);
    }
    let (line_step, along_step) = match s.mode {
        HoneycombMode::PerStep => (line_size, along_size),
        HoneycombMode::EndPoint => (
            if lines > 1 {
                line_size / (lines as f64 - 1.0)
            } else {
                0.0
            },
            if along > 1 {
                2.0 * along_size / (along as f64 - 1.0)
            } else {
                0.0
            },
        ),
    };
    let mut rng = SplitMix64::new(s.seed);
    let mut out = Vec::new();
    for c in 0..lines {
        let parity = c % 2;
        let count = if parity == 0 {
            along.div_ceil(2)
        } else {
            along / 2
        };
        let line_pos = (c as f64 - (lines as f64 - 1.0) / 2.0) * line_step;
        for m in 0..count {
            let along_pos = (m as f64 + s.offset * parity as f64) * along_step
                - (along as f64 - 1.0) / 4.0 * along_step;
            let a = along_pos + s.offset_variation * along_step * rng.next_signed() / 2.0;
            let l = line_pos + s.perpendicular_variation * line_step * rng.next_signed() / 2.0;
            let (w, h) = match s.offset_direction {
                OffsetDirection::Height => (l, a),
                OffsetDirection::Width => (a, l),
            };
            let pos = match s.orientation {
                Orientation::Z => DVec3::new(w, h, 0.0),
                Orientation::X => DVec3::new(0.0, h, w),
                Orientation::Y => DVec3::new(w, 0.0, h),
            };
            let uvw = DVec3::new(
                if lines > 1 {
                    c as f64 / (lines - 1) as f64
                } else {
                    0.0
                },
                if count > 1 {
                    m as f64 / (count - 1) as f64
                } else {
                    0.0
                },
                0.0,
            );
            out.push(CloneItem::at(DMat4::from_translation(pos), uvw));
        }
    }
    Ok(out)
}
