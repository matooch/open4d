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
    let (min, max) = match row.widget {
        crate::attributes::Widget::FloatDrag { min, max, .. } => (min, max),
        _ => (f64::NEG_INFINITY, f64::INFINITY),
    };
    let mut d = to_display(unit, v);
    let mut base = scrub::base_step(unit, false, row.step);
    let suffix = unit_suffix(unit);
    let mut range = (to_display(unit, min), to_display(unit, max));
    if unit == Unit::Distance {
        d = cx.distance.to_display(d);
        base = cx.distance.to_display(base);
        range = (
            cx.distance.to_display(range.0),
            cx.distance.to_display(range.1),
        );
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
            name: row.label.clone(),
            step: scrub::chevron_step(unit, false),
            range: Some(range).filter(|(lo, hi)| lo.is_finite() || hi.is_finite()),
            is_int: false,
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
