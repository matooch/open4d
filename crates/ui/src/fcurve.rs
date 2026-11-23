//! The Value Graph panel (F-curves): one curve per numeric track, drawn by sampling the same
//! evaluator as playback, keys with tangent handles on the selected keys, box select, move,
//! handle drag, frame all / selected, a normalised view, numeric entry for the selected key.
//! Every edit is a `key.*` command (docs/specs/ANIM.md section 2b). Curve colours follow the
//! axis (X red, Y green, Z blue). Own design, parity-pending G5 and O38 for the handle
//! gestures.

use std::collections::BTreeSet;

use egui::{Align2, FontId, Pos2, Rect, Sense, Stroke, Ui, vec2};
use forge_anim::{SpeedSide, speed_at, speed_at_key, speed_editable};
use forge_core::{Interp, NodeId, Track, TrackTarget, Unit, parse_track_target};
use serde_json::json;

use crate::fcurve_logic::{self as logic, KeyId, KeySelection, RowKind, TimeView, ValueView};
use crate::object_manager::Cmd;
use crate::state::EditorState;
use crate::theme::Theme;

pub const LIST_W: f32 = 170.0;
const KEY_R: f32 = 4.5;
const HANDLE_R: f32 = 4.0;
const SPEED_GRIP_X: f32 = 12.0;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum GraphMode {
    #[default]
    Value,
    Speed,
}

#[derive(Clone, Copy, PartialEq)]
enum DragKind {
    Box,
    Move,
    /// A tangent handle of a key: -1 left, 1 right.
    Handle(i8),
}

/// The panel's own state (kept in egui memory, not in the document).
#[derive(Clone, Default)]
pub struct ValueGraph {
    pub tview: TimeView,
    pub vview: ValueView,
    pub selection: KeySelection,
    pub normalised: bool,
    pub mode: GraphMode,
    alternate_vview: ValueView,
    hidden: BTreeSet<(u64, String)>,
    drag: Option<(DragKind, Pos2, Option<KeyId>)>,
    pub canvas: Option<Rect>,
}

fn memo_id() -> egui::Id {
    egui::Id::unique("forge-value-graph")
}

pub fn graph_state(ctx: &egui::Context) -> ValueGraph {
    ctx.data(|d| d.get_temp::<ValueGraph>(memo_id()))
        .unwrap_or_default()
}

pub fn set_graph_state(ctx: &egui::Context, g: ValueGraph) {
    ctx.data_mut(|d| d.insert_temp(memo_id(), g));
}

impl ValueGraph {
    /// The screen point of (`frame`, display `value`).
    pub fn point(&self, canvas: Rect, frame: f64, value: f64) -> Pos2 {
        let x0 = canvas.left();
        Pos2::new(
            self.tview.to_x(frame, x0),
            self.vview.to_y(value, canvas.top()),
        )
    }

    /// Side-specific speed grip; it stays slightly clear of the key's time line.
    pub fn speed_grip(&self, canvas: Rect, frame: f64, speed: f64, side: SpeedSide) -> Pos2 {
        let x = if side == SpeedSide::Left {
            -SPEED_GRIP_X
        } else {
            SPEED_GRIP_X
        };
        self.point(canvas, frame, speed) + vec2(x, 0.0)
    }
}

/// A curve to draw: its track, unit and the display range of its keys.
struct Curve {
    node: NodeId,
    track: String,
    unit: Unit,
    range: (f64, f64),
}

fn curves(st: &EditorState, hidden: &BTreeSet<(u64, String)>, speed: bool) -> Vec<Curve> {
    let scene = st.doc.scene();
    let only: Vec<NodeId> = st.selection.nodes.iter().copied().collect();
    logic::rows(scene, &only)
        .into_iter()
        .filter_map(|r| {
            let RowKind::Track { track, unit } = r.kind else {
                return None;
            };
            if hidden.contains(&(r.node.0, track.clone()))
                || !logic::has_curve(scene, r.node, &track)
            {
                return None;
            }
            let t = scene.node(r.node)?.tracks.get(&track)?;
            if speed
                && (t.quaternion
                    || !scene.node_type(r.node).is_some_and(|ty| {
                        matches!(
                            parse_track_target(&track, |p| ty.find_param(p)),
                            Ok(TrackTarget::Float(_)
                                | TrackTarget::Component(..)
                                | TrackTarget::Channel(..))
                        )
                    }))
            {
                return None;
            }
            let vals = t.keys.iter().map(|k| logic::display(unit, k.value));
            let lo = vals.clone().fold(f64::INFINITY, f64::min);
            let hi = vals.fold(f64::NEG_INFINITY, f64::max);
            Some(Curve {
                node: r.node,
                track,
                unit,
                range: (lo, hi),
            })
        })
        .collect()
}

