//! The number field shared by the float, int and vector widgets (docs/specs/UI.md §12;
//! observed, G4 item 1): a dark box with a left and a right chevron. A chevron steps the
//! value by one unit. Click to edit the text, double-click to select it, or drag to scrub.
//! Return commits the typed value.

use egui::accesskit::{Action, ActionData};
use egui::{Align2, FontId, Key, Pos2, Rect, Response, Sense, Stroke, TextEdit, Ui, vec2};

use super::scrub::{self, Mods};

pub struct NumberField<'a> {
    pub base: f64,
    pub prefix: String,
    pub suffix: &'a str,
    pub mixed: bool,
    pub label: egui::Id,
    /// The row's name, for the chevrons' accessible labels (`Increase <name>`).
    pub name: String,
    /// What a chevron click adds, in display units (see [`scrub::chevron_step`]).
    pub step: f64,
    /// Display-unit range the value stays in, if the parameter has one.
    pub range: Option<(f64, f64)>,
    /// Whole numbers only.
    pub is_int: bool,
}

const CHEVRON_W: f32 = 12.0;
const HEIGHT: f32 = 20.0;
const VALUE_W: f32 = 56.0;

/// The value after one chevron click: `dir` is -1 (left chevron) or +1 (right), the step is
/// one unit, and the result stays inside `range`.
pub fn step_value(value: f64, dir: i32, step: f64, range: Option<(f64, f64)>) -> f64 {
    let v = value + f64::from(dir) * step;
    // round away the binary noise of repeated steps (0.1 + 0.2)
    let v = (v * 1e9).round() / 1e9;
    match range {
        Some((lo, hi)) if lo <= hi => v.clamp(lo, hi),
        _ => v,
    }
}

/// The text of a value in the box: no trailing zeros, at most 4 decimals, none for ints.
pub fn format_value(value: f64, is_int: bool) -> String {
    if is_int {
        return format!("{}", value.round() as i64);
    }
    let t = format!("{value:.4}");
    let t = t.trim_end_matches('0').trim_end_matches('.');
    if t.is_empty() || t == "-" || t == "-0" {
        "0".to_string()
    } else {
        t.to_string()
    }
}

/// Reads the text a user typed: the number, with a unit suffix or the prefix ignored.
pub fn parse_value(text: &str, f: &NumberField) -> Option<f64> {
    let t = text.trim();
    let t = t.strip_prefix(f.prefix.trim()).unwrap_or(t).trim();
    let t = t.strip_suffix(f.suffix.trim()).unwrap_or(t).trim();
    let v: f64 = t.replace(',', ".").parse().ok()?;
    v.is_finite().then_some(v)
}

fn clamp(v: f64, f: &NumberField) -> f64 {
    let v = if f.is_int { v.round() } else { v };
    match f.range {
        Some((lo, hi)) if lo <= hi => v.clamp(lo, hi),
        _ => v,
    }
}

/// The edit buffer of a field being edited, kept in egui's memory under the field's id.
#[derive(Clone)]
struct Editing {
    text: String,
    first_click_x: Option<f32>,
}

fn chevron(ui: &mut Ui, left: bool, label: String) -> Response {
    let (rect, r) = ui.allocate_exact_size(vec2(CHEVRON_W, HEIGHT), Sense::click());
    r.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, ui.is_enabled(), label.clone()));
    let colour = if r.hovered() {
        ui.visuals().strong_text_color()
    } else {
        ui.visuals().weak_text_color()
    };
    let c = rect.center();
    let (dx, dy) = (2.0, 4.0);
    let (tip, tail) = if left { (-dx, dx) } else { (dx, -dx) };
    let stroke = Stroke::new(1.4, colour);
    ui.painter()
        .line_segment([c + vec2(tail, -dy), c + vec2(tip, 0.0)], stroke);
    ui.painter()
        .line_segment([c + vec2(tip, 0.0), c + vec2(tail, dy)], stroke);
    r
}

