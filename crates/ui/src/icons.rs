//! Icon set and icon atlas (docs/specs/UI.md §25). The glyphs are our own simple
//! geometric SVGs in `crates/ui/icons/`; the atlas renders them headless with resvg.

use std::collections::BTreeMap;

#[path = "icon_table.rs"]
mod table;

/// Glyphs per atlas row.
pub const COLUMNS: u32 = 16;
/// Transparent border around each glyph, in pixels.
pub const PAD: u32 = 1;
/// Glyph size in points (the SVG view box).
pub const BASE: u32 = 24;

/// The glyph name for a node-type id or command id, if it has one.
pub fn icon_name(id: &str) -> Option<&'static str> {
    table::ICON_MAP
        .binary_search_by(|(k, _)| (*k).cmp(id))
        .ok()
        .map(|i| table::ICON_MAP[i].1)
}

/// The id -> glyph table, sorted by id.
pub fn icon_map() -> &'static [(&'static str, &'static str)] {
    table::ICON_MAP
}

/// All glyph names with their SVG source, sorted by name.
pub fn glyphs() -> &'static [(&'static str, &'static str)] {
    table::GLYPHS
}

/// How to draw the icon of an id.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IconRef {
    Glyph(&'static str),
    /// Fallback: the label's first letter.
    Letter(char),
}

/// The icon of `id`, or the letter fallback from `label`.
pub fn icon_ref(id: &str, label: &str) -> IconRef {
    match icon_name(id) {
        Some(n) => IconRef::Glyph(n),
        None => IconRef::Letter(
            label
                .chars()
                .find(|c| c.is_alphanumeric())
                .map(|c| c.to_uppercase().next().unwrap_or(c))
                .unwrap_or('?'),
        ),
    }
}

/// Premultiplied RGBA8 atlas of every glyph at one UI scale.
#[derive(Debug, Clone, PartialEq)]
pub struct IconAtlas {
    pub pixels_per_point: f32,
    /// Glyph size in pixels (without padding).
    pub icon_px: u32,
    /// Cell size in pixels (glyph plus padding on both sides).
    pub cell: u32,
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
    rects: BTreeMap<&'static str, [u32; 4]>,
}

impl IconAtlas {
    /// Renders all glyphs. `pixels_per_point` is clamped to 0.5..=4.
    pub fn build(pixels_per_point: f32) -> IconAtlas {
        let scale = if pixels_per_point.is_finite() {
            pixels_per_point.clamp(0.5, 4.0)
        } else {
            1.0
        };
        let icon_px = (BASE as f32 * scale).ceil() as u32;
        let cell = icon_px + 2 * PAD;
        let n = table::GLYPHS.len() as u32;
        let rows = n.div_ceil(COLUMNS).max(1);
        let (width, height) = (COLUMNS * cell, rows * cell);
        let mut pixels = vec![0u8; (width * height * 4) as usize];
        let mut rects = BTreeMap::new();
        let opt = resvg::usvg::Options::default();
        for (i, (name, svg)) in table::GLYPHS.iter().enumerate() {
            let tree = match resvg::usvg::Tree::from_str(svg, &opt) {
                Ok(t) => t,
                Err(e) => {
                    tracing::warn!("icon {name} does not parse: {e}");
                    continue;
                }
            };
            let Some(mut pm) = resvg::tiny_skia::Pixmap::new(icon_px, icon_px) else {
                continue;
            };
            let k = icon_px as f32 / BASE as f32;
            resvg::render(
                &tree,
                resvg::tiny_skia::Transform::from_scale(k, k),
                &mut pm.as_mut(),
            );
            let (cx, cy) = (
                (i as u32 % COLUMNS) * cell + PAD,
                (i as u32 / COLUMNS) * cell + PAD,
            );
            for y in 0..icon_px {
                let src = (y * icon_px * 4) as usize;
                let dst = (((cy + y) * width + cx) * 4) as usize;
                let len = (icon_px * 4) as usize;
                pixels[dst..dst + len].copy_from_slice(&pm.data()[src..src + len]);
            }
            rects.insert(*name, [cx, cy, cx + icon_px, cy + icon_px]);
        }
        IconAtlas {
            pixels_per_point: scale,
            icon_px,
            cell,
            width,
            height,
            pixels,
            rects,
        }
    }

    /// Pixel rectangle `[x0, y0, x1, y1]` of a glyph.
    pub fn rect(&self, name: &str) -> Option<[u32; 4]> {
        self.rects.get(name).copied()
    }

    /// Texture coordinates of a glyph.
    pub fn uv(&self, name: &str) -> Option<egui::Rect> {
        let [x0, y0, x1, y1] = self.rect(name)?;
        let (w, h) = (self.width as f32, self.height as f32);
        Some(egui::Rect::from_min_max(
            egui::pos2(x0 as f32 / w, y0 as f32 / h),
            egui::pos2(x1 as f32 / w, y1 as f32 / h),
        ))
    }

    pub fn to_color_image(&self) -> egui::ColorImage {
        egui::ColorImage::from_rgba_premultiplied(
            [self.width as usize, self.height as usize],
            &self.pixels,
        )
    }

    /// Uploads the atlas (linear filtering).
    pub fn texture(&self, ctx: &egui::Context) -> egui::TextureHandle {
        ctx.load_texture(
            "forge-icon-atlas",
            self.to_color_image(),
            egui::TextureOptions::LINEAR,
        )
    }
}
