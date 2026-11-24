//! Per-user preferences: settings, descriptors and draft form (UI.md §24.2a).

use std::fmt;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use forge_core::{ParamDesc, ParamKind, RenderConfig, RenderFilter, Value};
use forge_platform::autosave::Settings as AutosaveSettings;
use forge_rpc::RpcSettings;
use serde_json::{Value as Json, json};

use crate::keymap::read_settings;
use crate::theme::ThemeKind;

pub const MIN_SCALE: f32 = 0.5;
pub const MAX_SCALE: f32 = 3.0;

/// True for a scale a command may set.
pub fn scale_in_range(factor: f64) -> bool {
    factor.is_finite() && (MIN_SCALE as f64..=MAX_SCALE as f64).contains(&factor)
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum DistanceUnit {
    Millimetre,
    #[default]
    Centimetre,
    Metre,
    Inch,
}

impl DistanceUnit {
    const OPTIONS: [(&str, &str); 4] = [
        ("mm", "Millimetres"),
        ("cm", "Centimetres"),
        ("m", "Metres"),
        ("in", "Inches"),
    ];

    pub fn suffix(self) -> &'static str {
        match self {
            Self::Millimetre => " mm",
            Self::Centimetre => " cm",
            Self::Metre => " m",
            Self::Inch => " in",
        }
    }

    fn parse(id: &str) -> Option<Self> {
        Some(match id {
            "mm" => Self::Millimetre,
            "cm" => Self::Centimetre,
            "m" => Self::Metre,
            "in" => Self::Inch,
            _ => return None,
        })
    }

    fn centimetres(self) -> f64 {
        match self {
            Self::Millimetre => 0.1,
            Self::Centimetre => 1.0,
            Self::Metre => 100.0,
            Self::Inch => 2.54,
        }
    }

    pub fn to_display(self, cm: f64) -> f64 {
        cm / self.centimetres()
    }

    pub fn to_centimetres(self, display: f64) -> f64 {
        display * self.centimetres()
    }
}

#[derive(Clone, PartialEq)]
pub struct UiPrefs {
    pub theme: ThemeKind,
    /// Multiplied with the OS scale (`pixels_per_point = native * scale`).
    pub scale: f32,
    pub distance: DistanceUnit,
    pub autosave: AutosaveSettings,
    pub use_render_defaults: bool,
    pub render_defaults: RenderConfig,
    pub rpc: RpcSettings,
    path: Option<PathBuf>,
}

impl fmt::Debug for UiPrefs {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("UiPrefs")
            .field("theme", &self.theme)
            .field("scale", &self.scale)
            .field("distance", &self.distance)
            .field("autosave", &self.autosave)
            .field("use_render_defaults", &self.use_render_defaults)
            .field("render_defaults", &self.render_defaults)
            .field("rpc_port", &self.rpc.port)
            .field("rpc_token_set", &self.rpc.token.is_some())
            .field("path", &self.path)
            .finish()
    }
}

impl Default for UiPrefs {
    fn default() -> Self {
        UiPrefs {
            theme: ThemeKind::Dark,
            scale: 1.0,
            distance: DistanceUnit::default(),
            autosave: AutosaveSettings::default(),
            use_render_defaults: false,
            render_defaults: RenderConfig::default(),
            rpc: RpcSettings {
                port: forge_rpc::DEFAULT_PORT,
                token: None,
            },
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
            let file = read_settings(p);
            let ui = &file["ui"];
            if let Some(k) = ui["theme"].as_str().and_then(ThemeKind::from_name) {
                self.theme = k;
            }
            if let Some(f) = ui["scale"].as_f64().filter(|f| scale_in_range(*f)) {
                self.scale = f as f32;
            }
            if let Some(unit) = ui["distance_unit"].as_str().and_then(DistanceUnit::parse) {
                self.distance = unit;
            }
            self.autosave = AutosaveSettings::from_json(&file);
            self.rpc = RpcSettings::resolve(&|_| None, &file);
            let render = &file["render_defaults"];
            if let Ok(config) = serde_json::from_value::<RenderConfig>(render["settings"].clone())
                && config.check().is_ok()
            {
                self.render_defaults = config;
                self.use_render_defaults = render["enabled"].as_bool().unwrap_or(false);
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
        let ui = section(&mut settings, "ui");
        ui.insert("theme".into(), json!(self.theme.name()));
        ui.insert("scale".into(), json!(self.scale));
        store(p, &settings)
    }

    /// Persist a validated Preferences draft without rewriting unrelated keys.
    pub fn save_all(&self) -> Result<(), String> {
        self.render_defaults.check()?;
        for (key, desc) in fields() {
            let value = self.value(*key);
            if desc.validate(value.clone()).ok().as_ref() != Some(&value) {
                return Err(format!("Invalid setting: {}", desc.label));
            }
        }
        if self.rpc.token.as_ref().is_some_and(|t| {
            t.len() > 1024 || t.trim().is_empty() || t.chars().any(char::is_control)
        }) {
            return Err(
                "RPC token must be nonblank, at most 1024 bytes and have no control characters."
                    .into(),
            );
        }
        let Some(path) = &self.path else {
            return Ok(());
        };
        let mut f = read_settings(path);
        let ui = section(&mut f, "ui");
        ui.insert("theme".into(), json!(self.theme.name()));
        ui.insert("scale".into(), json!(self.scale));
        ui.insert("distance_unit".into(), json!(self.distance.suffix().trim()));
        let autosave = section(&mut f, "autosave");
        autosave.insert("enabled".into(), json!(self.autosave.enabled));
        autosave.insert(
            "interval_seconds".into(),
            json!(self.autosave.interval_seconds),
        );
        let render = section(&mut f, "render_defaults");
        render.insert("enabled".into(), json!(self.use_render_defaults));
        render.insert("settings".into(), json!(self.render_defaults));
        let rpc = section(&mut f, "rpc");
        rpc.insert("port".into(), json!(self.rpc.port));
        if let Some(token) = &self.rpc.token {
            rpc.insert("token".into(), json!(token));
        } else {
            rpc.remove("token");
        }
        store(path, &f)
    }
}

fn section<'a>(f: &'a mut Json, name: &str) -> &'a mut serde_json::Map<String, Json> {
    let value = &mut f[name];
    if !value.is_object() {
        *value = json!({});
    }
    value.as_object_mut().expect("section is an object")
}

