//! The Dope Sheet panel: a track tree (objects, then their tracks) with a summary row per
//! object and the keys as diamonds on a frame ruler. Box select, move, scale about the current
//! frame, ripple, copy and paste, delete and interpolation changes. Every edit is a `key.*`
//! command (docs/specs/ANIM.md section 2b); nothing here touches the document. Own design,
//! parity-pending G5.

use std::collections::BTreeSet;

use egui::{Align2, FontId, Pos2, Rect, Sense, Shape, Stroke, Ui, vec2};
use forge_core::{Interp, NodeId};
use serde_json::json;

use crate::fcurve_logic::{self as logic, KeyId, KeySelection, Row, RowKind, TimeView};
use crate::object_manager::Cmd;
use crate::state::EditorState;
use crate::theme::Theme;

pub const ROW_H: f32 = 18.0;
pub const RULER_H: f32 = 18.0;
pub const LABEL_W: f32 = 170.0;
const KEY_R: f32 = 5.0;

/// A copied key: its track and the offset from the earliest copied frame.
#[derive(Debug, Clone, PartialEq)]
pub struct ClipKey {
    pub node: u64,
    pub track: String,
    pub dframe: f64,
    pub value: f64,
    pub interp: &'static str,
}

/// The panel's own state (kept in egui memory, not in the document).
#[derive(Clone, Default)]
pub struct DopeSheet {
    pub view: TimeView,
    pub selection: KeySelection,
    pub ripple: bool,
    pub only_selected: bool,
    pub clipboard: Vec<ClipKey>,
    collapsed: BTreeSet<u64>,
    drag: Option<(DragKind, Pos2)>,
    /// Canvas rectangle of the last frame (for tests).
    pub canvas: Option<Rect>,
    /// Row index of each `rows()` entry shown in the last frame, with its key frames.
    pub shown: Vec<Row>,
}

#[derive(Clone, Copy, PartialEq)]
enum DragKind {
    Box,
    Move,
    Scrub,
}

fn memo_id() -> egui::Id {
    egui::Id::unique("forge-dope-sheet")
}

/// A copy of the panel state, for tests.
pub fn sheet_state(ctx: &egui::Context) -> DopeSheet {
    ctx.data(|d| d.get_temp::<DopeSheet>(memo_id()))
        .unwrap_or_default()
}

/// Overwrites the panel state, for tests.
pub fn set_sheet_state(ctx: &egui::Context, s: DopeSheet) {
    ctx.data_mut(|d| d.insert_temp(memo_id(), s));
}

impl DopeSheet {
    /// The screen point of a key at `frame` on row `row_index` of the canvas.
    pub fn key_point(&self, canvas: Rect, row_index: usize, frame: f64) -> Pos2 {
        Pos2::new(
            self.view.to_x(frame, canvas.left() + LABEL_W),
            canvas.top() + RULER_H + (row_index as f32 + 0.5) * ROW_H,
        )
    }

    /// The rows shown (collapsed objects hide their tracks).
    fn visible(&self, rows: Vec<Row>) -> Vec<Row> {
        let mut hidden: Option<u64> = None;
        rows.into_iter()
            .filter(|r| match r.kind {
                RowKind::Object => {
                    hidden = self.collapsed.contains(&r.node.0).then_some(r.node.0);
                    true
                }
                RowKind::Track { .. } => hidden != Some(r.node.0),
            })
            .collect()
    }

    fn copy(&mut self, st: &EditorState) {
        let fps = f64::from(st.doc.scene().settings().fps);
        let mut items = Vec::new();
        for k in &self.selection.keys {
            let Some(t) = st
                .doc
                .scene()
                .node(NodeId(k.node))
                .and_then(|n| n.tracks.get(&k.track))
            else {
                continue;
            };
            if let Some(key) = t
                .keys
                .iter()
                .find(|x| (x.time * fps - k.frame).abs() < 1e-6)
            {
                let unit = logic::rows(st.doc.scene(), &[NodeId(k.node)])
                    .iter()
                    .find_map(|r| match &r.kind {
                        RowKind::Track { track, unit } if *track == k.track => Some(*unit),
                        _ => None,
                    })
                    .unwrap_or_default();
                items.push(ClipKey {
                    node: k.node,
                    track: k.track.clone(),
                    dframe: k.frame,
                    value: logic::display(unit, key.value),
                    interp: interp_name(key.interp),
                });
            }
        }
        let first = items.iter().map(|i| i.dframe).fold(f64::INFINITY, f64::min);
        for i in &mut items {
            i.dframe -= first;
        }
        self.clipboard = items;
    }

