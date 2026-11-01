//! Int: scrubbable whole number inside the parameter's range.

use forge_core::Value;
use serde_json::json;

use super::scrub::{self, Mods};
use super::{Cx, Edit, Phase};
use crate::attributes::AttrRow;

pub fn show(ui: &mut egui::Ui, cx: &Cx, row: &AttrRow, min: i64, max: i64) -> Option<Edit> {
    let Value::Int(v) = row.value else {
        return None;
    };
    let mut x = v;
    let mods = ui.input(|i| Mods::of(&i.modifiers));
    let mut dv = egui::DragValue::new(&mut x)
        .range(min..=max)
        .speed(scrub::drag_speed(
            scrub::base_step(forge_core::Unit::None, true, row.step),
            mods,
        ));
    if row.mixed {
        dv = dv.custom_formatter(|_, _| "mixed".to_string());
    }
    let r = ui.add(dv).labelled_by(cx.label);
    r.changed().then(|| Edit {
        value: json!(x),
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