fn store(path: &Path, file: &Json) -> Result<(), String> {
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let result = (|| -> std::io::Result<()> {
        std::fs::create_dir_all(parent)?;
        // NamedTempFile creates a unique private file (0600 on Unix). Rename atomically.
        let mut f = tempfile::NamedTempFile::new_in(parent)?;
        f.write_all(file.to_string().as_bytes())?;
        f.as_file().sync_all()?;
        f.persist(path).map_err(|e| e.error)?;
        Ok(())
    })();
    result.map_err(|e| format!("Could not store preferences: {e}"))
}

#[derive(Clone, Copy)]
enum Field {
    Theme,
    Scale,
    Distance,
    Autosave,
    Interval,
    Render,
    Width,
    Height,
    Samples,
    Depth,
    Filter,
    FilterWidth,
    Headlight,
    Port,
    Token,
}

fn fields() -> &'static [(Field, ParamDesc)] {
    static FIELDS: OnceLock<Vec<(Field, ParamDesc)>> = OnceLock::new();
    FIELDS.get_or_init(|| {
        use Field::*;
        let theme = ThemeKind::ALL.map(|k| (k.name(), k.name()));
        let a = AutosaveSettings::default();
        let r = RenderConfig::default();
        let dimension = |key, label, v| {
            ParamDesc::int(key, label, i64::from(v)).range(1.0, f64::from(RenderConfig::MAX_SIZE))
        };
        vec![
            (
                Theme,
                ParamDesc::enumeration("theme", "Theme", "dark", &theme).group("Interface"),
            ),
            (
                Scale,
                ParamDesc::float("scale", "UI scale", 1.0)
                    .range(0.5, 3.0)
                    .group("Interface"),
            ),
            (
                Distance,
                ParamDesc::enumeration(
                    "distance_unit",
                    "Distance unit",
                    "cm",
                    &DistanceUnit::OPTIONS,
                )
                .group("Interface"),
            ),
            (
                Autosave,
                ParamDesc::bool("enabled", "Enable autosave", a.enabled).group("Autosave"),
            ),
            (
                Interval,
                ParamDesc::int(
                    "interval_seconds",
                    "Interval (seconds)",
                    a.interval_seconds as i64,
                )
                .range(1.0, 86400.0)
                .group("Autosave"),
            ),
            (
                Render,
                ParamDesc::bool("use_defaults", "Use editor render defaults", false)
                    .group("Render defaults"),
            ),
            (
                Width,
                dimension("width", "Render width", r.width).group("Render defaults"),
            ),
            (
                Height,
                dimension("height", "Render height", r.height).group("Render defaults"),
            ),
            (
                Samples,
                ParamDesc::int("samples", "Samples per pixel", r.samples.into())
                    .range(1.0, 1_000_000.0)
                    .group("Render defaults"),
            ),
            (
                Depth,
                ParamDesc::int("max_depth", "Maximum ray depth", r.max_depth.into())
                    .range(1.0, 256.0)
                    .group("Render defaults"),
            ),
            (
                Filter,
                ParamDesc::enumeration(
                    "filter",
                    "Pixel filter",
                    r.filter.id(),
                    &[
                        ("box", "Box"),
                        ("triangle", "Triangle"),
                        ("gaussian", "Gaussian"),
                        ("cubic", "Cubic"),
                    ],
                )
                .group("Render defaults"),
            ),
            (
                FilterWidth,
                ParamDesc::float("filter_width", "Filter width", r.filter_width)
                    .range(0.01, 64.0)
                    .group("Render defaults"),
            ),
            (
                Headlight,
                ParamDesc::bool("default_light", "Default render light", r.default_light)
                    .group("Render defaults"),
            ),
            (
                Port,
                ParamDesc::int("port", "RPC port", forge_rpc::DEFAULT_PORT.into())
                    .range(0.0, 65535.0)
                    .group("RPC (restart required)"),
            ),
            (
                Token,
                ParamDesc::string("token", "RPC token", "").group("RPC (restart required)"),
            ),
        ]
    })
}