/// Shows `value`; true in `.1` if the user changed it. The returned response is the part
/// that changed it (a chevron, the value box when scrubbed or edited), so `dragged()` tells a
/// scrub from a single step.
pub fn show(ui: &mut Ui, value: &mut f64, f: &NumberField) -> (Response, bool) {
    let mods = ui.input(|i| Mods::of(&i.modifiers));
    let id = f.label.with(("number", &f.prefix, &f.name));
    let mut result: Option<Response> = None;
    let mut changed = false;
    let spacing = ui.spacing().item_spacing.x;
    ui.spacing_mut().item_spacing.x = 0.0;
    ui.horizontal(|ui| {
        let down = chevron(ui, true, format!("Decrease {}{}", f.prefix, f.name));
        if down.clicked() {
            *value = step_value(*value, -1, f.step, f.range);
            changed = true;
            result = Some(down.clone());
        }
        // the value box
        let (rect, mut r) = ui.allocate_exact_size(vec2(VALUE_W, HEIGHT), Sense::click_and_drag());
        r = r.labelled_by(f.label);
        r.widget_info(|| egui::WidgetInfo::drag_value(ui.is_enabled(), *value));
        ui.ctx().accesskit_node_builder(r.id, |builder| {
            builder.set_numeric_value_step(f.step);
            builder.add_action(Action::SetValue);
            builder.add_action(Action::Increment);
            builder.add_action(Action::Decrement);
        });
        // assistive technology sets or steps the value like a drag value does
        let requests = ui.input(|i| {
            let set = i
                .accesskit_action_requests(r.id, Action::SetValue)
                .filter_map(|q| match q.data {
                    Some(ActionData::NumericValue(v)) => Some(v),
                    _ => None,
                })
                .last();
            let inc = i.num_accesskit_action_requests(r.id, Action::Increment) as i32
                - i.num_accesskit_action_requests(r.id, Action::Decrement) as i32;
            (set, inc)
        });
        if let Some(v) = requests.0 {
            let v = clamp(v, f);
            if v != *value {
                *value = v;
                changed = true;
                result = Some(r.clone());
            }
        } else if requests.1 != 0 {
            *value = step_value(*value, requests.1.signum(), f.step, f.range);
            changed = true;
            result = Some(r.clone());
        }
        let editing: Option<Editing> = ui.data(|d| d.get_temp(id));
        if let Some(mut editing) = editing {
            let mut edit = ui.new_child(egui::UiBuilder::new().max_rect(rect));
            let mut output = TextEdit::singleline(&mut editing.text)
                .desired_width(VALUE_W)
                .align(Align2::CENTER_CENTER)
                .show(&mut edit);
            let te = output.response.response;
            te.widget_info(|| {
                egui::WidgetInfo::labeled(
                    egui::Role::TextInput,
                    true,
                    format!("Edit {}{}", f.prefix, f.name),
                )
            });
            let clicked = editing.first_click_x.take();
            if clicked.is_some() {
                te.request_focus();
            }
            let cursor = if te.double_clicked() {
                Some(egui::text::CCursorRange::select_all(&output.galley))
            } else {
                clicked.map(|x| {
                    egui::text::CCursorRange::one(
                        output
                            .galley
                            .cursor_from_pos(vec2(x - output.galley_pos.x, 0.0)),
                    )
                })
            };
            if let Some(cursor) = cursor {
                output.state.cursor.set_char_range(Some(cursor));
                output.state.store(ui.ctx(), te.id);
                ui.ctx().request_repaint();
            }
            let escape = ui.input(|i| i.key_pressed(Key::Escape));
            let enter = ui.input(|i| i.key_pressed(Key::Enter));
            if escape {
                ui.data_mut(|d| d.remove::<Editing>(id));
            } else if enter || te.lost_focus() {
                ui.data_mut(|d| d.remove::<Editing>(id));
                if let Some(v) = parse_value(&editing.text, f) {
                    let v = clamp(v, f);
                    if v != *value || f.mixed {
                        *value = v;
                        changed = true;
                        result = Some(te.clone());
                    }
                }
            } else {
                ui.data_mut(|d| d.insert_temp(id, editing));
            }
        } else {
            let fill = ui.visuals().extreme_bg_color;
            ui.painter().rect_filled(rect, 2.0, fill);
            let text = if f.mixed {
                "mixed".to_string()
            } else {
                format!("{}{}{}", f.prefix, format_value(*value, f.is_int), f.suffix)
            };
            let ink = ui.visuals().text_color();
            ui.painter().text(
                Pos2::new(rect.center().x, rect.center().y),
                Align2::CENTER_CENTER,
                text,
                FontId::proportional(13.0),
                ink,
            );
            if r.hovered() {
                ui.painter().rect_stroke(
                    Rect::from_min_max(rect.min, rect.max),
                    2.0,
                    Stroke::new(1.0, ui.visuals().weak_text_color()),
                    egui::StrokeKind::Inside,
                );
            }
            if r.clicked() {
                let text = if f.mixed {
                    String::new()
                } else {
                    format_value(*value, f.is_int)
                };
                let first_click_x = Some(r.interact_pointer_pos().map_or(rect.center().x, |p| p.x));
                ui.data_mut(|d| {
                    d.insert_temp(
                        id,
                        Editing {
                            text,
                            first_click_x,
                        },
                    )
                });
                ui.ctx().request_repaint();
            } else if r.dragged() && r.drag_delta().x != 0.0 {
                let speed = scrub::step(f.base, mods);
                let v = clamp(*value + f64::from(r.drag_delta().x) * speed, f);
                if v != *value {
                    *value = v;
                    changed = true;
                    result = Some(r.clone());
                }
            }
        }
        let up = chevron(ui, false, format!("Increase {}{}", f.prefix, f.name));
        if up.clicked() {
            *value = step_value(*value, 1, f.step, f.range);
            changed = true;
            result = Some(up.clone());
        }
        if result.is_none() {
            result = Some(up);
        }
    });
    ui.spacing_mut().item_spacing.x = spacing;
    (result.expect("the up chevron always answers"), changed)
}
