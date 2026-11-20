//! Linear mode (clone-toolset spec C4).

use glam::{DMat4, DVec3};

use crate::settings::{Linear, LinearMode, StepMode};
use crate::{CloneError, CloneItem, MAX_CLONES, hpb};

pub(crate) fn generate(s: &Linear) -> Result<Vec<CloneItem>, CloneError> {
    if s.count > MAX_CLONES {
        return Err(CloneError::TooMany);
    }
    let n = s.count;
    // An integral Offset prepends that many hidden clones (C4, `linear-offset`); the visible
    // ones keep `t = i + offset`. Other offsets only shift `t`.
    let lead = if s.offset >= 1.0 && s.offset.fract() == 0.0 {
        s.offset as usize
    } else {
        0
    };
    if n + lead > MAX_CLONES {
        return Err(CloneError::TooMany);
    }
    let shift = if lead > 0 { 0.0 } else { s.offset };
    let mut out = Vec::with_capacity(n + lead);
    for i in 0..n + lead {
        let t = i as f64 + shift;
        let u = if n > 1 { t / (n - 1) as f64 } else { 0.0 };
        let k = match s.mode {
            LinearMode::PerStep => t,
            LinearMode::EndPoint => u,
        } * s.amount;
        let steps = match s.step_mode {
            StepMode::Single => t,
            StepMode::Cumulative => t * (t + 1.0) / 2.0,
        };
        let position = s.position * k;
        let scale = DVec3::ONE + (s.scale - DVec3::ONE) * k;
        let rotation = s.rotation * k + s.step_rotation * (steps * s.step_size);
        let matrix = DMat4::from_translation(position) * hpb(rotation) * DMat4::from_scale(scale);
        let mut item = CloneItem::at(matrix, DVec3::new(u, 0.0, 0.0));
        item.visible = i >= lead;
        out.push(item);
    }
    Ok(out)
}
