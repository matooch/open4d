//! The File menu's actions and the dialogs behind them (docs/specs/IO.md §2).

use std::path::{Path, PathBuf};

use std::collections::BTreeMap;

use forge_core::{NodeId, ParamDesc, ParamKind, Value};
use forge_io::export_scene;
use forge_io::formats;
use serde_json::json;

use super::{Editor, cmd};

/// Which kind of file a dialog is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileKind {
    /// The native scene file, `.forge`.
    Scene,
    /// Wavefront OBJ, `.obj`.
    Obj,
    /// A Rhai script, `.rhai`.
    Script,
}

impl FileKind {
    pub fn extension(self) -> &'static str {
        match self {
            FileKind::Scene => "forge",
            FileKind::Obj => "obj",
            FileKind::Script => "rhai",
        }
    }

    fn label(self) -> &'static str {
        match self {
            FileKind::Scene => "Forge3D scene",
            FileKind::Obj => "Wavefront OBJ",
            FileKind::Script => "Rhai script",
        }
    }
}

/// The answer to "save your changes?".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unsaved {
    Save,
    DontSave,
    Cancel,
}

/// The native dialogs, behind a trait so tests can answer without a window.
pub trait Dialogs {
    fn pick_open(&mut self, kind: FileKind) -> Option<PathBuf>;
    fn pick_save(&mut self, kind: FileKind, default_name: &str) -> Option<PathBuf>;
    fn confirm_unsaved(&mut self, name: &str) -> Unsaved;
}

/// Dialogs from the `rfd` crate (XDG desktop portal on Linux, the system dialogs elsewhere).
pub struct NativeDialogs;

impl Dialogs for NativeDialogs {
    fn pick_open(&mut self, kind: FileKind) -> Option<PathBuf> {
        rfd::FileDialog::new()
            .add_filter(kind.label(), &[kind.extension()])
            .pick_file()
    }

    fn pick_save(&mut self, kind: FileKind, default_name: &str) -> Option<PathBuf> {
        rfd::FileDialog::new()
            .add_filter(kind.label(), &[kind.extension()])
            .set_file_name(default_name)
            .save_file()
    }

    fn confirm_unsaved(&mut self, name: &str) -> Unsaved {
        let answer = rfd::MessageDialog::new()
            .set_level(rfd::MessageLevel::Warning)
            .set_title("Unsaved changes")
            .set_description(format!("Save changes to {name}?"))
            .set_buttons(rfd::MessageButtons::YesNoCancelCustom(
                "Save".into(),
                "Don't Save".into(),
                "Cancel".into(),
            ))
            .show();
        match answer {
            rfd::MessageDialogResult::Yes => Unsaved::Save,
            rfd::MessageDialogResult::No => Unsaved::DontSave,
            rfd::MessageDialogResult::Custom(s) if s == "Save" => Unsaved::Save,
            rfd::MessageDialogResult::Custom(s) if s == "Don't Save" => Unsaved::DontSave,
            _ => Unsaved::Cancel,
        }
    }
}

/// The pending import/export options window (docs/specs/UI.md §27).
#[derive(Debug, Clone, PartialEq)]
pub struct IoDialog {
    /// Format id (`forge_io::formats`).
    pub format: String,
    /// True for an export, false for an import.
    pub export: bool,
    /// Export only the selection.
    pub selected: bool,
    /// The file chosen in the file chooser.
    pub path: PathBuf,
    /// The option descriptors the controls derive from.
    pub descs: Vec<ParamDesc>,
    /// Current values by descriptor id.
    pub values: BTreeMap<String, Value>,
}

impl IoDialog {
    fn title(&self) -> String {
        let label = formats::format_by_id(&self.format).map_or("", |f| f.label);
        format!(
            "{} {label} Options",
            if self.export { "Export" } else { "Import" }
        )
    }

    /// The values as the `options` argument of `file.import` / `file.export`.
    fn options_json(&self) -> serde_json::Value {
        let mut map = serde_json::Map::new();
        for (id, v) in &self.values {
            let json = match v {
                Value::Bool(b) => json!(b),
                Value::Int(i) => json!(i),
                Value::Float(f) => json!(f),
                Value::Enum(s) | Value::String(s) => json!(s),
                _ => continue,
            };
            map.insert(id.clone(), json);
        }
        serde_json::Value::Object(map)
    }
}

/// The file kind (chooser filter) of a format.
fn kind_of(_format: &str) -> FileKind {
    FileKind::Obj
}

fn with_extension(mut path: PathBuf, kind: FileKind) -> PathBuf {
    if path.extension().is_none() {
        path.set_extension(kind.extension());
    }
    path
}

impl Editor {
    /// Replaces the dialogs (tests inject scripted answers).
    pub fn set_dialogs(&mut self, dialogs: Box<dyn Dialogs>) {
        self.dialogs = dialogs;
    }

