//! Colour: the colour picker button (linear RGB, as stored).

use forge_core::Value;
use serde_json::json;

use super::{Cx, Edit, Phase};
use crate::attributes::AttrRow;

pub fn show(ui: &mut egui::Ui, _cx: &Cx, row: &AttrRow) -> Option<Edit> {
    let Value::Color(c) = row.value else {
        return None;
    };
    let mut rgb = c.map(|x| x as f32);
    let r = ui.horizontal(|ui| {
        let r = ui.color_edit_button_rgb(&mut rgb);
        if row.mixed {
            ui.weak("mixed");
        }
        r
    });
    r.inner.changed().then(|| Edit {
        value: json!(rgb.map(f64::from)),
        component: None,
        phase: Phase::Drag { started: false },
    })
}
