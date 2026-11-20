//! Pure logic shared by the dope sheet and the value graph: the time view (pan, zoom, snap),
//! the key selection, hit tests, box select, scaling about a pivot and the track rows
//! (docs/specs/ANIM.md section 2b). Own design, parity-pending G5. No egui types: positions
//! are `[x, y]` points so unit tests need no GPU.

use forge_core::{NodeId, ParamKind, Scene, TrackTarget, Unit, parse_track_target};

/// One key of one track, addressed by its frame (what the `key.*` commands take).
#[derive(Debug, Clone, PartialEq)]
pub struct KeyId {
    pub node: u64,
    pub track: String,
    pub frame: f64,
}

impl KeyId {
    pub fn json(&self) -> serde_json::Value {
        serde_json::json!({"node": self.node, "track": self.track, "frame": self.frame})
    }
}

/// The selected keys.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct KeySelection {
    pub keys: Vec<KeyId>,
}

impl KeySelection {
    pub fn contains(&self, k: &KeyId) -> bool {
        self.keys.iter().any(|s| same(s, k))
    }

    /// Replaces the selection by `k`, or toggles `k` when `additive`.
    pub fn click(&mut self, k: KeyId, additive: bool) {
        if additive {
            if let Some(i) = self.keys.iter().position(|s| same(s, &k)) {
                self.keys.remove(i);
            } else {
                self.keys.push(k);
            }
        } else {
            self.keys = vec![k];
        }
    }

    /// Replaces the selection (`additive`: adds to it).
    pub fn set(&mut self, keys: Vec<KeyId>, additive: bool) {
        if !additive {
            self.keys.clear();
        }
        for k in keys {
            if !self.contains(&k) {
                self.keys.push(k);
            }
        }
    }

    pub fn clear(&mut self) {
        self.keys.clear();
    }

    pub fn json(&self) -> serde_json::Value {
        serde_json::Value::Array(self.keys.iter().map(KeyId::json).collect())
    }

    /// The selection after the keys moved by `delta` frames (so it keeps pointing at them).
    pub fn moved(&self, delta: f64) -> KeySelection {
        KeySelection {
            keys: self
                .keys
                .iter()
                .map(|k| KeyId {
                    frame: k.frame + delta,
                    ..k.clone()
                })
                .collect(),
        }
    }
}

fn same(a: &KeyId, b: &KeyId) -> bool {
    a.node == b.node && a.track == b.track && (a.frame - b.frame).abs() < 1e-6
}

/// Horizontal view: the frame at the left edge, points per frame, snapping.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TimeView {
    pub start: f64,
    pub ppf: f64,
    pub snap: bool,
}

impl Default for TimeView {
    fn default() -> Self {
        TimeView {
            start: -5.0,
            ppf: 8.0,
            snap: true,
        }
    }
}

impl TimeView {
    pub fn to_x(&self, frame: f64, x0: f32) -> f32 {
        x0 + ((frame - self.start) * self.ppf) as f32
    }

    pub fn to_frame(&self, x: f32, x0: f32) -> f64 {
        self.start + f64::from(x - x0) / self.ppf
    }

    /// Whole frames when snapping is on.
    pub fn snapped(&self, frame: f64) -> f64 {
        if self.snap { frame.round() } else { frame }
    }

    pub fn pan(&mut self, dx_points: f32) {
        self.start -= f64::from(dx_points) / self.ppf;
    }

    /// Zooms by `factor`, keeping the frame under `x` fixed.
    pub fn zoom_about(&mut self, x: f32, x0: f32, factor: f64) {
        let f = self.to_frame(x, x0);
        self.ppf = (self.ppf * factor).clamp(0.5, 400.0);
        self.start = f - f64::from(x - x0) / self.ppf;
    }

    /// Fits `first..=last` into `width` points with a margin.
    pub fn frame_range(&mut self, first: f64, last: f64, width: f32) {
        let span = (last - first).max(1.0);
        self.ppf = (f64::from(width) * 0.9 / span).clamp(0.5, 400.0);
        self.start = first - span * 0.05;
    }
}

/// Vertical view of the value graph: the value at the top edge, points per value unit.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ValueView {
    pub top: f64,
    pub ppu: f64,
}

impl Default for ValueView {
    fn default() -> Self {
        ValueView {
            top: 110.0,
            ppu: 2.0,
        }
    }
}

impl ValueView {
    pub fn to_y(&self, v: f64, y0: f32) -> f32 {
        y0 + ((self.top - v) * self.ppu) as f32
    }

    pub fn to_value(&self, y: f32, y0: f32) -> f64 {
        self.top - f64::from(y - y0) / self.ppu
    }

    pub fn pan(&mut self, dy_points: f32) {
        self.top += f64::from(dy_points) / self.ppu;
    }

    pub fn zoom_about(&mut self, y: f32, y0: f32, factor: f64) {
        let v = self.to_value(y, y0);
        self.ppu = (self.ppu * factor).clamp(1e-4, 1e5);
        self.top = v + f64::from(y - y0) / self.ppu;
    }

    /// Fits `lo..=hi` into `height` points with a margin.
    pub fn frame_range(&mut self, lo: f64, hi: f64, height: f32) {
        let span = (hi - lo).max(1e-6);
        self.ppu = (f64::from(height) * 0.8 / span).clamp(1e-4, 1e5);
        self.top = hi + span * 0.1;
    }
}

/// The nearest point within `radius` of `pos`.
pub fn hit_test(points: &[(KeyId, [f32; 2])], pos: [f32; 2], radius: f32) -> Option<usize> {
    points
        .iter()
        .enumerate()
        .map(|(i, (_, p))| (i, (p[0] - pos[0]).hypot(p[1] - pos[1])))
        .filter(|(_, d)| *d <= radius)
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(i, _)| i)
}

