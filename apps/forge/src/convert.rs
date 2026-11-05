//! `forge convert` and the shared `--option` parsing (docs/specs/CLI.md, IO.md §3).

use std::path::{Path, PathBuf};

use forge_io::formats::{self, Options};
use serde_json::{Map, json};

use crate::{Fail, Json, Out, Outcome, new_app, open, user};

/// The arguments of one `forge convert` run.
pub struct Job<'a> {
    pub input: &'a str,
    pub out: &'a str,
    pub format: &'a str,
    pub options: &'a [String],
    pub frame: i64,
    pub recursive: bool,
    pub force: bool,
}

/// `key=value` pairs as a JSON map. A value that is `true`, `false` or a number is typed,
/// anything else is a string.
pub fn parse_pairs(pairs: &[String]) -> Result<Map<String, Json>, Fail> {
    let mut map = Map::new();
    for pair in pairs {
        let (key, value) = pair
            .split_once('=')
            .filter(|(k, _)| !k.is_empty())
            .ok_or_else(|| user(format!("--option {pair:?} is not key=value")))?;
        let value = match serde_json::from_str::<Json>(value) {
            Ok(v @ (Json::Bool(_) | Json::Number(_))) => v,
            _ => Json::String(value.to_string()),
        };
        map.insert(key.to_string(), value);
    }
    Ok(map)
}

/// Resolved export options of `format`; an unknown or import-only format is a user error that
/// names the supported ids.
pub fn export_options_for(format: &str, pairs: &[String]) -> Result<Options, Fail> {
    let descs = formats::format_by_id(format)
        .filter(|f| f.export)
        .and_then(|f| formats::export_options(f.id))
        .ok_or_else(|| {
            let ids: Vec<&str> = formats::formats()
                .iter()
                .filter(|f| f.export)
                .map(|f| f.id)
                .collect();
            user(format!(
                "unsupported format {format:?} (supported: {})",
                ids.join(", ")
            ))
        })?;
    formats::resolve_options(&descs, &parse_pairs(pairs)?).map_err(|e| user(e.to_string()))
}

/// The file extension of the converted files.
fn out_extension(format: &str) -> &'static str {
    formats::format_by_id(format).map_or("forge", |f| f.extensions[0])
}

fn extension_of(path: &Path) -> String {
    path.extension()
        .map(|e| e.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default()
}

/// Every file below `dir` (only direct children unless `recursive`), as paths relative to it.
fn walk(dir: &Path, rel: &Path, recursive: bool, out: &mut Vec<PathBuf>) -> Result<(), Fail> {
    let here = dir.join(rel);
    let entries = std::fs::read_dir(&here)
        .map_err(|e| user(format!("cannot read {}: {e}", here.display())))?;
    for entry in entries {
        let entry = entry.map_err(|e| user(format!("cannot read {}: {e}", here.display())))?;
        let rel_path = rel.join(entry.file_name());
        let kind = entry
            .file_type()
            .map_err(|e| user(format!("cannot read {}: {e}", entry.path().display())))?;
        if kind.is_dir() {
            if recursive {
                walk(dir, &rel_path, recursive, out)?;
            }
        } else {
            out.push(rel_path);
        }
    }
    Ok(())
}

pub fn run(job: &Job) -> Outcome {
    let native = job.format == "forge";
    // Validate the options before any file is touched.
    let pairs = parse_pairs(job.options)?;
    let export_opts = if native {
        Options::new()
    } else {
        export_options_for(job.format, job.options)?
    };
    if native {
        for f in formats::formats().iter().filter(|f| f.import) {
            let descs = formats::import_options(f.id).unwrap_or_default();
            formats::resolve_options(&descs, &pairs).map_err(|e| user(e.to_string()))?;
        }
    }

    let input = Path::new(job.input);
    let mut files: Vec<(PathBuf, PathBuf)> = Vec::new(); // (path, path relative to the output)
    if input.is_dir() {
        let mut rels = Vec::new();
        walk(input, Path::new(""), job.recursive, &mut rels)?;
        rels.sort_by(|a, b| {
            a.to_string_lossy()
                .as_bytes()
                .cmp(b.to_string_lossy().as_bytes())
        });
        files.extend(rels.into_iter().map(|r| (input.join(&r), r)));
    } else if input.is_file() {
        let name = PathBuf::from(input.file_name().unwrap_or_default());
        files.push((input.to_path_buf(), name));
    } else {
        return Err(user(format!("{}: no such file or folder", job.input)));
    }

    let ext = out_extension(job.format);
    let mut skipped = Vec::new();
    let mut done = Vec::new();
    let mut failed = 0usize;
    for (path, rel) in files {
        let source_ext = extension_of(&path);
        let source_format = formats::format_for_path(&path.to_string_lossy())
            .filter(|f| f.import)
            .map(|f| f.id);
        let convertible = if native {
            source_format.is_some()
        } else {
            source_ext == "forge"
        };
        if !convertible {
            skipped.push(path.to_string_lossy().into_owned());
            continue;
        }
        let output = Path::new(job.out).join(rel.with_extension(ext));
        let shown = path.to_string_lossy().into_owned();
        match convert_one(job, &path, &output, source_format, &export_opts, &pairs) {
            Ok(entry) => done.push(entry),
            Err(message) => {
                failed += 1;
                eprintln!("error: {shown}: {message}");
            }
        }
    }
    if failed > 0 {
        return Err(user(format!(
            "{failed} of {} file(s) failed to convert",
            failed + done.len()
        )));
    }
    if done.is_empty() {
        return Err(user(format!("no convertible files in {}", job.input)));
    }
    let text = done
        .iter()
        .map(|e| {
            format!(
                "{} -> {}",
                e["input"].as_str().unwrap_or(""),
                e["output"].as_str().unwrap_or("")
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    Ok(Out {
        json: json!({"format": job.format, "out": job.out, "converted": done.len(),
            "skipped": skipped, "files": done}),
        text: format!(
            "{text}\nconverted {} file(s), skipped {}",
            done.len(),
            skipped.len()
        ),
    })
}

fn convert_one(
    job: &Job,
    input: &Path,
    output: &Path,
    source_format: Option<&str>,
    export_opts: &Options,
    pairs: &Map<String, Json>,
) -> Result<Json, String> {
    if output.exists() && !job.force {
        return Err(format!(
            "{} exists (use --force to overwrite)",
            output.display()
        ));
    }
    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("cannot create {}: {e}", parent.display()))?;
    }
    let (input_s, output_s) = (input.to_string_lossy(), output.to_string_lossy());
    let mut entry = json!({"input": input_s, "output": output_s});
    if let Some(format) = source_format.filter(|_| job.format == "forge") {
        let mut app = new_app();
        let args = json!({"path": input_s, "format": format, "options": pairs});
        let result = app.run("file.import", args).map_err(|e| e.to_string())?;
        app.save(&output_s).map_err(|e| e.to_string())?;
        entry["nodes"] = result["nodes"].as_array().map_or(0, Vec::len).into();
    } else {
        let (app, _) = open(&input_s).map_err(|f| f.message)?;
        let n = app
            .export_at(&output_s, job.format, None, job.frame, export_opts)
            .map_err(|e| e.to_string())?;
        entry["objects"] = n.into();
    }
    Ok(entry)
}
