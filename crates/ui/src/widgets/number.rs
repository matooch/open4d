//! The scrubbable number field shared by the float, int and vector widgets.

use egui::{DragValue, Response, Ui};

use super::scrub::{self, Mods};

pub struct NumberField<'a> {
    pub base: f64,
    pub prefix: String,
    pub suffix: &'a str,
    pub mixed: bool,
    pub label: egui::Id,
}

/// Shows `value`; true in `.1` if the user changed it.
pub fn show(ui: &mut Ui, value: &mut f64, f: &NumberField) -> (Response, bool) {
    let mods = ui.input(|i| Mods::of(&i.modifiers));
    let mut dv = DragValue::new(value)
        .speed(scrub::drag_speed(f.base, mods))
        .prefix(&f.prefix)
        .suffix(f.suffix);
    if f.mixed {
        dv = dv.custom_formatter(|_, _| "mixed".to_string());
    }
    let r = ui.add(dv).labelled_by(f.label);
    let changed = r.changed();
    (r, changed)
}
