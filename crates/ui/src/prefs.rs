//! Interface preferences: theme and UI scale (docs/specs/UI.md §24.2).

use std::path::PathBuf;

use serde_json::json;

use crate::keymap::read_settings;
use crate::theme::ThemeKind;

pub const MIN_SCALE: f32 = 0.5;
pub const MAX_SCALE: f32 = 3.0;

/// True for a scale a command may set.
pub fn scale_in_range(factor: f64) -> bool {
    factor.is_finite() && (MIN_SCALE as f64..=MAX_SCALE as f64).contains(&factor)
}

#[derive(Debug, Clone, PartialEq)]
pub struct UiPrefs {
    pub theme: ThemeKind,
    /// Multiplied with the OS scale (`pixels_per_point = native * scale`).
    pub scale: f32,
    path: Option<PathBuf>,
}

impl Default for UiPrefs {
    fn default() -> Self {
        UiPrefs {
            theme: ThemeKind::Dark,
            scale: 1.0,
            path: None,
        }
    }
}

impl UiPrefs {
    /// Uses the settings file `path`: loads its `ui` key (bad fields keep the default) and
    /// stores later changes there.
    pub fn use_settings_file(&mut self, path: Option<PathBuf>) {
        *self = UiPrefs::default();
        if let Some(p) = &path {
            let ui = &read_settings(p)["ui"];
            if let Some(k) = ui["theme"].as_str().and_then(ThemeKind::from_name) {
                self.theme = k;
            }
            if let Some(f) = ui["scale"].as_f64().filter(|f| scale_in_range(*f)) {
                self.scale = f as f32;
            }
        }
        self.path = path;
    }

    /// Writes the preferences as the `ui` key of the settings file, keeping its other keys.
    pub fn save(&self) -> Result<(), String> {
        let Some(p) = &self.path else {
            return Ok(());
        };
        let mut settings = read_settings(p);
        settings["ui"] = json!({"theme": self.theme.name(), "scale": self.scale});
        p.parent()
            .map_or(Ok(()), std::fs::create_dir_all)
            .and_then(|()| std::fs::write(p, settings.to_string()))
            .map_err(|e| format!("Could not store the interface settings: {e}"))
    }
}
