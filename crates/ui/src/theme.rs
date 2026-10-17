//! Our own dark theme (docs/specs/UI.md §8).

use egui::{Color32, Stroke, Visuals};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Theme {
    pub window: Color32,
    pub panel: Color32,
    pub faint: Color32,
    pub text: Color32,
    pub weak_text: Color32,
    pub accent: Color32,
    pub accent_text: Color32,
    pub error: Color32,
}

impl Default for Theme {
    fn default() -> Self {
        Theme {
            window: Color32::from_rgb(0x1E, 0x20, 0x24),
            panel: Color32::from_rgb(0x25, 0x28, 0x2D),
            faint: Color32::from_rgb(0x2E, 0x32, 0x38),
            text: Color32::from_rgb(0xD6, 0xD9, 0xDE),
            weak_text: Color32::from_rgb(0x9A, 0xA0, 0xA8),
            accent: Color32::from_rgb(0xE0, 0x8A, 0x2E),
            accent_text: Color32::from_rgb(0x1A, 0x1A, 0x1A),
            error: Color32::from_rgb(0xE5, 0x53, 0x4B),
        }
    }
}

/// WCAG relative luminance of an sRGB colour.
fn luminance(c: Color32) -> f64 {
    let lin = |v: u8| {
        let s = v as f64 / 255.0;
        if s <= 0.04045 {
            s / 12.92
        } else {
            ((s + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * lin(c.r()) + 0.7152 * lin(c.g()) + 0.0722 * lin(c.b())
}

/// WCAG contrast ratio, 1–21.
pub fn contrast(a: Color32, b: Color32) -> f64 {
    let (la, lb) = (luminance(a), luminance(b));
    (la.max(lb) + 0.05) / (la.min(lb) + 0.05)
}

impl Theme {
    pub fn visuals(&self) -> Visuals {
        let mut v = Visuals::dark();
        v.override_text_color = Some(self.text);
        v.window_fill = self.window;
        v.panel_fill = self.panel;
        v.extreme_bg_color = self.window;
        v.faint_bg_color = self.faint;
        v.error_fg_color = self.error;
        v.hyperlink_color = self.accent;
        v.selection.bg_fill = self.accent;
        v.selection.stroke = Stroke::new(1.0, self.accent_text);
        v.widgets.noninteractive.bg_fill = self.panel;
        v.widgets.inactive.bg_fill = self.faint;
        v.widgets.inactive.weak_bg_fill = self.faint;
        v
    }

    pub fn apply(&self, ctx: &egui::Context) {
        ctx.set_visuals(self.visuals());
    }
}