impl UiPrefs {
    fn value(&self, key: Field) -> Value {
        use Field::*;
        match key {
            Theme => Value::Enum(self.theme.name().into()),
            Scale => Value::Float(self.scale.into()),
            Distance => Value::Enum(self.distance.suffix().trim().into()),
            Autosave => Value::Bool(self.autosave.enabled),
            Interval => {
                Value::Int(i64::try_from(self.autosave.interval_seconds).unwrap_or(i64::MAX))
            }
            Render => Value::Bool(self.use_render_defaults),
            Width => Value::Int(self.render_defaults.width.into()),
            Height => Value::Int(self.render_defaults.height.into()),
            Samples => Value::Int(self.render_defaults.samples.into()),
            Depth => Value::Int(self.render_defaults.max_depth.into()),
            Filter => Value::Enum(self.render_defaults.filter.id().into()),
            FilterWidth => Value::Float(self.render_defaults.filter_width),
            Headlight => Value::Bool(self.render_defaults.default_light),
            Port => Value::Int(self.rpc.port.into()),
            Token => Value::String(self.rpc.token.clone().unwrap_or_default()),
        }
    }

    fn set(&mut self, field: Field, value: Value) {
        use Field::*;
        match (field, value) {
            (Theme, Value::Enum(v)) => {
                self.theme = ThemeKind::from_name(&v).unwrap_or(self.theme);
            }
            (Scale, Value::Float(v)) => self.scale = v as f32,
            (Distance, Value::Enum(v)) => {
                self.distance = DistanceUnit::parse(&v).unwrap_or(self.distance);
            }
            (Autosave, Value::Bool(v)) => self.autosave.enabled = v,
            (Interval, Value::Int(v)) => self.autosave.interval_seconds = v as u64,
            (Render, Value::Bool(v)) => self.use_render_defaults = v,
            (Width, Value::Int(v)) => self.render_defaults.width = v as u32,
            (Height, Value::Int(v)) => self.render_defaults.height = v as u32,
            (Samples, Value::Int(v)) => self.render_defaults.samples = v as u32,
            (Depth, Value::Int(v)) => self.render_defaults.max_depth = v as u32,
            (Filter, Value::Enum(v)) => {
                self.render_defaults.filter =
                    RenderFilter::parse(&v).unwrap_or(self.render_defaults.filter);
            }
            (FilterWidth, Value::Float(v)) => self.render_defaults.filter_width = v,
            (Headlight, Value::Bool(v)) => self.render_defaults.default_light = v,
            (Port, Value::Int(v)) => self.rpc.port = v as u16,
            (Token, Value::String(v)) => self.rpc.token = (!v.is_empty()).then_some(v),
            _ => unreachable!("validated preference descriptor and value disagree"),
        }
    }

    pub(crate) fn form(&mut self, ui: &mut egui::Ui) {
        let mut group = "";
        for (key, desc) in fields() {
            if group != desc.ui.group {
                group = &desc.ui.group;
                ui.separator();
                ui.heading(group);
            }
            let mut value = self.value(*key);
            let before = value.clone();
            if let Value::Bool(v) = &mut value {
                ui.checkbox(v, &desc.label);
            } else {
                ui.horizontal(|ui| {
                    let label = ui.label(&desc.label);
                    match (&desc.kind, &mut value) {
                        (ParamKind::Float { min, max }, Value::Float(v)) => {
                            ui.add(egui::DragValue::new(v).speed(0.05).range(*min..=*max))
                                .labelled_by(label.id);
                        }
                        (ParamKind::Int { min, max }, Value::Int(v)) => {
                            ui.add(egui::DragValue::new(v).range(*min..=*max))
                                .labelled_by(label.id);
                        }
                        (ParamKind::Enum { options }, Value::Enum(v)) => {
                            let selected = options
                                .iter()
                                .find(|o| &o.id == v)
                                .map(|o| o.label.as_str())
                                .unwrap_or_default();
                            egui::ComboBox::from_id_salt(&desc.id)
                                .selected_text(selected)
                                .show_ui(ui, |ui| {
                                    for o in options {
                                        ui.selectable_value(v, o.id.clone(), &o.label);
                                    }
                                })
                                .response
                                .labelled_by(label.id);
                        }
                        (ParamKind::String, Value::String(v)) => {
                            ui.add(egui::TextEdit::singleline(v).password(true))
                                .labelled_by(label.id);
                        }
                        _ => unreachable!("unhandled preference descriptor"),
                    }
                });
            }
            if value != before
                && let Ok(value) = desc.validate(value)
            {
                self.set(*key, value);
            }
        }
        ui.weak("RPC stays on loopback. Environment settings take precedence at launch.");
    }
}
