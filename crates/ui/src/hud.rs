//! Viewport heads-up display: per-pane items, the pure draw list and safe frames
//! (docs/specs/UI.md §23). Own design, parity-pending G8. The egui painting is in `app`.

use std::collections::BTreeSet;

use forge_command::HudItemArg;
use forge_core::time::format_time;
use forge_core::{NodeId, TimeDisplay};
use forge_viewport::camera::Projection;
use forge_viewport::gizmo::GizmoMode;
use forge_viewport::panes::Pane;
use serde::{Deserialize, Serialize};

use forge_app::schedule::Frame;

use crate::state::EditorState;

/// One thing the HUD can show.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HudItem {
    Fps,
    Counts,
    Tool,
    Frame,
    Camera,
    SafeFrames,
}

impl HudItem {
    pub const ALL: [HudItem; 6] = [
        HudItem::Fps,
        HudItem::Counts,
        HudItem::Tool,
        HudItem::Frame,
        HudItem::Camera,
        HudItem::SafeFrames,
    ];

    /// The `view.hud` argument.
    pub fn id(self) -> &'static str {
        match self {
            HudItem::Fps => "fps",
            HudItem::Counts => "counts",
            HudItem::Tool => "tool",
            HudItem::Frame => "frame",
            HudItem::Camera => "camera",
            HudItem::SafeFrames => "safe_frames",
        }
    }

    /// The menu label.
    pub fn label(self) -> &'static str {
        match self {
            HudItem::Fps => "Frame rate",
            HudItem::Counts => "Counts",
            HudItem::Tool => "Tool and snap",
            HudItem::Frame => "Frame",
            HudItem::Camera => "Camera",
            HudItem::SafeFrames => "Safe frames",
        }
    }
}

impl From<HudItemArg> for HudItem {
    fn from(a: HudItemArg) -> HudItem {
        match a {
            HudItemArg::Fps => HudItem::Fps,
            HudItemArg::Counts => HudItem::Counts,
            HudItemArg::Tool => HudItem::Tool,
            HudItemArg::Frame => HudItem::Frame,
            HudItemArg::Camera => HudItem::Camera,
            HudItemArg::SafeFrames => HudItem::SafeFrames,
        }
    }
}

/// Which items one pane shows. Everything is off by default.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct HudFlags {
    pub fps: bool,
    pub counts: bool,
    pub tool: bool,
    pub frame: bool,
    pub camera: bool,
    pub safe_frames: bool,
}

impl HudFlags {
    pub fn get(&self, item: HudItem) -> bool {
        match item {
            HudItem::Fps => self.fps,
            HudItem::Counts => self.counts,
            HudItem::Tool => self.tool,
            HudItem::Frame => self.frame,
            HudItem::Camera => self.camera,
            HudItem::SafeFrames => self.safe_frames,
        }
    }

    pub fn set(&mut self, item: HudItem, on: bool) {
        match item {
            HudItem::Fps => self.fps = on,
            HudItem::Counts => self.counts = on,
            HudItem::Tool => self.tool = on,
            HudItem::Frame => self.frame = on,
            HudItem::Camera => self.camera = on,
            HudItem::SafeFrames => self.safe_frames = on,
        }
    }

    pub fn any(&self) -> bool {
        HudItem::ALL.into_iter().any(|i| self.get(i))
    }
}

/// The flags of the four panes.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct HudSettings {
    pub perspective: HudFlags,
    pub top: HudFlags,
    pub front: HudFlags,
    pub right: HudFlags,
}

impl HudSettings {
    pub fn flags(&self, pane: Pane) -> &HudFlags {
        match pane {
            Pane::Perspective => &self.perspective,
            Pane::Top => &self.top,
            Pane::Front => &self.front,
            Pane::Right => &self.right,
        }
    }

    pub fn set(&mut self, pane: Pane, item: HudItem, on: bool) {
        match pane {
            Pane::Perspective => self.perspective.set(item, on),
            Pane::Top => self.top.set(item, on),
            Pane::Front => self.front.set(item, on),
            Pane::Right => self.right.set(item, on),
        }
    }

    pub fn set_all(&mut self, item: HudItem, on: bool) {
        for p in Pane::ALL {
            self.set(p, item, on);
        }
    }
}

/// Object, point and polygon counts.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Counts {
    pub objects: usize,
    pub points: usize,
    pub polys: usize,
}

