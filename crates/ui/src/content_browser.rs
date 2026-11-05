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
    Recent,
}

/// Panel state that is not document state.
#[derive(Default)]
pub struct BrowserPanel {
    pub tab: Tab,
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
    ui.horizontal(|ui| {
        for (tab, label) in [(Tab::Presets, "Presets"), (Tab::Recent, "Recent")] {
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
        Tab::Recent => recent_tab(ui, &st.app.browser, &mut actions),
    }
    actions
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
