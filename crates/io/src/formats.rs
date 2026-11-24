//! Format table, option descriptors and text entry points (docs/specs/IO.md §3).

use std::collections::BTreeMap;

use forge_core::{EditError, NodeId, ParamDesc, ParamKind, Scene, Tx, Value};
use serde_json::Map;

use crate::export_scene;
use crate::obj::{self, ObjObject, ReadOptions, WriteOptions};
use crate::{abc, fbx, gltf, ply, stl, usda};

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

const FORMATS: &[FormatInfo] = &[
    FormatInfo {
        id: "obj",
        label: "Wavefront OBJ",
        extensions: &["obj"],
        import: true,
        export: true,
    },
    FormatInfo {
        id: "stl",
        label: "STL",
        extensions: &["stl"],
        import: true,
        export: true,
    },
    FormatInfo {
        id: "ply",
        label: "PLY",
        extensions: &["ply"],
        import: true,
        export: true,
    },
    FormatInfo {
        id: "gltf",
        label: "glTF 2.0",
        extensions: &["glb", "gltf"],
        import: true,
        export: true,
    },
    FormatInfo {
        id: "usda",
        label: "ASCII USD",
        extensions: &["usda"],
        import: true,
        export: true,
    },
    FormatInfo {
        id: "fbx",
        label: "FBX",
        extensions: &["fbx"],
        import: true,
        export: true,
    },
    FormatInfo {
        id: "abc",
        label: "Alembic (Ogawa)",
        extensions: &["abc"],
        import: true,
        export: true,
    },
];

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
        "stl" | "ply" | "abc" => Some(vec![scale_option()]),
        "gltf" => Some(vec![scale_option_with(100.0)]),
        "fbx" => Some(vec![scale_option()]),
        "usda" => Some(vec![]),
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
        "stl" | "ply" => Some(vec![
            ParamDesc::bool("binary", "Binary", true),
            scale_option(),
        ]),
        "gltf" => Some(vec![scale_option_with(0.01)]),
        "usda" => Some(vec![]),
        "abc" => Some(vec![scale_option()]),
        "fbx" => Some(vec![scale_option()]),
        _ => None,
    }
}

/// The multiplier of every coordinate of STL and PLY (IO.md §4).
fn scale_option() -> ParamDesc {
    scale_option_with(1.0)
}

fn scale_option_with(default: f64) -> ParamDesc {
    ParamDesc::float("scale", "Scale", default).range(1e-9, 1e9)
}