/// Everything the draw list reads.
#[derive(Debug, Clone, PartialEq)]
pub struct HudContext {
    /// Smoothed frames per second and frame time in milliseconds, once measured.
    pub fps: Option<(f64, f64)>,
    pub scene: Counts,
    pub selection: Counts,
    pub tool: String,
    pub axis_mode: bool,
    /// Snap off, or the mode label ("3D" / "2D").
    pub snap: Option<String>,
    pub frame: i64,
    pub end: i64,
    pub scene_fps: u32,
    pub display: TimeDisplay,
    pub pane: String,
    pub projection: String,
    /// Pixel size of the render, when the scene has render settings (none yet).
    pub render_size: Option<[u32; 2]>,
}

/// Which corner a text hangs from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Anchor {
    LeftTop,
    RightTop,
    RightBottom,
}

/// How a primitive is coloured: the painter maps it to the theme.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    Normal,
    Weak,
    Guide,
}

/// One drawing primitive, in pane-local points.
#[derive(Debug, Clone, PartialEq)]
pub enum HudPrim {
    Text {
        pos: [f32; 2],
        anchor: Anchor,
        text: String,
        tone: Tone,
    },
    /// A one point outline.
    Rect {
        min: [f32; 2],
        max: [f32; 2],
        tone: Tone,
    },
}

/// Left margin and first row of the left column; line height (UI.md §23.3).
pub const MARGIN: f32 = 8.0;
pub const TOP: f32 = 26.0;
pub const LINE: f32 = 14.0;

/// `1234567` as `"1,234,567"`.
pub fn format_count(n: usize) -> String {
    let digits = n.to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

/// Safe frame rectangles `[min_x, min_y, max_x, max_y]` in pane-local points.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SafeFrames {
    pub frame: [f32; 4],
    pub action: [f32; 4],
    pub title: [f32; 4],
}

fn scaled(r: [f32; 4], k: f32) -> [f32; 4] {
    let (cx, cy) = ((r[0] + r[2]) / 2.0, (r[1] + r[3]) / 2.0);
    let (hw, hh) = ((r[2] - r[0]) / 2.0 * k, (r[3] - r[1]) / 2.0 * k);
    [cx - hw, cy - hh, cx + hw, cy + hh]
}

/// The render frame fitted into the pane, with the action (90 %) and title (80 %) safe areas.
pub fn safe_frames(size: [f32; 2], render: Option<[u32; 2]>) -> SafeFrames {
    let [w, h] = size;
    let frame = match render.filter(|r| r[0] > 0 && r[1] > 0) {
        Some([rw, rh]) if w > 0.0 && h > 0.0 => {
            let aspect = rw as f32 / rh as f32;
            let (fw, fh) = if w / h > aspect {
                (h * aspect, h)
            } else {
                (w, w / aspect)
            };
            [
                (w - fw) / 2.0,
                (h - fh) / 2.0,
                (w + fw) / 2.0,
                (h + fh) / 2.0,
            ]
        }
        _ => [0.0, 0.0, w, h],
    };
    SafeFrames {
        frame,
        action: scaled(frame, 0.9),
        title: scaled(frame, 0.8),
    }
}

/// The primitives of one pane of `size` for the enabled items (UI.md §23.3).
pub fn draw_list(c: &HudContext, flags: &HudFlags, size: [f32; 2]) -> Vec<HudPrim> {
    let mut out = Vec::new();
    if flags.safe_frames {
        let s = safe_frames(size, c.render_size);
        if s.frame != [0.0, 0.0, size[0], size[1]] {
            out.push(HudPrim::Rect {
                min: [s.frame[0], s.frame[1]],
                max: [s.frame[2], s.frame[3]],
                tone: Tone::Weak,
            });
        }
        for r in [s.action, s.title] {
            out.push(HudPrim::Rect {
                min: [r[0], r[1]],
                max: [r[2], r[3]],
                tone: Tone::Guide,
            });
        }
    }
    let mut y = TOP;
    let mut left = |text: String, out: &mut Vec<HudPrim>| {
        out.push(HudPrim::Text {
            pos: [MARGIN, y],
            anchor: Anchor::LeftTop,
            text,
            tone: Tone::Normal,
        });
        y += LINE;
    };
    if flags.camera {
        left(format!("Camera  {} ({})", c.pane, c.projection), &mut out);
    }
    if flags.tool {
        let axis = if c.axis_mode { "  Axis mode" } else { "" };
        left(format!("Tool  {}{axis}", c.tool), &mut out);
        left(
            match &c.snap {
                None => "Snap  off".to_string(),
                Some(mode) => format!("Snap  on  {mode}"),
            },
            &mut out,
        );
    }
    if flags.counts {
        for (label, scene, sel) in [
            ("Objects", c.scene.objects, c.selection.objects),
            ("Points", c.scene.points, c.selection.points),
            ("Polygons", c.scene.polys, c.selection.polys),
        ] {
            left(
                format!(
                    "{label}  {}  (selected {})",
                    format_count(scene),
                    format_count(sel)
                ),
                &mut out,
            );
        }
    }
    if flags.fps {
        out.push(HudPrim::Text {
            pos: [size[0] - MARGIN, 6.0],
            anchor: Anchor::RightTop,
            text: match c.fps {
                Some((fps, ms)) => format!("{fps:.1} fps  {ms:.1} ms"),
                None => "-- fps".to_string(),
            },
            tone: Tone::Normal,
        });
    }
    if flags.frame {
        out.push(HudPrim::Text {
            pos: [size[0] - MARGIN, size[1] - 6.0],
            anchor: Anchor::RightBottom,
            text: format!(
                "Frame  {}  /  {}",
                format_time(c.frame, c.scene_fps, c.display),
                format_time(c.end, c.scene_fps, c.display)
            ),
            tone: Tone::Normal,
        });
    }
    out
}

