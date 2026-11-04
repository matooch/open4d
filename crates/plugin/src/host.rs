//! The host entry point: plugin folders and the shared registries (PLUGINS.md section 9).

use std::collections::BTreeSet;
use std::ffi::OsString;
use std::path::PathBuf;
use std::sync::Arc;

use forge_app::App;
use forge_command::{CommandContext, CommandError, CommandHandler, CommandRegistry};
use forge_core::Registry;
use serde_json::{Value as Json, json};

use crate::{RhaiPlugin, install_from};

/// Where plugins come from. Tests build this directly (temp dirs); programs use
/// [`HostOptions::from_env`].
#[derive(Debug, Clone, Default)]
pub struct HostOptions {
    /// `--plugins <dir>` folders, loaded last.
    pub extra_dirs: Vec<PathBuf>,
    /// Load nothing (`--no-plugins`, `FORGE_NO_PLUGINS`).
    pub disabled: bool,
    /// The `plugins/` folder inside the platform data dir; loaded first.
    pub data_dir: Option<PathBuf>,
    /// The raw `FORGE_PLUGIN_PATH` value; loaded second.
    pub env_path: Option<OsString>,
}

impl HostOptions {
    /// Options for the current process: platform data dir, `FORGE_PLUGIN_PATH` and
    /// `FORGE_NO_PLUGINS` from the environment, plus the command-line choices.
    pub fn from_env(extra_dirs: Vec<PathBuf>, no_plugins: bool) -> HostOptions {
        HostOptions::resolve(
            extra_dirs,
            no_plugins,
            &|k| std::env::var_os(k),
            forge_platform::dirs().map(|d| d.data.join("plugins")),
        )
    }

    /// [`HostOptions::from_env`] with an injectable environment and data folder.
    pub fn resolve(
        extra_dirs: Vec<PathBuf>,
        no_plugins: bool,
        env: &dyn Fn(&str) -> Option<OsString>,
        data_dir: Option<PathBuf>,
    ) -> HostOptions {
        let env_off = env("FORGE_NO_PLUGINS").is_some_and(|v| !v.is_empty() && v != "0");
        HostOptions {
            extra_dirs,
            disabled: no_plugins || env_off,
            data_dir,
            env_path: env("FORGE_PLUGIN_PATH"),
        }
    }

    /// Loading disabled, nothing else set.
    pub fn none() -> HostOptions {
        HostOptions {
            disabled: true,
            ..HostOptions::default()
        }
    }

    /// The folders to load, in load order (empty when disabled).
    pub fn folders(&self) -> Vec<PathBuf> {
        if self.disabled {
            return Vec::new();
        }
        let mut out: Vec<PathBuf> = self.data_dir.iter().cloned().collect();
        if let Some(path) = &self.env_path {
            out.extend(std::env::split_paths(path).filter(|p| !p.as_os_str().is_empty()));
        }
        out.extend(self.extra_dirs.iter().cloned());
        out
    }
}

/// One loaded plugin, for `plugin.list`.
#[derive(Debug, Clone, PartialEq)]
pub struct PluginInfo {
    pub name: String,
    pub version: String,
    pub id: String,
    pub dir: PathBuf,
    pub node_types: Vec<String>,
    pub commands: Vec<String>,
}

/// A plugin folder that was not loaded, and why.
#[derive(Debug, Clone, PartialEq)]
pub struct SkippedInfo {
    pub dir: PathBuf,
    pub reason: String,
}

/// What loading found: shown by `plugin.list`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct LoadReport {
    pub disabled: bool,
    pub folders: Vec<PathBuf>,
    pub plugins: Vec<PluginInfo>,
    pub skipped: Vec<SkippedInfo>,
}

impl LoadReport {
    pub fn to_json(&self) -> Json {
        let path = |p: &PathBuf| p.display().to_string();
        json!({
            "disabled": self.disabled,
            "folders": self.folders.iter().map(path).collect::<Vec<_>>(),
            "plugins": self.plugins.iter().map(|p| json!({
                "name": p.name, "version": p.version, "id": p.id, "dir": path(&p.dir),
                "node_types": p.node_types, "commands": p.commands,
            })).collect::<Vec<_>>(),
            "skipped": self.skipped.iter().map(|s| json!({
                "dir": path(&s.dir), "reason": s.reason,
            })).collect::<Vec<_>>(),
        })
    }
}

/// The shared registries of a host, with every plugin installed and bound.
pub struct Host {
    pub registry: Arc<Registry>,
    pub commands: Arc<CommandRegistry>,
    pub report: LoadReport,
    pub plugins: Vec<Arc<RhaiPlugin>>,
}

impl Host {
    /// An empty application over these registries.
    pub fn new_app(&self) -> App {
        App::with_commands(self.registry.clone(), self.commands.clone())
    }
}

/// Builds the default registries plus the plugins of `options` and `plugin.list`. A plugin
/// that fails to load or clashes is skipped and reported, never fatal.
pub fn load_host(options: &HostOptions) -> Host {
    let mut registry = forge_app::default_registry().expect("built-in plugins register");
    let mut commands = forge_app::default_command_registry();
    let folders = options.folders();
    let mut report = LoadReport {
        disabled: options.disabled,
        folders: folders.clone(),
        ..LoadReport::default()
    };
    let mut names = BTreeSet::new();
    let mut plugins = Vec::new();
    for folder in &folders {
        let out = install_from(folder, &mut registry, &mut commands, &mut names);
        for (dir, reason) in out.failures {
            report.skipped.push(SkippedInfo { dir, reason });
        }
        for p in out.plugins {
            let m = p.manifest();
            report.plugins.push(PluginInfo {
                name: m.name.clone(),
                version: m.version.clone(),
                id: p.id.clone(),
                dir: p.dir().to_path_buf(),
                node_types: m
                    .node_types
                    .iter()
                    .map(|t| t.node_type.id.clone())
                    .collect(),
                commands: m.commands.iter().map(|c| c.id.clone()).collect(),
            });
            plugins.push(p);
        }
    }
    commands
        .register(Box::new(PluginList {
            report: report.clone(),
        }))
        .expect("plugin.list is unique");
    let commands = Arc::new(commands);
    for p in &plugins {
        p.bind(&commands);
    }
    Host {
        registry: Arc::new(registry),
        commands,
        report,
        plugins,
    }
}

struct PluginList {
    report: LoadReport,
}

impl CommandHandler for PluginList {
    fn id(&self) -> &str {
        "plugin.list"
    }

    fn label(&self) -> &str {
        "List Plugins"
    }

    fn args_schema(&self) -> Json {
        json!({"type": "object", "additionalProperties": false, "properties": {}})
    }

    fn run(&self, _ctx: &mut dyn CommandContext, _args: Json) -> Result<Json, CommandError> {
        Ok(self.report.to_json())
    }

    fn undoable(&self) -> bool {
        false
    }
}
