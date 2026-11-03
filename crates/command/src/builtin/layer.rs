//! layer.*: layers as a document level concept (docs/specs/CORE.md §7, COMMANDS.md §6).

use forge_core::layers::{self, LAYER};
use forge_core::{EditError, Layer, LayerId, NodeId, Value as Param};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};

use super::{spec, target_nodes};
use crate::context::CommandContext;
use crate::error::CommandError;
use crate::registry::CommandRegistry;

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct CreateArgs {
    /// Name; "Layer N" when absent.
    name: Option<String>,
    /// Colour `[r, g, b]`, each 0..1; the next palette colour when absent.
    color: Option<[f64; 3]>,
    /// Put `nodes` (the selection when absent) on the new layer.
    #[serde(default)]
    assign: bool,
    nodes: Option<Vec<u64>>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct DeleteArgs {
    /// Layer ids.
    layers: Vec<u64>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct RenameArgs {
    layer: u64,
    name: String,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct SetArgs {
    /// One layer id.
    layer: Option<u64>,
    /// Several layer ids (instead of `layer`).
    layers: Option<Vec<u64>>,
    name: Option<String>,
    color: Option<[f64; 3]>,
    /// Only soloed layers are shown while any is soloed.
    solo: Option<bool>,
    /// Viewport visibility.
    view: Option<bool>,
    /// Render visibility.
    render: Option<bool>,
    /// Listed in the object manager.
    manager: Option<bool>,
    /// Not pickable or draggable in the viewport.
    locked: Option<bool>,
    /// Tracks of the members are evaluated.
    animation: Option<bool>,
    /// Generator members produce geometry.
    generators: Option<bool>,
    /// Deformer members deform.
    deformers: Option<bool>,
    /// Expression tags of the members run.
    expressions: Option<bool>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct AssignArgs {
    /// The layer; null (or absent) removes the nodes from their layer.
    layer: Option<u64>,
    /// Nodes; the current selection when absent.
    nodes: Option<Vec<u64>>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct MergeArgs {
    /// Layers to merge, at least two, including `into`.
    layers: Vec<u64>,
    /// The layer that stays and receives the members.
    into: u64,
}

fn known(ctx: &dyn CommandContext, id: u64) -> Result<LayerId, CommandError> {
    let id = NodeId(id);
    if ctx.doc().scene().layers().contains(id) {
        Ok(id)
    } else {
        Err(CommandError::InvalidArgs(format!(
            "layer {id} does not exist"
        )))
    }
}

fn known_all(ctx: &dyn CommandContext, ids: &[u64]) -> Result<Vec<LayerId>, CommandError> {
    ids.iter().map(|i| known(ctx, *i)).collect()
}

/// Hierarchy nodes among the targets (tags take their host's layer).
fn hierarchy(ctx: &dyn CommandContext, nodes: Option<Vec<u64>>) -> Vec<NodeId> {
    let scene = ctx.doc().scene();
    target_nodes(ctx, nodes)
        .into_iter()
        .filter(|id| scene.node(*id).is_some_and(|n| !n.is_tag()))
        .collect()
}

fn create(ctx: &mut dyn CommandContext, a: CreateArgs) -> Result<Value, CommandError> {
    let members = if a.assign {
        hierarchy(ctx, a.nodes)
    } else {
        Vec::new()
    };
    let layer = ctx.doc_mut().transact("New Layer", |tx| {
        let layer = tx.add_layer(a.name.as_deref(), a.color)?;
        for id in &members {
            tx.set_param(*id, LAYER, Param::Link(Some(layer)))?;
        }
        Ok(layer)
    })?;
    Ok(json!({"layer": layer.0}))
}

fn delete(ctx: &mut dyn CommandContext, a: DeleteArgs) -> Result<Value, CommandError> {
    if a.layers.is_empty() {
        return Err(CommandError::InvalidArgs(
            "layer.delete needs `layers`".into(),
        ));
    }
    let ids = known_all(ctx, &a.layers)?;
    let released: usize = ids
        .iter()
        .map(|l| layers::members(ctx.doc().scene(), *l).len())
        .sum();
    let mut unique = ids.clone();
    unique.sort();
    unique.dedup();
    ctx.doc_mut().transact("Delete Layer", |tx| {
        unique.iter().try_for_each(|l| tx.remove_layer(*l))
    })?;
    Ok(json!({"deleted": unique.len(), "released": released}))
}

fn rename(ctx: &mut dyn CommandContext, a: RenameArgs) -> Result<Value, CommandError> {
    let id = known(ctx, a.layer)?;
    if a.name.trim().is_empty() {
        return Err(CommandError::InvalidArgs(
            "a layer name cannot be empty".into(),
        ));
    }
    let mut layer = layer_of(ctx, id);
    layer.name = a.name;
    ctx.doc_mut()
        .transact("Rename Layer", |tx| tx.update_layer(layer))?;
    Ok(Value::Null)
}

fn layer_of(ctx: &dyn CommandContext, id: LayerId) -> Layer {
    ctx.doc()
        .scene()
        .layers()
        .get(id)
        .expect("checked layer")
        .clone()
}

fn set(ctx: &mut dyn CommandContext, a: SetArgs) -> Result<Value, CommandError> {
    let ids = match (a.layer, &a.layers) {
        (Some(_), Some(_)) => {
            return Err(CommandError::InvalidArgs(
                "give `layer` or `layers`, not both".into(),
            ));
        }
        (Some(l), None) => known_all(ctx, &[l])?,
        (None, Some(ls)) => known_all(ctx, ls)?,
        (None, None) => return Err(CommandError::InvalidArgs("layer.set needs `layer`".into())),
    };
    let any = a.name.is_some()
        || a.color.is_some()
        || [
            a.solo,
            a.view,
            a.render,
            a.manager,
            a.locked,
            a.animation,
            a.generators,
            a.deformers,
            a.expressions,
        ]
        .iter()
        .any(Option::is_some);
    if !any {
        return Err(CommandError::InvalidArgs(
            "layer.set needs at least one field to change".into(),
        ));
    }
    if a.name.as_deref().is_some_and(|n| n.trim().is_empty()) {
        return Err(CommandError::InvalidArgs(
            "a layer name cannot be empty".into(),
        ));
    }
    let before = ctx.doc().scene().layers().clone();
    ctx.doc_mut().transact("Layer", |tx| {
        for id in &ids {
            let mut l = tx.scene().layers().get(*id).expect("checked").clone();
            if let Some(n) = &a.name {
                l.name = n.clone();
            }
            if let Some(c) = a.color {
                l.color = c;
            }
            l.solo = a.solo.unwrap_or(l.solo);
            l.view = a.view.unwrap_or(l.view);
            l.render = a.render.unwrap_or(l.render);
            l.manager = a.manager.unwrap_or(l.manager);
            l.locked = a.locked.unwrap_or(l.locked);
            l.animation = a.animation.unwrap_or(l.animation);
            l.generators = a.generators.unwrap_or(l.generators);
            l.deformers = a.deformers.unwrap_or(l.deformers);
            l.expressions = a.expressions.unwrap_or(l.expressions);
            tx.update_layer(l)?;
        }
        Ok(())
    })?;
    let after = ctx.doc().scene().layers();
    let changed = ids
        .iter()
        .filter(|id| before.get(**id) != after.get(**id))
        .count();
    Ok(json!({"changed": changed}))
}

fn assign(ctx: &mut dyn CommandContext, a: AssignArgs) -> Result<Value, CommandError> {
    let layer = a.layer.map(|l| known(ctx, l)).transpose()?;
    let nodes = hierarchy(ctx, a.nodes);
    ctx.doc_mut().transact("Assign Layer", |tx| {
        nodes
            .iter()
            .try_for_each(|id| tx.set_param(*id, LAYER, Param::Link(layer)))
    })?;
    Ok(json!({"assigned": nodes.len()}))
}

fn merge(ctx: &mut dyn CommandContext, a: MergeArgs) -> Result<Value, CommandError> {
    let into = known(ctx, a.into)?;
    let from = known_all(ctx, &a.layers)?;
    if !from.contains(&into) || from.iter().filter(|l| **l != into).count() == 0 {
        return Err(CommandError::InvalidArgs(
            "layer.merge needs `into` and at least one other layer in `layers`".into(),
        ));
    }
    let merged = from.iter().filter(|l| **l != into).count();
    let members = ctx
        .doc_mut()
        .transact("Merge Layers", |tx| tx.merge_layers(&from, into))
        .map_err(|e| match e {
            EditError::UnknownLayer(_) => CommandError::InvalidArgs(e.to_string()),
            e => e.into(),
        })?;
    Ok(json!({"merged": merged, "members": members}))
}

pub(super) fn register(r: &mut CommandRegistry) -> Result<(), CommandError> {
    r.register_fn(
        spec(
            "layer.create",
            "New Layer",
            true,
            || json!({"name": "Props"}),
        ),
        create,
    )?;
    r.register_fn(
        spec(
            "layer.delete",
            "Delete Layer",
            true,
            || json!({"layers": [10]}),
        ),
        delete,
    )?;
    r.register_fn(
        spec(
            "layer.rename",
            "Rename Layer",
            true,
            || json!({"layer": 10, "name": "Walls"}),
        ),
        rename,
    )?;
    r.register_fn(
        spec(
            "layer.set",
            "Set Layer",
            true,
            || json!({"layer": 10, "view": false, "locked": true}),
        ),
        set,
    )?;
    r.register_fn(
        spec(
            "layer.assign",
            "Assign to Layer",
            true,
            || json!({"layer": 10, "nodes": [2, 3]}),
        ),
        assign,
    )?;
    r.register_fn(
        spec(
            "layer.merge",
            "Merge Layers",
            true,
            || json!({"layers": [10, 11], "into": 10}),
        ),
        merge,
    )?;
    Ok(())
}
