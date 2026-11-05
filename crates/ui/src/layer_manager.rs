//! The Layers panel: a grid of layers with their toggles and the buttons that create,
//! delete, merge and assign (docs/specs/UI.md §22). Every edit is a `layer.*` command.

use std::collections::BTreeSet;

use egui::{Color32, RichText, Sense, Ui, vec2};
use forge_core::{Layer, LayerId, NodeId, Scene};
use serde_json::{Value as Json, json};

use crate::object_manager::Cmd;
use crate::state::EditorState;

/// One toggle column of the grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Toggle {
    Solo,
    View,
    Render,
    Manager,
    Locked,
    Animation,
    Generators,
    Deformers,
    Expressions,
}

impl Toggle {
    /// Column order (UI.md §22.1).
    pub const ALL: [Toggle; 9] = [
        Toggle::Solo,
        Toggle::View,
        Toggle::Render,
        Toggle::Manager,
        Toggle::Locked,
        Toggle::Animation,
        Toggle::Generators,
        Toggle::Deformers,
        Toggle::Expressions,
    ];

    pub fn letter(self) -> &'static str {
        match self {
            Toggle::Solo => "S",
            Toggle::View => "V",
            Toggle::Render => "R",
            Toggle::Manager => "M",
            Toggle::Locked => "L",
            Toggle::Animation => "A",
            Toggle::Generators => "G",
            Toggle::Deformers => "D",
            Toggle::Expressions => "E",
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Toggle::Solo => "Solo",
            Toggle::View => "View",
            Toggle::Render => "Render",
            Toggle::Manager => "Manager",
            Toggle::Locked => "Locked",
            Toggle::Animation => "Animation",
            Toggle::Generators => "Generators",
            Toggle::Deformers => "Deformers",
            Toggle::Expressions => "Expressions",
        }
    }

    /// The `layer.set` argument of this toggle.
    pub fn key(self) -> &'static str {
        match self {
            Toggle::Solo => "solo",
            Toggle::View => "view",
            Toggle::Render => "render",
            Toggle::Manager => "manager",
            Toggle::Locked => "locked",
            Toggle::Animation => "animation",
            Toggle::Generators => "generators",
            Toggle::Deformers => "deformers",
            Toggle::Expressions => "expressions",
        }
    }

    pub fn get(self, l: &Layer) -> bool {
        match self {
            Toggle::Solo => l.solo,
            Toggle::View => l.view,
            Toggle::Render => l.render,
            Toggle::Manager => l.manager,
            Toggle::Locked => l.locked,
            Toggle::Animation => l.animation,
            Toggle::Generators => l.generators,
            Toggle::Deformers => l.deformers,
            Toggle::Expressions => l.expressions,
        }
    }

    /// The accessibility label of the cell of `layer`.
    pub fn label(self, layer: &str) -> String {
        format!("{}: {layer}", self.name())
    }
}

/// Panel state that is not document state.
#[derive(Default)]
pub struct LayerPanel {
    /// Selected rows (a selection of layers, not of objects).
    pub selected: BTreeSet<LayerId>,
    /// Row being renamed, with its edit buffer.
    pub renaming: Option<(LayerId, String)>,
    /// A colour being dragged in a picker; committed when the pointer goes up.
    pending_color: Option<(LayerId, [f32; 3])>,
}

/// The rows a toggle click on `row` acts on: the whole selection when the row is in it.
pub fn toggle_targets(selected: &BTreeSet<LayerId>, row: LayerId) -> Vec<u64> {
    if selected.contains(&row) {
        selected.iter().map(|l| l.0).collect()
    } else {
        vec![row.0]
    }
}

/// `layer.set` arguments for a click on `toggle` of `layer`: every target gets the
/// opposite of the clicked row's value.
pub fn toggle_command(
    selected: &BTreeSet<LayerId>,
    layer: &Layer,
    toggle: Toggle,
) -> (&'static str, Json) {
    let mut args = json!({ "layers": toggle_targets(selected, layer.id) });
    args[toggle.key()] = json!(!toggle.get(layer));
    ("layer.set", args)
}

/// `layer.merge` arguments for the selected rows: they merge into the first one in table
/// order; `None` for fewer than two.
pub fn merge_command(scene: &Scene, selected: &BTreeSet<LayerId>) -> Option<Cmd> {
    let ordered: Vec<LayerId> = scene
        .layers()
        .iter()
        .map(|l| l.id)
        .filter(|id| selected.contains(id))
        .collect();
    (ordered.len() >= 2).then(|| {
        (
            "layer.merge",
            json!({"layers": ordered.iter().map(|l| l.0).collect::<Vec<_>>(), "into": ordered[0].0}),
        )
    })
}

/// `layer.assign` arguments putting `objects` on the selected row; `None` unless exactly one
/// row is selected and there are objects.
pub fn assign_command(selected: &BTreeSet<LayerId>, objects: &BTreeSet<NodeId>) -> Option<Cmd> {
    let mut it = selected.iter();
    let (Some(layer), None) = (it.next(), it.next()) else {
        return None;
    };
    (!objects.is_empty()).then(|| {
        (
            "layer.assign",
            json!({"layer": layer.0, "nodes": objects.iter().map(|n| n.0).collect::<Vec<_>>()}),
        )
    })
}

fn rgb(c: &[f64; 3]) -> [f32; 3] {
    c.map(|x| x as f32)
}

