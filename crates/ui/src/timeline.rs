//! The timeline model lives in `forge-command` so commands can drive it headless; this
//! module draws the time controls and the frame strip with the preview range and the
//! markers (docs/specs/UI.md §7, ANIM.md §2.9). Own design, parity-pending G5. Every edit is
//! returned as a registry command; nothing here touches the document.

use egui::{Align2, Color32, FontId, Pos2, Rect, Sense, Stroke, Ui, vec2};
use forge_core::time::{color_hex, format_time};
use forge_core::{Scene, TimeDisplay};
use serde_json::json;

pub use forge_command::Timeline;

use crate::theme::Theme;

/// A command id with its arguments.
pub type Cmd = (&'static str, serde_json::Value);

/// Transport navigation order, shared by Timeline, Dope Sheet and F-Curve.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TransportStep {
    First,
    PreviousKey,
    PreviousFrame,
    Play,
    NextFrame,
    NextKey,
    Last,
}

pub const NAVIGATION: [TransportStep; 7] = [
    TransportStep::First,
    TransportStep::PreviousKey,
    TransportStep::PreviousFrame,
    TransportStep::Play,
    TransportStep::NextFrame,
    TransportStep::NextKey,
    TransportStep::Last,
];

impl TransportStep {
    pub fn presentation(self) -> (&'static str, &'static str, &'static str) {
        match self {
            Self::First => (
                "range_start",
                "Go to Start",
                "Jump to the first project frame",
            ),
            Self::PreviousKey => (
                "previous_key",
                "Previous Key",
                "Jump to the previous key on the selected objects",
            ),
            Self::PreviousFrame => (
                "previous_frame",
                "Previous Frame",
                "Move backward one frame",
            ),
            Self::Play => ("play", "Play", "Play from the current frame"),
            Self::NextFrame => ("next_frame", "Next Frame", "Move forward one frame"),
            Self::NextKey => (
                "next_key",
                "Next Key",
                "Jump to the next key on the selected objects",
            ),
            Self::Last => ("range_end", "Go to End", "Jump to the last project frame"),
        }
    }
}

pub(crate) fn navigation_command(step: TransportStep, first: i64, last: i64) -> Cmd {
    match step {
        TransportStep::First => ("timeline.set_frame", json!({"frame": first})),
        TransportStep::PreviousKey => ("key.jump", json!({"forward": false})),
        TransportStep::PreviousFrame => ("timeline.step", json!({"delta": -1})),
        TransportStep::Play => ("timeline.toggle_play", json!({})),
        TransportStep::NextFrame => ("timeline.step", json!({"delta": 1})),
        TransportStep::NextKey => ("key.jump", json!({"forward": true})),
        TransportStep::Last => ("timeline.set_frame", json!({"frame": last})),
    }
}

pub(crate) const EDITOR_RULER_HEIGHT: f32 = 20.0;
pub(crate) const MAX_TRACK_WIDTH: f32 = 170.0;

/// Keep room for a timeline canvas when a user docks an editor in a narrow area.
pub(crate) fn track_column_width(width: f32) -> f32 {
    (width * 0.40).min(MAX_TRACK_WIDTH)
}

/// Shared ruler and vertical frame guides for the two expanded animation views.
pub(crate) fn paint_editor_ruler(
    ui: &Ui,
    ruler: Rect,
    body: Rect,
    view: &crate::fcurve_logic::TimeView,
    current: f64,
) {
    let theme = Theme::current(ui.ctx());
    let painter = ui.painter_at(ruler);
    let guide = ui.painter_at(body);
    painter.rect_filled(ruler, 0.0, theme.input);
    let rect = Rect::from_min_max(ruler.min, body.max);
    let width = ruler.width();
    if width <= 0.0 {
        return;
    }
    let span = f64::from(width) / view.ppf;
    let step = ruler_step(span as f32, width) as f64;
    let mut frame = (view.to_frame(ruler.left(), ruler.left()) / step).ceil() * step;
    let end = view.to_frame(ruler.right(), ruler.left());
    for _ in 0..((width / 40.0).ceil() as usize + 3) {
        if frame > end {
            break;
        }
        let x = view.to_x(frame, ruler.left());
        painter.line_segment(
            [
                Pos2::new(x, ruler.bottom() - 5.0),
                Pos2::new(x, ruler.bottom()),
            ],
            Stroke::new(1.0, theme.weak_text),
        );
        painter.text(
            Pos2::new(x + 3.0, ruler.center().y - 1.0),
            Align2::LEFT_CENTER,
            format!("{frame:.0}"),
            FontId::monospace(10.0),
            theme.weak_text,
        );
        guide.line_segment(
            [Pos2::new(x, body.top()), Pos2::new(x, body.bottom())],
            Stroke::new(1.0, theme.divider.gamma_multiply(0.65)),
        );
        frame += step;
    }
    let x = view.to_x(current, ruler.left());
    painter.rect_filled(
        Rect::from_min_size(Pos2::new(x - 3.0, ruler.top()), vec2(6.0, 4.0)),
        1.0,
        theme.time,
    );
    ui.painter_at(rect).line_segment(
        [Pos2::new(x, ruler.top()), Pos2::new(x, body.bottom())],
        Stroke::new(1.0, theme.time),
    );
}