/// The keys whose point lies in the rectangle spanned by the corners `a` and `b`.
pub fn box_select(points: &[(KeyId, [f32; 2])], a: [f32; 2], b: [f32; 2]) -> Vec<KeyId> {
    let (x0, x1) = (a[0].min(b[0]), a[0].max(b[0]));
    let (y0, y1) = (a[1].min(b[1]), a[1].max(b[1]));
    points
        .iter()
        .filter(|(_, p)| (x0..=x1).contains(&p[0]) && (y0..=y1).contains(&p[1]))
        .map(|(k, _)| k.clone())
        .collect()
}

/// A frame scaled about `pivot` by `factor`.
pub fn scale_about(frame: f64, pivot: f64, factor: f64) -> f64 {
    pivot + (frame - pivot) * factor
}

/// A whole-frame (or free) move of `dx` points as frames.
pub fn drag_frames(view: &TimeView, dx_points: f32) -> f64 {
    let d = f64::from(dx_points) / view.ppf;
    if view.snap { d.round() } else { d }
}

/// What a row of the track tree is.
#[derive(Debug, Clone, PartialEq)]
pub enum RowKind {
    /// An object (summary row: every key of its tracks).
    Object,
    /// One track of the object.
    Track { track: String, unit: Unit },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Row {
    pub node: NodeId,
    pub label: String,
    pub kind: RowKind,
    /// Key frames of the row (summary rows: the union, sorted, without duplicates).
    pub frames: Vec<f64>,
}

/// The tree rows for the animated nodes of `scene`: per object a summary row, then its
/// tracks. With a non-empty `only`, just those nodes. Point tracks are listed (their keys are
/// editable) but have no value curve.
pub fn rows(scene: &Scene, only: &[NodeId]) -> Vec<Row> {
    let fps = f64::from(scene.settings().fps);
    let mut out = Vec::new();
    for id in forge_anim::track_nodes(scene) {
        if !only.is_empty() && !only.contains(&id) {
            continue;
        }
        let Some(node) = scene.node(id) else { continue };
        let ty = scene.registry().get(&node.type_id);
        let mut tracks = Vec::new();
        let mut all: Vec<f64> = Vec::new();
        for (key, t) in &node.tracks {
            let frames: Vec<f64> = t.keys.iter().map(|k| k.time * fps).collect();
            all.extend(&frames);
            let unit = ty
                .and_then(|ty| {
                    let target = parse_track_target(key, |p| ty.find_param(p)).ok()?;
                    let p = match target {
                        TrackTarget::Float(p)
                        | TrackTarget::Component(p, _)
                        | TrackTarget::Int(p) => p,
                        _ => return None,
                    };
                    ty.find_param(&p).map(|d| d.ui.unit)
                })
                .unwrap_or_default();
            tracks.push(Row {
                node: id,
                label: track_label(key),
                kind: RowKind::Track {
                    track: key.clone(),
                    unit,
                },
                frames,
            });
        }
        all.sort_by(f64::total_cmp);
        all.dedup_by(|a, b| (*a - *b).abs() < 1e-6);
        out.push(Row {
            node: id,
            label: node.name.clone(),
            kind: RowKind::Object,
            frames: all,
        });
        out.extend(tracks);
    }
    out
}

/// A readable name for a track key: `transform.pos.x` becomes `Position . X`.
pub fn track_label(key: &str) -> String {
    let (base, axis) = match key.rsplit_once('.') {
        Some((b, a)) if matches!(a, "x" | "y" | "z" | "r" | "g" | "b") => (b, Some(a)),
        _ => (key, None),
    };
    let base = match base {
        "transform.pos" => "Position".to_string(),
        "transform.rot" => "Rotation".to_string(),
        "transform.scale" => "Scale".to_string(),
        other => other.rsplit('.').next().unwrap_or(other).replace('_', " "),
    };
    match axis {
        Some(a) => format!("{base} . {}", a.to_uppercase()),
        None => base,
    }
}

/// Display value of a stored track value: degrees for angles, percent as shown.
pub fn display(unit: Unit, v: f64) -> f64 {
    match unit {
        Unit::Angle => v.to_degrees(),
        Unit::Percent => v * 100.0,
        _ => v,
    }
}

/// Stored value of a display value.
pub fn stored(unit: Unit, v: f64) -> f64 {
    match unit {
        Unit::Angle => v.to_radians(),
        Unit::Percent => v / 100.0,
        _ => v,
    }
}

/// Curve colour of a track by axis: X red, Y green, Z blue (the reference's convention), grey
/// otherwise.
pub fn axis_color(track: &str) -> [u8; 3] {
    match track.rsplit('.').next() {
        Some("x" | "r") => [230, 80, 80],
        Some("y" | "g") => [90, 200, 100],
        Some("z" | "b") => [90, 140, 240],
        _ => [200, 200, 200],
    }
}

/// Whether the track of `track` on `scene`'s node `id` is numeric and has a value curve.
pub fn has_curve(scene: &Scene, id: NodeId, track: &str) -> bool {
    let Some(ty) = scene
        .node(id)
        .and_then(|n| scene.registry().get(&n.type_id))
    else {
        return false;
    };
    match parse_track_target(track, |p| ty.find_param(p)) {
        Ok(
            TrackTarget::Float(_)
            | TrackTarget::Component(..)
            | TrackTarget::Int(_)
            | TrackTarget::Channel(..),
        ) => true,
        Ok(TrackTarget::Bool(p) | TrackTarget::Enum(p) | TrackTarget::Link(p)) => ty
            .find_param(&p)
            .is_some_and(|d| !matches!(d.kind, ParamKind::Mesh)),
        _ => false,
    }
}
