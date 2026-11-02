//! Link: a drop-down of accepted objects, a clear button, and a drop target for objects
//! dragged from the object manager.

use forge_core::{Category, NodeId, Scene, Value};
use serde_json::json;

use super::{Cx, Edit};
use crate::attributes::AttrRow;
use crate::tree::tree_rows;

/// Objects (other than `exclude`) of the accepted categories, in tree order.
pub fn candidates(scene: &Scene, accepts: &[Category], exclude: NodeId) -> Vec<(NodeId, String)> {
    let mut all = std::collections::BTreeSet::new();
    for r in scene.roots() {
        all.insert(*r);
        all.extend(scene.descendants(*r));
    }
    tree_rows(scene, &all)
        .into_iter()
        .filter(|r| r.id != exclude)
        .filter(|r| {
            accepts.is_empty()
                || scene
                    .node_type(r.id)
                    .is_some_and(|t| accepts.contains(&t.category))
        })
        .map(|r| (r.id, r.name))
        .collect()
}

pub fn name_of(scene: &Scene, id: NodeId) -> String {
    scene
        .node(id)
        .map_or(format!("(missing {id})"), |n| n.name.clone())
}

pub fn show(ui: &mut egui::Ui, cx: &Cx, row: &AttrRow, accepts: &[Category]) -> Option<Edit> {
    let Value::Link(target) = &row.value else {
        return None;
    };
    let current = if row.mixed {
        "(mixed)".to_string()
    } else {
        target.map_or("None".to_string(), |t| name_of(cx.scene, t))
    };
    let mut edit = None;
    ui.horizontal(|ui| {
        let combo = egui::ComboBox::from_id_salt(("link", cx.node, &row.param))
            .selected_text(current)
            .show_ui(ui, |ui| {
                if ui
                    .selectable_label(!row.mixed && target.is_none(), "None")
                    .clicked()
                {
                    edit = Some(Edit::commit(json!(null)));
                }
                for (cid, name) in candidates(cx.scene, accepts, cx.node) {
                    if ui
                        .selectable_label(!row.mixed && *target == Some(cid), name)
                        .clicked()
                    {
                        edit = Some(Edit::commit(json!(cid.0)));
                    }
                }
            });
        combo.response.clone().labelled_by(cx.label);
        if let Some(dropped) = combo.response.dnd_release_payload::<NodeId>() {
            edit = Some(Edit::commit(json!(dropped.0)));
        }
        if target.is_some() || row.mixed {
            let clear = ui.small_button("x").on_hover_text("Clear");
            clear.widget_info(|| {
                egui::WidgetInfo::labeled(egui::Role::Button, true, format!("Clear {}", row.label))
            });
            if clear.clicked() {
                edit = Some(Edit::commit(json!(null)));
            }
        }
    });
    edit
}