/// Height of the marker flags at the top of the strip, in points. The strip keeps its old
/// height of 30 so the panel does not grow.
const MARKER_LANE: f32 = 10.0;
/// Half-width of a handle or marker hit rectangle.
const GRAB: f32 = 5.0;
/// Colours of the context menu's marker palette.
const PALETTE: [(&str, &str); 5] = [
    ("Orange", "#e0a030"),
    ("Cyan", "#38c6e8"),
    ("Green", "#5fbf6a"),
    ("Red", "#e5534b"),
    ("Violet", "#9b7be0"),
];

/// Frame to strip x and back for a strip spanning `start..=end` between `x0` and `x1`.
#[derive(Clone, Copy)]
pub struct StripScale {
    pub start: i64,
    pub end: i64,
    pub x0: f32,
    pub x1: f32,
}

impl StripScale {
    fn span(&self) -> f32 {
        (self.end as i128 - self.start as i128).max(1) as f32
    }

    pub fn to_x(&self, frame: f64) -> f32 {
        self.x0
            + ((frame - self.start as f64) / f64::from(self.span())) as f32 * (self.x1 - self.x0)
    }

    /// The nearest frame inside the project range.
    pub fn to_frame(&self, x: f32) -> i64 {
        let f = self.start as f64 + f64::from((x - self.x0) / (self.x1 - self.x0) * self.span());
        (f.round() as i64).clamp(self.start, self.end)
    }
}

/// Playback mode, timing and display controls (the end of row 1 of the panel).
pub fn playback_controls(ui: &mut Ui, scene: &Scene, out: &mut Vec<Cmd>) {
    let time = scene.time();
    {
        let modes = [
            ("once", "Once"),
            ("loop", "Loop"),
            ("ping_pong", "Ping-Pong"),
        ];
        let shown = modes
            .iter()
            .find(|(id, _)| *id == time.mode.id())
            .map_or("Loop", |m| m.1);
        egui::ComboBox::from_label("Playback mode")
            .selected_text(shown)
            .show_ui(ui, |ui| {
                for (id, label) in modes {
                    if ui.selectable_label(time.mode.id() == id, label).clicked() {
                        out.push(("timeline.set_mode", json!({"mode": id})));
                    }
                }
            });
        let mut every = !time.drop_frames;
        if ui.checkbox(&mut every, "Every frame").changed() {
            out.push(("timeline.set_frame_dropping", json!({"enabled": !every})));
        }
        let displays = [
            (TimeDisplay::Frames, "Frames"),
            (TimeDisplay::Smpte, "SMPTE"),
            (TimeDisplay::Seconds, "Seconds"),
        ];
        let shown = displays
            .iter()
            .find(|(d, _)| *d == time.display)
            .map_or("Frames", |d| d.1);
        egui::ComboBox::from_label("Time display")
            .selected_text(shown)
            .show_ui(ui, |ui| {
                for (d, label) in displays {
                    if ui.selectable_label(time.display == d, label).clicked() {
                        out.push(("timeline.set_display", json!({"format": d.id()})));
                    }
                }
            });
    }
}

/// Preview range and marker controls (the end of row 2 of the panel).
pub fn range_controls(ui: &mut Ui, scene: &Scene, out: &mut Vec<Cmd>) {
    let (settings, time) = (scene.settings(), scene.time());
    {
        let mut on = time.preview.is_some();
        if ui.checkbox(&mut on, "Preview range").changed() {
            out.push(if on {
                (
                    "timeline.set_preview",
                    json!({"start": settings.start, "end": settings.end}),
                )
            } else {
                ("timeline.clear_preview", json!({}))
            });
        }
        if let Some((a, b)) = time.preview {
            let (mut na, mut nb) = (a, b);
            let l = ui.label("Preview start");
            ui.add(egui::DragValue::new(&mut na).range(settings.start..=b))
                .labelled_by(l.id);
            let l = ui.label("Preview end");
            ui.add(egui::DragValue::new(&mut nb).range(a..=settings.end))
                .labelled_by(l.id);
            if (na, nb) != (a, b) {
                out.push(("timeline.set_preview", json!({"start": na, "end": nb})));
            }
        }
        ui.separator();
        for (icon, label, help, command) in [
            (
                "marker",
                "Add Marker",
                "Add a marker at the current frame",
                ("marker.add", json!({})),
            ),
            (
                "previous_key",
                "Previous Marker",
                "Jump to the previous timeline marker",
                ("marker.jump", json!({"forward": false})),
            ),
            (
                "next_key",
                "Next Marker",
                "Jump to the next timeline marker",
                ("marker.jump", json!({"forward": true})),
            ),
        ] {
            if crate::icons::action(ui, icon, label, help, None).clicked() {
                out.push(command);
            }
        }
    }
}

