//! Script Manager panel state and drawing (docs/specs/UI.md §10, SCRIPTING.md).

use std::path::{Path, PathBuf};

use egui::{Key, Modifiers, RichText, Ui};
use serde_json::{Value, json};

use crate::app::{Dialogs, FileKind};
use crate::state::EditorState;
use crate::theme::Theme;

/// Recent scripts kept in the settings file.
pub const MAX_RECENT: usize = 10;
/// Console lines kept.
const MAX_CONSOLE: usize = 1000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineKind {
    /// A `print` line.
    Output,
    /// A failure message.
    Error,
    /// A note from the panel ("ran 3 commands").
    Info,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsoleLine {
    pub text: String,
    pub kind: LineKind,
}

/// Source text, console and recent list of the Script Manager.
#[derive(Debug, Clone, Default)]
pub struct ScriptPanelState {
    pub source: String,
    pub console: Vec<ConsoleLine>,
    /// Newest first, at most [`MAX_RECENT`].
    pub recent: Vec<String>,
    /// The file the source was last opened from or saved to.
    pub path: Option<String>,
    /// The settings file holding `scripts.recent`; `None` keeps the list in memory only.
    pub settings: Option<PathBuf>,
}

impl ScriptPanelState {
    /// Uses the shared settings file (RPC.md §2) and loads the recent list from it.
    pub fn use_settings_file(&mut self, path: Option<PathBuf>) {
        self.settings = path;
        self.recent = self
            .read_settings()
            .and_then(|v| v["scripts"]["recent"].as_array().cloned())
            .map(|a| {
                a.iter()
                    .filter_map(|s| s.as_str().map(String::from))
                    .take(MAX_RECENT)
                    .collect()
            })
            .unwrap_or_default();
    }

    fn read_settings(&self) -> Option<Value> {
        let text = std::fs::read_to_string(self.settings.as_ref()?).ok()?;
        serde_json::from_str(&text).ok()
    }

    pub fn push(&mut self, text: impl Into<String>, kind: LineKind) {
        self.console.push(ConsoleLine {
            text: text.into(),
            kind,
        });
        if self.console.len() > MAX_CONSOLE {
            self.console.remove(0);
        }
    }

    /// Moves `path` to the front of the recent list and stores the list.
    pub fn remember(&mut self, path: &str) {
        self.recent.retain(|p| p != path);
        self.recent.insert(0, path.to_string());
        self.recent.truncate(MAX_RECENT);
        let Some(file) = self.settings.clone() else {
            return;
        };
        // Keep every other key (the RPC token lives in the same file).
        let mut settings = match self.read_settings() {
            Some(Value::Object(o)) => Value::Object(o),
            _ => json!({}),
        };
        if !settings["scripts"].is_object() {
            settings["scripts"] = json!({});
        }
        settings["scripts"]["recent"] = json!(self.recent);
        let written = file
            .parent()
            .map_or(Ok(()), std::fs::create_dir_all)
            .and_then(|()| std::fs::write(&file, settings.to_string()));
        if let Err(e) = written {
            self.push(
                format!("Could not store recent scripts: {e}"),
                LineKind::Error,
            );
        }
    }

    /// Loads a script file into the editor.
    pub fn load_file(&mut self, path: &Path) -> bool {
        match std::fs::read_to_string(path) {
            Ok(text) => {
                self.source = text;
                let path = path.to_string_lossy().into_owned();
                self.remember(&path);
                self.path = Some(path);
                true
            }
            Err(e) => {
                self.push(
                    format!("Open failed: {}: {e}", path.display()),
                    LineKind::Error,
                );
                false
            }
        }
    }

    /// Writes the source to `path`.
    pub fn save_file(&mut self, path: &Path) -> bool {
        match std::fs::write(path, &self.source) {
            Ok(()) => {
                let path = path.to_string_lossy().into_owned();
                self.remember(&path);
                self.path = Some(path);
                true
            }
            Err(e) => {
                self.push(
                    format!("Save failed: {}: {e}", path.display()),
                    LineKind::Error,
                );
                false
            }
        }
    }
}

impl EditorState {
    /// Runs the panel's source on the editor (one undo step, rolled back on failure) and
    /// writes the result to the console.
    pub fn run_script(&mut self) {
        let source = self.script.source.clone();
        let commands = self.app.commands.clone();
        let result = forge_script::run_script(
            self,
            &commands,
            &source,
            &forge_script::ScriptOptions::default(),
        );
        match result {
            Ok(report) => {
                for line in report.output {
                    self.script.push(line, LineKind::Output);
                }
                self.script.push(
                    format!("ran {} command(s)", report.commands),
                    LineKind::Info,
                );
            }
            Err(failure) => {
                for line in &failure.output {
                    self.script.push(line.clone(), LineKind::Output);
                }
                self.script.push(failure.to_string(), LineKind::Error);
            }
        }
        if let Some(path) = self.script.path.clone() {
            self.script.remember(&path);
        }
    }
}

/// Draws the panel.
pub fn script_panel_ui(ui: &mut Ui, state: &mut EditorState, dialogs: &mut dyn Dialogs) {
    let theme = Theme::current(ui.ctx());
    let mut run = false;
    ui.horizontal(|ui| {
        run = ui.button("Run").clicked();
        if ui.button("Clear").clicked() {
            state.script.console.clear();
        }
        if ui.button("Open...").clicked()
            && let Some(path) = dialogs.pick_open(FileKind::Script)
        {
            state.script.load_file(&path);
        }
        if ui.button("Save...").clicked() {
            let name = state
                .script
                .path
                .as_deref()
                .and_then(|p| Path::new(p).file_name())
                .map_or_else(
                    || "script.rhai".to_string(),
                    |n| n.to_string_lossy().into_owned(),
                );
            if let Some(path) = dialogs.pick_save(FileKind::Script, &name) {
                state.script.save_file(&path);
            }
        }
        match &state.script.path {
            Some(p) => ui.weak(p),
            None => ui.weak("Untitled script"),
        };
    });

    let mut picked = None;
    if !state.script.recent.is_empty() {
        ui.collapsing("Recent scripts", |ui| {
            for path in &state.script.recent {
                if ui.link(path).clicked() {
                    picked = Some(PathBuf::from(path));
                }
            }
        });
    }
    if let Some(path) = picked {
        state.script.load_file(&path);
    }

    let height = (ui.available_height() * 0.55).max(80.0);
    let editor = egui::ScrollArea::vertical()
        .id_salt("script_source")
        .max_height(height)
        .show(ui, |ui| {
            ui.add(
                egui::TextEdit::multiline(&mut state.script.source)
                    .code_editor()
                    .desired_width(f32::INFINITY)
                    .desired_rows(8)
                    .hint_text("Rhai script"),
            )
        })
        .inner;
    if editor.has_focus() && ui.input_mut(|i| i.consume_key(Modifiers::COMMAND, Key::Enter)) {
        run = true;
    }
    if run {
        state.run_script();
    }

    ui.separator();
    egui::ScrollArea::vertical()
        .id_salt("script_console")
        .stick_to_bottom(true)
        .auto_shrink([false, false])
        .show(ui, |ui| {
            for line in &state.script.console {
                let text = RichText::new(&line.text).monospace();
                match line.kind {
                    LineKind::Output => ui.label(text),
                    LineKind::Error => ui.label(text.color(theme.error)),
                    LineKind::Info => ui.label(text.weak()),
                };
            }
        });
}