    /// The `key.paste` command for the clipboard at `frame`.
    pub fn paste_command(&self, frame: f64) -> Option<Cmd> {
        if self.clipboard.is_empty() {
            return None;
        }
        let keys: Vec<_> = self
            .clipboard
            .iter()
            .map(|c| {
                json!({"node": c.node, "track": c.track, "frame": frame + c.dframe,
                       "value": c.value, "interp": c.interp})
            })
            .collect();
        Some(("key.paste", json!({"keys": keys})))
    }
}

fn interp_name(i: Interp) -> &'static str {
    match i {
        Interp::Spline => "spline",
        Interp::Linear => "linear",
        Interp::Step => "step",
    }
}

/// Draws the panel and returns the commands the user's actions ask for.
pub fn dope_sheet_ui(ui: &mut Ui, st: &mut EditorState) -> Vec<Cmd> {
    let mut sheet = sheet_state(ui.ctx());
    let mut cmds: Vec<Cmd> = Vec::new();
    let theme = Theme::current(ui.ctx());
    let current = st.timeline.current as f64;

    ui.horizontal_wrapped(|ui| {
        let has = !sheet.selection.keys.is_empty();
        if ui.add_enabled(has, egui::Button::new("Delete")).clicked() {
            cmds.push(("key.delete", json!({"keys": sheet.selection.json()})));
            sheet.selection.clear();
        }
        for (label, name) in [("Spline", "spline"), ("Linear", "linear"), ("Step", "step")] {
            if ui.add_enabled(has, egui::Button::new(label)).clicked() {
                cmds.push((
                    "key.set_interp",
                    json!({"keys": sheet.selection.json(), "interp": name}),
                ));
            }
        }
        ui.separator();
        if ui.add_enabled(has, egui::Button::new("Copy")).clicked() {
            sheet.copy(st);
        }
        if ui
            .add_enabled(!sheet.clipboard.is_empty(), egui::Button::new("Paste"))
            .clicked()
            && let Some(c) = sheet.paste_command(current)
        {
            cmds.push(c);
        }
        for (label, factor) in [("Scale 1/2", 0.5), ("Scale 2", 2.0)] {
            if ui.add_enabled(has, egui::Button::new(label)).clicked() {
                cmds.push((
                    "key.scale",
                    json!({"keys": sheet.selection.json(), "pivot_frame": current,
                           "factor": factor}),
                ));
            }
        }
        ui.separator();
        ui.checkbox(&mut sheet.ripple, "Ripple");
        ui.checkbox(&mut sheet.view.snap, "Snap");
        ui.checkbox(&mut sheet.only_selected, "Selected objects only");
    });

    let only: Vec<NodeId> = if sheet.only_selected {
        st.selection.nodes.iter().copied().collect()
    } else {
        Vec::new()
    };
    let all_rows = logic::rows(st.doc.scene(), &only);
    let rows = sheet.visible(all_rows);

    let avail = ui.available_rect_before_wrap();
    let height = (RULER_H + rows.len() as f32 * ROW_H).max(avail.height().min(300.0));
    let (canvas, resp) =
        ui.allocate_exact_size(vec2(avail.width(), height), Sense::click_and_drag());
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Label, true, "Dope sheet"));
    let x0 = canvas.left() + LABEL_W;
    let painter = ui.painter_at(canvas);
    painter.rect_filled(canvas, 0.0, theme.panel);

    // ruler
    let ruler = Rect::from_min_size(canvas.min, vec2(canvas.width(), RULER_H));
    painter.rect_filled(ruler, 0.0, theme.faint);
    let step = ruler_step(sheet.view.ppf);
    let first = (sheet.view.to_frame(x0, x0) / step).floor() * step;
    let mut f = first;
    while sheet.view.to_x(f, x0) < canvas.right() {
        let x = sheet.view.to_x(f, x0);
        if x >= x0 {
            painter.line_segment(
                [
                    Pos2::new(x, ruler.bottom() - 6.0),
                    Pos2::new(x, ruler.bottom()),
                ],
                Stroke::new(1.0, theme.weak_text),
            );
            painter.text(
                Pos2::new(x + 2.0, ruler.top()),
                Align2::LEFT_TOP,
                format!("{f:.0}"),
                FontId::proportional(10.0),
                theme.weak_text,
            );
        }
        f += step;
    }

    // rows, labels and keys
    let mut points: Vec<(Vec<KeyId>, Pos2)> = Vec::new();
    for (i, row) in rows.iter().enumerate() {
        let y = canvas.top() + RULER_H + i as f32 * ROW_H;
        let r = Rect::from_min_size(Pos2::new(canvas.left(), y), vec2(canvas.width(), ROW_H));
        if i % 2 == 1 {
            painter.rect_filled(r, 0.0, theme.faint.gamma_multiply(0.5));
        }
        let (text, indent) = match row.kind {
            RowKind::Object => {
                let arrow = if sheet.collapsed.contains(&row.node.0) {
                    ">"
                } else {
                    "v"
                };
                (format!("{arrow} {}", row.label), 0.0)
            }
            RowKind::Track { .. } => (row.label.clone(), 14.0),
        };
        painter.text(
            Pos2::new(canvas.left() + 4.0 + indent, y + ROW_H / 2.0),
            Align2::LEFT_CENTER,
            text,
            FontId::proportional(12.0),
            theme.text,
        );
        for &frame in &row.frames {
            let p = Pos2::new(sheet.view.to_x(frame, x0), y + ROW_H / 2.0);
            if p.x < x0 || p.x > canvas.right() {
                continue;
            }
            let ids: Vec<KeyId> = match &row.kind {
                RowKind::Track { track, .. } => vec![KeyId {
                    node: row.node.0,
                    track: track.clone(),
                    frame,
                }],
                RowKind::Object => rows
                    .iter()
                    .filter_map(|r| match &r.kind {
                        RowKind::Track { track, .. }
                            if r.node == row.node
                                && r.frames.iter().any(|f| (f - frame).abs() < 1e-6) =>
                        {
                            Some(KeyId {
                                node: row.node.0,
                                track: track.clone(),
                                frame,
                            })
                        }
                        _ => None,
                    })
                    .collect(),
            };
            points.push((ids, p));
        }
    }
    let delta_preview = match sheet.drag {
        Some((DragKind::Move, origin)) => resp
            .interact_pointer_pos()
            .map_or(0.0, |p| logic::drag_frames(&sheet.view, p.x - origin.x)),
        _ => 0.0,
    };
    for (ids, p) in &points {
        let selected = ids.iter().all(|k| sheet.selection.contains(k)) && !ids.is_empty();
        let p = if selected && delta_preview != 0.0 {
            Pos2::new(p.x + (delta_preview * sheet.view.ppf) as f32, p.y)
        } else {
            *p
        };
        let color = if selected {
            theme.selection
        } else {
            theme.text
        };
        // a selected key is one pixel larger, so it reads in greyscale (UI.md §24.1)
        let r = if selected { KEY_R + 1.0 } else { KEY_R };
        painter.add(Shape::convex_polygon(
            vec![
                p + vec2(0.0, -r),
                p + vec2(r, 0.0),
                p + vec2(0.0, r),
                p + vec2(-r, 0.0),
            ],
            color,
            Stroke::new(1.0, theme.window),
        ));
    }
    // current frame
    let cx = sheet.view.to_x(current, x0);
    if cx >= x0 && cx <= canvas.right() {
        painter.line_segment(
            [Pos2::new(cx, canvas.top()), Pos2::new(cx, canvas.bottom())],
            Stroke::new(1.0, theme.time),
        );
    }

    // interaction
    let hit = |pos: Pos2| -> Option<usize> {
        let p: Vec<(KeyId, [f32; 2])> = points
            .iter()
            .enumerate()
            .map(|(i, (_, q))| {
                (
                    KeyId {
                        node: i as u64,
                        track: String::new(),
                        frame: 0.0,
                    },
                    [q.x, q.y],
                )
            })
            .collect();
        logic::hit_test(&p, [pos.x, pos.y], KEY_R + 2.0)
    };
    let mods = ui.input(|i| i.modifiers);
    if resp.drag_started_by(egui::PointerButton::Primary)
        && let Some(pos) = ui.input(|i| i.pointer.press_origin())
    {
        sheet.drag = Some(if pos.y < canvas.top() + RULER_H {
            (DragKind::Scrub, pos)
        } else if let Some(i) = hit(pos) {
            let ids = points[i].0.clone();
            if !ids.iter().all(|k| sheet.selection.contains(k)) {
                sheet.selection.set(ids, mods.shift);
            }
            (DragKind::Move, pos)
        } else if pos.x >= x0 {
            (DragKind::Box, pos)
        } else {
            (DragKind::Scrub, pos)
        });
    }
    if let Some((kind, origin)) = sheet.drag {
        if let Some(pos) = resp.interact_pointer_pos() {
            match kind {
                DragKind::Scrub => {
                    let f = sheet.view.to_frame(pos.x, x0).round() as i64;
                    if f != st.timeline.current && pos.y < canvas.top() + RULER_H + 1.0 {
                        cmds.push(("timeline.set_frame", json!({"frame": f})));
                    }
                }
                DragKind::Box => {
                    painter.rect_stroke(
                        Rect::from_two_pos(origin, pos),
                        0.0,
                        Stroke::new(1.0, theme.selection),
                        egui::StrokeKind::Inside,
                    );
                }
                DragKind::Move => {}
            }
        }
        if resp.drag_stopped() {
            let end = ui
                .input(|i| i.pointer.latest_pos())
                .or_else(|| resp.interact_pointer_pos())
                .unwrap_or(origin);
            match kind {
                DragKind::Box => {
                    let sel: Vec<KeyId> = points
                        .iter()
                        .filter(|(_, p)| Rect::from_two_pos(origin, end).contains(*p))
                        .flat_map(|(ids, _)| ids.clone())
                        .collect();
                    sheet.selection.set(sel, mods.shift);
                }
                DragKind::Move => {
                    let d = logic::drag_frames(&sheet.view, end.x - origin.x);
                    if d != 0.0 && !sheet.selection.keys.is_empty() {
                        cmds.push((
                            "key.move",
                            json!({"keys": sheet.selection.json(), "delta_frames": d,
                                   "ripple": sheet.ripple}),
                        ));
                        sheet.selection = sheet.selection.moved(d);
                    }
                }
                DragKind::Scrub => {}
            }
            sheet.drag = None;
        }
    } else if resp.clicked()
        && let Some(pos) = resp.interact_pointer_pos()
    {
        if pos.y < canvas.top() + RULER_H || pos.x < x0 {
            let f = sheet.view.to_frame(pos.x.max(x0), x0).round() as i64;
            if pos.y < canvas.top() + RULER_H {
                cmds.push(("timeline.set_frame", json!({"frame": f})));
            } else if let Some(row) = row_at(&rows, canvas, pos.y)
                && matches!(row.kind, RowKind::Object)
                && !sheet.collapsed.remove(&row.node.0)
            {
                sheet.collapsed.insert(row.node.0);
            }
        } else if let Some(i) = hit(pos) {
            sheet.selection.click(points[i].0[0].clone(), mods.shift);
            if points[i].0.len() > 1 || !mods.shift {
                sheet.selection.set(points[i].0.clone(), mods.shift);
            }
        } else if !mods.shift {
            sheet.selection.clear();
        }
    }
    // pan and zoom
    if resp.hovered() {
        let (scroll, zoom, pos) = ui.input(|i| {
            (
                i.smooth_scroll_delta,
                i.zoom_delta(),
                i.pointer.hover_pos().unwrap_or(canvas.center()),
            )
        });
        if zoom != 1.0 {
            sheet.view.zoom_about(pos.x, x0, f64::from(zoom));
        } else if scroll.x != 0.0 || scroll.y != 0.0 {
            sheet.view.pan(scroll.x + scroll.y);
        }
        if ui.input(|i| i.key_pressed(egui::Key::Delete)) && !sheet.selection.keys.is_empty() {
            cmds.push(("key.delete", json!({"keys": sheet.selection.json()})));
            sheet.selection.clear();
        }
    }
    sheet.canvas = Some(canvas);
    sheet.shown = rows;
    set_sheet_state(ui.ctx(), sheet);
    cmds
}

fn row_at(rows: &[Row], canvas: Rect, y: f32) -> Option<&Row> {
    let i = ((y - canvas.top() - RULER_H) / ROW_H).floor();
    (i >= 0.0).then(|| rows.get(i as usize)).flatten()
}

/// A ruler tick spacing in frames that keeps ticks at least 40 points apart.
fn ruler_step(ppf: f64) -> f64 {
    [1.0, 2.0, 5.0, 10.0, 20.0, 50.0, 100.0, 200.0, 500.0, 1000.0]
        .into_iter()
        .find(|s| s * ppf >= 40.0)
        .unwrap_or(1000.0)
}

/// Frames all keys into the view (the "frame all" button and the key shortcut).
pub fn frame_all(sheet: &mut DopeSheet, width: f32) {
    let frames: Vec<f64> = sheet.shown.iter().flat_map(|r| r.frames.clone()).collect();
    if let (Some(lo), Some(hi)) = (
        frames.iter().copied().reduce(f64::min),
        frames.iter().copied().reduce(f64::max),
    ) {
        sheet.view.frame_range(lo, hi, width - LABEL_W);
    }
}
