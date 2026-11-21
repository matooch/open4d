//! The Materials panel: the document's material list with the commands that create, assign and
//! delete (docs/specs/MATERIAL.md M6). Every edit is a `material.*` command.

use egui::{Color32, RichText, Sense, Ui, vec2};
use forge_core::{NodeId, Value};
use serde_json::json;

use crate::object_manager::Cmd;
use crate::state::EditorState;

/// The materials as (id, name, icon colour as 8-bit sRGB), in list order.
pub fn rows(s: &EditorState) -> Vec<(NodeId, String, [u8; 3])> {
    let scene = s.doc.scene();
    scene
        .materials()
        .iter()
        .filter_map(|m| {
            let n = scene.node(*m)?;
            let c = match n.params.get("icon_color") {
                Some(Value::Color(c)) => c.map(|x| (x.clamp(0.0, 1.0) * 255.0).round() as u8),
                _ => [200; 3],
            };
            Some((*m, n.name.clone(), c))
        })
        .collect()
}

/// Draws the panel and returns the commands the buttons ask for.
pub fn material_manager_ui(ui: &mut Ui, s: &mut EditorState) -> Vec<Cmd> {
    let mut cmds: Vec<Cmd> = Vec::new();
    let has_selection = !s.selection.nodes.is_empty();
    ui.horizontal(|ui| {
        if ui.button("New").clicked() {
            cmds.push(("material.create", json!({})));
        }
    });
    ui.separator();
    let rows = rows(s);
    if rows.is_empty() {
        ui.weak("No materials. New creates one.");
    }
    for (id, name, c) in rows {
        ui.horizontal(|ui| {
            let (rect, _) = ui.allocate_exact_size(vec2(18.0, 18.0), Sense::hover());
            ui.painter()
                .rect_filled(rect, 3.0, Color32::from_rgb(c[0], c[1], c[2]));
            ui.label(RichText::new(name));
            ui.add_enabled_ui(has_selection, |ui| {
                if ui
                    .button("Assign")
                    .on_hover_text("Adds a texture tag with this material to the selected objects")
                    .clicked()
                {
                    cmds.push(("material.assign", json!({ "material": id.0 })));
                }
            });
            if ui.button("Delete").clicked() {
                cmds.push(("material.delete", json!({ "materials": [id.0] })));
            }
        });
    }
    cmds
}
