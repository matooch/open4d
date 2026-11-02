//! The symbol fallback font covers what the UI may draw (docs/specs/PLATFORM.md §4).

use forge_ui::theme::{SYMBOL_FONT, install_fonts};

#[test]
fn symbols_resolve_after_install_and_not_before() {
    let ctx = egui::Context::default();
    let id = egui::FontId::proportional(14.0);
    let symbols =
        "\u{2318}\u{21E7}\u{2325}\u{2190}\u{2192}\u{25B6}\u{25A0}\u{2713}\u{2022}\u{2264}\u{00B0}";
    // Run a pass so the font set exists, then ask.
    let covered = |ctx: &egui::Context| {
        let mut ok = false;
        let mut out = ctx.run_ui(Default::default(), |ui| {
            ok = ui.fonts_mut(|f| f.has_glyphs(&id, symbols));
        });
        out.textures_delta.clear();
        ok
    };
    assert!(!covered(&ctx), "egui's defaults lack these symbols");
    install_fonts(&ctx);
    assert!(covered(&ctx), "the fallback font must cover them");
    assert_eq!(SYMBOL_FONT, "forge-symbols");
}

#[test]
fn keymap_display_text_has_glyphs_on_every_os() {
    let ctx = egui::Context::default();
    install_fonts(&ctx);
    let id = egui::FontId::proportional(14.0);
    // macOS spells chords with these signs.
    let mac = "\u{2318}\u{21E7}\u{2325}\u{2303}Z";
    let mut ok = false;
    let mut out = ctx.run_ui(Default::default(), |ui| {
        ok = ui.fonts_mut(|f| f.has_glyphs(&id, mac));
    });
    out.textures_delta.clear();
    assert!(ok);
}
