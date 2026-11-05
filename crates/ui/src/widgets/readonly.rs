//! Read-only summary for kinds without an editor yet (mesh, index list, unknown types).
//! Spline curve and gradient editors arrive with their parameter kinds.

use forge_core::Value;

use crate::attributes::AttrRow;

pub fn summary(v: &Value) -> String {
    match v {
        Value::Mesh(m) => format!("{} points, {} polygons", m.points.len(), m.polys.len()),
        Value::IndexList(l) => format!("{} indices", l.len()),
        Value::Vec3List(l) => format!("{} points", l.len()),
        other => format!("{other:?}"),
    }
}

pub fn show(ui: &mut egui::Ui, row: &AttrRow) -> Option<super::Edit> {
    ui.weak(summary(&row.value));
    None
}
