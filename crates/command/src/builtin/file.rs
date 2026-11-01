//! file.*

use forge_core::{Document, NodeId, file};
use forge_io::obj::{ReadOptions, WriteOptions, read, write};
use forge_io::{export_scene, import_into};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};

use super::{register_plain, spec};
use crate::context::CommandContext;
use crate::error::CommandError;
use crate::registry::CommandRegistry;

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct PathArgs {
    path: String,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct SaveArgs {
    /// Target file; the current file when absent.
    path: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct ExportArgs {
    path: String,
    /// Export only the selected objects.
    #[serde(default)]
    selected: bool,
}

pub(super) fn register(r: &mut CommandRegistry) -> Result<(), CommandError> {
    register_plain(r, "file.new", "New", false, |c| {
        let registry = c.doc().scene().registry().clone();
        c.reset_document(Document::new(registry), None, true);
        Ok(Value::Null)
    })?;
    r.register_fn(
        spec(
            "file.open",
            "Open",
            false,
            || json!({"path": "scene.forge"}),
        ),
        open,
    )?;
    r.register_fn(
        spec(
            "file.save",
            "Save",
            false,
            || json!({"path": "scene.forge"}),
        ),
        |c, a: SaveArgs| {
            let path = a
                .path
                .or_else(|| c.file_path())
                .ok_or_else(|| CommandError::failed("Save failed: no file name"))?;
            save_to(c, path)
        },
    )?;
    r.register_fn(
        spec(
            "file.save_as",
            "Save As",
            false,
            || json!({"path": "scene.forge"}),
        ),
        |c, a: PathArgs| save_to(c, a.path),
    )?;
    r.register_fn(
        spec(
            "file.import_obj",
            "Import OBJ",
            true,
            || json!({"path": "model.obj"}),
        ),
        import_obj,
    )?;
    r.register_fn(
        spec(
            "file.export_obj",
            "Export OBJ",
            false,
            || json!({"path": "model.obj", "selected": false}),
        ),
        export_obj,
    )
}

fn open(ctx: &mut dyn CommandContext, a: PathArgs) -> Result<Value, CommandError> {
    let registry = ctx.doc().scene().registry().clone();
    let loaded = std::fs::read_to_string(&a.path)
        .map_err(|e| e.to_string())
        .and_then(|text| file::load(registry, &text).map_err(|e| e.to_string()));
    let (scene, report) = loaded.map_err(|e| CommandError::Failed(format!("Open failed: {e}")))?;
    ctx.reset_document(Document::from_scene(scene), Some(a.path), false);
    ctx.set_status(report.warnings.join("; "));
    Ok(json!({"warnings": report.warnings}))
}

fn save_to(ctx: &mut dyn CommandContext, path: String) -> Result<Value, CommandError> {
    let text = file::save(ctx.doc().scene());
    std::fs::write(&path, text).map_err(|e| CommandError::Failed(format!("Save failed: {e}")))?;
    ctx.mark_saved(path.clone());
    ctx.set_status(String::new());
    Ok(json!({"path": path}))
}

fn import_obj(ctx: &mut dyn CommandContext, a: PathArgs) -> Result<Value, CommandError> {
    let objects = std::fs::read_to_string(&a.path)
        .map_err(|e| e.to_string())
        .and_then(|text| read(&text, &ReadOptions::default()).map_err(|e| e.to_string()))
        .map_err(|e| CommandError::Failed(format!("Import failed: {e}")))?;
    let ids = ctx
        .doc_mut()
        .transact("Import OBJ", |tx| import_into(tx, &objects))
        .map_err(|e| CommandError::Failed(format!("Import failed: {e}")))?;
    let Some((first, rest)) = ids.split_first() else {
        return Err(CommandError::failed("Import: no polygons found"));
    };
    ctx.set_status(String::new());
    let sel = ctx.selection_mut();
    sel.click(*first);
    for id in rest {
        sel.toggle(*id);
    }
    Ok(json!({"nodes": ids.iter().map(|n| n.0).collect::<Vec<_>>()}))
}

fn export_obj(ctx: &mut dyn CommandContext, a: ExportArgs) -> Result<Value, CommandError> {
    let roots: Vec<NodeId> = ctx.selection().nodes.iter().copied().collect();
    let objects = export_scene(ctx.doc().scene(), a.selected.then_some(&roots[..]));
    if objects.is_empty() {
        return Err(CommandError::failed("Nothing to export"));
    }
    let text = write(&objects, &WriteOptions::default());
    std::fs::write(&a.path, text)
        .map_err(|e| CommandError::Failed(format!("Export failed: {e}")))?;
    ctx.set_status(String::new());
    Ok(json!({"objects": objects.len()}))
}
