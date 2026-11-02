//! Link list (ordered, e.g. an effector or field list): one line per entry with an enable box,
//! a strength field (field layers), move and remove buttons, an "add" drop-down and a drop
//! target for dragged objects (docs/specs/UI.md §16.1).

use forge_core::visibility::ENABLED;
use forge_core::{Category, NodeId, Scene, Unit, Value};
use serde_json::json;

use super::link::{candidates, name_of};
use super::{Cx, Edit};
use crate::attributes::AttrRow;

/// `items` after moving entry `i` by `delta` places (no change when out of range).
pub fn moved(items: &[NodeId], i: usize, delta: isize) -> Vec<NodeId> {
    let mut out = items.to_vec();
    let j = i as isize + delta;
    if i < out.len() && j >= 0 && (j as usize) < out.len() {
        out.swap(i, j as usize);
    }
    out
}

fn ids(items: &[NodeId]) -> serde_json::Value {
    json!(items.iter().map(|n| n.0).collect::<Vec<_>>())
}

/// The parameter that switches a list entry on and off, and its value: the entry's own
/// `enabled` parameter (effectors, deformers), else the object state `object.enabled`
/// (fields). `None` for entries without either (tags, missing nodes).
pub fn enable_param(scene: &Scene, id: NodeId) -> Option<(&'static str, bool)> {
    ["enabled", ENABLED]
        .into_iter()
        .find_map(|p| match scene.param(id, p) {
            Some(Value::Bool(b)) => Some((p, *b)),
            _ => None,
        })
}

/// A list entry's row strength in percent: the node's `layer_strength` (fields, FIELDS.md).
pub fn row_strength(scene: &Scene, id: NodeId) -> Option<f64> {
    match scene.param(id, forge_fields::LAYER_STRENGTH) {
        Some(Value::Float(v)) => Some(crate::attributes::to_display(Unit::Percent, *v)),
        _ => None,
    }
}

fn button(ui: &mut egui::Ui, text: &str, label: String) -> bool {
    let r = ui.small_button(text);
    r.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, label.clone()));
    r.clicked()
}

pub fn show(ui: &mut egui::Ui, cx: &Cx, row: &AttrRow, accepts: &[Category]) -> Option<Edit> {
    let Value::LinkList(items) = &row.value else {
        return None;
    };
    let mut edit = None;
    ui.vertical(|ui| {
        if row.mixed {
            ui.weak("mixed (the list shown is the active object's)");
        }
        for (i, id) in items.iter().enumerate() {
            let name = name_of(cx.scene, *id);
            ui.horizontal(|ui| {
                if let Some((param, on)) = enable_param(cx.scene, *id) {
                    let mut b = on;
                    let r = ui.add(egui::Checkbox::without_text(&mut b));
                    r.widget_info(|| {
                        egui::WidgetInfo::selected(
                            egui::Role::CheckBox,
                            true,
                            on,
                            format!("Enable {name}"),
                        )
                    });
                    if r.changed() {
                        edit = Some(Edit::on_node(*id, param, json!(b)));
                    }
                }
                ui.label(&name);
                if let Some(pct) = row_strength(cx.scene, *id) {
                    let mut v = pct;
                    let r = ui.add(egui::DragValue::new(&mut v).speed(1.0).suffix(" %"));
                    r.widget_info(|| {
                        let mut info = egui::WidgetInfo::drag_value(true, pct);
                        info.label = Some(format!("Strength {name}"));
                        info
                    });
                    if r.changed() {
                        edit = Some(Edit::on_node(*id, forge_fields::LAYER_STRENGTH, json!(v)));
                    }
                }
                if button(ui, "^", format!("Move {name} up")) {
                    edit = Some(Edit::commit(ids(&moved(items, i, -1))));
                }
                if button(ui, "v", format!("Move {name} down")) {
                    edit = Some(Edit::commit(ids(&moved(items, i, 1))));
                }
                if button(ui, "x", format!("Remove {name}")) {
                    let mut rest = items.clone();
                    rest.remove(i);
                    edit = Some(Edit::commit(ids(&rest)));
                }
            });
        }
        let combo = egui::ComboBox::from_id_salt(("linklist", cx.node, &row.param))
            .selected_text("Add...")
            .show_ui(ui, |ui| {
                for (cid, name) in candidates(cx.scene, accepts, cx.node) {
                    if !items.contains(&cid) && ui.selectable_label(false, name).clicked() {
                        let mut more = items.clone();
                        more.push(cid);
                        edit = Some(Edit::commit(ids(&more)));
                    }
                }
            });
        combo.response.clone().labelled_by(cx.label);
        if let Some(dropped) = combo.response.dnd_release_payload::<NodeId>()
            && !items.contains(&dropped)
        {
            let mut more = items.clone();
            more.push(*dropped);
            edit = Some(Edit::commit(ids(&more)));
        }
    });
    edit
}
