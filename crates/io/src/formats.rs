//! Format table, option descriptors and text entry points (docs/specs/IO.md §3).

use std::collections::BTreeMap;

use forge_core::{NodeId, ParamDesc, ParamKind, Scene, Value};
use serde_json::Map;

use crate::export_scene;
use crate::obj::{self, ObjObject, ReadOptions, WriteOptions};

/// One exchange format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FormatInfo {
    pub id: &'static str,
    pub label: &'static str,
    /// Lower-case file extensions without the dot; the first is the default.
    pub extensions: &'static [&'static str],
    pub import: bool,
    pub export: bool,
}

const FORMATS: &[FormatInfo] = &[FormatInfo {
    id: "obj",
    label: "Wavefront OBJ",
    extensions: &["obj"],
    import: true,
    export: true,
}];

/// Option values by descriptor id.
pub type Options = BTreeMap<String, Value>;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum IoError {
    #[error("unknown format \"{0}\"")]
    UnknownFormat(String),
    #[error("format \"{0}\" cannot {1}")]
    Unsupported(String, &'static str),
    #[error("{0}")]
    BadOption(String),
    #[error("Nothing to export")]
    NothingToExport,
    #[error("{0}")]
    Parse(String),
}

/// Every format, in a fixed order.
pub fn formats() -> &'static [FormatInfo] {
    FORMATS
}

pub fn format_by_id(id: &str) -> Option<&'static FormatInfo> {
    FORMATS.iter().find(|f| f.id == id)
}

/// The format whose extension matches the path's (case-insensitive).
pub fn format_for_path(path: &str) -> Option<&'static FormatInfo> {
    let ext = std::path::Path::new(path)
        .extension()?
        .to_string_lossy()
        .to_ascii_lowercase();
    FORMATS
        .iter()
        .find(|f| f.extensions.contains(&ext.as_str()))
}

/// The import options of a format (`None`: unknown id).
pub fn import_options(id: &str) -> Option<Vec<ParamDesc>> {
    match id {
        "obj" => Some(vec![ParamDesc::bool("flip_z", "Flip Z", false)]),
        _ => None,
    }
}

/// The export options of a format (`None`: unknown id).
pub fn export_options(id: &str) -> Option<Vec<ParamDesc>> {
    match id {
        "obj" => Some(vec![
            ParamDesc::bool("normals", "Write normals", true),
            ParamDesc::bool("weld", "Merge equal points", true),
        ]),
        _ => None,
    }
}

/// The defaults of a descriptor list.
pub fn default_options(descs: &[ParamDesc]) -> Options {
    descs
        .iter()
        .map(|d| (d.id.clone(), d.default.clone()))
        .collect()
}

/// Defaults overridden by `given` (JSON). Unknown keys and wrong types are errors.
pub fn resolve_options(
    descs: &[ParamDesc],
    given: &Map<String, serde_json::Value>,
) -> Result<Options, IoError> {
    let mut out = default_options(descs);
    for (key, json) in given {
        let desc = descs
            .iter()
            .find(|d| &d.id == key)
            .ok_or_else(|| IoError::BadOption(format!("unknown option \"{key}\"")))?;
        let bad = |what: &str| IoError::BadOption(format!("option \"{key}\" must be {what}"));
        let value = match &desc.kind {
            ParamKind::Bool => Value::Bool(json.as_bool().ok_or_else(|| bad("a boolean"))?),
            ParamKind::Int { .. } => Value::Int(json.as_i64().ok_or_else(|| bad("an integer"))?),
            ParamKind::Float { .. } => Value::Float(json.as_f64().ok_or_else(|| bad("a number"))?),
            ParamKind::Enum { .. } => Value::Enum(
                json.as_str()
                    .ok_or_else(|| bad("an option id"))?
                    .to_string(),
            ),
            ParamKind::String => {
                Value::String(json.as_str().ok_or_else(|| bad("a string"))?.to_string())
            }
            _ => return Err(bad("a supported type")),
        };
        let value = desc
            .validate(value)
            .map_err(|e| IoError::BadOption(format!("option \"{key}\": {e}")))?;
        out.insert(key.clone(), value);
    }
    Ok(out)
}

fn flag(options: &Options, id: &str, default: bool) -> bool {
    match options.get(id) {
        Some(Value::Bool(b)) => *b,
        _ => default,
    }
}

pub fn obj_read_options(options: &Options) -> ReadOptions {
    ReadOptions {
        flip_z: flag(options, "flip_z", false),
    }
}

pub fn obj_write_options(options: &Options) -> WriteOptions {
    let d = WriteOptions::default();
    WriteOptions {
        normals: flag(options, "normals", d.normals),
        weld: flag(options, "weld", d.weld),
    }
}

/// The scene (or the listed nodes with their descendants) as the format's text, plus the
/// number of objects written.
pub fn export_text(
    id: &str,
    scene: &Scene,
    roots: Option<&[NodeId]>,
    options: &Options,
) -> Result<(String, usize), IoError> {
    let info = format_by_id(id).ok_or_else(|| IoError::UnknownFormat(id.to_string()))?;
    if !info.export {
        return Err(IoError::Unsupported(id.to_string(), "export"));
    }
    let objects = export_scene(scene, roots);
    if objects.is_empty() {
        return Err(IoError::NothingToExport);
    }
    match id {
        "obj" => Ok((
            obj::write(&objects, &obj_write_options(options)),
            objects.len(),
        )),
        _ => Err(IoError::Unsupported(id.to_string(), "export")),
    }
}

/// Parses `text` in the format into objects.
pub fn read_text(id: &str, text: &str, options: &Options) -> Result<Vec<ObjObject>, IoError> {
    let info = format_by_id(id).ok_or_else(|| IoError::UnknownFormat(id.to_string()))?;
    if !info.import {
        return Err(IoError::Unsupported(id.to_string(), "import"));
    }
    match id {
        "obj" => {
            obj::read(text, &obj_read_options(options)).map_err(|e| IoError::Parse(e.to_string()))
        }
        _ => Err(IoError::Unsupported(id.to_string(), "import")),
    }
}
