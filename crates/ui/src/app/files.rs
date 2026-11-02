//! The File menu's actions and the dialogs behind them (docs/specs/IO.md §2).

use std::path::{Path, PathBuf};

use forge_core::NodeId;
use forge_io::export_scene;
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

fn path_arg(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}
