//! Int: whole number inside the parameter's range, with the chevrons of `number`.

use forge_core::Value;
use serde_json::json;

use super::number::{self, NumberField};
use super::scrub;
use super::{Cx, Edit, Phase};
use crate::attributes::AttrRow;

pub fn show(ui: &mut egui::Ui, cx: &Cx, row: &AttrRow, min: i64, max: i64) -> Option<Edit> {
    let Value::Int(v) = row.value else {
        return None;
    };
    let mut x = v as f64;
    let (r, changed) = number::show(
        ui,
        &mut x,
        &NumberField {
            base: scrub::base_step(forge_core::Unit::None, true, row.step),
            prefix: String::new(),
            suffix: "",
            mixed: row.mixed,
            label: cx.label,
            name: row.label.clone(),
            step: scrub::chevron_step(forge_core::Unit::None, true),
            range: Some((min as f64, max as f64)),
            is_int: true,
        },
    );
    changed.then(|| Edit {
        value: json!(x.round() as i64),
        component: None,
        target: None,
        create: None,
        phase: if r.dragged() {
            Phase::Drag {
                started: r.drag_started(),
            }
        } else {
            Phase::Commit
        },
    })
}
