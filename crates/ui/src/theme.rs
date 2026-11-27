//! Our own themes: colour roles for state, time, selection and recording (docs/specs/UI.md §8, §24.1).

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

/// Button sizes of the bars and strips, in points (observed, G10 item 1, at scale 1): the
/// command bar's buttons are about 27 x 26, the vertical palettes' 34 x 34. Shared by every
/// theme.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ButtonMetrics {
    pub command: [f32; 2],
    pub palette: [f32; 2],
}

pub const BUTTONS: ButtonMetrics = ButtonMetrics {
    command: [27.0, 26.0],
    palette: [34.0, 34.0],
};

/// A colour role (docs/specs/UI.md §24.1, owner note N11). Every use of a role colour in the UI
/// and the viewport has exactly one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Role {
    /// Active tool, mode, checked toggle, active tab (mint).
    State,
    /// The playhead, the current frame, key diamonds around it (mint).
    Time,
    /// What is selected, everywhere (amber).
    Selection,
    /// Recording and errors (red).
    Record,
    /// Keyboard focus ring, IME underline.
    Focus,
    /// Selected points, edges and polygons of the component target.
    Component,
}

impl Role {
    pub const ALL: [Role; 6] = [
        Role::State,
        Role::Time,
        Role::Selection,
        Role::Record,
        Role::Focus,
        Role::Component,
    ];
}

/// The non-colour cue that keeps a role legible in greyscale.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cue {
    /// A filled or checked control.
    Fill,
    /// An outline or ring.
    Outline,
    /// An underline.
    Underline,
    /// A line, band or bar.
    Line,
    /// A larger or thicker mark.
    Size,
    /// A word or label next to the colour.
    Text,
}

/// One widget that draws a role colour.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RoleUse {
    pub role: Role,
    pub widget: &'static str,
    pub cue: Cue,
}

const fn use_of(role: Role, widget: &'static str, cue: Cue) -> RoleUse {
    RoleUse { role, widget, cue }
}

/// The widgets per role (UI.md §24.1, "Role audit"). The old single `accent` is gone: each of its
/// uses is one entry here.
pub fn role_audit() -> &'static [RoleUse] {
    use Cue::*;
    use Role::*;
    const AUDIT: &[RoleUse] = &[
        use_of(
            State,
            "active tool, mode and toggle buttons (checked fill)",
            Fill,
        ),
        use_of(
            State,
            "active attribute tab and active layout tab",
            Underline,
        ),
        use_of(
            State,
            "hyperlinks and the history panel's current step",
            Text,
        ),
        use_of(State, "console result lines and the HUD guide text", Text),
        use_of(State, "command palette shortcut hints", Text),
        use_of(
            State,
            "active viewport pane outline in the four-view layout",
            Outline,
        ),
        use_of(
            State,
            "object manager drop target (row fill and outline, insertion line)",
            Outline,
        ),
        use_of(State, "layer manager toggles that are on", Fill),
        use_of(Time, "timeline playhead and current-frame line", Line),
        use_of(
            Time,
            "timeline key ticks and preview range band and handles",
            Line,
        ),
        use_of(Time, "dope sheet and F-curve current-frame line", Line),
        use_of(
            Time,
            "key diamond: key at this frame and animated elsewhere",
            Fill,
        ),
        use_of(
            Selection,
            "viewport selected object outline and wire edges",
            Outline,
        ),
        use_of(
            Selection,
            "object manager selected row (fill, left bar) and name",
            Line,
        ),
        use_of(
            Selection,
            "dope sheet and F-curve selected keys (larger)",
            Size,
        ),
        use_of(
            Selection,
            "box-select rectangles (dope sheet, F-curve, node canvas)",
            Outline,
        ),
        use_of(Selection, "selected nodes in the node canvas", Outline),
        use_of(Selection, "selected UV points and joint list rows", Fill),
        use_of(Record, "auto-key record button while recording", Text),
        use_of(
            Record,
            "errors in the console, scripts, key map and palette",
            Text,
        ),
        use_of(Record, "UV seam edges", Size),
        use_of(Focus, "keyboard focus ring and IME underline", Outline),
        use_of(
            Component,
            "selected components of the edit target in the viewport",
            Fill,
        ),
    ];
    AUDIT
}

