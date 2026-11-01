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
use crate::registry::{CommandHandler, CommandRegistry, NoArgs};
use crate::state::UiRequest;

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
    for (id, label, request) in [
        ("file.open_dialog", "Open...", UiRequest::Open),
        ("file.save_dialog", "Save", UiRequest::Save),
        ("file.save_as_dialog", "Save As...", UiRequest::SaveAs),
    ] {
        r.register(Box::new(DialogCommand { id, label, request }))?;
    }
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

/// The absolute folder of a scene file, the base of its external references.
fn scene_dir(path: &str) -> Option<std::path::PathBuf> {
    let abs = std::path::absolute(path).ok()?;
    abs.parent().map(std::path::Path::to_path_buf)
}

fn open(ctx: &mut dyn CommandContext, a: PathArgs) -> Result<Value, CommandError> {
    let registry = ctx.doc().scene().registry().clone();
    let opts = file::LoadOptions {
        base_dir: scene_dir(&a.path),
        check_external: true,
    };
    let loaded = std::fs::read(&a.path)
        .map_err(|e| e.to_string())
        .and_then(|bytes| file::load_with(registry, &bytes, &opts).map_err(|e| e.to_string()));
    let (scene, report) = loaded.map_err(|e| CommandError::Failed(format!("Open failed: {e}")))?;
    ctx.reset_document(Document::from_scene(scene), Some(a.path), false);
    ctx.set_status(report.warnings.join("; "));
    Ok(json!({"warnings": report.warnings}))
}

fn save_to(ctx: &mut dyn CommandContext, path: String) -> Result<Value, CommandError> {
    let opts = file::SaveOptions {
        base_dir: scene_dir(&path),
        ..file::SaveOptions::default()
    };
    let bytes = file::save_with(ctx.doc().scene(), &opts);
    std::fs::write(&path, bytes).map_err(|e| CommandError::Failed(format!("Save failed: {e}")))?;
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
    let scene = ctx.evaluated_scene();
    let objects = export_scene(&scene, a.selected.then_some(&roots[..]));
    if objects.is_empty() {
        return Err(CommandError::failed("Nothing to export"));
    }
    let text = write(&objects, &WriteOptions::default());
    std::fs::write(&a.path, text)
        .map_err(|e| CommandError::Failed(format!("Export failed: {e}")))?;
    ctx.set_status(String::new());
    Ok(json!({"objects": objects.len()}))
}

/// `file.open_dialog`, `file.save_dialog`, `file.save_as_dialog`: the File menu items that
/// ask for a file name, so a shortcut can run them (the window shows the dialog).
struct DialogCommand {
    id: &'static str,
    label: &'static str,
    request: UiRequest,
}

impl CommandHandler for DialogCommand {
    fn id(&self) -> &str {
        self.id
    }

    fn label(&self) -> &str {
        self.label
    }

    fn args_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(NoArgs)).unwrap_or(Value::Null)
    }

    fn run(&self, ctx: &mut dyn CommandContext, args: Value) -> Result<Value, CommandError> {
        if !(args.is_null() || args.as_object().is_some_and(|o| o.is_empty())) {
            return Err(CommandError::InvalidArgs(format!(
                "{} takes no arguments",
                self.id
            )));
        }
        ctx.request_ui(self.request).map_err(CommandError::Failed)?;
        Ok(Value::Null)
    }

    fn undoable(&self) -> bool {
        false
    }
}
