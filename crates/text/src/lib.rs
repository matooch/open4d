//! Font outlines and text layout for the Text spline (docs/specs/TEXT.md).
//!
//! Pure geometry: a [`Font`] turns a string into closed cubic Bezier [`Contour`]s in the
//! plane (x right, y up, baseline of the first line at y = 0). The generators crate turns
//! them into a spline.

use std::borrow::Cow;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

use skrifa::instance::{LocationRef, Size};
use skrifa::outline::{DrawSettings, OutlinePen};
use skrifa::raw::FileRef;
use skrifa::string::StringId;
use skrifa::{FontRef, GlyphId, MetadataProvider};

/// The `font` param value that selects the bundled font.
pub const DEFAULT_FONT: &str = "default";

/// The bundled default font: DejaVu Sans, subset (docs/specs/TEXT.md section 3). Its
/// licence is `fonts/LICENSE-DejaVu.txt`.
static BUNDLED: &[u8] = include_bytes!("../fonts/DejaVuSans-subset.ttf");

/// A point in the text plane.
pub type Point = [f64; 2];

/// One closed outline: cubic Bezier pieces `[p0, c1, c2, p3]`, each starting where the
/// previous one ended, the last ending at the first piece's start.
#[derive(Debug, Clone, PartialEq)]
pub struct Contour {
    pub pieces: Vec<[Point; 4]>,
}

/// Horizontal alignment of every line (section 2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Align {
    Left,
    Middle,
    Right,
}

/// The layout settings of the Text spline.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Layout {
    /// The em size.
    pub height: f64,
    pub align: Align,
    /// Added after every glyph but the last of a line.
    pub horizontal_spacing: f64,
    /// Added to the line step.
    pub vertical_spacing: f64,
}

impl Default for Layout {
    fn default() -> Layout {
        Layout {
            height: 200.0,
            align: Align::Left,
            horizontal_spacing: 0.0,
            vertical_spacing: 0.0,
        }
    }
}

/// A parsed font file (one face of it).
#[derive(Debug, Clone)]
pub struct Font {
    data: Cow<'static, [u8]>,
    index: u32,
}

impl Font {
    /// The bundled default font; it is always available.
    pub fn bundled() -> Font {
        Font {
            data: Cow::Borrowed(BUNDLED),
            index: 0,
        }
    }

    /// A font from the bytes of a TrueType/OpenType file (face `index` of a collection);
    /// `None` if they do not parse or the face has no usable metrics.
    pub fn from_data(data: Vec<u8>, index: u32) -> Option<Font> {
        let font = Font {
            data: Cow::Owned(data),
            index,
        };
        let face = font.face()?;
        (face
            .metrics(Size::unscaled(), LocationRef::default())
            .units_per_em
            > 0)
        .then_some(font)
    }

    /// The font a `font` param names: [`DEFAULT_FONT`] is the bundled one; any other value
    /// is a family looked up in the system font folders, falling back to the bundled
    /// font when it is not found (section 3).
    pub fn resolve(name: &str) -> Font {
        let name = name.trim();
        if name.is_empty() || name.eq_ignore_ascii_case(DEFAULT_FONT) {
            return Font::bundled();
        }
        Font::system(name).unwrap_or_else(Font::bundled)
    }

    /// The regular face of an installed family; looked up once per name and cached.
    pub fn system(family: &str) -> Option<Font> {
        type Cache = Mutex<HashMap<String, Option<Arc<Font>>>>;
        static CACHE: OnceLock<Cache> = OnceLock::new();
        let key = family.to_lowercase();
        let cache = CACHE.get_or_init(Default::default);
        if let Some(hit) = cache.lock().ok()?.get(&key) {
            return hit.as_deref().cloned();
        }
        let found = find_system(&key).map(Arc::new);
        cache.lock().ok()?.insert(key, found.clone());
        found.as_deref().cloned()
    }