fn scale_from(options: &Options, fallback: f64) -> f64 {
    match options.get("scale") {
        Some(Value::Float(scale)) => *scale,
        _ => fallback,
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

fn float(options: &Options, id: &str, default: f64) -> f64 {
    match options.get(id) {
        Some(Value::Float(f)) => *f,
        _ => default,
    }
}

pub fn stl_write_options(options: &Options) -> stl::WriteOptions {
    stl::WriteOptions {
        binary: flag(options, "binary", true),
        scale: float(options, "scale", 1.0),
    }
}

pub fn stl_read_options(options: &Options) -> stl::ReadOptions {
    stl::ReadOptions {
        scale: float(options, "scale", 1.0),
    }
}

pub fn ply_write_options(options: &Options) -> ply::WriteOptions {
    ply::WriteOptions {
        binary: flag(options, "binary", true),
        scale: float(options, "scale", 1.0),
    }
}

pub fn ply_read_options(options: &Options) -> ply::ReadOptions {
    ply::ReadOptions {
        scale: float(options, "scale", 1.0),
    }
}

/// The scene (or the listed nodes with their descendants) as the format's text, plus the
/// number of objects written. A format whose output is not text (binary STL and PLY) is
/// `Unsupported(id, "text export")`: use [`export_bytes`].
pub fn export_text(
    id: &str,
    scene: &Scene,
    roots: Option<&[NodeId]>,
    options: &Options,
) -> Result<(String, usize), IoError> {
    let (bytes, count) = export_bytes(id, scene, roots, options)?;
    let text = String::from_utf8(bytes)
        .map_err(|_| IoError::Unsupported(id.to_string(), "text export"))?;
    Ok((text, count))
}

/// The scene (or the listed nodes with their descendants) as the format's file content,
/// plus the number of objects written (IO.md §4).
pub fn export_bytes(
    id: &str,
    scene: &Scene,
    roots: Option<&[NodeId]>,
    options: &Options,
) -> Result<(Vec<u8>, usize), IoError> {
    let info = format_by_id(id).ok_or_else(|| IoError::UnknownFormat(id.to_string()))?;
    if !info.export {
        return Err(IoError::Unsupported(id.to_string(), "export"));
    }
    if id == "gltf" {
        return gltf::write(scene, roots, scale_from(options, 0.01))
            .map_err(|e| IoError::Parse(e.to_string()));
    }
    if id == "usda" {
        return usda::write(scene, roots)
            .map(|(text, n)| (text.into_bytes(), n))
            .map_err(|e| match e {
                usda::Error::Empty => IoError::NothingToExport,
                e => IoError::Parse(e.to_string()),
            });
    }
    if id == "abc" {
        return abc::write(scene, roots, scale_from(options, 1.0))
            .map_err(|e| IoError::Parse(e.to_string()));
    }
    if id == "fbx" {
        return fbx::write(scene, roots, scale_from(options, 1.0)).map_err(|e| match e {
            fbx::Error::Empty => IoError::NothingToExport,
            e => IoError::Parse(e.to_string()),
        });
    }
    let objects = export_scene(scene, roots);
    if objects.is_empty() {
        return Err(IoError::NothingToExport);
    }
    let bytes = match id {
        "obj" => obj::write(&objects, &obj_write_options(options)).into_bytes(),
        "stl" => stl::write(&objects, &stl_write_options(options)),
        "ply" => ply::write(&objects, &ply_write_options(options)),
        _ => return Err(IoError::Unsupported(id.to_string(), "export")),
    };
    Ok((bytes, objects.len()))
}

/// Parses `text` in the format into objects.
pub fn read_text(id: &str, text: &str, options: &Options) -> Result<Vec<ObjObject>, IoError> {
    read_bytes(id, text.as_bytes(), options)
}

/// Parses a file's content in the format into objects.
pub fn read_bytes(id: &str, bytes: &[u8], options: &Options) -> Result<Vec<ObjObject>, IoError> {
    let info = format_by_id(id).ok_or_else(|| IoError::UnknownFormat(id.to_string()))?;
    if !info.import {
        return Err(IoError::Unsupported(id.to_string(), "import"));
    }
    let parse = |e: &dyn std::fmt::Display| IoError::Parse(e.to_string());
    match id {
        "obj" => {
            let text = std::str::from_utf8(bytes).map_err(|e| parse(&e))?;
            obj::read(text, &obj_read_options(options)).map_err(|e| parse(&e))
        }
        "stl" => stl::read(bytes, &stl_read_options(options)).map_err(|e| parse(&e)),
        "ply" => ply::read(bytes, &ply_read_options(options)).map_err(|e| parse(&e)),
        _ => Err(IoError::Unsupported(id.to_string(), "import")),
    }
}

/// A decoded scene before it is added to the document.
pub enum Import {
    Flat(Vec<ObjObject>),
    Gltf(gltf::ImportScene),
    Usda(usda::ImportScene),
    Abc(abc::ImportScene),
    Fbx(fbx::ImportScene),
}

impl Import {
    pub fn apply(&self, tx: &mut Tx<'_>) -> Result<Vec<NodeId>, EditError> {
        match self {
            Self::Flat(objects) => crate::import_into(tx, objects),
            Self::Gltf(scene) => scene.apply(tx),
            Self::Usda(scene) => scene.apply(tx),
            Self::Abc(scene) => scene.apply(tx),
            Self::Fbx(scene) => scene.apply(tx),
        }
    }
}

pub fn read_scene(id: &str, bytes: &[u8], options: &Options) -> Result<Import, IoError> {
    if id == "gltf" {
        gltf::read(bytes, scale_from(options, 100.0))
            .map(Import::Gltf)
            .map_err(|e| IoError::Parse(e.to_string()))
    } else if id == "usda" {
        let text = std::str::from_utf8(bytes).map_err(|e| IoError::Parse(e.to_string()))?;
        usda::read(text)
            .map(Import::Usda)
            .map_err(|e| IoError::Parse(e.to_string()))
    } else if id == "abc" {
        abc::read(bytes, scale_from(options, 1.0))
            .map(Import::Abc)
            .map_err(|e| IoError::Parse(e.to_string()))
    } else if id == "fbx" {
        fbx::read(bytes, scale_from(options, 1.0))
            .map(Import::Fbx)
            .map_err(|e| IoError::Parse(e.to_string()))
    } else {
        read_bytes(id, bytes, options).map(Import::Flat)
    }
}

/// The animated format preserves authored local transforms and the unposed bind mesh.
pub fn preserves_animation(id: &str) -> bool {
    matches!(id, "gltf" | "usda" | "abc" | "fbx")
}

/// GLB output must not be labelled as a JSON `.gltf` file.
pub fn validate_export_path(id: &str, path: &str) -> Result<(), IoError> {
    if id == "gltf"
        && std::path::Path::new(path)
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("gltf"))
    {
        return Err(IoError::Unsupported(
            id.into(),
            "write JSON glTF (use .glb)",
        ));
    }
    Ok(())
}
