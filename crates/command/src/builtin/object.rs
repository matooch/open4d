//! object.*, edit.*, view.*, select.*, transform.set.

use std::collections::BTreeSet;

use forge_core::transform::{POS, ROT, SCALE};
use forge_core::{Category, NodeId, Value as Param};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};

use super::{register_plain, spec, target_nodes};
use crate::context::CommandContext;
use crate::error::CommandError;
use crate::registry::CommandRegistry;
use crate::state::{ModeArg, SelectOp, ShadingMode};

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct CreateArgs {
    /// Node type id, e.g. `forge.cube`.
    #[serde(rename = "type")]
    type_id: String,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct NodesArgs {
    /// Node ids; the current selection when absent.
    nodes: Option<Vec<u64>>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct RenameArgs {
    node: u64,
    name: String,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct ReparentArgs {
    node: u64,
    /// New parent; null moves to the root level. Appended as the last child.
    parent: Option<u64>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct ShadingArgs {
    mode: ShadingMode,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct NodeArgs {
    node: u64,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct ModeArgs {
    mode: ModeArg,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct TransformArgs {
    node: u64,
    /// Position `[x, y, z]`.
    pos: Option<[f64; 3]>,
    /// Rotation `[h, p, b]` in degrees.
    rot: Option<[f64; 3]>,
    /// Scale `[x, y, z]`.
    scale: Option<[f64; 3]>,
}

pub(super) fn register(r: &mut CommandRegistry) -> Result<(), CommandError> {
    r.register_fn(
        spec(
            "object.create",
            "Create",
            true,
            || json!({"type": "forge.cube"}),
        ),
        create,
    )?;
    r.register_fn(
        spec("object.delete", "Delete", true, || json!({"nodes": [1]})),
        delete,
    )?;
    r.register_fn(
        spec(
            "object.rename",
            "Rename",
            true,
            || json!({"node": 1, "name": "Box"}),
        ),
        rename,
    )?;
    r.register_fn(
        spec(
            "object.reparent",
            "Move",
            true,
            || json!({"node": 2, "parent": 1}),
        ),
        reparent,
    )?;
    register_plain(r, "edit.undo", "Undo", false, |c| undo_redo(c, true))?;
    register_plain(r, "edit.redo", "Redo", false, |c| undo_redo(c, false))?;
    r.register_fn(
        spec(
            "view.shading",
            "Shading",
            false,
            || json!({"mode": "smooth_wire"}),
        ),
        |c, a: ShadingArgs| {
            c.set_shading(a.mode);
            Ok(Value::Null)
        },
    )?;
    register_plain(r, "view.frame_all", "Frame All", false, |c| {
        c.frame_all();
        Ok(Value::Null)
    })?;
    r.register_fn(
        spec(
            "view.toggle_expanded",
            "Expand / Collapse",
            false,
            || json!({"node": 1}),
        ),
        |c, a: NodeArgs| {
            c.toggle_expanded(NodeId(a.node));
            Ok(Value::Null)
        },
    )?;
    r.register_fn(
        spec(
            "select.mode",
            "Selection Mode",
            false,
            || json!({"mode": "points"}),
        ),
        |c, a: ModeArgs| {
            c.set_edit_mode(a.mode);
            Ok(Value::Null)
        },
    )?;
    register_plain(r, "select.all", "Select All", true, |c| {
        select(c, SelectOp::All)
    })?;
    register_plain(r, "select.none", "Deselect All", true, |c| {
        select(c, SelectOp::None)
    })?;
    register_plain(r, "select.invert", "Invert Selection", true, |c| {
        select(c, SelectOp::Invert)
    })?;
    register_plain(r, "select.grow", "Grow Selection", true, |c| {
        select(c, SelectOp::Grow)
    })?;
    register_plain(r, "select.shrink", "Shrink Selection", true, |c| {
        select(c, SelectOp::Shrink)
    })?;
    r.register_fn(
        spec("transform.set", "Set Transform", true, || {
            json!({"node": 1, "pos": [0.0, 100.0, 0.0], "rot": [90.0, 0.0, 0.0], "scale": [1.0, 1.0, 1.0]})
        }),
        transform_set,
    )?;
    Ok(())
}

fn create(ctx: &mut dyn CommandContext, a: CreateArgs) -> Result<Value, CommandError> {
    let type_id = a.type_id;
    let ty = ctx
        .doc()
        .scene()
        .registry()
        .get(&type_id)
        .cloned()
        .ok_or_else(|| CommandError::Failed(format!("unknown type {type_id}")))?;
    if ty.category == Category::Tag {
        let host = ctx
            .selection()
            .active
            .ok_or_else(|| CommandError::failed("select an object to add a tag to"))?;
        let id = ctx.doc_mut().transact(&format!("Add {}", ty.label), |tx| {
            tx.attach_tag(&type_id, host, None)
        })?;
        return Ok(json!({"node": id.0}));
    }
    // New generators come with a smoothing tag (docs/specs/SMOOTHING.md §2).
    let smooth = ty.category == Category::Generator
        && ctx
            .doc()
            .scene()
            .registry()
            .get(forge_core::smoothing::TAG)
            .is_some();
    let id = ctx
        .doc_mut()
        .transact(&format!("Create {}", ty.label), |tx| {
            let id = tx.create(&type_id, None, None)?;
            if smooth {
                tx.attach_tag(forge_core::smoothing::TAG, id, None)?;
            }
            Ok(id)
        })?;
    ctx.selection_mut().click(id);
    Ok(json!({"node": id.0}))
}

fn delete(ctx: &mut dyn CommandContext, a: NodesArgs) -> Result<Value, CommandError> {
    let chosen: BTreeSet<NodeId> = target_nodes(ctx, a.nodes).into_iter().collect();
    let scene = ctx.doc().scene();
    // Skip nodes inside another chosen node's subtree: they go with it.
    let roots: Vec<NodeId> = chosen
        .iter()
        .copied()
        .filter(|id| {
            let mut p = scene.node(*id).and_then(|n| n.parent.or(n.host));
            while let Some(pid) = p {
                if chosen.contains(&pid) {
                    return false;
                }
                p = scene.node(pid).and_then(|n| n.parent.or(n.host));
            }
            scene.contains(*id)
        })
        .collect();
    if !roots.is_empty() {
        ctx.doc_mut().transact("Delete", |tx| {
            roots.iter().try_for_each(|id| tx.delete(*id))
        })?;
    }
    ctx.selection_mut().clear();
    Ok(json!({"deleted": roots.len()}))
}

fn rename(ctx: &mut dyn CommandContext, a: RenameArgs) -> Result<Value, CommandError> {
    ctx.doc_mut()
        .transact("Rename", |tx| tx.rename(NodeId(a.node), &a.name))?;
    Ok(Value::Null)
}

fn reparent(ctx: &mut dyn CommandContext, a: ReparentArgs) -> Result<Value, CommandError> {
    ctx.doc_mut().transact("Move", |tx| {
        tx.move_node(NodeId(a.node), a.parent.map(NodeId), None)
    })?;
    Ok(Value::Null)
}

fn undo_redo(ctx: &mut dyn CommandContext, undo: bool) -> Result<Value, CommandError> {
    let done = if undo {
        ctx.doc_mut().undo()
    } else {
        ctx.doc_mut().redo()
    };
    ctx.retain_selection();
    ctx.sync_timeline();
    Ok(json!({"done": done}))
}

fn select(ctx: &mut dyn CommandContext, op: SelectOp) -> Result<Value, CommandError> {
    ctx.select_components(op).map_err(CommandError::Failed)?;
    Ok(Value::Null)
}

fn transform_set(ctx: &mut dyn CommandContext, a: TransformArgs) -> Result<Value, CommandError> {
    if a.pos.is_none() && a.rot.is_none() && a.scale.is_none() {
        return Err(CommandError::InvalidArgs(
            "transform.set needs at least one of pos, rot, scale".into(),
        ));
    }
    let id = NodeId(a.node);
    ctx.doc_mut().transact("Set Transform", |tx| {
        if let Some(v) = a.pos {
            tx.set_param(id, POS, Param::Vec3(v))?;
        }
        if let Some(v) = a.rot {
            tx.set_param(id, ROT, Param::Vec3(v.map(f64::to_radians)))?;
        }
        if let Some(v) = a.scale {
            tx.set_param(id, SCALE, Param::Vec3(v))?;
        }
        Ok(())
    })?;
    Ok(Value::Null)
}
