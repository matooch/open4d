//! browser.*: the preset library and the recent file list (docs/specs/UI.md §26,
//! COMMANDS.md §6).

use forge_core::{Document, NodeId, file};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};

use super::file::{PathArgs, open};
use super::object::top_level;
use super::{register_plain, spec, target_nodes};
use crate::browser::{Browser, file_name, graft, parse_preset};
use crate::context::CommandContext;
use crate::error::CommandError;
use crate::registry::CommandRegistry;

/// What a preset holds.
#[derive(Deserialize, JsonSchema, Clone, Copy)]
#[serde(rename_all = "lowercase")]
enum Kind {
    Objects,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct ListArgs {
    /// Preset kind; only `objects` exists.
    kind: Option<Kind>,
    /// Keep the presets whose name contains this text (case-insensitive).
    query: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct LoadArgs {
    /// `builtin:<name>` or `user:<name>`.
    id: String,
    /// Parent node of the loaded objects; the top level when absent.
    parent: Option<u64>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct SaveArgs {
    /// Preset name; the file name is derived from it.
    name: String,
    /// Nodes to save; the selection when absent.
    nodes: Option<Vec<u64>>,
    /// Replace an existing preset of the same name.
    #[serde(default)]
    overwrite: bool,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct DeleteArgs {
    /// A `user:<name>` preset id.
    id: String,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct OpenRecentArgs {
    /// Position in the recent list, 0 = newest.
    index: usize,
}

fn browser(ctx: &dyn CommandContext) -> Result<&Browser, CommandError> {
    ctx.browser()
        .ok_or_else(|| CommandError::failed("the content browser needs the application"))
}

pub(super) fn register(r: &mut CommandRegistry) -> Result<(), CommandError> {
    r.register_fn(
        spec(
            "browser.list",
            "List Presets",
            false,
            || json!({"query": "table"}),
        ),
        list,
    )?;
    r.register_fn(
        spec(
            "browser.load",
            "Load Preset",
            true,
            || json!({"id": "builtin:table"}),
        ),
        load,
    )?;
    r.register_fn(
        spec(
            "browser.save_preset",
            "Save Preset",
            false,
            || json!({"name": "My preset", "overwrite": false}),
        ),
        save_preset,
    )?;
    r.register_fn(
        spec(
            "browser.delete_preset",
            "Delete Preset",
            false,
            || json!({"id": "user:my-preset"}),
        ),
        delete_preset,
    )?;
    register_plain(r, "browser.recent", "Recent Files", false, |c| {
        let b = browser(c)?;
        let files: Vec<Value> = b
            .recent
            .iter()
            .enumerate()
            .map(|(i, p)| {
                json!({"index": i, "path": p, "name": file_name(p),
                       "exists": std::path::Path::new(p).is_file()})
            })
            .collect();
        Ok(json!({ "files": files }))
    })?;
    r.register_fn(
        spec(
            "browser.open_recent",
            "Open Recent",
            false,
            || json!({"index": 0}),
        ),
        open_recent,
    )?;
    register_plain(
        r,
        "browser.clear_recent",
        "Clear Recent Files",
        false,
        |c| {
            let b = c
                .browser_mut()
                .ok_or_else(|| CommandError::failed("the content browser needs the application"))?;
            Ok(json!({"cleared": b.clear_recent()}))
        },
    )
}

fn list(ctx: &mut dyn CommandContext, a: ListArgs) -> Result<Value, CommandError> {
    let b = browser(ctx)?;
    let presets: Vec<Value> = b
        .list(a.query.as_deref())
        .into_iter()
        .map(|p| json!({"id": p.id, "name": p.name, "kind": "objects", "source": p.source.name()}))
        .collect();
    let _ = a.kind;
    Ok(json!({ "presets": presets }))
}

fn load(ctx: &mut dyn CommandContext, a: LoadArgs) -> Result<Value, CommandError> {
    let bytes = browser(ctx)?
        .read(&a.id)
        .map_err(CommandError::InvalidArgs)?;
    let registry = ctx.doc().scene().registry().clone();
    let (src, warnings) = parse_preset(registry, &bytes)
        .map_err(|e| CommandError::Failed(format!("Load Preset: {e}")))?;
    if src.roots().is_empty() {
        return Err(CommandError::failed("Load Preset: the preset is empty"));
    }
    let parent = a.parent.map(NodeId);
    if let Some(p) = parent
        && !ctx.doc().scene().node(p).is_some_and(|n| !n.is_tag())
    {
        return Err(CommandError::InvalidArgs(format!(
            "node {p} is not an object"
        )));
    }
    let roots = src.roots().to_vec();
    let created = ctx
        .doc_mut()
        .transact("Load Preset", |tx| graft(&src, &roots, tx, parent))?;
    let sel = ctx.selection_mut();
    sel.nodes = created.iter().copied().collect();
    sel.active = created.last().copied();
    sel.anchor = sel.active;
    ctx.set_status(warnings);
    Ok(json!({"preset": a.id, "nodes": created.iter().map(|n| n.0).collect::<Vec<_>>()}))
}

fn save_preset(ctx: &mut dyn CommandContext, a: SaveArgs) -> Result<Value, CommandError> {
    let b = browser(ctx)?;
    let ids: Vec<NodeId> = target_nodes(ctx, a.nodes);
    let scene = ctx.doc().scene();
    if let Some(bad) = ids.iter().find(|i| scene.node(**i).is_none()) {
        return Err(CommandError::InvalidArgs(format!(
            "node {bad} does not exist"
        )));
    }
    let roots = top_level(scene, &ids);
    if roots.is_empty() {
        return Err(CommandError::failed("Save Preset: select objects"));
    }
    if b.user_dir().is_none() {
        return Err(CommandError::failed(
            "Save Preset: there is no user preset folder",
        ));
    }
    let mut doc = Document::new(scene.registry().clone());
    doc.transact("Save Preset", |tx| graft(scene, &roots, tx, None))?;
    let bytes = file::save_with(doc.scene(), &file::SaveOptions::default());
    let (id, path) = b
        .write_user(&a.name, &bytes, a.overwrite)
        .map_err(|e| CommandError::Failed(format!("Save Preset: {e}")))?;
    Ok(json!({"id": id, "path": path.to_string_lossy()}))
}

fn delete_preset(ctx: &mut dyn CommandContext, a: DeleteArgs) -> Result<Value, CommandError> {
    browser(ctx)?
        .delete_user(&a.id)
        .map_err(CommandError::Failed)?;
    Ok(json!({"deleted": a.id}))
}

fn open_recent(ctx: &mut dyn CommandContext, a: OpenRecentArgs) -> Result<Value, CommandError> {
    let path = browser(ctx)?
        .recent
        .get(a.index)
        .cloned()
        .ok_or_else(|| CommandError::InvalidArgs(format!("no recent file {}", a.index)))?;
    if !std::path::Path::new(&path).is_file() {
        if let Some(b) = ctx.browser_mut() {
            b.remove_recent(a.index);
        }
        return Err(CommandError::Failed(format!(
            "Open failed: {path} no longer exists"
        )));
    }
    open(ctx, PathArgs { path })
}
