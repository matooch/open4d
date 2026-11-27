//! Descriptor-derived key indicator, row commands and transient edits (UI.md §12).

use std::collections::BTreeMap;

use egui::{Color32, Sense, Stroke, Ui};
use forge_app::parameter_keys;
use forge_command::{display_to_stored, param_value_from_json};
use forge_core::{NodeId, ParamDesc, Scene, Value};
use serde_json::{Value as Json, json};

use crate::attributes::to_display;
use crate::object_manager::Cmd;
use crate::theme::Theme;
use crate::widgets::Edit;

#[derive(Default)]
pub(crate) struct Drafts {
    scope: Option<(u64, NodeId, i64, bool)>,
    pub values: BTreeMap<String, Value>,
}

impl Drafts {
    pub fn refresh(&mut self, revision: u64, node: NodeId, frame: i64, enabled: bool) {
        let scope = (revision, node, frame, enabled);
        if self.scope != Some(scope) {
            self.values.clear();
            self.scope = Some(scope);
        }
    }

    pub fn clear(&mut self) {
        self.values.clear();
        self.scope = None;
    }
}

pub(crate) fn draft_value(desc: &ParamDesc, current: &Value, edit: &Edit) -> Option<Value> {
    let value = if let Some(axis) = edit.component {
        let Value::Vec3(mut vector) = *current else {
            return None;
        };
        *vector.get_mut(usize::from(axis))? = display_to_stored(desc.ui.unit, edit.value.as_f64()?);
        Value::Vec3(vector)
    } else {
        param_value_from_json(desc, &edit.value).ok()?
    };
    desc.validate(value).ok()
}

pub(crate) struct RowKeys {
    node: NodeId,
    param: String,
    frame: i64,
    unit: forge_core::Unit,
    pub animated: bool,
    here: Vec<Json>,
    prev: Option<i64>,
    next: Option<i64>,
}

impl RowKeys {
    pub fn of(scene: &Scene, node: NodeId, param: &str, frame: i64) -> Option<Self> {
        let desc = scene.param_desc(node, param)?;
        let tracks = parameter_keys::tracks(desc);
        if tracks.is_empty() {
            return None;
        }
        let settings = scene.settings();
        let fps = f64::from(settings.fps);
        let keys = &scene.node(node)?.tracks;
        let mut info = Self {
            node,
            param: param.into(),
            frame,
            unit: desc.ui.unit,
            animated: false,
            here: Vec::new(),
            prev: None,
            next: None,
        };
        let mut frames = Vec::new();
        for track in tracks {
            let Some(data) = keys.get(&track) else {
                continue;
            };
            info.animated = true;
            for key in &data.keys {
                let at = key.time * fps;
                if (at - frame as f64).abs() < 1e-6 {
                    info.here
                        .push(json!({"node": node.0, "track": track, "frame": at}));
                } else if at >= settings.start as f64 - 1e-6 && at <= settings.end as f64 + 1e-6 {
                    frames.push(at);
                }
            }
        }
        frames.sort_by(f64::total_cmp);
        info.prev = forge_app::animate::prev_key(&frames, frame);
        info.next = forge_app::animate::next_key(&frames, frame);
        Some(info)
    }

    fn set(&self, value: &Value) -> Option<Cmd> {
        let value = match value {
            Value::Float(v) => json!(to_display(self.unit, *v)),
            Value::Int(v) => json!(v),
            Value::Vec3(v) => json!(v.map(|v| to_display(self.unit, v))),
            Value::Color(v) => json!(v),
            Value::Bool(v) => json!(v),
            Value::Enum(v) => json!(v),
            Value::Link(v) => json!(v.map(|id| id.0)),
            _ => return None,
        };
        Some((
            "key.set_parameter",
            json!({
                "node": self.node.0, "param": self.param, "frame": self.frame, "value": value,
            }),
        ))
    }

    /// What a click on the button does (UI.md §12, owner note N3): with a key at this frame
    /// and no uncommitted edit it removes the row's keys here (`key.delete`); in every other
    /// state it adds or updates the key.
    pub fn click(&self, value: &Value, changed: bool) -> Option<Cmd> {
        if !changed && !self.here.is_empty() {
            Some(("key.delete", json!({"keys": self.here})))
        } else {
            self.set(value)
        }
    }

    pub fn show(&self, ui: &mut Ui, label: &str, value: &Value, changed: bool) -> Option<Cmd> {
        let theme = Theme::current(ui.ctx());
        let clear = Color32::TRANSPARENT;
        let (state, stroke, fill) = if changed {
            let warn = ui.visuals().warn_fg_color;
            ("changed", warn, warn)
        } else if !self.here.is_empty() {
            ("keyed", theme.accent, theme.accent)
        } else if self.animated {
            ("animated", theme.accent, clear)
        } else {
            ("unkeyed", theme.weak_text, clear)
        };
        let (rect, response) = ui.allocate_exact_size(egui::vec2(12.0, 12.0), Sense::click());
        response.widget_info(|| {
            egui::WidgetInfo::labeled(
                egui::Role::Button,
                ui.is_enabled(),
                format!("{label} {state}"),
            )
        });
        let c = rect.center();
        let r = 4.5;
        ui.painter().add(egui::Shape::convex_polygon(
            vec![
                c + egui::vec2(0.0, -r),
                c + egui::vec2(r, 0.0),
                c + egui::vec2(0.0, r),
                c + egui::vec2(-r, 0.0),
            ],
            fill,
            Stroke::new(1.5, stroke),
        ));
        if response.has_focus() {
            ui.painter().rect_stroke(
                rect,
                2.0,
                Stroke::new(1.0, theme.focus),
                egui::StrokeKind::Inside,
            );
        }
        let mut cmd = if response.clicked() {
            self.click(value, changed)
        } else {
            None
        };
        response.context_menu(|ui| {
            if ui.button("Add / Update Key on Parameter").clicked() {
                cmd = self.set(value);
                ui.close();
            }
            if ui
                .add_enabled(
                    !self.here.is_empty(),
                    egui::Button::new("Delete Key on Parameter"),
                )
                .clicked()
            {
                cmd = Some(("key.delete", json!({"keys": self.here})));
                ui.close();
            }
            for (label, frame) in [
                ("Previous Key on Parameter", self.prev),
                ("Next Key on Parameter", self.next),
            ] {
                if ui
                    .add_enabled(frame.is_some(), egui::Button::new(label))
                    .clicked()
                    && let Some(frame) = frame
                {
                    cmd = Some(("timeline.set_frame", json!({"frame": frame})));
                    ui.close();
                }
            }
        });
        cmd
    }
}
