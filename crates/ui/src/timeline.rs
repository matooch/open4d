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
        (self.end - self.start).max(1) as f32
    }

    pub fn to_x(&self, frame: f64) -> f32 {
        self.x0 + (frame as f32 - self.start as f32) / self.span() * (self.x1 - self.x0)
    }

    /// The nearest frame inside the project range.
    pub fn to_frame(&self, x: f32) -> i64 {
        let f = self.start as f32 + (x - self.x0) / (self.x1 - self.x0) * self.span();
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
        if ui.button("Add Marker").clicked() {
            out.push(("marker.add", json!({})));
        }
        if ui.button("Previous Marker").clicked() {
            out.push(("marker.jump", json!({"forward": false})));
        }
        if ui.button("Next Marker").clicked() {
            out.push(("marker.jump", json!({"forward": true})));
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
    painter.rect_filled(rect, 2.0, theme.faint);
    // preview range: the outside is dimmed, the inside gets an accent band
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
            theme.accent,
        );
    }
    // ruler
    let span = scale.span();
    let step = [1, 2, 5, 10, 20, 50, 100, 200, 500, 1000, 5000]
        .into_iter()
        .find(|s| (*s as f32 / span) * (scale.x1 - scale.x0) >= 40.0)
        .unwrap_or(10000);
    let first = settings.start.div_euclid(step) * step;
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
        painter.line_segment(
            [Pos2::new(x, body.top()), Pos2::new(x, body.top() + 8.0)],
            Stroke::new(2.0, theme.accent),
        );
    }
    let x = scale.to_x(current as f64);
    painter.line_segment(
        [Pos2::new(x, rect.top()), Pos2::new(x, rect.bottom())],
        Stroke::new(1.5, theme.component_selection),
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
                theme.accent,
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