/// Smoothed frame time (UI.md §23.5).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct FrameTimer {
    avg: Option<f64>,
}

impl FrameTimer {
    const FACTOR: f64 = 0.1;

    /// Adds a frame time in seconds; non-finite or non-positive values are ignored.
    pub fn push(&mut self, dt: f64) {
        if !dt.is_finite() || dt <= 0.0 {
            return;
        }
        self.avg = Some(match self.avg {
            None => dt,
            Some(a) => a + (dt - a) * Self::FACTOR,
        });
    }

    pub fn fps(&self) -> Option<f64> {
        self.avg.map(|a| 1.0 / a)
    }

    pub fn ms(&self) -> Option<f64> {
        self.avg.map(|a| a * 1000.0)
    }
}

/// Counts of the drawn objects of `frame` and of those among them that are selected.
pub fn counts(frame: &Frame, selected: &BTreeSet<NodeId>) -> (Counts, Counts) {
    let (mut scene, mut sel) = (Counts::default(), Counts::default());
    for id in &frame.drawn {
        let Some(out) = frame.outputs.get(id) else {
            continue;
        };
        let (mut points, mut polys) = (0, 0);
        if let Some(m) = &out.mesh {
            (points, polys) = (m.points.len(), m.polys.len());
        } else if let Some(i) = &out.instances {
            for (k, proto) in i.prototypes.iter().enumerate() {
                let n = (0..i.clones.len())
                    .filter(|c| i.clones.visible[*c] && i.clones.child[*c] as usize == k)
                    .count();
                points += n * proto.points.len();
                polys += n * proto.polys.len();
            }
        }
        let add = |c: &mut Counts| {
            c.objects += 1;
            c.points += points;
            c.polys += polys;
        };
        add(&mut scene);
        if selected.contains(id) {
            add(&mut sel);
        }
    }
    (scene, sel)
}

/// The pane's projection in words.
pub fn projection_label(p: &Projection) -> String {
    match p {
        Projection::Perspective { fov_y } => {
            format!("Perspective, fov {:.0}\u{b0}", fov_y.to_degrees())
        }
        Projection::Orthographic { .. } => "Orthographic".to_string(),
    }
}

/// Builds the context for `pane` from the editor state and the scheduled frame.
pub fn context(state: &EditorState, pane: Pane, frame: &Frame, timer: &FrameTimer) -> HudContext {
    let (scene, selection) = counts(frame, &state.selection.nodes);
    let settings = state.doc.scene().settings();
    let snap = &state.app.transform_settings.snap;
    let tool = match &state.tool {
        Some(t) => t.tool().label().to_string(),
        None => match state.gizmo_mode {
            GizmoMode::Translate => "Move",
            GizmoMode::Rotate => "Rotate",
            GizmoMode::Scale => "Scale",
        }
        .to_string(),
    };
    HudContext {
        fps: timer.fps().zip(timer.ms()),
        scene,
        selection,
        tool,
        axis_mode: state.app.transform_settings.axis_mode,
        snap: snap.enabled.then(|| snap.mode.id().to_uppercase()),
        frame: state.timeline.current,
        end: settings.end,
        scene_fps: settings.fps,
        display: state.doc.scene().time().display,
        pane: pane.label().to_string(),
        projection: projection_label(&state.pane_camera(pane).projection),
        render_size: None,
    }
}