    fn face(&self) -> Option<FontRef<'_>> {
        FontRef::from_index(&self.data, self.index).ok()
    }

    /// The family name (name id 16, else 1).
    pub fn family(&self) -> Option<String> {
        family_of(&self.face()?)
    }

    pub fn units_per_em(&self) -> f64 {
        self.face().map_or(1.0, |f| {
            f64::from(
                f.metrics(Size::unscaled(), LocationRef::default())
                    .units_per_em,
            )
        })
    }

    /// Distance between two baselines in font units: ascender - descender + line gap.
    pub fn line_height_units(&self) -> f64 {
        self.face().map_or(0.0, |f| {
            let m = f.metrics(Size::unscaled(), LocationRef::default());
            f64::from(m.ascent) - f64::from(m.descent) + f64::from(m.leading)
        })
    }

    /// The outlines of `text` (section 2), glyph by glyph, line by line, each glyph's
    /// contours in font order. Empty when nothing has an outline.
    pub fn layout(&self, text: &str, settings: &Layout) -> Vec<Contour> {
        let Some(face) = self.face() else {
            return Vec::new();
        };
        let upem = self.units_per_em().max(1.0);
        let charmap = face.charmap();
        let outlines = face.outline_glyphs();
        let advances = face.glyph_metrics(Size::unscaled(), LocationRef::default());
        let u = settings.height / upem;
        let step = self.line_height_units() * u + settings.vertical_spacing;
        let mut out = Vec::new();
        for (i, line) in text.replace("\r\n", "\n").split('\n').enumerate() {
            let base = -(i as f64) * step;
            let mut line_contours: Vec<Contour> = Vec::new();
            let mut pen = 0.0;
            for ch in line.chars() {
                let id = charmap.map(ch).unwrap_or(GlyphId::NOTDEF);
                let mut b = Collect::new(pen, base, u);
                let drawn = outlines.get(id).is_some_and(|glyph| {
                    let settings = DrawSettings::unhinted(Size::unscaled(), LocationRef::default());
                    glyph.draw(settings, &mut b).is_ok()
                });
                if drawn {
                    line_contours.extend(outers_first(b.finish()));
                }
                let advance = advances.advance_width(id).map_or(0.0, f64::from);
                pen += advance * u + settings.horizontal_spacing;
            }
            if let Some((min, max)) = ink_range(&line_contours) {
                let shift = match settings.align {
                    Align::Left => 0.0,
                    Align::Middle => -(min + max) / 2.0,
                    Align::Right => -max,
                };
                for c in &mut line_contours {
                    for piece in &mut c.pieces {
                        for p in piece.iter_mut() {
                            p[0] += shift;
                        }
                    }
                }
            }
            out.extend(line_contours);
        }
        out
    }
}

/// Signed area (counter-clockwise positive) of the polygon through a contour's piece
/// end points and mid points.
fn signed_area(c: &Contour) -> f64 {
    let pts: Vec<Point> = c
        .pieces
        .iter()
        .flat_map(|p| {
            let mid = |i: usize| cubic([p[0][i], p[1][i], p[2][i], p[3][i]], 0.5);
            [p[0], [mid(0), mid(1)]]
        })
        .collect();
    (0..pts.len())
        .map(|i| {
            let (a, b) = (pts[i], pts[(i + 1) % pts.len()]);
            a[0] * b[1] - b[0] * a[1]
        })
        .sum::<f64>()
        / 2.0
}

/// A glyph's contours with the outer ones (winding like the largest contour) before the
/// holes, each group in font order (`a`, `o`, `word`: the reference lists a letter's outline
/// before its counters whatever the font's own order).
fn outers_first(contours: Vec<Contour>) -> Vec<Contour> {
    let areas: Vec<f64> = contours.iter().map(signed_area).collect();
    let Some(biggest) = areas
        .iter()
        .copied()
        .max_by(|a, b| a.abs().total_cmp(&b.abs()))
    else {
        return contours;
    };
    let (outer, holes): (Vec<_>, Vec<_>) = contours
        .into_iter()
        .zip(areas)
        .partition(|(_, a)| a * biggest >= 0.0);
    outer.into_iter().chain(holes).map(|(c, _)| c).collect()
}

/// The x extent of the true curves (not their control polygons) of some contours.
fn ink_range(contours: &[Contour]) -> Option<(f64, f64)> {
    let mut range: Option<(f64, f64)> = None;
    for piece in contours.iter().flat_map(|c| &c.pieces) {
        let x = [piece[0][0], piece[1][0], piece[2][0], piece[3][0]];
        let mut ts = vec![0.0, 1.0];
        ts.extend(derivative_roots(x));
        for t in ts {
            let v = cubic(x, t);
            range = Some(match range {
                None => (v, v),
                Some((lo, hi)) => (lo.min(v), hi.max(v)),
            });
        }
    }
    range
}

fn cubic(p: [f64; 4], t: f64) -> f64 {
    let u = 1.0 - t;
    u * u * u * p[0] + 3.0 * u * u * t * p[1] + 3.0 * u * t * t * p[2] + t * t * t * p[3]
}

/// Parameters in (0, 1) where the derivative of the 1D cubic Bezier vanishes.
fn derivative_roots(p: [f64; 4]) -> Vec<f64> {
    // B'(t)/3 = a t^2 + b t + c
    let a = -p[0] + 3.0 * p[1] - 3.0 * p[2] + p[3];
    let b = 2.0 * (p[0] - 2.0 * p[1] + p[2]);
    let c = p[1] - p[0];
    let mut roots = Vec::new();
    if a.abs() < 1e-12 {
        if b.abs() > 1e-12 {
            roots.push(-c / b);
        }
    } else {
        let disc = b * b - 4.0 * a * c;
        if disc >= 0.0 {
            let s = disc.sqrt();
            roots.push((-b + s) / (2.0 * a));
            roots.push((-b - s) / (2.0 * a));
        }
    }
    roots.retain(|t| *t > 0.0 && *t < 1.0);
    roots
}

/// Collects a glyph's outline as closed cubic contours, placed at `(pen, base)` and scaled.
struct Collect {
    pen: f64,
    base: f64,
    scale: f64,
    contours: Vec<Contour>,
    pieces: Vec<[Point; 4]>,
    start: Point,
    at: Point,
}

