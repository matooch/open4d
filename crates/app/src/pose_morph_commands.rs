//! `pose_morph.*`: making and editing Pose Morph targets as editor commands
//! (docs/specs/RIG.md section 15.4). Strengths are ordinary parameters (`param.set`, keys).

use forge_command::{CommandContext, CommandError, CommandRegistry, CommandSpec};
use forge_core::{NodeId, Value};
use forge_rig::pose_morph::{self, POSE_MORPH_TAG, find_pose_morph_tag, has_base};
use glam::DVec3;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value as Json, json};

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct AddArgs {
    /// The polygon object (default: the active node). Its Pose Morph tag is created when missing.
    object: Option<u64>,
    /// Name of the target (default `Morph N`).
    name: Option<String>,
    /// Point positions of the target; the object's current points when absent.
    points: Option<Vec<[f64; 3]>>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct UpdateArgs {
    /// The Pose Morph tag.
    tag: u64,
    /// The target slot (0-based).
    slot: usize,
    /// New point positions; the host object's current points when absent.
    points: Option<Vec<[f64; 3]>>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct SlotArgs {
    /// The Pose Morph tag.
    tag: u64,
    /// The target slot (0-based).
    slot: usize,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct BaseArgs {
    /// The Pose Morph tag.
    tag: u64,
    /// New base pose; the host object's current points when absent.
    points: Option<Vec<[f64; 3]>>,
}

/// Registers the Pose Morph commands.
pub fn register(r: &mut CommandRegistry) -> Result<(), CommandError> {
    r.register_fn(
        CommandSpec {
            id: "pose_morph.add_target",
            label: "Add Morph Target",
            undoable: true,
            sample: || json!({"name": "Smile"}),
        },
        add_target,
    )?;
    r.register_fn(
        CommandSpec {
            id: "pose_morph.update_target",
            label: "Update Morph Target",
            undoable: true,
            sample: || json!({"tag": 1, "slot": 0}),
        },
        update_target,
    )?;
    r.register_fn(
        CommandSpec {
            id: "pose_morph.clear_target",
            label: "Clear Morph Target",
            undoable: true,
            sample: || json!({"tag": 1, "slot": 0}),
        },
        clear_target,
    )?;
    r.register_fn(
        CommandSpec {
            id: "pose_morph.set_base",
            label: "Set Morph Base Pose",
            undoable: true,
            sample: || json!({"tag": 1}),
        },
        set_base,
    )
}

/// The stored points of a polygon object.
fn own_points(ctx: &dyn CommandContext, object: NodeId) -> Result<Vec<DVec3>, CommandError> {
    match ctx.doc().scene().param(object, "mesh") {
        Some(Value::Mesh(m)) => Ok(m.points.iter().map(|p| DVec3::from_array(*p)).collect()),
        _ => Err(CommandError::failed("select a polygon object")),
    }
}

fn given(points: Vec<[f64; 3]>) -> Result<Vec<DVec3>, CommandError> {
    if points.iter().flatten().any(|c| !c.is_finite()) {
        return Err(CommandError::InvalidArgs("points must be finite".into()));
    }
    Ok(points.into_iter().map(DVec3::from_array).collect())
}

/// The tag and its host object.
fn tag_and_host(ctx: &dyn CommandContext, tag: u64) -> Result<(NodeId, NodeId), CommandError> {
    let tag = NodeId(tag);
    let node = ctx
        .doc()
        .scene()
        .node(tag)
        .filter(|n| n.type_id == POSE_MORPH_TAG)
        .ok_or_else(|| CommandError::failed(format!("node {} is not a Pose Morph tag", tag.0)))?;
    let host = node
        .host
        .ok_or_else(|| CommandError::failed("the tag has no host"))?;
    Ok((tag, host))
}

fn add_target(ctx: &mut dyn CommandContext, a: AddArgs) -> Result<Json, CommandError> {
    let object = match a.object {
        Some(o) => NodeId(o),
        None => ctx
            .selection()
            .active
            .ok_or_else(|| CommandError::failed("select a polygon object"))?,
    };
    if !ctx.doc().scene().contains(object) {
        return Err(CommandError::failed(format!("unknown node {}", object.0)));
    }
    let own = own_points(ctx, object)?;
    let points = match a.points {
        Some(p) => given(p)?,
        None => own.clone(),
    };
    let existing = find_pose_morph_tag(ctx.doc().scene(), object);
    let (tag, slot) = ctx.doc_mut().transact("Add Morph Target", |tx| {
        let tag = match existing {
            Some(t) => t,
            None => tx.attach_tag(POSE_MORPH_TAG, object, None)?,
        };
        if !has_base(tx.scene(), tag) {
            pose_morph::capture_base(tx, tag, &own)?;
        }
        let slot = pose_morph::add_target(tx, tag, a.name.as_deref(), &points)?;
        Ok((tag, slot))
    })?;
    Ok(json!({"tag": tag.0, "slot": slot}))
}

fn update_target(ctx: &mut dyn CommandContext, a: UpdateArgs) -> Result<Json, CommandError> {
    let (tag, host) = tag_and_host(ctx, a.tag)?;
    let points = match a.points {
        Some(p) => given(p)?,
        None => own_points(ctx, host)?,
    };
    ctx.doc_mut().transact("Update Morph Target", |tx| {
        pose_morph::set_target(tx, tag, a.slot, &points)
    })?;
    Ok(json!({}))
}

fn clear_target(ctx: &mut dyn CommandContext, a: SlotArgs) -> Result<Json, CommandError> {
    let (tag, _) = tag_and_host(ctx, a.tag)?;
    ctx.doc_mut().transact("Clear Morph Target", |tx| {
        pose_morph::clear_target(tx, tag, a.slot)
    })?;
    Ok(json!({}))
}

fn set_base(ctx: &mut dyn CommandContext, a: BaseArgs) -> Result<Json, CommandError> {
    let (tag, host) = tag_and_host(ctx, a.tag)?;
    let points = match a.points {
        Some(p) => given(p)?,
        None => own_points(ctx, host)?,
    };
    ctx.doc_mut().transact("Set Morph Base Pose", |tx| {
        pose_morph::capture_base(tx, tag, &points)
    })?;
    Ok(json!({}))
}
