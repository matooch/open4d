//! The Remapping tab (docs/specs/FIELDS.md F4).

use crate::FieldError;

/// Contour mode of the Remapping tab.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Contour {
    None,
    Quadratic,
    Step,
    Quantize,
    /// The Remapping spline; its rules are not derived.
    Curve,
}

/// The Remapping tab. Fractions are stored as fractions (1.0 is 100 %).
#[derive(Debug, Clone, PartialEq)]
pub struct Remap {
    pub enabled: bool,
    pub strength: f64,
    pub invert: bool,
    pub inner_offset: f64,
    pub min: f64,
    pub clamp_min: bool,
    pub max: f64,
    pub clamp_max: bool,
    pub multiplier: f64,
    pub contour: Contour,
    /// Quadratic contour curve, -1..1.
    pub curve: f64,
    pub steps: u32,
}

impl Remap {
    /// The tab with the defaults of a field whose Inner Offset default is `inner_offset`.
    pub fn with_inner_offset(inner_offset: f64) -> Remap {
        Remap {
            enabled: true,
            strength: 1.0,
            invert: false,
            inner_offset,
            min: 0.0,
            clamp_min: true,
            max: 1.0,
            clamp_max: true,
            multiplier: 1.0,
            contour: Contour::None,
            curve: 0.0,
            steps: 1,
        }
    }

    /// Refuses options whose rules are not derived.
    pub fn check(&self) -> Result<(), FieldError> {
        if self.enabled && self.contour == Contour::Curve {
            return Err(FieldError::Unsupported("remapping curve"));
        }
        Ok(())
    }

    /// Maps the ramp position `t` (1 in the core, 0 outside) to the field value.
    pub fn apply(&self, t: f64) -> f64 {
        if !self.enabled {
            return t;
        }
        let keep = 1.0 - self.inner_offset;
        let mut v = if t >= keep { 1.0 } else { t / keep };
        v = match self.contour {
            Contour::None | Contour::Curve => v,
            Contour::Quadratic => quadratic(v, self.curve),
            Contour::Step => (v * f64::from(self.steps)).ceil() / f64::from(self.steps),
            Contour::Quantize => (v * f64::from(self.steps) + 0.5).floor() / f64::from(self.steps),
        };
        if self.invert {
            v = 1.0 - v;
        }
        v = self.min + v * (self.max - self.min);
        v *= self.strength * self.multiplier;
        let (lo, hi) = (self.min.min(self.max), self.min.max(self.max));
        if self.clamp_min {
            v = v.max(lo);
        }
        if self.clamp_max {
            v = v.min(hi);
        }
        v
    }
}

/// Symmetric quadratic ease blended with the identity by `curve`; negative curves use
/// the inverse ease.
fn quadratic(v: f64, curve: f64) -> f64 {
    let ease = |v: f64| {
        if v < 0.5 {
            2.0 * v * v
        } else {
            1.0 - 2.0 * (1.0 - v) * (1.0 - v)
        }
    };
    let inverse = |v: f64| {
        if v < 0.5 {
            (v / 2.0).sqrt()
        } else {
            1.0 - ((1.0 - v) / 2.0).sqrt()
        }
    };
    if curve >= 0.0 {
        v + curve * (ease(v) - v)
    } else {
        v + (-curve) * (inverse(v) - v)
    }
}
