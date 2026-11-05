//! Link list (ordered, e.g. an effector list): one line per entry with an enable box,
//! move and remove buttons, an "add" drop-down and a drop
//! target for dragged objects (docs/specs/UI.md §16.1).

use forge_core::visibility::ENABLED;
use forge_core::{Category, EnumOption, LinkRow, NodeId, Scene, Value};
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

fn rows_json(rows: &[LinkRow]) -> serde_json::Value {
    json!(
        rows.iter()
            .map(|r| json!({
                "target": r.target.0,
                "blend": r.blend,
                "strength": r.strength * 100.0,
                "enabled": r.enabled,
            }))
            .collect::<Vec<_>>()
    )
}

/// A list with a row of data per entry (a field list, FIELDS.md F7): enable box, name, blend
/// mode, strength in percent, move and remove buttons. Every change writes the whole list, one
/// undo step.
pub fn show_rows(
    ui: &mut egui::Ui,
    cx: &Cx,
    row: &AttrRow,
    accepts: &[Category],
    blends: &[EnumOption],
) -> Option<Edit> {
    let Value::LinkRows(items) = &row.value else {
        return None;
    };
    let mut edit = None;
    let default_blend = blends.first().map(|b| b.id.as_str()).unwrap_or_default();
    ui.vertical(|ui| {
        if row.mixed {
            ui.weak("mixed (the list shown is the active object's)");
        }
        for (i, item) in items.iter().enumerate() {
            let name = name_of(cx.scene, item.target);
            ui.horizontal(|ui| {
                let mut on = item.enabled;
                let r = ui.add(egui::Checkbox::without_text(&mut on));
                r.widget_info(|| {
                    egui::WidgetInfo::selected(
                        egui::Role::CheckBox,
                        true,
                        item.enabled,
                        format!("Enable {name}"),
                    )
                });
                if r.changed() {
                    let mut rows = items.clone();
                    rows[i].enabled = on;
                    edit = Some(Edit::commit(rows_json(&rows)));
                }
                ui.label(&name);
                let current = blends
                    .iter()
                    .find(|b| b.id == item.blend)
                    .map_or(item.blend.as_str(), |b| b.label.as_str());
                let combo = egui::ComboBox::from_id_salt(("rowblend", cx.node, &row.param, i))
                    .selected_text(current)
                    .show_ui(ui, |ui| {
                        for b in blends {
                            if ui.selectable_label(b.id == item.blend, &b.label).clicked() {
                                let mut rows = items.clone();
                                rows[i].blend = b.id.clone();
                                edit = Some(Edit::commit(rows_json(&rows)));
                            }
                        }
                    });
                combo.response.widget_info(|| {
                    egui::WidgetInfo::labeled(egui::Role::ComboBox, true, format!("Blend {name}"))
                });
                let pct = item.strength * 100.0;
                let mut v = pct;
                let r = ui.add(egui::DragValue::new(&mut v).speed(1.0).suffix(" %"));
                r.widget_info(|| {
                    let mut info = egui::WidgetInfo::drag_value(true, pct);
                    info.label = Some(format!("Strength {name}"));
                    info
                });
                if r.changed() {
                    let mut rows = items.clone();
                    rows[i].strength = v / 100.0;
                    edit = Some(Edit::commit(rows_json(&rows)));
                }
                for (text, delta, what) in [("^", -1isize, "up"), ("v", 1, "down")] {
                    if button(ui, text, format!("Move {name} {what}")) {
                        let mut rows = items.clone();
                        let j = i as isize + delta;
                        if j >= 0 && (j as usize) < rows.len() {
                            rows.swap(i, j as usize);
                        }
                        edit = Some(Edit::commit(rows_json(&rows)));
                    }
                }
                if button(ui, "x", format!("Remove {name}")) {
                    let mut rows = items.clone();
                    rows.remove(i);
                    edit = Some(Edit::commit(rows_json(&rows)));
                }
            });
        }
        let has = |id: NodeId| items.iter().any(|r| r.target == id);
        let combo = egui::ComboBox::from_id_salt(("linkrows", cx.node, &row.param))
            .selected_text("Add...")
            .show_ui(ui, |ui| {
                for (cid, name) in candidates(cx.scene, accepts, cx.node) {
                    if !has(cid) && ui.selectable_label(false, name).clicked() {
                        let mut rows = items.clone();
                        rows.push(LinkRow::new(cid, default_blend));
                        edit = Some(Edit::commit(rows_json(&rows)));
                    }
                }
            });
        combo.response.clone().labelled_by(cx.label);
        if let Some(dropped) = combo.response.dnd_release_payload::<NodeId>()
            && !has(*dropped)
        {
            let mut rows = items.clone();
            rows.push(LinkRow::new(*dropped, default_blend));
            edit = Some(Edit::commit(rows_json(&rows)));
        }
    });
    edit
}