/// The frame label next to the frame field: the seconds for the frame display, else the
/// time in the chosen format.
pub fn time_label(frame: i64, scene: &Scene) -> String {
    let (s, t) = (scene.settings(), scene.time());
    match t.display {
        TimeDisplay::Frames => format_time(frame, s.fps, TimeDisplay::Seconds),
        d => format_time(frame, s.fps, d),
    }
}

fn color32(c: [u8; 3]) -> Color32 {
    Color32::from_rgb(c[0], c[1], c[2])
}

/// The ruler's label interval in frames: the smallest of 1, 2, 5, 10, ... that keeps labels
/// at least 40 points apart over `width` points showing `span` frames. At the reference's
/// 1920 x 1080 window the strip is about 1413 points wide for a 0 - 90 project, which labels
/// every 5 frames (observed, G10 A7).
pub fn ruler_step(span: f32, width: f32) -> i64 {
    let minimum = f64::from(span) * 40.0 / f64::from(width.max(1.0));
    let base = 10.0_f64.powf(minimum.max(1.0).log10().floor());
    for factor in [1.0, 2.0, 5.0, 10.0] {
        let step = base * factor;
        if step >= minimum {
            return (step as i64).max(1);
        }
    }
    i64::MAX
}

/// The frame strip: scrubbing, ruler, key ticks, the preview range with its two handles and
/// the markers. `keys` are key frames of the selected objects.
pub fn strip(ui: &mut Ui, scene: &Scene, current: i64, keys: &[f64], out: &mut Vec<Cmd>) {
    let (settings, time) = (scene.settings(), scene.time());
    let theme = Theme::current(ui.ctx());
    let width = ui.available_width().max(60.0);
    let (rect, response) = ui.allocate_exact_size(vec2(width, 30.0), Sense::click_and_drag());
    response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Slider, true, "Frame strip"));
    let scale = StripScale {
        start: settings.start,
        end: settings.end,
        x0: rect.left() + 8.0,
        x1: rect.right() - 8.0,
    };
    let body = Rect::from_min_max(
        Pos2::new(rect.left(), rect.top() + MARKER_LANE),
        rect.right_bottom(),
    );
    if (response.dragged() || response.clicked())
        && let Some(pos) = response.interact_pointer_pos()
    {
        out.push((
            "timeline.set_frame",
            json!({"frame": scale.to_frame(pos.x)}),
        ));
    }
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 2.0, theme.input);
    painter.rect_filled(
        Rect::from_min_max(rect.min, body.right_top()),
        0.0,
        theme.faint,
    );
    painter.hline(
        rect.x_range(),
        rect.bottom() - 1.0,
        Stroke::new(1.0, theme.divider),
    );
    // preview range: the outside is dimmed, the inside gets a time-coloured band
    if let Some((a, b)) = time.preview {
        let (xa, xb) = (scale.to_x(a as f64), scale.to_x(b as f64));
        let dim = Color32::from_black_alpha(110);
        painter.rect_filled(
            Rect::from_min_max(body.left_top(), Pos2::new(xa, body.bottom())),
            0.0,
            dim,
        );
        painter.rect_filled(
            Rect::from_min_max(Pos2::new(xb, body.top()), body.right_bottom()),
            0.0,
            dim,
        );
        painter.rect_filled(
            Rect::from_min_max(
                Pos2::new(xa, body.bottom() - 3.0),
                Pos2::new(xb, body.bottom()),
            ),
            0.0,
            theme.time,
        );
    }
    // ruler
    let span = scale.span();
    let step = ruler_step(span, scale.x1 - scale.x0);
    let first = settings.start.div_euclid(step).saturating_mul(step);
    for f in (first..=settings.end).step_by(step as usize) {
        if f < settings.start {
            continue;
        }
        let x = scale.to_x(f as f64);
        painter.line_segment(
            [
                Pos2::new(x, rect.bottom() - 6.0),
                Pos2::new(x, rect.bottom()),
            ],
            Stroke::new(1.0, theme.weak_text),
        );
        painter.text(
            Pos2::new(x + 2.0, rect.bottom() - 7.0),
            Align2::LEFT_BOTTOM,
            f.to_string(),
            FontId::proportional(10.0),
            theme.weak_text,
        );
    }
    for f in keys {
        if *f < settings.start as f64 - 1e-6 || *f > settings.end as f64 + 1e-6 {
            continue;
        }
        let x = scale.to_x(*f);
        let y = body.top() + 5.0;
        painter.add(egui::Shape::convex_polygon(
            vec![
                Pos2::new(x, y - 3.5),
                Pos2::new(x + 3.5, y),
                Pos2::new(x, y + 3.5),
                Pos2::new(x - 3.5, y),
            ],
            theme.selection,
            Stroke::NONE,
        ));
    }
    let x = scale.to_x(current as f64);
    painter.rect_filled(
        Rect::from_min_size(Pos2::new(x - 3.0, rect.top()), vec2(6.0, 4.0)),
        1.0,
        theme.time,
    );
    painter.line_segment(
        [Pos2::new(x, rect.top()), Pos2::new(x, rect.bottom())],
        Stroke::new(1.5, theme.time),
    );
    // widgets on top of the scrub area: preview handles, then markers
    if let Some((a, b)) = time.preview {
        for (label, frame, is_start) in [
            ("Preview start handle", a, true),
            ("Preview end handle", b, false),
        ] {
            let x = scale.to_x(frame as f64);
            let hit = Rect::from_min_max(
                Pos2::new(x - GRAB, body.top()),
                Pos2::new(x + GRAB, body.bottom()),
            );
            let r = ui.interact(hit, response.id.with(label), Sense::click_and_drag());
            r.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Slider, true, label));
            ui.painter_at(rect).rect_filled(
                Rect::from_center_size(Pos2::new(x, body.bottom() - 6.0), vec2(4.0, 12.0)),
                1.0,
                theme.time,
            );
            if r.dragged()
                && let Some(pos) = r.interact_pointer_pos()
            {
                let f = scale.to_frame(pos.x);
                let (ns, ne) = if is_start {
                    (f.min(b), b)
                } else {
                    (a, f.max(a))
                };
                if (ns, ne) != (a, b) {
                    out.push(("timeline.set_preview", json!({"start": ns, "end": ne})));
                }
            }
        }
    }
    for m in &time.markers {
        if m.frame < settings.start || m.frame > settings.end {
            continue;
        }
        let x = scale.to_x(m.frame as f64);
        let hit = Rect::from_min_max(
            Pos2::new(x - GRAB, rect.top()),
            Pos2::new(x + GRAB, rect.top() + MARKER_LANE),
        );
        let label = format!("Marker {}", m.name);
        let r = ui.interact(
            hit,
            response.id.with(("marker", m.id)),
            Sense::click_and_drag(),
        );
        r.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, &label));
        let color = color32(m.color);
        let p = ui.painter_at(rect);
        p.line_segment(
            [Pos2::new(x, rect.top()), Pos2::new(x, rect.bottom())],
            Stroke::new(1.0, color.gamma_multiply(0.6)),
        );
        p.add(egui::Shape::convex_polygon(
            vec![
                Pos2::new(x - GRAB, rect.top()),
                Pos2::new(x + GRAB, rect.top()),
                Pos2::new(x, rect.top() + MARKER_LANE - 2.0),
            ],
            color,
            Stroke::NONE,
        ));
        if !m.name.is_empty() {
            p.text(
                Pos2::new(x + GRAB + 2.0, rect.top()),
                Align2::LEFT_TOP,
                &m.name,
                FontId::proportional(10.0),
                color,
            );
        }
        if r.dragged()
            && let Some(pos) = r.interact_pointer_pos()
        {
            let f = scale.to_frame(pos.x);
            if f != m.frame {
                out.push(("marker.move", json!({"id": m.id, "frame": f})));
            }
        } else if r.clicked() {
            out.push(("timeline.set_frame", json!({"frame": m.frame})));
        }
        r.context_menu(|ui| {
            let mut name = m.name.clone();
            let l = ui.label("Marker name");
            if ui
                .text_edit_singleline(&mut name)
                .labelled_by(l.id)
                .changed()
            {
                out.push(("marker.rename", json!({"id": m.id, "name": name})));
            }
            for (label, hex) in PALETTE {
                if hex != color_hex(m.color) && ui.button(format!("Color {label}")).clicked() {
                    out.push(("marker.set_color", json!({"id": m.id, "color": hex})));
                    ui.close();
                }
            }
            if ui.button("Delete Marker").clicked() {
                out.push(("marker.remove", json!({"id": m.id})));
                ui.close();
            }
        });
    }
}