impl Collect {
    fn new(pen: f64, base: f64, scale: f64) -> Collect {
        Collect {
            pen,
            base,
            scale,
            contours: Vec::new(),
            pieces: Vec::new(),
            start: [0.0; 2],
            at: [0.0; 2],
        }
    }

    fn point(&self, x: f32, y: f32) -> Point {
        [
            self.pen + f64::from(x) * self.scale,
            self.base + f64::from(y) * self.scale,
        ]
    }

    fn push(&mut self, c1: Point, c2: Point, to: Point) {
        if to == self.at && c1 == self.at && c2 == self.at {
            return;
        }
        self.pieces.push([self.at, c1, c2, to]);
        self.at = to;
    }

    fn line(&mut self, to: Point) {
        let a = self.at;
        let third = |t: f64| [a[0] + (to[0] - a[0]) * t, a[1] + (to[1] - a[1]) * t];
        self.push(third(1.0 / 3.0), third(2.0 / 3.0), to);
    }

    fn finish(mut self) -> Vec<Contour> {
        OutlinePen::close(&mut self);
        self.contours
    }
}

impl OutlinePen for Collect {
    fn move_to(&mut self, x: f32, y: f32) {
        OutlinePen::close(self);
        self.start = self.point(x, y);
        self.at = self.start;
    }

    fn line_to(&mut self, x: f32, y: f32) {
        let to = self.point(x, y);
        self.line(to);
    }

    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        let (p0, q, p2) = (self.at, self.point(x1, y1), self.point(x, y));
        let raise = |a: Point, b: Point| {
            [
                a[0] + 2.0 / 3.0 * (b[0] - a[0]),
                a[1] + 2.0 / 3.0 * (b[1] - a[1]),
            ]
        };
        self.push(raise(p0, q), raise(p2, q), p2);
    }

    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        let (c1, c2, to) = (self.point(x1, y1), self.point(x2, y2), self.point(x, y));
        self.push(c1, c2, to);
    }

    fn close(&mut self) {
        if self.pieces.is_empty() {
            return;
        }
        if self.at != self.start {
            let start = self.start;
            self.line(start);
        }
        self.contours.push(Contour {
            pieces: std::mem::take(&mut self.pieces),
        });
        self.at = self.start;
    }
}

fn family_of(face: &FontRef) -> Option<String> {
    [StringId::TYPOGRAPHIC_FAMILY_NAME, StringId::FAMILY_NAME]
        .into_iter()
        .find_map(|id| {
            face.localized_strings(id)
                .english_or_first()
                .map(|s| s.to_string())
        })
}

/// The system font folders searched by [`Font::system`].
fn font_dirs() -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = [
        "/usr/share/fonts",
        "/usr/local/share/fonts",
        "/Library/Fonts",
        "/System/Library/Fonts",
        "C:\\Windows\\Fonts",
    ]
    .iter()
    .map(PathBuf::from)
    .collect();
    if let Some(home) = std::env::var_os("HOME").map(PathBuf::from) {
        dirs.push(home.join(".local/share/fonts"));
        dirs.push(home.join(".fonts"));
        dirs.push(home.join("Library/Fonts"));
    }
    dirs
}

/// Font files under `dir` whose name contains `needle` (lower case, no spaces), sorted.
fn candidates(dir: &Path, needle: &str, depth: usize, out: &mut Vec<PathBuf>) {
    let Ok(read) = std::fs::read_dir(dir) else {
        return;
    };
    let mut entries: Vec<PathBuf> = read.flatten().map(|e| e.path()).collect();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            if depth > 0 {
                candidates(&path, needle, depth - 1, out);
            }
            continue;
        }
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .map(str::to_lowercase);
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .map(|s| s.to_lowercase().replace([' ', '-', '_'], ""));
        if matches!(ext.as_deref(), Some("ttf" | "otf" | "ttc"))
            && stem.is_some_and(|s| s.contains(needle))
        {
            out.push(path);
        }
    }
}

/// The regular face of `family` (lower case) among the installed fonts. Candidate files are
/// those whose name contains the family name; the first regular face whose family matches
/// wins, a bold or italic one is the fallback.
fn find_system(family: &str) -> Option<Font> {
    let needle = family.replace([' ', '-', '_'], "");
    let mut files = Vec::new();
    for dir in font_dirs() {
        candidates(&dir, &needle, 8, &mut files);
    }
    let mut fallback = None;
    for path in files {
        let Ok(data) = std::fs::read(&path) else {
            continue;
        };
        let count = match FileRef::new(&data) {
            Ok(FileRef::Collection(c)) => c.len(),
            _ => 1,
        };
        for index in 0..count {
            let Ok(face) = FontRef::from_index(&data, index) else {
                continue;
            };
            if family_of(&face).is_none_or(|f| f.to_lowercase() != family) {
                continue;
            }
            let attrs = face.attributes();
            let regular =
                attrs.weight.value() < 600.0 && attrs.style == skrifa::attribute::Style::Normal;
            if let Some(font) = Font::from_data(data.clone(), index) {
                if regular {
                    return Some(font);
                }
                fallback.get_or_insert(font);
            }
        }
    }
    fallback
}
