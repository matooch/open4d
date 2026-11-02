//! String: a single-line text field; empty with a "mixed" hint while the selection disagrees.

use forge_core::Value;
use serde_json::json;

use super::{Cx, Edit, Phase};
use crate::attributes::AttrRow;

pub fn show(ui: &mut egui::Ui, cx: &Cx, row: &AttrRow) -> Option<Edit> {
    let Value::String(s) = &row.value else {
        return None;
    };
    let mut buf = if row.mixed { String::new() } else { s.clone() };
    let mut te = egui::TextEdit::singleline(&mut buf);
    if row.mixed {
        te = te.hint_text("mixed");
    }
    let r = ui.add(te).labelled_by(cx.label);
    r.changed().then(|| Edit {
        value: json!(buf),
        component: None,
        target: None,
        phase: Phase::Drag {
            started: r.gained_focus(),
        },
    })
}
