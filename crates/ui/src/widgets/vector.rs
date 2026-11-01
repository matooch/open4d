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
    let mut edit = None;
    ui.horizontal(|ui| {
        for (k, axis) in ["X", "Y", "Z"].iter().enumerate() {
            let mut d = to_display(unit, v[k]);
            let (r, changed) = number::show(
                ui,
                &mut d,
                &NumberField {
                    base: scrub::base_step(unit, false, row.step),
                    prefix: format!("{axis} "),
                    suffix: unit_suffix(unit),
                    mixed: row.mixed_axes[k],
                    label: cx.label,
                },
            );
            if changed {
                edit = Some(Edit {
                    value: json!(d),
                    component: Some(k as u8),
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