fn key_speed(t: &Track, i: usize) -> f64 {
    speed_at_key(t, i, SpeedSide::Right)
        .or_else(|| speed_at_key(t, i, SpeedSide::Left))
        .unwrap_or(0.0)
}

/// Normalised view: each curve's key range mapped to 0..100.
fn norm(c: &Curve, v: f64, on: bool) -> f64 {
    if !on {
        return v;
    }
    let (lo, hi) = c.range;
    if hi - lo < 1e-12 {
        50.0
    } else {
        (v - lo) / (hi - lo) * 100.0
    }
}

fn denorm(c: &Curve, v: f64, on: bool) -> f64 {
    if !on {
        return v;
    }
    let (lo, hi) = c.range;
    lo + v / 100.0 * (hi - lo)
}

/// Draws the panel and returns the commands the user's actions ask for.
pub fn value_graph_ui(ui: &mut Ui, st: &mut EditorState) -> Vec<Cmd> {
    let mut g = graph_state(ui.ctx());
    let mut cmds: Vec<Cmd> = Vec::new();
    let theme = Theme::current(ui.ctx());
    let fps = f64::from(st.doc.scene().settings().fps);
    let current = st.timeline.current as f64;
    let mut curves = curves(st, &g.hidden, g.mode == GraphMode::Speed);
    let (mut frame_all, mut frame_sel) = (false, false);

    ui.horizontal_wrapped(|ui| {
        let previous = g.mode;
        ui.selectable_value(&mut g.mode, GraphMode::Value, "Value");
        ui.selectable_value(&mut g.mode, GraphMode::Speed, "Speed (/s)");
        let speed = g.mode == GraphMode::Speed;
        if g.mode != previous {
            std::mem::swap(&mut g.vview, &mut g.alternate_vview);
            g.drag = None;
            curves = self::curves(st, &g.hidden, speed);
        }
        frame_all = ui.button("Frame all").clicked();
        frame_sel = ui.button("Frame selected").clicked();
        ui.checkbox(&mut g.tview.snap, "Snap");
        ui.add_enabled(!speed, egui::Checkbox::new(&mut g.normalised, "Normalised"));
        ui.separator();
        let has = !g.selection.keys.is_empty();
        if speed
            && ui
                .add_enabled(has, egui::Button::new("Constant speed"))
                .clicked()
        {
            cmds.push((
                "key.set_interp",
                json!({"keys": g.selection.json(), "interp": "linear"}),
            ));
        }
        let value_presets = [
            ("Auto", "auto"),
            ("Linear", "linear"),
            ("Ease", "ease"),
            ("Ease in", "ease_in"),
            ("Ease out", "ease_out"),
            ("Zero angle", "zero_angle"),
            ("Zero length", "zero_length"),
        ];
        let speed_presets = [
            ("Ease", "ease"),
            ("Ease in", "ease_in"),
            ("Ease out", "ease_out"),
        ];
        for &(label, preset) in if speed {
            speed_presets.as_slice()
        } else {
            value_presets.as_slice()
        } {
            if ui.add_enabled(has, egui::Button::new(label)).clicked() {
                for k in &g.selection.keys {
                    cmds.push((
                        "key.set_tangent",
                        json!({"node": k.node, "track": k.track, "frame": k.frame,
                               "preset": preset}),
                    ));
                }
            }
        }
        for (label, broken, weighted) in [
            ("Break", Some(true), None),
            ("Unify", Some(false), None),
            ("Weighted", None, Some(true)),
            ("Unweighted", None, Some(false)),
        ] {
            if ui.add_enabled(has, egui::Button::new(label)).clicked() {
                for k in &g.selection.keys {
                    let mut a = json!({"node": k.node, "track": k.track, "frame": k.frame,
                                       "preset": "user"});
                    if let Some(b) = broken {
                        a["broken"] = json!(b);
                    }
                    if let Some(w) = weighted {
                        a["weighted"] = json!(w);
                    }
                    cmds.push(("key.set_tangent", a));
                }
            }
        }
        // numeric entry for one selected key
        if !speed && let [k] = g.selection.keys.as_slice() {
            let unit = unit_of(&curves, k);
            let value = st
                .doc
                .scene()
                .node(NodeId(k.node))
                .and_then(|n| n.tracks.get(&k.track))
                .and_then(|t| {
                    t.keys
                        .iter()
                        .find(|x| (x.time * fps - k.frame).abs() < 1e-6)
                        .map(|x| logic::display(unit, x.value))
                });
            if let Some(v) = value {
                ui.separator();
                let mut frame = k.frame;
                let mut val = v;
                let l = ui.label("Key frame");
                let rf = ui
                    .add(egui::DragValue::new(&mut frame).speed(0.1))
                    .labelled_by(l.id);
                let l = ui.label("Key value");
                let rv = ui
                    .add(egui::DragValue::new(&mut val).speed(0.1))
                    .labelled_by(l.id);
                if rf.changed() || rv.changed() {
                    cmds.push((
                        "key.move",
                        json!({"keys": g.selection.json(), "delta_frames": frame - k.frame,
                               "delta_value": val - v}),
                    ));
                    let moved = g.selection.moved(frame - k.frame);
                    g.selection = moved;
                }
            }
        }
    });
    let speed = g.mode == GraphMode::Speed;
    let normalised = g.normalised && !speed;

    let avail = ui.available_rect_before_wrap();
    let (outer, resp) = ui.allocate_exact_size(
        vec2(avail.width(), avail.height().clamp(160.0, 420.0)),
        Sense::click_and_drag(),
    );
    resp.widget_info(|| {
        egui::WidgetInfo::labeled(
            egui::Role::Label,
            true,
            if speed { "Speed graph" } else { "Value graph" },
        )
    });
    let canvas = Rect::from_min_max(Pos2::new(outer.left() + LIST_W, outer.top()), outer.max);
    let painter = ui.painter_at(outer);
    painter.rect_filled(outer, 0.0, theme.panel);

    if frame_all || frame_sel {
        frame(&mut g, &curves, st, frame_sel, canvas);
    }

    // curve list
    for (i, c) in curves.iter().enumerate() {
        painter.text(
            Pos2::new(outer.left() + 6.0, outer.top() + 4.0 + i as f32 * 16.0),
            Align2::LEFT_TOP,
            format!(
                "{} / {}",
                st.doc.scene().node(c.node).map_or("?", |n| n.name.as_str()),
                logic::track_label(&c.track)
            ),
            FontId::proportional(11.0),
            color32(logic::axis_color(&c.track)),
        );
    }
    // grid
    let painter = ui.painter_at(canvas);
    let step = grid_step(g.vview.ppu);
    let top = g.vview.to_value(canvas.top(), canvas.top());
    let bottom = g.vview.to_value(canvas.bottom(), canvas.top());
    let mut v = (bottom / step).floor() * step;
    while v <= top {
        let y = g.vview.to_y(v, canvas.top());
        painter.line_segment(
            [Pos2::new(canvas.left(), y), Pos2::new(canvas.right(), y)],
            Stroke::new(1.0, theme.faint),
        );
        painter.text(
            Pos2::new(canvas.left() + 2.0, y),
            Align2::LEFT_BOTTOM,
            format!("{v:.0}"),
            FontId::proportional(9.0),
            theme.weak_text,
        );
        v += step;
    }
    let cx = g.tview.to_x(current, canvas.left());
    painter.line_segment(
        [Pos2::new(cx, canvas.top()), Pos2::new(cx, canvas.bottom())],
        Stroke::new(1.0, theme.accent),
    );

    // curves
    let to_screen = |c: &Curve, frame: f64, value: f64| {
        Pos2::new(
            g.tview.to_x(frame, canvas.left()),
            g.vview.to_y(norm(c, value, normalised), canvas.top()),
        )
    };
    let mut keys: Vec<(KeyId, [f32; 2])> = Vec::new();
    let mut grips = Vec::new();
    for c in &curves {
        let Some(t) = st
            .doc
            .scene()
            .node(c.node)
            .and_then(|n| n.tracks.get(&c.track))
        else {
            continue;
        };
        let color = color32(logic::axis_color(&c.track));
        let mut pts = Vec::new();
        let mut x = canvas.left();
        while x <= canvas.right() {
            let frame = g.tview.to_frame(x, canvas.left());
            let v = logic::display(
                c.unit,
                if speed {
                    speed_at(t, frame / fps)
                } else {
                    forge_anim::value_at_exact(t, &[], frame / fps)
                },
            );
            pts.push(Pos2::new(
                x,
                g.vview.to_y(norm(c, v, normalised), canvas.top()),
            ));
            x += 2.0;
        }
        painter.add(egui::Shape::line(pts, Stroke::new(1.5, color)));
        for (i, k) in t.keys.iter().enumerate() {
            let frame = k.time * fps;
            let v = if speed { key_speed(t, i) } else { k.value };
            let p = to_screen(c, frame, logic::display(c.unit, v));
            let id = KeyId {
                node: c.node.0,
                track: c.track.clone(),
                frame,
            };
            let selected = g.selection.contains(&id);
            painter.circle(
                p,
                KEY_R,
                if selected { theme.accent } else { color },
                Stroke::new(1.0, theme.window),
            );
            if selected && speed {
                for (side, s) in [(SpeedSide::Left, -1), (SpeedSide::Right, 1)] {
                    if speed_editable(t, i, side)
                        && let Some(v) = speed_at_key(t, i, side)
                    {
                        let v = logic::display(c.unit, v);
                        let h = g.speed_grip(canvas, frame, v, side);
                        painter.line_segment(
                            [to_screen(c, frame, v), h],
                            Stroke::new(1.0, theme.weak_text),
                        );
                        painter.circle_stroke(h, HANDLE_R, Stroke::new(1.0, theme.text));
                        grips.push((id.clone(), s, h));
                    }
                }
            } else if selected
                && k.interp == Interp::Spline
                && !normalised
                && let Some((l, r)) = forge_anim::handles(t, i)
            {
                {
                    for (side, (dt, dv)) in [(-1i8, l), (1, r)] {
                        if (side < 0 && i == 0) || (side > 0 && i + 1 == t.keys.len()) {
                            continue;
                        }
                        let h =
                            to_screen(c, frame + dt * fps, logic::display(c.unit, k.value + dv));
                        painter.line_segment([p, h], Stroke::new(1.0, theme.weak_text));
                        painter.circle_stroke(h, HANDLE_R, Stroke::new(1.0, theme.text));
                        grips.push((id.clone(), side, h));
                    }
                }
            }
            keys.push((id, [p.x, p.y]));
        }
    }

    // interaction
    let mods = ui.input(|i| i.modifiers);
    let handle_hit = |pos: Pos2| {
        grips
            .iter()
            .find(|(_, _, h)| h.distance(pos) <= HANDLE_R + 3.0)
            .map(|(id, side, _)| (id.clone(), *side))
    };
    let key_hit =
        |pos: Pos2| logic::hit_test(&keys, [pos.x, pos.y], KEY_R + 3.0).map(|i| keys[i].0.clone());
    if resp.drag_started_by(egui::PointerButton::Primary)
        && let Some(pos) = ui.input(|i| i.pointer.press_origin())
        && pos.x >= canvas.left()
    {
        g.drag = Some(if let Some((id, side)) = handle_hit(pos) {
            (DragKind::Handle(side), pos, Some(id))
        } else if let Some(id) = key_hit(pos) {
            if !g.selection.contains(&id) {
                g.selection.click(id, mods.shift);
            }
            (DragKind::Move, pos, None)
        } else {
            (DragKind::Box, pos, None)
        });
    }
    if let Some((kind, origin, handle_key)) = g.drag.clone() {
        let now = ui
            .input(|i| i.pointer.latest_pos())
            .or_else(|| resp.interact_pointer_pos())
            .unwrap_or(origin);
        if kind == DragKind::Box && resp.dragged() {
            painter.rect_stroke(
                Rect::from_two_pos(origin, now),
                0.0,
                Stroke::new(1.0, theme.accent),
                egui::StrokeKind::Inside,
            );
        }
        if resp.drag_stopped() {
            match kind {
                DragKind::Box => {
                    let sel = logic::box_select(&keys, [origin.x, origin.y], [now.x, now.y]);
                    g.selection.set(sel, mods.shift);
                }
                DragKind::Move => {
                    let df = logic::drag_frames(&g.tview, now.x - origin.x);
                    let dv = if normalised || speed {
                        0.0
                    } else {
                        -f64::from(now.y - origin.y) / g.vview.ppu
                    };
                    if (df != 0.0 || dv != 0.0) && !g.selection.keys.is_empty() {
                        cmds.push((
                            "key.move",
                            json!({"keys": g.selection.json(), "delta_frames": df,
                                   "delta_value": dv}),
                        ));
                        g.selection = g.selection.moved(df);
                    }
                }
                DragKind::Handle(side) => {
                    if let Some(id) = handle_key {
                        if speed {
                            if now.y != origin.y
                                && let Some((_, _, grip)) =
                                    grips.iter().find(|(key, s, _)| key == &id && *s == side)
                            {
                                let value =
                                    g.vview.to_value(grip.y + now.y - origin.y, canvas.top());
                                cmds.push(("key.set_speed", json!({
                                    "node": id.node, "track": id.track, "frame": id.frame,
                                    "side": if side < 0 { "left" } else { "right" }, "speed": value,
                                })));
                            }
                        } else {
                            let c = curves
                                .iter()
                                .find(|c| c.node.0 == id.node && c.track == id.track);
                            if let Some(c) = c {
                                let frame = g.tview.to_frame(now.x, canvas.left());
                                let value =
                                    denorm(c, g.vview.to_value(now.y, canvas.top()), normalised);
                                let key_value = st
                                    .doc
                                    .scene()
                                    .node(c.node)
                                    .and_then(|n| n.tracks.get(&c.track))
                                    .and_then(|t| {
                                        t.keys
                                            .iter()
                                            .find(|k| (k.time * fps - id.frame).abs() < 1e-6)
                                            .map(|k| logic::display(c.unit, k.value))
                                    })
                                    .unwrap_or(0.0);
                                let h =
                                    json!({"dt_frames": frame - id.frame, "dv": value - key_value});
                                let mut a = json!({"node": id.node, "track": id.track,
                                               "frame": id.frame, "preset": "user"});
                                a[if side < 0 { "left" } else { "right" }] = h;
                                cmds.push(("key.set_tangent", a));
                            }
                        }
                    }
                }
            }
            g.drag = None;
        }
    } else if resp.clicked()
        && let Some(pos) = resp.interact_pointer_pos()
        && pos.x >= canvas.left()
    {
        match key_hit(pos) {
            Some(id) => g.selection.click(id, mods.shift),
            None if !mods.shift => g.selection.clear(),
            None => {}
        }
    }
    if resp.dragged_by(egui::PointerButton::Middle) {
        let d = resp.drag_delta();
        g.tview.pan(d.x);
        g.vview.pan(d.y);
    }
    if resp.hovered() {
        let (scroll, zoom, pos) = ui.input(|i| {
            (
                i.smooth_scroll_delta,
                i.zoom_delta(),
                i.pointer.hover_pos().unwrap_or(canvas.center()),
            )
        });
        if zoom != 1.0 {
            g.tview.zoom_about(pos.x, canvas.left(), f64::from(zoom));
            if mods.shift {
                g.vview.zoom_about(pos.y, canvas.top(), f64::from(zoom));
            }
        } else if scroll.x != 0.0 {
            g.tview.pan(scroll.x);
        } else if scroll.y != 0.0 {
            g.vview.pan(scroll.y);
        }
        if ui.input(|i| i.key_pressed(egui::Key::Delete)) && !g.selection.keys.is_empty() {
            cmds.push(("key.delete", json!({"keys": g.selection.json()})));
            g.selection.clear();
        }
    }
    // list toggles under the graph
    ui.horizontal_wrapped(|ui| {
        let all = logic::rows(st.doc.scene(), &[]);
        for r in all {
            if let RowKind::Track { track, .. } = r.kind
                && logic::has_curve(st.doc.scene(), r.node, &track)
            {
                let key = (r.node.0, track.clone());
                let mut on = !g.hidden.contains(&key);
                let name = format!("{} {}", r.label, logic::track_label(&track));
                if ui.checkbox(&mut on, name).changed() {
                    if on {
                        g.hidden.remove(&key);
                    } else {
                        g.hidden.insert(key);
                    }
                }
            }
        }
    });
    g.canvas = Some(canvas);
    set_graph_state(ui.ctx(), g);
    cmds
}

