//! forge-plugin: Rhai plugin packages (docs/specs/PLUGINS.md).
//!
//! A package is a folder with `plugin.toml` and an entry script. It registers node types
//! (geometry from a Rhai function, run in a pure sandbox) and commands (Rhai functions in
//! the command registry). Every script runs under the `forge-script` sandbox limits.

mod manifest;

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock, Weak};

use forge_command::{
    CommandContext, CommandError, CommandHandler, CommandPlugin, CommandRegistry, valid_command_id,
};
use forge_core::{MeshData, NodeId, Plugin, Registry, RegistryError, Scene};
use forge_geometry::PolyMesh;
use forge_script::{ScriptOptions, eval_function, run_function, script_functions};
use glam::DVec3;
use serde_json::{Map, Value as Json, json};

pub use manifest::{CommandDef, Manifest, NodeTypeDef, PERMISSIONS};

/// File name of the manifest inside a plugin folder.
pub const MANIFEST_FILE: &str = "plugin.toml";
/// Largest entry script accepted.
pub const MAX_SCRIPT_BYTES: u64 = 1 << 20;
/// Largest mesh a build function may return (points, and polygons).
pub const MAX_MESH_ELEMENTS: usize = 1_000_000;

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum PluginError {
    #[error("{0}")]
    Io(String),
    #[error("manifest: {0}")]
    Manifest(String),
    #[error("script: {0}")]
    Script(String),
    #[error("{type_id}: {message}")]
    Build { type_id: String, message: String },
}

/// One loaded, validated plugin package.
#[derive(Debug)]
pub struct RhaiPlugin {
    id: String,
    dir: PathBuf,
    manifest: Manifest,
    source: Arc<str>,
    registry: Arc<OnceLock<Weak<CommandRegistry>>>,
}

impl RhaiPlugin {
    /// Reads and validates the package in `dir`; registers nothing.
    pub fn load(dir: &Path) -> Result<RhaiPlugin, PluginError> {
        let text = std::fs::read_to_string(dir.join(MANIFEST_FILE))
            .map_err(|e| PluginError::Io(format!("{}: {e}", dir.join(MANIFEST_FILE).display())))?;
        let manifest = Manifest::parse(&text)?;
        let entry = dir.join(&manifest.entry);
        let meta = std::fs::metadata(&entry)
            .map_err(|e| PluginError::Io(format!("{}: {e}", entry.display())))?;
        if !meta.is_file() || meta.len() > MAX_SCRIPT_BYTES {
            return Err(PluginError::Manifest(format!(
                "entry {:?} must be a file of at most {MAX_SCRIPT_BYTES} bytes",
                manifest.entry
            )));
        }
        // A symlink must not lead out of the plugin folder.
        let inside = entry
            .canonicalize()
            .ok()
            .zip(dir.canonicalize().ok())
            .is_some_and(|(e, d)| e.starts_with(d));
        if !inside {
            return Err(PluginError::Manifest(format!(
                "entry {:?} is outside the plugin folder",
                manifest.entry
            )));
        }
        let source = std::fs::read_to_string(&entry)
            .map_err(|e| PluginError::Io(format!("{}: {e}", entry.display())))?;
        Self::from_parts(dir, manifest, source)
    }

    /// Builds a plugin from a parsed manifest and the entry source (no file access).
    pub fn from_parts(
        dir: &Path,
        manifest: Manifest,
        source: String,
    ) -> Result<RhaiPlugin, PluginError> {
        let functions =
            script_functions(&source).map_err(|e| PluginError::Script(e.to_string()))?;
        let needed = manifest
            .node_types
            .iter()
            .map(|t| t.build.as_str())
            .chain(manifest.commands.iter().map(|c| c.function.as_str()));
        for name in needed {
            if !functions.iter().any(|(f, n)| f == name && *n == 1) {
                return Err(PluginError::Script(format!(
                    "function {name}(arg) is not defined in {}",
                    manifest.entry
                )));
            }
        }
        // Dry-run the registration so descriptor errors surface at load time.
        let mut scratch = Registry::new();
        for t in &manifest.node_types {
            scratch
                .register(t.node_type.clone())
                .map_err(|e| PluginError::Manifest(e.to_string()))?;
        }
        let mut seen = std::collections::BTreeSet::new();
        for c in &manifest.commands {
            if !seen.insert(c.id.as_str()) || !valid_command_id(&c.id) {
                return Err(PluginError::Manifest(format!(
                    "duplicate or malformed command id {:?}",
                    c.id
                )));
            }
        }
        Ok(RhaiPlugin {
            id: format!("plugin.{}", manifest.name),
            dir: dir.to_path_buf(),
            manifest,
            source: source.into(),
            registry: Arc::default(),
        })
    }

