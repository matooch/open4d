//! External, indexed libraries of native Forge material presets.
use crate::content_browser::BrowserAction;
use serde::Deserialize;
use std::fs::{self, File};
use std::path::{Component, Path, PathBuf};
use std::sync::mpsc::{Receiver, TryRecvError, channel};

#[derive(Deserialize)]
struct Entry {
    name: String,
    category: String,
    file: PathBuf,
    swatch: [u8; 3],
}

#[derive(Deserialize)]
struct Catalog {
    format: String,
    materials: Vec<Entry>,
}

impl Catalog {
    fn read(path: &Path) -> Result<Self, String> {
        let bytes = fs::read(path).map_err(|e| e.to_string())?;
        let catalog: Self = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
        if catalog.format != "forge-material-library-v1" || catalog.materials.is_empty() {
            return Err("Choose a Forge material library index.".into());
        }
        for entry in &catalog.materials {
            if entry.file.extension().and_then(|x| x.to_str()) != Some("forge")
                || !entry
                    .file
                    .components()
                    .all(|c| matches!(c, Component::Normal(_)))
            {
                return Err(format!("Invalid preset path for {}", entry.name));
            }
        }
        Ok(catalog)
    }
}

/// Install in a fresh directory so a failed import leaves the previous library usable
/// and existing scenes keep the paths of any textures they already reference.
fn install_zip(path: &Path, root: &Path) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let mut archive = zip::ZipArchive::new(File::open(path)?)?;
    fs::create_dir_all(root)?;
    let staging = tempfile::Builder::new()
        .prefix("library-")
        .tempdir_in(root)?;
    let mut catalog = None;
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index)?;
        let Some(relative) = entry.enclosed_name().filter(|path| {
            path.components()
                .all(|part| matches!(part, Component::Normal(_)))
        }) else {
            return Err("ZIP contains an invalid file path".into());
        };
        if relative.starts_with("__MACOSX")
            || relative.file_name().is_some_and(|name| name == ".DS_Store")
        {
            continue;
        }
        if entry.is_symlink() {
            return Err("ZIP contains a symbolic link".into());
        }
        let output = staging.path().join(&relative);
        if entry.is_dir() {
            fs::create_dir_all(output)?;
        } else {
            if let Some(parent) = output.parent() {
                fs::create_dir_all(parent)?;
            }
            std::io::copy(&mut entry, &mut File::create_new(output)?)?;
            if relative
                .file_name()
                .is_some_and(|name| name == "library.json")
                && catalog.replace(relative).is_some()
            {
                return Err("ZIP contains more than one library.json".into());
            }
        }
    }
    let relative = catalog.ok_or("ZIP needs a Forge material library.json")?;
    let index = staging.path().join(&relative);
    let catalog = Catalog::read(&index)?;
    let asset_dir = index.parent().ok_or("Library index has no folder")?;
    for entry in catalog.materials {
        if !asset_dir.join(&entry.file).is_file() {
            return Err(format!("ZIP is missing the preset for {}", entry.name).into());
        }
    }
    Ok(staging.keep().join(relative))
}

#[derive(Default)]
pub(crate) struct LibraryPanel {
    catalog: Option<Catalog>,
    folder: PathBuf,
    query: String,
    category: String,
    show_library: bool,
    initialized: bool,
    notice: String,
    importing: Option<Receiver<Result<PathBuf, String>>>,
}

impl LibraryPanel {
    fn load(&mut self, path: &Path) -> Result<(), String> {
        let catalog = Catalog::read(path)?;
        self.folder = path.parent().unwrap_or(Path::new(".")).into();
        self.catalog = Some(catalog);
        self.category.clear();
        self.query.clear();
        self.show_library = true;
        self.notice.clear();
        Ok(())
    }