/// Draws the panel and returns the commands to run, in order.
pub fn layer_panel_ui(ui: &mut Ui, st: &mut EditorState) -> Vec<Cmd> {
    let mut cmds: Vec<Cmd> = Vec::new();
    let scene = st.app.doc.scene().clone();
    let panel = &mut st.layer_panel;
    panel.selected.retain(|l| scene.layers().contains(*l));
    if panel
        .renaming
        .as_ref()
        .is_some_and(|(l, _)| !scene.layers().contains(*l))
    {
        panel.renaming = None;
    }

    if scene.layers().is_empty() {
        ui.weak("No layers");
    }
    egui::ScrollArea::vertical()
        .auto_shrink([false, true])
        .max_height((ui.available_height() - 36.0).max(60.0))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.add_space(30.0);
                ui.weak(format!("{:<18}", "Name"));
                for t in Toggle::ALL {
                    ui.add_sized(
                        vec2(22.0, 14.0),
                        egui::Label::new(RichText::new(t.letter()).weak()),
                    );
                }
            });
            for layer in scene.layers().iter() {
                row_ui(ui, panel, layer, &mut cmds);
            }
        });
    ui.separator();
    ui.horizontal_wrapped(|ui| {
        if ui.button("New layer").clicked() {
            cmds.push(("layer.create", json!({})));
        }
        let some = !panel.selected.is_empty();
        if ui.add_enabled(some, egui::Button::new("Delete")).clicked() {
            let ids: Vec<u64> = panel.selected.iter().map(|l| l.0).collect();
            cmds.push(("layer.delete", json!({ "layers": ids })));
        }
        let merge = merge_command(&scene, &panel.selected);
        if ui
            .add_enabled(merge.is_some(), egui::Button::new("Merge"))
            .clicked()
            && let Some(c) = merge
        {
            cmds.push(c);
        }
        let assign = assign_command(&panel.selected, &st.app.selection.nodes);
        if ui
            .add_enabled(assign.is_some(), egui::Button::new("Assign selection"))
            .clicked()
            && let Some(c) = assign
        {
            cmds.push(c);
        }
        let objects = !st.app.selection.nodes.is_empty();
        if ui
            .add_enabled(objects, egui::Button::new("Remove from layer"))
            .clicked()
        {
            let nodes: Vec<u64> = st.app.selection.nodes.iter().map(|n| n.0).collect();
            cmds.push(("layer.assign", json!({"layer": null, "nodes": nodes})));
        }
    });
    cmds
}

fn row_ui(ui: &mut Ui, panel: &mut LayerPanel, layer: &Layer, cmds: &mut Vec<Cmd>) {
    ui.horizontal(|ui| {
        // colour swatch with a picker
        let mut c = match panel.pending_color {
            Some((id, c)) if id == layer.id => c,
            _ => rgb(&layer.color),
        };
        let r = egui::color_picker::color_edit_button_rgb(ui, &mut c);
        r.widget_info(|| {
            egui::WidgetInfo::labeled(egui::Role::Button, true, format!("Colour: {}", layer.name))
        });
        if r.changed() {
            panel.pending_color = Some((layer.id, c));
        }
        if let Some((id, c)) = panel.pending_color
            && id == layer.id
            && !ui.input(|i| i.pointer.any_down())
        {
            panel.pending_color = None;
            let c = c.map(f64::from);
            if c != layer.color {
                cmds.push(("layer.set", json!({"layer": layer.id.0, "color": c})));
            }
        }
        // name
        if let Some((id, buf)) = &mut panel.renaming
            && *id == layer.id
        {
            let r = ui.add(egui::TextEdit::singleline(buf).desired_width(110.0));
            r.widget_info(|| {
                egui::WidgetInfo::labeled(egui::Role::TextInput, true, "Rename layer")
            });
            if !r.has_focus() && !r.lost_focus() {
                r.request_focus();
            }
            if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                panel.renaming = None;
            } else if r.lost_focus() {
                if !buf.trim().is_empty() && *buf != layer.name {
                    cmds.push(("layer.rename", json!({"layer": layer.id.0, "name": buf})));
                }
                panel.renaming = None;
            }
        } else {
            let selected = panel.selected.contains(&layer.id);
            let r = ui
                .add_sized(
                    vec2(110.0, 18.0),
                    egui::Button::selectable(selected, &layer.name),
                )
                .interact(Sense::click());
            r.widget_info(|| {
                egui::WidgetInfo::labeled(egui::Role::Button, true, format!("Layer {}", layer.name))
            });
            if r.double_clicked() {
                panel.renaming = Some((layer.id, layer.name.clone()));
            } else if r.clicked() {
                if ui.input(|i| i.modifiers.command) {
                    if !panel.selected.remove(&layer.id) {
                        panel.selected.insert(layer.id);
                    }
                } else {
                    panel.selected.clear();
                    panel.selected.insert(layer.id);
                }
            }
        }
        // toggles
        for t in Toggle::ALL {
            let on = t.get(layer);
            let fill = if on {
                ui.visuals().selection.bg_fill
            } else {
                Color32::TRANSPARENT
            };
            let b = egui::Button::new(t.letter())
                .fill(fill)
                .min_size(vec2(22.0, 18.0));
            let r = ui.add(b);
            let label = t.label(&layer.name);
            r.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, label.clone()));
            if r.clicked() {
                cmds.push(toggle_command(&panel.selected, layer, t));
            }
        }
    });
}
