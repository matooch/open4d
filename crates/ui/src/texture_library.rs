//! External, indexed libraries of native Forge material presets.
use crate::content_browser::BrowserAction;
use serde::Deserialize;
use std::path::{Component, Path, PathBuf};

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

#[derive(Default)]
pub(crate) struct LibraryPanel {
    catalog: Option<Catalog>,
    folder: PathBuf,
    query: String,
    category: String,
    show_library: bool,
    initialized: bool,
    notice: String,
}

impl LibraryPanel {
    fn load(&mut self, path: &Path) -> Result<(), String> {
        let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
        let catalog: Catalog = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
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
        self.folder = path.parent().unwrap_or(Path::new(".")).into();
        self.catalog = Some(catalog);
        self.category.clear();
        self.query.clear();
        self.show_library = true;
        self.notice.clear();
        Ok(())
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
        ui.horizontal_wrapped(|ui| {
            if ui.button("Import library…").on_hover_text("Choose the library.json from an extracted Forge material library").clicked()
                && let Some(path) = rfd::FileDialog::new().add_filter("Forge material library", &["json"]).pick_file() {
                match self.load(&path) {
                    Err(e) => self.notice = e,
                    Ok(()) => if let Some(b) = bookmark && let Err(e) = std::fs::create_dir_all(b.parent().unwrap())
                        .and_then(|()| std::fs::write(&b, serde_json::to_vec(&path)?)) {
                        self.notice = format!("Library opened for this session. Could not remember its location: {e}");
                    }
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
