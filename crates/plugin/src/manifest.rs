//! The `plugin.toml` manifest (docs/specs/PLUGINS.md §2): parsing and validation.

use forge_core::{Category, NodeType, ParamDesc, Unit};
use serde::Deserialize;

use crate::PluginError;

/// Permissions a manifest may grant.
pub const PERMISSIONS: &[&str] = &["commands"];

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawManifest {
    pub name: String,
    pub version: String,
    pub entry: String,
    #[serde(default)]
    pub permissions: Vec<String>,
    #[serde(default)]
    pub node_types: Vec<RawNodeType>,
    #[serde(default)]
    pub commands: Vec<RawCommand>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawNodeType {
    pub id: String,
    pub label: String,
    pub build: String,
    pub category: Option<String>,
    pub in_create_menu: Option<bool>,
    #[serde(default)]
    pub params: Vec<RawParam>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawParam {
    pub id: String,
    pub label: Option<String>,
    pub kind: String,
    pub default: Option<toml::Value>,
    pub min: Option<toml::Value>,
    pub max: Option<toml::Value>,
    pub unit: Option<String>,
    #[serde(default)]
    pub options: Vec<RawOption>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawOption {
    pub id: String,
    pub label: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawCommand {
    pub id: String,
    pub label: String,
    pub function: String,
}

/// A validated node type: the registry descriptor plus the function that builds it.
#[derive(Debug, Clone)]
pub struct NodeTypeDef {
    pub node_type: NodeType,
    /// Rhai function called with the parameter map.
    pub build: String,
    /// The plugin-defined param ids (the registry adds transform params to hierarchy types).
    pub param_ids: Vec<String>,
}

/// A validated command declaration.
#[derive(Debug, Clone)]
pub struct CommandDef {
    pub id: String,
    pub label: String,
    pub function: String,
}

/// A validated manifest.
#[derive(Debug, Clone)]
pub struct Manifest {
    pub name: String,
    pub version: String,
    pub entry: String,
    pub permissions: Vec<String>,
    pub node_types: Vec<NodeTypeDef>,
    pub commands: Vec<CommandDef>,
}

impl Manifest {
    pub fn has_permission(&self, name: &str) -> bool {
        self.permissions.iter().any(|p| p == name)
    }

    /// Parses and validates manifest text.
    pub fn parse(text: &str) -> Result<Manifest, PluginError> {
        let raw: RawManifest = toml::from_str(text).map_err(|e| bad(e.message()))?;
        validate(raw)
    }
}

fn bad(message: impl Into<String>) -> PluginError {
    PluginError::Manifest(message.into())
}

fn valid_name(name: &str) -> bool {
    let mut chars = name.chars();
    chars.next().is_some_and(|c| c.is_ascii_lowercase())
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

fn valid_version(v: &str) -> bool {
    let parts: Vec<&str> = v.split('.').collect();
    parts.len() == 3
        && parts
            .iter()
            .all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()))
}

fn valid_entry(path: &str) -> bool {
    let p = std::path::Path::new(path);
    !path.is_empty()
        && p.is_relative()
        && !path.contains('\\')
        && p.components()
            .all(|c| matches!(c, std::path::Component::Normal(_)))
}

fn valid_function(name: &str) -> bool {
    let mut chars = name.chars();
    chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

fn validate(raw: RawManifest) -> Result<Manifest, PluginError> {
    if !valid_name(&raw.name) {
        return Err(bad(format!(
            "name {:?}: expected [a-z][a-z0-9_]*",
            raw.name
        )));
    }
    if !valid_version(&raw.version) {
        return Err(bad(format!(
            "version {:?}: expected MAJOR.MINOR.PATCH",
            raw.version
        )));
    }
    if !valid_entry(&raw.entry) {
        return Err(bad(format!(
            "entry {:?}: expected a relative path inside the plugin folder",
            raw.entry
        )));
    }
    for p in &raw.permissions {
        if !PERMISSIONS.contains(&p.as_str()) {
            return Err(bad(format!("unknown permission {p:?}")));
        }
    }
    let prefix = format!("{}.", raw.name);
    let mut node_types = Vec::new();
    for t in &raw.node_types {
        if !t.id.starts_with(&prefix) {
            return Err(bad(format!(
                "node type {:?} must start with {prefix:?}",
                t.id
            )));
        }
        if !valid_function(&t.build) {
            return Err(bad(format!(
                "node type {}: bad function {:?}",
                t.id, t.build
            )));
        }
        let category = match t.category.as_deref() {
            None | Some("generator") => Category::Generator,
            Some("object") => Category::Object,
            Some(other) => {
                return Err(bad(format!(
                    "node type {}: category {other:?} is not generator or object",
                    t.id
                )));
            }
        };
        let mut ty = NodeType::new(&t.id, &t.label, category);
        if t.in_create_menu == Some(false) {
            ty = ty.not_in_create_menu();
        }
        let mut param_ids = Vec::new();
        for p in &t.params {
            ty = ty.param(param_desc(&t.id, p)?);
            param_ids.push(p.id.clone());
        }
        node_types.push(NodeTypeDef {
            node_type: ty,
            build: t.build.clone(),
            param_ids,
        });
    }
    let mut commands = Vec::new();
    for c in &raw.commands {
        if !c.id.starts_with(&prefix) || !forge_command::valid_command_id(&c.id) {
            return Err(bad(format!(
                "command {:?} must be a command id starting with {prefix:?}",
                c.id
            )));
        }
        if !valid_function(&c.function) {
            return Err(bad(format!(
                "command {}: bad function {:?}",
                c.id, c.function
            )));
        }
        commands.push(CommandDef {
            id: c.id.clone(),
            label: c.label.clone(),
            function: c.function.clone(),
        });
    }
    Ok(Manifest {
        name: raw.name,
        version: raw.version,
        entry: raw.entry,
        permissions: raw.permissions,
        node_types,
        commands,
    })
}

fn number(ty: &str, param: &str, key: &str, v: &toml::Value) -> Result<f64, PluginError> {
    let n = match v {
        toml::Value::Integer(i) => *i as f64,
        toml::Value::Float(f) => *f,
        _ => return Err(bad(format!("{ty}.{param}: {key} must be a number"))),
    };
    if n.is_finite() {
        Ok(n)
    } else {
        Err(bad(format!("{ty}.{param}: {key} must be finite")))
    }
}

fn integer(ty: &str, param: &str, key: &str, v: &toml::Value) -> Result<i64, PluginError> {
    v.as_integer()
        .ok_or_else(|| bad(format!("{ty}.{param}: {key} must be an integer")))
}

fn vec3(ty: &str, param: &str, v: &toml::Value) -> Result<[f64; 3], PluginError> {
    let items = v
        .as_array()
        .filter(|a| a.len() == 3)
        .ok_or_else(|| bad(format!("{ty}.{param}: default must be [x, y, z]")))?;
    let mut out = [0.0; 3];
    for (slot, item) in out.iter_mut().zip(items) {
        *slot = number(ty, param, "default", item)?;
    }
    Ok(out)
}

fn param_desc(ty: &str, p: &RawParam) -> Result<ParamDesc, PluginError> {
    let label = p.label.as_deref().unwrap_or(&p.id);
    let unit = match p.unit.as_deref() {
        None | Some("none") => Unit::None,
        Some("distance") => Unit::Distance,
        Some("angle") => Unit::Angle,
        Some("percent") => Unit::Percent,
        Some("time") => Unit::Time,
        Some(other) => return Err(bad(format!("{ty}.{}: unknown unit {other:?}", p.id))),
    };
    // Manifest numbers are in display units; angles are stored in radians, percent as a
    // fraction (PLUGINS.md §2).
    let scale = |v: f64| match unit {
        Unit::Angle => v.to_radians(),
        Unit::Percent => v / 100.0,
        _ => v,
    };
    let id = p.id.as_str();
    let num = |key: &str, v: &Option<toml::Value>, fallback: f64| -> Result<f64, PluginError> {
        match v {
            Some(v) => number(ty, id, key, v).map(scale),
            None => Ok(fallback),
        }
    };
    let desc = match p.kind.as_str() {
        "bool" => {
            let d = match &p.default {
                None => false,
                Some(v) => v
                    .as_bool()
                    .ok_or_else(|| bad(format!("{ty}.{id}: default must be a bool")))?,
            };
            ParamDesc::bool(id, label, d)
        }
        "int" => {
            let d = match &p.default {
                None => 0,
                Some(v) => integer(ty, id, "default", v)?,
            };
            let lo = match &p.min {
                Some(v) => integer(ty, id, "min", v)?,
                None => i64::MIN,
            };
            let hi = match &p.max {
                Some(v) => integer(ty, id, "max", v)?,
                None => i64::MAX,
            };
            if lo > hi {
                return Err(bad(format!("{ty}.{id}: min > max")));
            }
            ParamDesc::int(id, label, d).range(lo, hi)
        }
        "float" => {
            let d = num("default", &p.default, 0.0)?;
            let lo = num("min", &p.min, f64::MIN)?;
            let hi = num("max", &p.max, f64::MAX)?;
            if lo > hi {
                return Err(bad(format!("{ty}.{id}: min > max")));
            }
            ParamDesc::float(id, label, d).range(lo, hi)
        }
        "vec3" => {
            let d = match &p.default {
                None => [0.0; 3],
                Some(v) => vec3(ty, id, v)?.map(scale),
            };
            let lo = num("min", &p.min, f64::MIN)?;
            let hi = num("max", &p.max, f64::MAX)?;
            if lo > hi {
                return Err(bad(format!("{ty}.{id}: min > max")));
            }
            ParamDesc::vec3(id, label, d).range(lo, hi)
        }
        "color" => {
            let d = match &p.default {
                None => [0.0; 3],
                Some(v) => vec3(ty, id, v)?,
            };
            ParamDesc::color(id, label, d)
        }
        "string" => {
            let d = match &p.default {
                None => "",
                Some(v) => v
                    .as_str()
                    .ok_or_else(|| bad(format!("{ty}.{id}: default must be a string")))?,
            };
            ParamDesc::string(id, label, d)
        }
        "enum" => {
            let options: Vec<(&str, &str)> = p
                .options
                .iter()
                .map(|o| (o.id.as_str(), o.label.as_str()))
                .collect();
            let d = match (&p.default, options.first()) {
                (Some(v), _) => v
                    .as_str()
                    .ok_or_else(|| bad(format!("{ty}.{id}: default must be an option id")))?,
                (None, Some((first, _))) => first,
                (None, None) => {
                    return Err(bad(format!("{ty}.{id}: an enum needs options")));
                }
            };
            ParamDesc::enumeration(id, label, d, &options)
        }
        other => {
            return Err(bad(format!("{ty}.{id}: unknown kind {other:?}")));
        }
    };
    Ok(desc.unit(unit))
}