    /// The file the scene was last saved to or loaded from.
    pub fn file_path(&self) -> Option<&Path> {
        self.state.file_path.as_deref().map(Path::new)
    }

    /// True when the scene changed since the last save or load.
    pub fn is_dirty(&self) -> bool {
        self.state.doc.scene().revision() != self.state.saved_revision
    }

    fn doc_name(&self) -> String {
        self.file_path().and_then(|p| p.file_name()).map_or_else(
            || "Untitled".to_string(),
            |n| n.to_string_lossy().into_owned(),
        )
    }

    /// `<file name>[*] - Forge3D`; `*` marks unsaved changes.
    pub fn window_title(&self) -> String {
        let star = if self.is_dirty() { "*" } else { "" };
        format!("{}{star} - Forge3D", self.doc_name())
    }

    /// True if the current scene may be replaced: it is clean, or the user saved it or
    /// chose not to.
    pub(super) fn confirm_discard(&mut self) -> bool {
        if !self.is_dirty() {
            return true;
        }
        let name = self.doc_name();
        match self.dialogs.confirm_unsaved(&name) {
            Unsaved::Save => self.save(),
            Unsaved::DontSave => true,
            Unsaved::Cancel => false,
        }
    }

    /// File > New.
    pub fn new_scene(&mut self) {
        if !self.confirm_discard() {
            return;
        }
        self.run(cmd("file.new"));
    }

    /// File > Open...
    pub fn open_scene(&mut self) {
        let Some(path) = self.dialogs.pick_open(FileKind::Scene) else {
            return;
        };
        if self.confirm_discard() {
            self.open_path(&path);
        }
    }

    /// File > Open Recent / the Content Browser's recent tab: entry `index` of the recent list.
    pub fn open_recent(&mut self, index: usize) {
        if self.confirm_discard() {
            self.run(("browser.open_recent", json!({"index": index})));
        }
    }

    /// Loads `path` and replaces the document; on any error the document stays.
    pub fn open_path(&mut self, path: &Path) {
        self.run(("file.open", json!({"path": path_arg(path)})));
    }

    /// File > Save: to the current file, or asks for one. True if saved.
    pub fn save(&mut self) -> bool {
        match self.file_path().map(Path::to_path_buf) {
            Some(path) => self.save_to(&path),
            None => self.save_as(),
        }
    }

    /// File > Save As...: True if saved.
    pub fn save_as(&mut self) -> bool {
        let default = self.file_path().and_then(|p| p.file_name()).map_or_else(
            || format!("Untitled.{}", FileKind::Scene.extension()),
            |n| n.to_string_lossy().into_owned(),
        );
        match self.dialogs.pick_save(FileKind::Scene, &default) {
            Some(path) => self.save_to(&with_extension(path, FileKind::Scene)),
            None => false,
        }
    }

    pub fn save_to(&mut self, path: &Path) -> bool {
        self.run(("file.save_as", json!({"path": path_arg(path)})))
    }

    /// The pending options window, if any.
    pub fn io_dialog(&self) -> Option<&IoDialog> {
        self.io_dialog.as_ref()
    }

    /// File > Import...: asks for a file, then opens the options window (nothing if the
    /// chooser is cancelled or the format cannot import).
    pub fn begin_import(&mut self, format: &str) {
        let Some(descs) = formats::import_options(format) else {
            self.state.status = format!("Import failed: unknown format \"{format}\"");
            return;
        };
        let Some(path) = self.dialogs.pick_open(kind_of(format)) else {
            return;
        };
        let values = formats::default_options(&descs);
        self.io_dialog = Some(IoDialog {
            format: format.to_string(),
            export: false,
            selected: false,
            path,
            descs,
            values,
        });
    }

    /// File > Export... / Export Selected...: asks for a file, then opens the options window.
    pub fn begin_export(&mut self, format: &str, selected: bool) {
        let Some(descs) = formats::export_options(format) else {
            self.state.status = format!("Export failed: unknown format \"{format}\"");
            return;
        };
        let roots: Vec<NodeId> = self.state.selection.nodes.iter().copied().collect();
        if export_scene(self.state.doc.scene(), selected.then_some(&roots[..])).is_empty() {
            self.state.status = "Nothing to export".into();
            return;
        }
        let stem = self.file_path().and_then(|p| p.file_stem()).map_or_else(
            || "Untitled".to_string(),
            |n| n.to_string_lossy().into_owned(),
        );
        let kind = kind_of(format);
        let default = format!("{stem}.{}", kind.extension());
        let Some(path) = self.dialogs.pick_save(kind, &default) else {
            return;
        };
        let values = formats::default_options(&descs);
        self.io_dialog = Some(IoDialog {
            format: format.to_string(),
            export: true,
            selected,
            path: with_extension(path, kind),
            descs,
            values,
        });
    }

