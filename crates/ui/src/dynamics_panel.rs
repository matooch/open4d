//! Dynamics Settings panel and selection bake controls (DYNAMICS.md §5.2).

use forge_app::dynamics::DynamicsSettings;
use forge_core::{Scene, SceneSettings};
use serde_json::json;

use crate::object_manager::Cmd;
use crate::state::EditorState;

#[derive(Default)]
pub(crate) struct DynamicsPanel {
    initialized: bool,
    start: i64,
    end: i64,
}

impl DynamicsPanel {
    pub fn reset_range(&mut self, settings: &SceneSettings) {
        self.initialized = true;
        self.start = settings.start;
        self.end = settings.end;
    }

    pub fn show(&mut self, ui: &mut egui::Ui, editor: &EditorState) -> Vec<Cmd> {
        let mut cmds = Vec::new();
        let scene = editor.doc.scene();
        let project = scene.settings();
        if !self.initialized {
            self.reset_range(&project);
        }
        let nodes: Vec<_> = editor.selection.nodes.iter().map(|id| id.0).collect();
        self.start = self.start.clamp(project.start, project.end);
        self.end = self.end.clamp(self.start, project.end);
        egui::ScrollArea::vertical().show(ui, |ui| {
            controls(ui, scene, &mut cmds);
            ui.separator();
            ui.heading("Bake Selection");
            egui::Grid::new("dynamics-bake-range").show(ui, |ui| {
                let label = ui.label("Bake Start");
                ui.add(egui::DragValue::new(&mut self.start).range(project.start..=self.end))
                    .labelled_by(label.id);
                ui.end_row();
                let label = ui.label("Bake End");
                ui.add(egui::DragValue::new(&mut self.end).range(self.start..=project.end))
                    .labelled_by(label.id);
                ui.end_row();
            });
            ui.horizontal(|ui| {
                for (label, mode) in [("Bake to Keys", "keys"), ("Bake to Cache Tag", "cache")] {
                    if ui
                        .add_enabled(!nodes.is_empty(), egui::Button::new(label))
                        .clicked()
                    {
                        cmds.push((
                            "dynamics.bake",
                            json!({
                                "nodes": nodes, "start": self.start, "end": self.end, "mode": mode,
                            }),
                        ));
                    }
                }
            });
        });
        cmds
    }
}

fn controls(ui: &mut egui::Ui, scene: &Scene, cmds: &mut Vec<Cmd>) {
    let mut settings = DynamicsSettings::of_scene(scene);
    if ui
        .checkbox(&mut settings.enabled, "Simulation Enabled")
        .changed()
    {
        cmds.push(("dynamics.settings", json!({"enabled": settings.enabled})));
    }
    egui::Grid::new("dynamics-settings-fields").show(ui, |ui| {
        let label = ui.label("Gravity (cm/s²)");
        if ui
            .add(egui::DragValue::new(&mut settings.gravity).speed(1.0))
            .labelled_by(label.id)
            .changed()
        {
            cmds.push(("dynamics.settings", json!({"gravity": settings.gravity})));
        }
        ui.end_row();
        let label = ui.label("Time Scale");
        if ui
            .add(
                egui::DragValue::new(&mut settings.time_scale)
                    .speed(0.01)
                    .range(0.0..=100.0),
            )
            .labelled_by(label.id)
            .changed()
        {
            cmds.push((
                "dynamics.settings",
                json!({"time_scale": settings.time_scale}),
            ));
        }
        ui.end_row();
        let label = ui.label("Steps per Frame");
        if ui
            .add(egui::DragValue::new(&mut settings.steps_per_frame).range(1..=100))
            .labelled_by(label.id)
            .changed()
        {
            cmds.push((
                "dynamics.settings",
                json!({"steps_per_frame": settings.steps_per_frame}),
            ));
        }
        ui.end_row();
    });
}
