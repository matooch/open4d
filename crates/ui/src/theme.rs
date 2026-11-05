//! Our own dark theme (docs/specs/UI.md §8).

use egui::{Color32, Stroke, Visuals};

/// Which palette is in use (docs/specs/UI.md §24.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ThemeKind {
    #[default]
    Dark,
    Light,
    HighContrast,
}

impl ThemeKind {
    pub const ALL: [ThemeKind; 3] = [ThemeKind::Dark, ThemeKind::Light, ThemeKind::HighContrast];

    pub fn name(self) -> &'static str {
        match self {
            ThemeKind::Dark => "dark",
            ThemeKind::Light => "light",
            ThemeKind::HighContrast => "high-contrast",
        }
    }

    pub fn from_name(name: &str) -> Option<ThemeKind> {
        ThemeKind::ALL.into_iter().find(|k| k.name() == name)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Theme {
    pub kind: ThemeKind,
    pub window: Color32,
    pub panel: Color32,
    pub faint: Color32,
    pub text: Color32,
    pub weak_text: Color32,
    pub accent: Color32,
    pub accent_text: Color32,
    pub error: Color32,
    /// Selected points, edges and polygons of the component target (EDITABLE.md §2.6).
    pub component_selection: Color32,
    /// Keyboard focus ring and IME composition underline (UI.md §24.1).
    pub focus: Color32,
}

impl Default for Theme {
    fn default() -> Self {
        Theme::of(ThemeKind::Dark)
    }
}

const fn rgb(hex: u32) -> Color32 {
    Color32::from_rgb((hex >> 16) as u8, (hex >> 8) as u8, hex as u8)
}

impl Theme {
    /// The token set of a theme (UI.md §24.1).
    pub fn of(kind: ThemeKind) -> Theme {
        match kind {
            ThemeKind::Dark => Theme {
                kind,
                window: rgb(0x1E2024),
                panel: rgb(0x25282D),
                faint: rgb(0x2E3238),
                text: rgb(0xD6D9DE),
                weak_text: rgb(0x9AA0A8),
                accent: rgb(0xE08A2E),
                accent_text: rgb(0x1A1A1A),
                error: rgb(0xE5534B),
                component_selection: rgb(0x38C6E8),
                focus: rgb(0x5AA9FF),
            },
            ThemeKind::Light => Theme {
                kind,
                window: rgb(0xF4F5F7),
                panel: rgb(0xECEEF1),
                faint: rgb(0xDDE0E5),
                text: rgb(0x1B1E22),
                weak_text: rgb(0x4A515A),
                accent: rgb(0xA64B00),
                accent_text: rgb(0xFFFFFF),
                error: rgb(0xB3261E),
                component_selection: rgb(0x006C99),
                focus: rgb(0x0B57D0),
            },
            ThemeKind::HighContrast => Theme {
                kind,
                window: rgb(0x000000),
                panel: rgb(0x000000),
                faint: rgb(0x1A1A1A),
                text: rgb(0xFFFFFF),
                weak_text: rgb(0xD8D8D8),
                accent: rgb(0xFFD400),
                accent_text: rgb(0x000000),
                error: rgb(0xFF7B72),
                component_selection: rgb(0x00E5FF),
                focus: rgb(0x5CE1FF),
            },
        }
    }

    /// The theme last applied to `ctx` (dark before the first apply).
    pub fn current(ctx: &egui::Context) -> Theme {
        ctx.data(|d| d.get_temp::<Theme>(egui::Id::unique(CTX_KEY)))
            .unwrap_or_default()
    }
}

const CTX_KEY: &str = "forge-theme";

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
        let mut v = match self.kind {
            ThemeKind::Light => Visuals::light(),
            _ => Visuals::dark(),
        };
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
        // The focused (and pressed) widget gets a ring, so keyboard focus is visible.
        v.widgets.active.bg_stroke = Stroke::new(2.0, self.focus);
        v.ime_composition.active_underline_stroke = Stroke::new(2.0, self.focus);
        v.ime_composition.inactive_underline_stroke = Stroke::new(1.0, self.focus);
        v
    }

    /// Installs the visuals and remembers the theme in `ctx`, without touching the fonts.
    pub fn apply_visuals(&self, ctx: &egui::Context) {
        ctx.set_visuals(self.visuals());
        ctx.data_mut(|d| d.insert_temp(egui::Id::unique(CTX_KEY), *self));
    }

    pub fn apply(&self, ctx: &egui::Context) {
        self.apply_visuals(ctx);
        install_fonts(ctx);
    }
}

/// Name of the bundled symbol font.
pub const SYMBOL_FONT: &str = "forge-symbols";

/// A subset of DejaVu Sans (arrows, maths, shapes, symbols; licence next to the file,
/// docs/specs/PLATFORM.md §4).
static SYMBOL_FONT_DATA: &[u8] = include_bytes!("../assets/fonts/ForgeSymbols.ttf");

/// Registers the symbol font as the last fallback of the Proportional and Monospace
/// families, so a character the default fonts lack (for example the Cmd sign) draws
/// instead of tofu or a panic under `MissingGlyphPolicy::Panic`.
pub fn install_fonts(ctx: &egui::Context) {
    install_fonts_with(ctx, system_cjk_font());
}

/// Name of the optional system CJK font (UI.md §24.5).
pub const CJK_FONT: &str = "forge-system-cjk";

/// Where a CJK-capable font usually lives, first hit wins. The font is read from the
/// system at start-up and never bundled.
const CJK_FONT_CANDIDATES: &[&str] = &[
    "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
    "/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc",
    "/usr/share/fonts/truetype/noto/NotoSansCJK-Regular.ttc",
    "/usr/share/fonts/google-noto-cjk/NotoSansCJK-Regular.ttc",
    "/usr/share/fonts/truetype/wqy/wqy-microhei.ttc",
    "/System/Library/Fonts/PingFang.ttc",
    "/System/Library/Fonts/Hiragino Sans GB.ttc",
    "C:\\Windows\\Fonts\\msyh.ttc",
    "C:\\Windows\\Fonts\\YuGothR.ttc",
];

/// The bytes of the first system CJK font found, if any.
pub fn system_cjk_font() -> Option<Vec<u8>> {
    CJK_FONT_CANDIDATES
        .iter()
        .find_map(|p| std::fs::read(p).ok())
}

/// [`install_fonts`] with an optional extra fallback font (the system CJK font), so composed
/// East Asian text draws instead of tofu.
pub fn install_fonts_with(ctx: &egui::Context, extra: Option<Vec<u8>>) {
    let mut fonts = egui::FontDefinitions::default();
    if let Some(bytes) = extra {
        fonts.font_data.insert(
            CJK_FONT.to_owned(),
            std::sync::Arc::new(egui::FontData::from_owned(bytes)),
        );
    }
    fonts.font_data.insert(
        SYMBOL_FONT.to_owned(),
        std::sync::Arc::new(egui::FontData::from_static(SYMBOL_FONT_DATA)),
    );
    for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
        fonts
            .families
            .entry(family.clone())
            .or_default()
            .push(SYMBOL_FONT.to_owned());
        if fonts.font_data.contains_key(CJK_FONT) {
            fonts
                .families
                .entry(family)
                .or_default()
                .push(CJK_FONT.to_owned());
        }
    }
    ctx.set_fonts(fonts);
}