    pub fn manifest(&self) -> &Manifest {
        &self.manifest
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// Lets this plugin's commands run registry commands: they need the registry they are
    /// registered in. Call once after the registry is shared (`Arc`); only a weak reference
    /// is kept.
    pub fn bind(&self, commands: &Arc<CommandRegistry>) {
        let _ = self.registry.set(Arc::downgrade(commands));
    }

    fn node_def(&self, type_id: &str) -> Option<&NodeTypeDef> {
        self.manifest
            .node_types
            .iter()
            .find(|t| t.node_type.id == type_id)
    }

    /// True if `type_id` is a node type of this plugin.
    pub fn owns(&self, type_id: &str) -> bool {
        self.node_def(type_id).is_some()
    }

    /// Builds the mesh of `type_id` from parameter values in display units (`params` maps
    /// param id to JSON; missing ones take the default).
    pub fn build_mesh(
        &self,
        type_id: &str,
        params: &Map<String, Json>,
    ) -> Result<PolyMesh, PluginError> {
        let def = self.node_def(type_id).ok_or_else(|| PluginError::Build {
            type_id: type_id.to_string(),
            message: "not a node type of this plugin".into(),
        })?;
        let fail = |message: String| PluginError::Build {
            type_id: type_id.to_string(),
            message,
        };
        let mut arg = default_params(&def.node_type);
        for (k, v) in params {
            arg.insert(k.clone(), v.clone());
        }
        let report = eval_function(
            &self.source,
            &def.build,
            &Json::Object(arg),
            &ScriptOptions::default(),
        )
        .map_err(|e| fail(e.to_string()))?;
        mesh_from_json(&report.value).map_err(fail)
    }

    /// The mesh of node `id` if its type belongs to this plugin.
    pub fn evaluate(&self, scene: &Scene, id: NodeId) -> Option<Result<PolyMesh, PluginError>> {
        let node = scene.node(id)?;
        let def = self.node_def(&node.type_id)?;
        let ty = scene.node_type(id)?;
        let mut params = Map::new();
        for pid in &def.param_ids {
            let Some(desc) = ty.find_param(pid) else {
                continue;
            };
            let value = node.params.get(pid).unwrap_or(&desc.default);
            params.insert(
                pid.clone(),
                forge_app::query::display_value(value, desc.ui.unit),
            );
        }
        Some(self.build_mesh(&node.type_id, &params))
    }
}

/// Defaults of the plugin-defined params in display units.
fn default_params(ty: &forge_core::NodeType) -> Map<String, Json> {
    ty.params
        .iter()
        .filter(|p| !p.id.starts_with("transform.") && !p.id.starts_with("visibility."))
        .map(|p| {
            (
                p.id.clone(),
                forge_app::query::display_value(&p.default, p.ui.unit),
            )
        })
        .collect()
}

fn mesh_from_json(v: &Json) -> Result<PolyMesh, String> {
    let obj = v
        .as_object()
        .ok_or("the build function must return #{ points, polys }")?;
    let points = obj
        .get("points")
        .and_then(Json::as_array)
        .ok_or("result has no `points` array")?;
    let polys = obj
        .get("polys")
        .and_then(Json::as_array)
        .ok_or("result has no `polys` array")?;
    if points.len() > MAX_MESH_ELEMENTS || polys.len() > MAX_MESH_ELEMENTS {
        return Err(format!("mesh larger than {MAX_MESH_ELEMENTS} elements"));
    }
    let mut data = MeshData::default();
    for (i, p) in points.iter().enumerate() {
        let c = p
            .as_array()
            .filter(|c| c.len() == 3)
            .ok_or_else(|| format!("point {i} is not [x, y, z]"))?;
        let mut xyz = [0.0; 3];
        for (slot, n) in xyz.iter_mut().zip(c) {
            *slot = n
                .as_f64()
                .ok_or_else(|| format!("point {i} has a non-number"))?;
        }
        data.points.push(xyz);
    }
    for (i, poly) in polys.iter().enumerate() {
        let corners = poly
            .as_array()
            .ok_or_else(|| format!("polygon {i} is not an array"))?;
        let mut out = Vec::with_capacity(corners.len());
        for c in corners {
            let idx = c
                .as_u64()
                .and_then(|n| u32::try_from(n).ok())
                .ok_or_else(|| format!("polygon {i} has an invalid index"))?;
            out.push(idx);
        }
        data.polys.push(out);
    }
    data.validate().map_err(|e| format!("invalid mesh: {e}"))?;
    Ok(PolyMesh {
        points: data.points.iter().map(|p| DVec3::from_array(*p)).collect(),
        polys: data.polys,
    })
}

impl Plugin for RhaiPlugin {
    fn id(&self) -> &str {
        &self.id
    }