    /// Sets one option of the pending window (the control's job in the UI).
    pub fn io_dialog_set(&mut self, id: &str, value: Value) {
        if let Some(d) = &mut self.io_dialog
            && d.values.contains_key(id)
        {
            d.values.insert(id.to_string(), value);
        }
    }

    /// OK: runs `file.import` / `file.export` with the chosen values and closes the window.
    pub fn io_dialog_confirm(&mut self) {
        let Some(d) = self.io_dialog.take() else {
            return;
        };
        let options = d.options_json();
        let path = path_arg(&d.path);
        if d.export {
            self.run((
                "file.export",
                json!({"path": path, "format": d.format, "selected": d.selected,
                    "options": options}),
            ));
        } else {
            self.run((
                "file.import",
                json!({"path": path, "format": d.format, "options": options}),
            ));
        }
    }

    /// Cancel: closes the window, nothing happens.
    pub fn io_dialog_cancel(&mut self) {
        self.io_dialog = None;
    }

    /// The options window (egui), drawn every frame while it is pending.
    pub(super) fn io_dialog_window(&mut self, ctx: &egui::Context) {
        let Some(mut d) = self.io_dialog.take() else {
            return;
        };
        let (mut ok, mut cancel) = (false, ctx.input(|i| i.key_pressed(egui::Key::Escape)));
        egui::Window::new(d.title())
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.weak(d.path.display().to_string());
                for desc in &d.descs {
                    if let Some(value) = d.values.get_mut(&desc.id) {
                        option_control(ui, desc, value);
                    }
                }
                ui.horizontal(|ui| {
                    ok = ui
                        .button(if d.export { "Export" } else { "Import" })
                        .clicked();
                    cancel |= ui.button("Cancel").clicked();
                });
            });
        if ok {
            self.io_dialog = Some(d);
            self.io_dialog_confirm();
        } else if !cancel {
            self.io_dialog = Some(d);
        }
    }

    /// File > Import OBJ...: one polygon object per OBJ object, one undo step.
    pub fn import_obj(&mut self) {
        let Some(path) = self.dialogs.pick_open(FileKind::Obj) else {
            return;
        };
        self.import_obj_path(&path);
    }

    pub fn import_obj_path(&mut self, path: &Path) {
        self.run(("file.import_obj", json!({"path": path_arg(path)})));
    }

    /// File > Export OBJ... (`selected == false`) and Export Selected as OBJ... (true).
    pub fn export_obj(&mut self, selected: bool) {
        let roots: Vec<NodeId> = self.state.selection.nodes.iter().copied().collect();
        if export_scene(self.state.doc.scene(), selected.then_some(&roots[..])).is_empty() {
            self.state.status = "Nothing to export".into();
            return;
        }
        let stem = self.file_path().and_then(|p| p.file_stem()).map_or_else(
            || "Untitled".to_string(),
            |n| n.to_string_lossy().into_owned(),
        );
        let default = format!("{stem}.{}", FileKind::Obj.extension());
        let Some(path) = self.dialogs.pick_save(FileKind::Obj, &default) else {
            return;
        };
        let path = with_extension(path, FileKind::Obj);
        self.run((
            "file.export_obj",
            json!({"path": path_arg(&path), "selected": selected}),
        ));
    }
}

/// One control for one option descriptor.
fn option_control(ui: &mut egui::Ui, desc: &ParamDesc, value: &mut Value) {
    match (&desc.kind, value) {
        (ParamKind::Bool, Value::Bool(b)) => {
            ui.checkbox(b, &desc.label);
        }
        (ParamKind::Int { min, max }, Value::Int(i)) => {
            ui.horizontal(|ui| {
                let label = ui.label(&desc.label);
                ui.add(egui::DragValue::new(i).range(*min..=*max))
                    .labelled_by(label.id);
            });
        }
        (ParamKind::Float { min, max }, Value::Float(f)) => {
            ui.horizontal(|ui| {
                let label = ui.label(&desc.label);
                ui.add(egui::DragValue::new(f).range(*min..=*max))
                    .labelled_by(label.id);
            });
        }
        (ParamKind::Enum { options }, Value::Enum(current)) => {
            ui.horizontal(|ui| {
                let label = ui.label(&desc.label);
                let shown = options
                    .iter()
                    .find(|o| &o.id == current)
                    .map_or(current.clone(), |o| o.label.clone());
                egui::ComboBox::from_id_salt(&desc.id)
                    .selected_text(shown)
                    .show_ui(ui, |ui| {
                        for o in options {
                            ui.selectable_value(current, o.id.clone(), &o.label);
                        }
                    })
                    .response
                    .labelled_by(label.id);
            });
        }
        (ParamKind::String, Value::String(s)) => {
            ui.horizontal(|ui| {
                let label = ui.label(&desc.label);
                ui.text_edit_singleline(s).labelled_by(label.id);
            });
        }
        _ => {}
    }
}

fn path_arg(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}
