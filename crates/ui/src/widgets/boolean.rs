//! Bool: a checkbox carrying the row label; indeterminate while the selection disagrees.

use forge_core::Value;
use serde_json::json;

use super::{Cx, Edit};
use crate::attributes::AttrRow;

pub fn show(ui: &mut egui::Ui, _cx: &Cx, row: &AttrRow) -> Option<Edit> {
    let Value::Bool(v) = row.value else {
        return None;
    };
    ui.label("");
    let mut b = v;
    ui.add(egui::Checkbox::new(&mut b, &row.label).indeterminate(row.mixed))
        .changed()
        .then(|| Edit::commit(json!(b)))
}
