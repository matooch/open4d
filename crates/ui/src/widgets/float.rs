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
    let mut base = scrub::base_step(unit, false, row.step);
    let suffix = unit_suffix(unit);
    if unit == Unit::Distance {
        d = cx.distance.to_display(d);
        base = cx.distance.to_display(base);
    }
    let (r, changed) = number::show(
        ui,
        &mut d,
        &NumberField {
            base,
            prefix: String::new(),
            suffix,
            mixed: row.mixed,
            label: cx.label,
        },
    );
    changed.then(|| Edit {
        value: json!(if unit == Unit::Distance {
            cx.distance.to_centimetres(d)
        } else {
            d
        }),
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
