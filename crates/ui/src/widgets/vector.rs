//! Vec3: three scrubbable fields X, Y, Z; an edit sets only the component touched, so a
//! multi-selection keeps its other components.

use forge_core::{Unit, Value};
use serde_json::json;

use super::number::{self, NumberField};
use super::scrub;
use super::{Cx, Edit, Phase};
use crate::attributes::{AttrRow, to_display, unit_suffix};

pub fn show(ui: &mut egui::Ui, cx: &Cx, row: &AttrRow, unit: Unit) -> Option<Edit> {
    let Value::Vec3(v) = row.value else {
        return None;
    };
    let (min, max) = match row.widget {
        crate::attributes::Widget::Vec3Drag { min, max, .. } => (min, max),
        _ => (f64::NEG_INFINITY, f64::INFINITY),
    };
    let mut edit = None;
    ui.horizontal(|ui| {
        for (k, axis) in ["X", "Y", "Z"].iter().enumerate() {
            let mut d = to_display(unit, v[k]);
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
                    prefix: format!("{axis} "),
                    suffix,
                    mixed: row.mixed_axes[k],
                    label: cx.label,
                    name: row.label.clone(),
                    step: scrub::chevron_step(unit, false),
                    range: Some(range).filter(|(lo, hi)| lo.is_finite() || hi.is_finite()),
                    is_int: false,
                },
            );
            if changed {
                edit = Some(Edit {
                    value: json!(if unit == Unit::Distance {
                        cx.distance.to_centimetres(d)
                    } else {
                        d
                    }),
                    component: Some(k as u8),
                    target: None,
                    create: None,
                    phase: if r.dragged() {
                        Phase::Drag {
                            started: r.drag_started(),
                        }
                    } else {
                        Phase::Commit
                    },
                });
            }
        }
    });
    edit
}
