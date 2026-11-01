//! Number scrubbing (own design, **parity-pending G4**).
//!
//! Dragging a number field changes it by `step` display units per pixel. Shift makes the
//! step 10 times finer, Ctrl 10 times coarser; any other combination leaves it unchanged.
//! Up/Down arrow keys in a focused field add one step. Pure maths, no egui state.

use forge_core::Unit;

pub const FINE: f64 = 0.1;
pub const COARSE: f64 = 10.0;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Mods {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
}

impl Mods {
    pub fn of(m: &egui::Modifiers) -> Mods {
        Mods {
            shift: m.shift,
            ctrl: m.ctrl,
            alt: m.alt,
        }
    }
}

/// The step multiplier of the held modifiers.
pub fn factor(m: Mods) -> f64 {
    match (m.shift, m.ctrl, m.alt) {
        (true, false, false) => FINE,
        (false, true, false) => COARSE,
        _ => 1.0,
    }
}

/// Display units per pixel before modifiers: the parameter's own step, else by kind/unit.
pub fn base_step(unit: Unit, is_int: bool, hint: Option<f64>) -> f64 {
    if let Some(h) = hint.filter(|h| h.is_finite() && *h > 0.0) {
        return h;
    }
    if is_int {
        return 1.0;
    }
    match unit {
        Unit::Angle | Unit::Percent => 0.1,
        _ => 0.01,
    }
}

/// The step in effect with `m` held.
pub fn step(base: f64, m: Mods) -> f64 {
    base * factor(m)
}

/// The speed to give egui's drag value: egui itself divides by 10 while Shift alone is held
/// during a drag, so that case is compensated to land on [`step`].
pub fn drag_speed(base: f64, m: Mods) -> f64 {
    let shift_only = m.shift && !m.ctrl && !m.alt;
    if shift_only { base } else { step(base, m) }
}

/// Value after dragging `pixels` (positive = right/up) from `start`.
pub fn scrubbed(start: f64, pixels: f64, base: f64, m: Mods) -> f64 {
    start + pixels * step(base, m)
}
