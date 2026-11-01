//! Enum: a drop-down of the option labels.

use forge_core::{EnumOption, Value};
use serde_json::json;

use super::{Cx, Edit};
use crate::attributes::AttrRow;

pub fn show(ui: &mut egui::Ui, cx: &Cx, row: &AttrRow, options: &[EnumOption]) -> Option<Edit> {
    let Value::Enum(cur) = &row.value else {
        return None;
    };
    let current = if row.mixed {
        "(mixed)"
    } else {
        options
            .iter()
            .find(|o| o.id == *cur)
            .map_or(cur.as_str(), |o| o.label.as_str())
    };
    let mut edit = None;
    egui::ComboBox::from_id_salt(("combo", cx.node, &row.param))
        .selected_text(current)
        .show_ui(ui, |ui| {
            for o in options {
                if ui
                    .selectable_label(!row.mixed && o.id == *cur, &o.label)
                    .clicked()
                {
                    edit = Some(Edit::commit(json!(o.id)));
                }
            }
        })
        .response
        .labelled_by(cx.label);
    edit
}
