//! The Content Browser panel: our own preset library and the recent files
//! (docs/specs/UI.md §26). Every action is a `browser.*` command.

use egui::{RichText, Ui};
use forge_command::browser::{Browser, Source, file_name};
use serde_json::json;

use crate::object_manager::Cmd;
use crate::state::EditorState;

/// The two tabs of the panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Tab {
    #[default]
    Presets,
    Materials,
    Scenes,
    Recent,
}

/// Panel state that is not document state.
#[derive(Default)]
pub struct BrowserPanel {
    pub tab: Tab,
    library: super::texture_library::LibraryPanel,
    scene_previews: std::collections::BTreeMap<&'static str, egui::TextureHandle>,
    /// Search text of the presets tab.
    pub query: String,
    /// Name box of "Save selection as preset".
    pub save_name: String,
}

/// What a click asks for. Opening a file replaces the document, so the editor asks about
/// unsaved changes first.
#[derive(Debug, Clone, PartialEq)]
pub enum BrowserAction {
    Run(Cmd),
    OpenRecent(usize),
    OpenSample(&'static str),
}

/// The `browser.load` command of a preset id.
pub fn load_command(id: &str) -> Cmd {
    ("browser.load", json!({ "id": id }))
}

/// The accessibility label of a preset button.
pub fn preset_label(name: &str) -> String {
    format!("Load {name}")
}

/// The save command for the name box, if the name has letters or digits.
pub fn save_command(name: &str) -> Option<Cmd> {
    (!forge_command::browser::slugify(name).is_empty())
        .then(|| ("browser.save_preset", json!({ "name": name.trim() })))
}

/// The panel body.
pub fn content_browser_ui(ui: &mut Ui, st: &mut EditorState) -> Vec<BrowserAction> {
    let mut actions = Vec::new();
    ui.horizontal_wrapped(|ui| {
        for (tab, label) in [
            (Tab::Presets, "Presets"),
            (Tab::Scenes, "Scenes"),
            (Tab::Materials, "Material library"),
            (Tab::Recent, "Recent"),
        ] {
            if ui
                .selectable_label(st.browser_panel.tab == tab, label)
                .clicked()
            {
                st.browser_panel.tab = tab;
            }
        }
    });
    ui.separator();
    match st.browser_panel.tab {
        Tab::Presets => presets_tab(ui, st, &mut actions),
        Tab::Materials => {
            let selected = st.browser_panel.library.show(
                ui,
                st.app.browser.user_dir().as_deref(),
                &mut actions,
            );
            if !selected {
                material_samples(ui, &mut actions);
            }
        }
        Tab::Scenes => {
            egui::ScrollArea::vertical().show(ui, |ui| {
                for sample in forge_command::browser::SAMPLE_SCENES {
                    let texture = st
                        .browser_panel
                        .scene_previews
                        .entry(sample.id)
                        .or_insert_with(|| {
                            let image = forge_image::decode(sample.preview, &Default::default())
                                .expect("bundled sample thumbnail")
                                .image;
                            let pixels = image
                                .data
                                .chunks_exact(4)
                                .map(|p| {
                                    let [r, g, b] =
                                        [p[0], p[1], p[2]].map(forge_image::linear_to_srgb8);
                                    egui::Color32::from_rgb(r, g, b)
                                })
                                .collect();
                            ui.ctx().load_texture(
                                sample.id,
                                egui::ColorImage::new(
                                    [image.width as usize, image.height as usize],
                                    pixels,
                                ),
                                egui::TextureOptions::LINEAR,
                            )
                        });
                    let width = ui.available_width().min(192.0);
                    let preview = ui.add(
                        egui::Image::new((texture.id(), egui::vec2(width, width * 9.0 / 16.0)))
                            .sense(egui::Sense::click()),
                    );
                    if ui
                        .button(sample.name)
                        .on_hover_text("Open an editable copy of this scene")
                        .clicked()
                        || preview.clicked()
                    {
                        actions.push(BrowserAction::OpenSample(sample.id));
                    }
                    ui.label(sample.exercise);
                    ui.separator();
                }
            });
        }
        Tab::Recent => recent_tab(ui, &st.app.browser, &mut actions),
    }
    actions
}

/// Reflectance widths and strengths are display percentages for material.create.
fn material_samples(ui: &mut Ui, actions: &mut Vec<BrowserAction>) {
    egui::ScrollArea::vertical().show(ui, |ui| {
        for (category, samples) in [
            ("Metal", [
                ("Polished chrome", [0.90, 0.92, 0.95], 9, 100),
                ("Satin aluminum", [0.83, 0.86, 0.90], 42, 96),
                ("Warm copper", [0.95, 0.53, 0.32], 23, 98),
                ("Polished gold", [1.0, 0.76, 0.34], 15, 99),
            ]),
            ("Gloss", [
                ("Obsidian glaze", [0.018, 0.022, 0.03], 12, 22),
                ("Porcelain", [0.82, 0.86, 0.90], 18, 12),
                ("Cobalt enamel", [0.018, 0.075, 0.42], 20, 19),
                ("Ruby lacquer", [0.40, 0.012, 0.025], 10, 21),
            ]),
        ] {
            ui.strong(category);
            let metal = category == "Metal";
            for (name, color, width, strength) in samples {
                ui.horizontal(|ui| {
                    let [r,g,b] = color.map(|v| (forge_image::linear_to_srgb(v) * 255.0).round() as u8);
                    let (swatch, _) = ui.allocate_exact_size(egui::vec2(14.0, 14.0), egui::Sense::hover());
                    ui.painter().circle_filled(swatch.center(), 6.0, egui::Color32::from_rgb(r,g,b));
                    if ui.button(name).on_hover_text(format!("Add {name} to Materials")).clicked() {
                        actions.push(BrowserAction::Run(("material.create", json!({
                            "name": name,
                            "params": {
                                "color.color": color,
                                "icon_color": color,
                                "reflectance.spec.attenuation": if metal { "metal" } else { "additive" },
                                "reflectance.spec.color": if metal { color } else { [1.0; 3] },
                                "reflectance.spec.width": width,
                                "reflectance.spec.strength": strength
                            }
                        }))));
                    }
                });
            }
        }
    });
}

fn presets_tab(ui: &mut Ui, st: &mut EditorState, actions: &mut Vec<BrowserAction>) {
    ui.horizontal(|ui| {
        ui.label("Search");
        ui.add(
            egui::TextEdit::singleline(&mut st.browser_panel.query)
                .hint_text("Search presets")
                .desired_width(160.0),
        );
    });
    let list = st.app.browser.list(Some(&st.browser_panel.query));
    if list.is_empty() {
        ui.weak("No presets");
    }
    egui::ScrollArea::vertical()
        .auto_shrink([false, true])
        .max_height((ui.available_height() - 40.0).max(60.0))
        .show(ui, |ui| {
            for p in &list {
                let tip = match p.source {
                    Source::Builtin => "Built-in preset",
                    Source::User => "Your preset",
                };
                let r = ui
                    .add(egui::Button::new(&p.name).frame(false))
                    .on_hover_text(tip);
                r.widget_info(|| {
                    egui::WidgetInfo::labeled(egui::Role::Button, true, preset_label(&p.name))
                });
                if r.clicked() {
                    actions.push(BrowserAction::Run(load_command(&p.id)));
                }
            }
        });
    if st.app.browser.user_dir().is_some() {
        ui.separator();
        let some = !st.app.selection.nodes.is_empty();
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut st.browser_panel.save_name)
                    .hint_text("Preset name")
                    .desired_width(120.0),
            );
            let cmd = save_command(&st.browser_panel.save_name);
            if ui
                .add_enabled(
                    some && cmd.is_some(),
                    egui::Button::new("Save selection as preset"),
                )
                .clicked()
                && let Some(c) = cmd
            {
                actions.push(BrowserAction::Run(c));
            }
        });
    }
}

