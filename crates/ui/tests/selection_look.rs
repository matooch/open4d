//! Brief 234 (owner note N11): the viewport palette comes from the theme tokens, the selected
//! object is outlined in amber in the editor's own render path (the screenshot uses the same
//! highlight and passes as the live viewport), unselected objects are not (docs/specs/VIEWPORT.md
//! §16, UI.md §24.1). GPU tests skip without an adapter unless FORGE_REQUIRE_GPU=1.

mod common;

use forge_ui::commands::execute;
use forge_ui::theme::{Theme, ThemeKind};
use forge_ui::viewport_panel::{SELECTED_TINT_MIX, palette_from_theme, selected_fill};
use forge_viewport::render::Palette;
use serde_json::json;

fn rgb(c: egui::Color32) -> [f32; 3] {
    [c.r(), c.g(), c.b()].map(|v| f32::from(v) / 255.0)
}

#[test]
fn the_palette_takes_its_colours_from_the_theme() {
    for kind in ThemeKind::ALL {
        let t = Theme::of(kind);
        let p = palette_from_theme(&t, false);
        let n = kind.name();
        assert_eq!(
            p.background,
            rgb(t.viewport),
            "{n}: backdrop is the viewport token"
        );
        assert_eq!(
            p.selection,
            rgb(t.selection),
            "{n}: object selection is amber"
        );
        assert_eq!(p.outline, rgb(t.selection), "{n}: outline is amber");
        assert_ne!(
            p.outline_active, p.outline,
            "{n}: the active outline is stronger"
        );
        assert_eq!(p.grid_fade, 1.0, "{n}: the editor fades the grid");
        // the grid sits between the backdrop and the divider and majors are stronger than minors
        let dist = |c: [f32; 3]| (0..3).map(|i| (c[i] - p.background[i]).abs()).sum::<f32>();
        assert!(
            dist(p.grid_minor) > 0.0 && dist(p.grid_minor) < dist(p.grid_major),
            "{n}"
        );
        assert!(dist(p.grid_major) <= dist(rgb(t.divider)) + 1e-6, "{n}");
        // a component mode keeps component highlights distinct from the object outline
        let c = palette_from_theme(&t, true);
        assert_eq!(c.selection, rgb(t.component_selection), "{n}");
        assert_eq!(
            c.outline, p.outline,
            "{n}: the outline token does not change"
        );
        assert_ne!(c.selection, c.outline, "{n}");
    }
}

#[test]
fn the_active_outline_moves_away_from_the_backdrop() {
    let lum = |c: [f32; 3]| 0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2];
    let dark = palette_from_theme(&Theme::of(ThemeKind::Dark), false);
    assert!(
        lum(dark.outline_active) > lum(dark.outline),
        "brighter on dark"
    );
    let light = palette_from_theme(&Theme::of(ThemeKind::Light), false);
    assert!(
        lum(light.outline_active) < lum(light.outline),
        "darker on light"
    );
}

#[test]
fn a_selected_object_keeps_its_colour_with_a_faint_warm_tint() {
    let p = Palette::default();
    let fill = selected_fill(&p);
    assert_ne!(fill, p.object);
    for (i, f) in fill.iter().enumerate() {
        let want = p.object[i] + (p.outline[i] - p.object[i]) * SELECTED_TINT_MIX;
        assert!((f - want).abs() < 1e-6);
        // the tint stays a small part of the way: the object does not turn orange
        assert!((f - p.object[i]).abs() < (p.outline[i] - p.object[i]).abs() * 0.2);
    }
}

fn skip_without_adapter(message: &str) {
    if std::env::var("FORGE_REQUIRE_GPU").as_deref() == Ok("1") {
        panic!("FORGE_REQUIRE_GPU=1 but the screenshot failed: {message}");
    }
    eprintln!("skipping GPU test: {message}");
}

fn decode(png_bytes: &[u8]) -> (u32, u32, Vec<u8>) {
    let mut reader = png::Decoder::new(std::io::Cursor::new(png_bytes))
        .read_info()
        .expect("valid PNG");
    let mut pixels = vec![0; reader.output_buffer_size().unwrap()];
    let info = reader.next_frame(&mut pixels).unwrap();
    (info.width, info.height, pixels)
}

#[test]
fn the_editors_render_path_outlines_only_the_selected_object() {
    let mut st = common::state();
    let a = common::create(&mut st, "forge.cube");
    let b = common::create(&mut st, "forge.cube");
    execute(
        &mut st,
        "object.set_param",
        json!({"nodes": [b.0], "param": "transform.pos", "value": [500.0, 0.0, 0.0]}),
    )
    .ok();
    execute(&mut st, "view.frame_all", json!({})).unwrap();
    let (w, h) = (240u32, 160u32);
    st.app.selection.clear();
    let plain = match forge_ui::screenshot::render_png(&mut st, w, h) {
        Ok(p) => p,
        Err(e) if e.contains("no GPU") => return skip_without_adapter(&e),
        Err(e) => panic!("{e}"),
    };
    st.app.selection.click(a);
    let marked = forge_ui::screenshot::render_png(&mut st, w, h).unwrap();
    let (_, _, plain_px) = decode(&plain);
    let (_, _, marked_px) = decode(&marked);
    // a click makes the cube the active object, so its ring is the stronger amber
    let pal = Palette::default();
    let strong = pal.outline_active.map(|v| (v * 255.0).round() as u8);
    let amber = pal.outline.map(|v| (v * 255.0).round() as u8);
    let count_of = |px: &[u8], want: [u8; 3]| {
        px.chunks(4)
            .filter(|p| p[..3].iter().zip(want).all(|(a, b)| a.abs_diff(b) <= 1))
            .count()
    };
    let count = |px: &[u8]| count_of(px, amber) + count_of(px, strong);
    assert_ne!(amber, strong);
    assert_eq!(
        count(&plain_px),
        0,
        "nothing is amber while nothing is selected"
    );
    assert!(
        count_of(&marked_px, strong) > 30,
        "the active cube has a bright amber ring: {}",
        count_of(&marked_px, strong)
    );
    assert_eq!(
        count_of(&marked_px, amber),
        0,
        "only the active outline here"
    );
    // selecting the other cube moves the ring: the two images differ
    st.app.selection.click(b);
    let other = forge_ui::screenshot::render_png(&mut st, w, h).unwrap();
    assert_ne!(decode(&other).2, marked_px);
    assert!(count(&decode(&other).2) > 30);
}
