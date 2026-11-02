//! Float: scrubbable number in display units (degrees, percent).

use forge_core::{Unit, Value};
use serde_json::json;

use super::number::{self, NumberField};
use super::scrub;
use super::{Cx, Edit, Phase};
use crate::attributes::{AttrRow, to_display, unit_suffix};

pub fn show(ui: &mut egui::Ui, cx: &Cx, row: &AttrRow, unit: Unit) -> Option<Edit> {
    let Value::Float(v) = row.value else {
        return None;
    };
    let mut d = to_display(unit, v);
    let (r, changed) = number::show(
        ui,
        &mut d,
        &NumberField {
            base: scrub::base_step(unit, false, row.step),
            prefix: String::new(),
            suffix: unit_suffix(unit),
            mixed: row.mixed,
            label: cx.label,
        },
    );
    changed.then(|| Edit {
        value: json!(d),
        component: None,
        phase: if r.dragged() {
            Phase::Drag {
                started: r.drag_started(),
            }
        } else {
            Phase::Commit
        },
    })
}
