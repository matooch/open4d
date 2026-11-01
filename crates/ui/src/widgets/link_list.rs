//! Link list (ordered, e.g. an effector or field list): one line per entry with move and
//! remove buttons, an "add" drop-down and a drop target for dragged objects.

use forge_core::{Category, NodeId, Value};
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