/// The colour tokens of a theme (docs/specs/UI.md §24.1, revised by brief 234). Roles, not one
/// accent: mint marks state and time, amber marks selection, red marks recording and errors.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Theme {
    pub kind: ThemeKind,
    pub window: Color32,
    pub panel: Color32,
    /// The canvas backdrop of the 3D viewport.
    pub viewport: Color32,
    /// Recessed controls: text fields and drag values.
    pub input: Color32,
    pub faint: Color32,
    /// Panel edges and quiet grid lines.
    pub divider: Color32,
    pub text: Color32,
    pub weak_text: Color32,
    /// Active tool, mode, checked toggle and active tab (mint).
    pub state: Color32,
    /// Text on a `state` fill.
    pub state_text: Color32,
    /// The playhead and keys around it (mint; equal to `state`).
    pub time: Color32,
    /// Selected objects, rows, keys and nodes (amber).
    pub selection: Color32,
    /// Text on a `selection` fill.
    pub selection_text: Color32,
    /// Recording (red).
    pub record: Color32,
    /// Errors (red; equal to `record`).
    pub error: Color32,
    /// Selected points, edges and polygons of the component target (EDITABLE.md §2.6).
    pub component_selection: Color32,
    /// Keyboard focus ring and IME composition underline (UI.md §24.1).
    pub focus: Color32,
    /// A parameter key indicator with a key at this frame and its hollow form (UI.md §12); the
    /// time colour.
    pub key: Color32,
    /// A parameter key indicator with an unkeyed draft (UI.md §12). A violet, not amber (amber
    /// means selected); its hue differs from `key` and `selection` by at least 30 degrees in
    /// every theme (tested).
    pub modified: Color32,
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
                window: rgb(0x171B21),
                panel: rgb(0x20262E),
                viewport: rgb(0x252C35),
                input: rgb(0x181D24),
                faint: rgb(0x2A323C),
                divider: rgb(0x343D49),
                text: rgb(0xE8EDF2),
                weak_text: rgb(0xA7B2C0),
                state: rgb(0x73D6BC),
                state_text: rgb(0x0E221C),
                time: rgb(0x73D6BC),
                selection: rgb(0xF3B36B),
                selection_text: rgb(0x2A1805),
                record: rgb(0xF17A80),
                error: rgb(0xF17A80),
                component_selection: rgb(0x38C6E8),
                focus: rgb(0x5AA9FF),
                key: rgb(0x73D6BC),
                modified: rgb(0xB79CF2),
            },
            ThemeKind::Light => Theme {
                kind,
                window: rgb(0xEEF1F5),
                panel: rgb(0xE6EAF0),
                viewport: rgb(0xD9DFE7),
                input: rgb(0xFFFFFF),
                faint: rgb(0xD8DEE6),
                divider: rgb(0xB4BEC9),
                text: rgb(0x161B22),
                weak_text: rgb(0x465162),
                state: rgb(0x0B6B57),
                state_text: rgb(0xFFFFFF),
                time: rgb(0x0B6B57),
                selection: rgb(0x8F4C00),
                selection_text: rgb(0xFFFFFF),
                record: rgb(0xB3202E),
                error: rgb(0xB3202E),
                component_selection: rgb(0x006C99),
                focus: rgb(0x0B57D0),
                key: rgb(0x0B6B57),
                modified: rgb(0x6B3FC4),
            },
            ThemeKind::HighContrast => Theme {
                kind,
                window: rgb(0x000000),
                panel: rgb(0x000000),
                viewport: rgb(0x0A0A0A),
                input: rgb(0x000000),
                faint: rgb(0x1A1A1A),
                divider: rgb(0xBFBFBF),
                text: rgb(0xFFFFFF),
                weak_text: rgb(0xD8D8D8),
                state: rgb(0x5CFFD8),
                state_text: rgb(0x000000),
                time: rgb(0x5CFFD8),
                selection: rgb(0xFFC266),
                selection_text: rgb(0x000000),
                record: rgb(0xFF8A90),
                error: rgb(0xFF8A90),
                component_selection: rgb(0x00E5FF),
                focus: rgb(0x5CE1FF),
                key: rgb(0x5CFFD8),
                modified: rgb(0xD2B0FF),
            },
        }
    }

    /// The colour of a role.
    pub fn role_colour(&self, role: Role) -> Color32 {
        match role {
            Role::State => self.state,
            Role::Time => self.time,
            Role::Selection => self.selection,
            Role::Record => self.record,
            Role::Focus => self.focus,
            Role::Component => self.component_selection,
        }
    }

    /// Runs `add` with egui's selection colours set to the selection role, for lists whose
    /// rows are selected things (egui's default `selection` is the state colour, which marks
    /// modes and tabs).
    pub fn selection_scope<R>(&self, ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
        let (fill, stroke) = {
            let s = &ui.visuals().selection;
            (s.bg_fill, s.stroke)
        };
        {
            let s = &mut ui.visuals_mut().selection;
            s.bg_fill = self.selection;
            s.stroke = Stroke::new(1.0, self.selection_text);
        }
        let out = add(ui);
        let s = &mut ui.visuals_mut().selection;
        s.bg_fill = fill;
        s.stroke = stroke;
        out
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
        v.extreme_bg_color = self.input;
        v.faint_bg_color = self.faint;
        v.error_fg_color = self.error;
        v.hyperlink_color = self.state;
        // egui's checked buttons and selectable labels are modes and tabs: the state colour.
        // Selected things use `selection` explicitly (`selection_scope`).
        v.selection.bg_fill = self.state;
        v.selection.stroke = Stroke::new(1.0, self.state_text);
        v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, self.divider);
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
