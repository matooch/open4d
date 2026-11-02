//! Grid Array mode (clone-toolset spec C6).

use glam::{DMat4, DVec3};

use crate::settings::{Grid, GridForm, GridMode};
use crate::{CloneError, CloneItem, MAX_CLONES};

/// Tolerance of the form tests on the normalised radius.
const EPS: f64 = 1e-9;

pub(crate) fn generate(s: &Grid) -> Result<Vec<CloneItem>, CloneError> {
    if s.form == GridForm::Object {
        return Err(CloneError::Unsupported("cloner grid object form"));
    }
    let [nx, ny, nz] = s.count;
    let total = nx
        .checked_mul(ny)
        .and_then(|v| v.checked_mul(nz))
        .filter(|t| *t <= MAX_CLONES)
        .ok_or(CloneError::TooMany)?;
    let n = [nx, ny, nz];
    let size = s.size.to_array();
    // Centred coordinate of index `i` on axis `k`, in cm and normalised to [-1, 1].
    let axis = |k: usize, i: usize| -> (f64, f64) {
        let half = (n[k] as f64 - 1.0) / 2.0;
        let c = i as f64 - half;
        let step = match s.mode {
            GridMode::PerStep => size[k],
            GridMode::Endpoint if n[k] > 1 => size[k] / (n[k] as f64 - 1.0),
            GridMode::Endpoint => 0.0,
        };
        (c * step, if half > 0.0 { c / half } else { 0.0 })
    };
    let frac = |k: usize, i: usize| {
        if n[k] > 1 {
            i as f64 / (n[k] - 1) as f64
        } else {
            0.0
        }
    };
    let mut out = Vec::with_capacity(total);
    for iz in 0..nz {
        for iy in 0..ny {
            for ix in 0..nx {
                let (x, ux) = axis(0, ix);
                let (y, uy) = axis(1, iy);
                let (z, uz) = axis(2, iz);
                let r = match s.form {
                    GridForm::Cubic | GridForm::Object => None,
                    GridForm::Sphere => Some((ux * ux + uy * uy + uz * uz).sqrt()),
                    GridForm::Cylinder => Some((ux * ux + uz * uz).sqrt()),
                };
                let visible = r.is_none_or(|r| r <= 1.0 + EPS && r >= 1.0 - s.fill - EPS);
                let mut item = CloneItem::at(
                    DMat4::from_translation(DVec3::new(x, y, z)),
                    DVec3::new(frac(0, ix), frac(1, iy), frac(2, iz)),
                );
                item.visible = visible;
                out.push(item);
            }
        }
    }
    Ok(out)
}