fn recent_tab(ui: &mut Ui, browser: &Browser, actions: &mut Vec<BrowserAction>) {
    if browser.recent.is_empty() {
        ui.weak("No recent files");
    }
    egui::ScrollArea::vertical()
        .auto_shrink([false, true])
        .max_height((ui.available_height() - 40.0).max(60.0))
        .show(ui, |ui| {
            for (i, path) in browser.recent.iter().enumerate() {
                let exists = std::path::Path::new(path).is_file();
                let name = file_name(path);
                let text = if exists {
                    RichText::new(&name)
                } else {
                    RichText::new(&name).weak().strikethrough()
                };
                let r = ui
                    .add(egui::Button::new(text).frame(false))
                    .on_hover_text(path);
                r.widget_info(|| {
                    egui::WidgetInfo::labeled(egui::Role::Button, true, format!("Recent {name}"))
                });
                if r.clicked() {
                    actions.push(BrowserAction::OpenRecent(i));
                }
            }
        });
    ui.separator();
    if ui
        .add_enabled(
            !browser.recent.is_empty(),
            egui::Button::new("Clear recent"),
        )
        .clicked()
    {
        actions.push(BrowserAction::Run(("browser.clear_recent", json!({}))));
    }
}

/// Entries of the File > Open Recent sub menu: `(index, file name)`.
pub fn menu_entries(browser: &Browser) -> Vec<(usize, String)> {
    browser
        .recent
        .iter()
        .enumerate()
        .map(|(i, p)| (i, file_name(p)))
        .collect()
}