    fn open_and_remember(&mut self, path: &Path, bookmark: Option<&Path>) {
        if let Err(error) = self.load(path) {
            self.notice = error;
        } else if let Some(bookmark) = bookmark
            && let Some(folder) = bookmark.parent()
            && let Err(error) = fs::create_dir_all(folder)
                .and_then(|()| fs::write(bookmark, serde_json::to_vec(path)?))
        {
            self.notice = format!(
                "Library opened for this session. Could not remember its location: {error}"
            );
        }
    }

    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        user_objects: Option<&Path>,
        actions: &mut Vec<BrowserAction>,
    ) -> bool {
        let bookmark = user_objects
            .and_then(Path::parent)
            .map(|p| p.join("material-library-location.json"));
        if !self.initialized {
            self.initialized = true;
            if let Some(path) = bookmark
                .as_ref()
                .and_then(|b| std::fs::read(b).ok())
                .and_then(|b| serde_json::from_slice::<PathBuf>(&b).ok())
                && let Err(e) = self.load(&path)
            {
                self.notice = format!("Library unavailable: {e}");
            }
        }
        if let Some(worker) = self.importing.take() {
            match worker.try_recv() {
                Ok(Ok(path)) => self.open_and_remember(&path, bookmark.as_deref()),
                Ok(Err(error)) => self.notice = format!("Could not import ZIP: {error}"),
                Err(TryRecvError::Disconnected) => self.notice = "ZIP import stopped.".into(),
                Err(TryRecvError::Empty) => self.importing = Some(worker),
            }
        }
        ui.horizontal_wrapped(|ui| {
            if ui
                .add_enabled(
                    self.importing.is_none(),
                    egui::Button::new("Import library…"),
                )
                .on_hover_text("Choose a Forge material library ZIP or library.json")
                .clicked()
                && let Some(path) = rfd::FileDialog::new()
                    .add_filter("Forge material library", &["zip", "json"])
                    .pick_file()
            {
                let is_zip = path
                    .extension()
                    .and_then(|extension| extension.to_str())
                    .is_some_and(|extension| extension.eq_ignore_ascii_case("zip"));
                if is_zip {
                    let root = bookmark
                        .as_deref()
                        .and_then(Path::parent)
                        .map(Path::to_path_buf)
                        .or_else(forge_platform::presets_dir);
                    if let Some(root) = root {
                        let (send, receive) = channel();
                        let ctx = ui.ctx().clone();
                        let work = std::thread::Builder::new()
                            .name("material-library-import".into())
                            .spawn(move || {
                                let result = install_zip(&path, &root.join("material-libraries"))
                                    .map_err(|error| error.to_string());
                                let _ = send.send(result);
                                ctx.request_repaint();
                            });
                        match work {
                            Ok(_) => {
                                self.importing = Some(receive);
                                self.notice = "Importing material library…".into();
                            }
                            Err(error) => self.notice = error.to_string(),
                        }
                    } else {
                        self.notice = "Forge could not locate its material library folder.".into();
                    }
                } else {
                    self.open_and_remember(&path, bookmark.as_deref());
                }
            }
            if self.catalog.is_some() {
                ui.selectable_value(&mut self.show_library, false, "Built-in");
                ui.selectable_value(&mut self.show_library, true, "Imported");
            }
        });
        if !self.notice.is_empty() {
            ui.label(&self.notice);
        }
        let Some(catalog) = self.catalog.as_ref().filter(|_| self.show_library) else {
            return false;
        };
        ui.add(egui::TextEdit::singleline(&mut self.query).hint_text("Search materials"));
        let categories: std::collections::BTreeSet<_> = catalog
            .materials
            .iter()
            .map(|e| e.category.as_str())
            .collect();
        egui::ComboBox::from_label("Category")
            .selected_text(if self.category.is_empty() {
                "All categories"
            } else {
                &self.category
            })
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut self.category, String::new(), "All categories");
                for cat in categories {
                    ui.selectable_value(&mut self.category, cat.to_owned(), cat);
                }
            });
        let query = self.query.to_lowercase();
        egui::ScrollArea::vertical().show(ui, |ui| {
            for e in catalog.materials.iter().filter(|e| {
                (self.category.is_empty() || self.category == e.category)
                    && (e.name.to_lowercase().contains(&query)
                        || e.category.to_lowercase().contains(&query))
            }) {
                ui.horizontal(|ui| {
                    let (r, _) = ui.allocate_exact_size(egui::vec2(14., 14.), egui::Sense::hover());
                    let [r8, g, b] = e.swatch;
                    ui.painter()
                        .circle_filled(r.center(), 6., egui::Color32::from_rgb(r8, g, b));
                    if ui
                        .button(&e.name)
                        .on_hover_text(format!("Add {} to Materials", e.name))
                        .clicked()
                    {
                        actions.push(BrowserAction::Run((
                            "browser.import_material",
                            serde_json::json!({"path":self.folder.join(&e.file)}),
                        )));
                    }
                });
            }
        });
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn zip_import_keeps_bundled_textures_available_to_materials() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("materials.zip");
        let mut zip = zip::ZipWriter::new(File::create(&path).unwrap());
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        for (name, data) in [
            ("pack/library.json", br#"{"format":"forge-material-library-v1","materials":[{"name":"Weave","category":"Fabric","file":"Fabric/weave.forge","swatch":[120,90,80]}]}"#.as_slice()),
            ("pack/Fabric/weave.forge", br#"(format:"forge3d-scene",version:13,roots:[],materials:[(id:1,type_id:"forge.material.standard",name:"Weave",params:{"color.texture":Link(Some(2))},tags:[],children:[(id:2,type_id:"forge.shader.bitmap",name:"Base",params:{"file":String("../Textures/weave.png"),"srgb_input":Bool(true)},tags:[],children:[])])])"#.as_slice()),
            ("pack/Textures/weave.png", forge_command::browser::SAMPLE_SCENES[0].preview),
        ] {
            zip.start_file(name, options).unwrap();
            zip.write_all(data).unwrap();
        }
        zip.finish().unwrap();

        let root = temp.path().join("installed");
        let index = install_zip(&path, &root).unwrap();
        let mut panel = LibraryPanel::default();
        panel.open_and_remember(&index, Some(&temp.path().join("bookmark.json")));
        assert!(panel.notice.is_empty());
        let material = &panel.catalog.as_ref().unwrap().materials[0];
        let mut app = forge_app::App::new_default();
        let result = app
            .run(
                "browser.import_material",
                serde_json::json!({
                    "path": panel.folder.join(&material.file)
                }),
            )
            .unwrap();
        assert_eq!(result["warnings"], serde_json::json!([]));
        let scene = app.doc.scene();
        let material = scene.node(scene.materials()[0]).unwrap();
        let Some(forge_core::Value::String(texture)) = scene.param(material.children[0], "file")
        else {
            panic!("Imported material has no bitmap file");
        };
        assert_eq!(
            fs::read(texture).unwrap(),
            forge_command::browser::SAMPLE_SCENES[0].preview
        );
    }

    #[test]
    fn zip_without_index_leaves_no_installed_library() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("missing-index.zip");
        let mut zip = zip::ZipWriter::new(File::create(&path).unwrap());
        zip.start_file("material.forge", zip::write::SimpleFileOptions::default())
            .unwrap();
        zip.write_all(b"material").unwrap();
        zip.finish().unwrap();
        let root = temp.path().join("installed");
        assert!(install_zip(&path, &root).is_err());
        assert_eq!(fs::read_dir(root).unwrap().count(), 0);
    }
}