fn color32(c: [u8; 3]) -> egui::Color32 {
    egui::Color32::from_rgb(c[0], c[1], c[2])
}

fn unit_of(curves: &[Curve], k: &KeyId) -> Unit {
    curves
        .iter()
        .find(|c| c.node.0 == k.node && c.track == k.track)
        .map(|c| c.unit)
        .unwrap_or_default()
}

/// A grid line spacing in value units (1, 2 or 5 times a power of ten) that keeps the lines
/// at least 30 points apart.
fn grid_step(ppu: f64) -> f64 {
    let mut e = 1e-3;
    loop {
        for m in [1.0, 2.0, 5.0] {
            if m * e * ppu >= 30.0 || e > 1e9 {
                return m * e;
            }
        }
        e *= 10.0;
    }
}

/// Frames the view on all keys of the visible curves, or on the selected ones.
fn frame(
    g: &mut ValueGraph,
    curves: &[Curve],
    st: &EditorState,
    selected_only: bool,
    canvas: Rect,
) {
    let fps = f64::from(st.doc.scene().settings().fps);
    let speed = g.mode == GraphMode::Speed;
    let (mut f0, mut f1, mut v0, mut v1) = (
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
    );
    for c in curves {
        let Some(t) = st
            .doc
            .scene()
            .node(c.node)
            .and_then(|n| n.tracks.get(&c.track))
        else {
            continue;
        };
        let mut time_range: Option<(f64, f64)> = None;
        for (i, k) in t.keys.iter().enumerate() {
            let frame = k.time * fps;
            let id = KeyId {
                node: c.node.0,
                track: c.track.clone(),
                frame,
            };
            if selected_only && !g.selection.contains(&id) {
                continue;
            }
            f0 = f0.min(frame);
            f1 = f1.max(frame);
            if speed {
                time_range = Some((time_range.map_or(k.time, |(lo, _)| lo), k.time));
                for side in [SpeedSide::Left, SpeedSide::Right] {
                    if let Some(v) = speed_at_key(t, i, side) {
                        let v = logic::display(c.unit, v);
                        v0 = v0.min(v);
                        v1 = v1.max(v);
                    }
                }
            } else {
                let v = norm(c, logic::display(c.unit, k.value), g.normalised);
                v0 = v0.min(v);
                v1 = v1.max(v);
            }
        }
        if let Some((lo, hi)) = time_range {
            for s in 0..=128 {
                let time = lo + (hi - lo) * f64::from(s) / 128.0;
                let v = logic::display(c.unit, speed_at(t, time));
                v0 = v0.min(v);
                v1 = v1.max(v);
            }
        }
    }
    if f0.is_finite() {
        g.tview.frame_range(f0, f1, canvas.width());
        if speed && v1 - v0 < 1.0 {
            v0 -= 0.5;
            v1 += 0.5;
        }
        g.vview.frame_range(v0, v1, canvas.height());
    }
}