    fn register(&self, registry: &mut Registry) -> Result<(), RegistryError> {
        for t in &self.manifest.node_types {
            registry.register(t.node_type.clone())?;
        }
        Ok(())
    }
}

impl CommandPlugin for RhaiPlugin {
    fn register_commands(&self, commands: &mut CommandRegistry) -> Result<(), CommandError> {
        let granted = self.manifest.has_permission("commands");
        let mut opts = ScriptOptions::default();
        if !granted {
            // No permission: the command may run no registry command (PLUGINS.md §3.2).
            opts.max_commands = 0;
        }
        for c in &self.manifest.commands {
            commands.register(Box::new(PluginCommand {
                def: c.clone(),
                source: self.source.clone(),
                undoable: granted,
                opts: opts.clone(),
                registry: self.registry.clone(),
            }))?;
        }
        Ok(())
    }
}

struct PluginCommand {
    def: CommandDef,
    source: Arc<str>,
    undoable: bool,
    opts: ScriptOptions,
    registry: Arc<OnceLock<Weak<CommandRegistry>>>,
}

impl CommandHandler for PluginCommand {
    fn id(&self) -> &str {
        &self.def.id
    }

    fn label(&self) -> &str {
        &self.def.label
    }

    fn args_schema(&self) -> Json {
        json!({"type": "object", "additionalProperties": true})
    }

    fn run(&self, ctx: &mut dyn CommandContext, args: Json) -> Result<Json, CommandError> {
        let args = if args.is_null() { json!({}) } else { args };
        if !args.is_object() {
            return Err(CommandError::InvalidArgs(format!(
                "{}: arguments must be an object",
                self.def.id
            )));
        }
        let registry = self.registry.get().and_then(Weak::upgrade).ok_or_else(|| {
            CommandError::Failed(format!(
                "{}: plugin is not bound to a command registry",
                self.def.id
            ))
        })?;
        run_function(
            ctx,
            &registry,
            &self.source,
            &self.def.function,
            &args,
            &self.opts,
        )
        .map(|report| report.value)
        .map_err(|e| CommandError::Failed(format!("{}: {e}", self.def.id)))
    }

    fn undoable(&self) -> bool {
        self.undoable
    }
}

/// What [`install_all`] did.
#[derive(Debug, Default)]
pub struct Installed {
    pub plugins: Vec<Arc<RhaiPlugin>>,
    /// Folders that failed to load or register, with the reason.
    pub failures: Vec<(PathBuf, String)>,
}

/// Loads every subfolder of `root` that has a manifest, sorted by folder name. A missing
/// `root` yields nothing.
pub fn discover(root: &Path) -> (Vec<RhaiPlugin>, Vec<(PathBuf, PluginError)>) {
    let mut dirs: Vec<PathBuf> = std::fs::read_dir(root)
        .map(|rd| {
            rd.filter_map(Result::ok)
                .map(|e| e.path())
                .filter(|p| p.join(MANIFEST_FILE).is_file())
                .collect()
        })
        .unwrap_or_default();
    dirs.sort();
    let mut ok = Vec::new();
    let mut failed = Vec::new();
    for dir in dirs {
        match RhaiPlugin::load(&dir) {
            Ok(p) => ok.push(p),
            Err(e) => failed.push((dir, e)),
        }
    }
    (ok, failed)
}

/// Discovers the plugins in `root` and registers their node types and commands. A plugin
/// that fails to register is skipped (and listed in `failures`). The caller then calls
/// [`RhaiPlugin::bind`] on each returned plugin once the command registry is shared.
pub fn install_all(
    root: &Path,
    registry: &mut Registry,
    commands: &mut CommandRegistry,
) -> Installed {
    let (found, failed) = discover(root);
    let mut out = Installed {
        failures: failed
            .into_iter()
            .map(|(p, e)| (p, e.to_string()))
            .collect(),
        ..Installed::default()
    };
    let mut names = BTreeSet::new();
    for plugin in found {
        let dir = plugin.dir.clone();
        if !names.insert(plugin.manifest.name.clone()) {
            out.failures
                .push((dir, "another plugin has the same name".into()));
            continue;
        }
        if let Some(clash) = plugin
            .manifest
            .commands
            .iter()
            .find(|c| commands.get(&c.id).is_some())
        {
            out.failures
                .push((dir, format!("command {} already exists", clash.id)));
            continue;
        }
        // Command ids were validated at load and checked above, so registering them below
        // cannot fail after the node types went in.
        if let Err(e) = registry.install(&plugin) {
            out.failures.push((dir, e.to_string()));
            continue;
        }
        if let Err(e) = plugin.register_commands(commands) {
            out.failures.push((dir, e.to_string()));
            continue;
        }
        out.plugins.push(Arc::new(plugin));
    }
    out
}
