//! Icon set and atlas (docs/specs/UI.md §25). Headless: no window, no GPU.

use std::collections::BTreeSet;

use forge_ui::icons::{self, IconAtlas, IconRef};

fn names_in_dir() -> BTreeSet<String> {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("icons");
    std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|x| x == "svg"))
        .map(|p| p.file_stem().unwrap().to_string_lossy().into_owned())
        .collect()
}

#[test]
fn every_node_type_and_command_has_an_icon() {
    let app = forge_app::App::new_default();
    let have: BTreeSet<&str> = icons::glyphs().iter().map(|g| g.0).collect();
    let mut missing = Vec::new();
    for t in app.doc.scene().registry().types() {
        match icons::icon_name(&t.id) {
            Some(g) if have.contains(g) => {}
            _ => missing.push(t.id.clone()),
        }
    }
    for h in app.commands.iter() {
        match icons::icon_name(h.id()) {
            Some(g) if have.contains(g) => {}
            _ => missing.push(h.id().to_string()),
        }
    }
    assert!(missing.is_empty(), "no icon: {missing:?}");
}

#[test]
fn map_is_sorted_unique_and_files_match_glyphs() {
    let map = icons::icon_map();
    assert!(map.windows(2).all(|w| w[0].0 < w[1].0), "sorted, no dups");
    let used: BTreeSet<&str> = map.iter().map(|m| m.1).collect();
    let listed: BTreeSet<String> = icons::glyphs().iter().map(|g| g.0.to_string()).collect();
    let files = names_in_dir();
    assert_eq!(files, listed, "icons/ folder vs compiled glyph list");
    let unused: Vec<_> = listed
        .iter()
        .filter(|g| !used.contains(g.as_str()))
        .collect();
    assert!(unused.is_empty(), "unused glyphs: {unused:?}");
}

#[test]
fn glyph_files_share_one_form() {
    for (name, svg) in icons::glyphs() {
        assert!(
            name.chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_'),
            "{name}"
        );
        assert!(svg.contains(r#"viewBox="0 0 24 24""#), "{name} view box");
        assert!(svg.contains(r##"stroke="#fff""##), "{name} stroke");
        for bad in ["<text", "<image", "href", "<script", "<style"] {
            assert!(!svg.contains(bad), "{name} contains {bad}");
        }
    }
}

#[test]
fn atlas_has_the_documented_size_and_inked_glyphs() {
    let a = IconAtlas::build(1.0);
    let n = icons::glyphs().len() as u32;
    assert_eq!(a.icon_px, 24);
    assert_eq!(a.cell, 26);
    assert_eq!(a.width, 16 * 26);
    assert_eq!(a.height, n.div_ceil(16) * 26);
    assert_eq!(a.pixels.len() as u32, a.width * a.height * 4);
    let alpha = |x: u32, y: u32| a.pixels[((y * a.width + x) * 4 + 3) as usize];
    for (name, _) in icons::glyphs() {
        let [x0, y0, x1, y1] = a
            .rect(name)
            .unwrap_or_else(|| panic!("{name} not rendered"));
        assert_eq!((x1 - x0, y1 - y0), (24, 24));
        let ink = (y0..y1)
            .flat_map(|y| (x0..x1).map(move |x| (x, y)))
            .filter(|&(x, y)| alpha(x, y) > 0)
            .count();
        assert!(ink >= 10, "{name} has only {ink} inked pixels");
        // The padding stays empty so bilinear sampling never bleeds.
        for x in x0 - 1..=x1 {
            assert_eq!(alpha(x, y0 - 1), 0, "{name} top pad");
            assert_eq!(alpha(x, y1), 0, "{name} bottom pad");
        }
        for y in y0 - 1..=y1 {
            assert_eq!(alpha(x0 - 1, y), 0, "{name} left pad");
            assert_eq!(alpha(x1, y), 0, "{name} right pad");
        }
    }
}

#[test]
fn atlas_scales_is_deterministic_and_clamps() {
    let a1 = IconAtlas::build(1.0);
    let a2 = IconAtlas::build(2.0);
    assert_eq!(a2.icon_px, 48);
    assert_eq!(a2.cell, 50);
    assert_eq!(a2.width, 16 * 50);
    assert_eq!(a1, IconAtlas::build(1.0));
    assert_eq!(IconAtlas::build(100.0).pixels_per_point, 4.0);
    assert_eq!(IconAtlas::build(0.0).pixels_per_point, 0.5);
    assert_eq!(IconAtlas::build(f32::NAN).pixels_per_point, 1.0);
}

#[test]
fn uv_and_color_image_match_the_atlas() {
    let a = IconAtlas::build(1.5);
    let uv = a.uv("cube").unwrap();
    assert!(uv.min.x >= 0.0 && uv.max.x <= 1.0 && uv.max.y <= 1.0 && uv.area() > 0.0);
    assert!(a.uv("no_such_glyph").is_none());
    let img = a.to_color_image();
    assert_eq!(img.size, [a.width as usize, a.height as usize]);
}

#[test]
fn unknown_ids_fall_back_to_a_letter() {
    assert_eq!(
        icons::icon_ref("forge.cube", "Cube"),
        IconRef::Glyph("cube")
    );
    assert_eq!(icons::icon_ref("test.box", "box"), IconRef::Letter('B'));
    assert_eq!(icons::icon_ref("x.y", "  3D thing"), IconRef::Letter('3'));
    assert_eq!(icons::icon_ref("x.y", "--"), IconRef::Letter('?'));
    assert_eq!(icons::icon_name("nope.nothing"), None);
}
