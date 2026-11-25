//! `joint.create_chain`, `joint.align` and `joint.mirror`: the joint tools of forge-rig as
//! editor commands (docs/specs/RIG.md section 7). They live here because forge-rig cannot
//! depend on forge-command.

use forge_command::{CommandContext, CommandError, CommandRegistry, CommandSpec};
use forge_core::NodeId;
use forge_rig::Axis;
use forge_rig::joint::is_joint;
use forge_rig::mirror::{Plane, RenameRule, mirror_chain};
use forge_rig::tools::{align_joint, create_chain};
use glam::DVec3;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};

#[derive(Deserialize, JsonSchema, Clone, Copy, PartialEq)]
#[serde(rename_all = "snake_case")]
enum AxisArg {
    X,
    Y,
    Z,
}

impl AxisArg {
    fn axis(self) -> Axis {
        match self {
            AxisArg::X => Axis::X,
            AxisArg::Y => Axis::Y,
            AxisArg::Z => Axis::Z,
        }
    }
}

#[derive(Deserialize, JsonSchema, Clone, Copy, Default, PartialEq)]
#[serde(rename_all = "snake_case")]
enum PlaneArg {
    /// Reflects x.
    #[default]
    Yz,
    /// Reflects y.
    Xz,
    /// Reflects z.
    Xy,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct ChainArgs {
    /// Joint positions `[x, y, z]` in the parent's space; at least one.
    points: Vec<[f64; 3]>,
    /// Parent of the first joint (the scene root when absent).
    parent: Option<u64>,
    /// Name prefix; the joints are `prefix1`, `prefix2`, ... (default `Joint`).
    prefix: Option<String>,
    /// Turn this local axis of every joint but the last to its child.
    align: Option<AxisArg>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct AlignArgs {
    /// Joints; the current selection when absent.
    nodes: Option<Vec<u64>>,
    /// The local axis that points at the child joint (default `z`).
    axis: Option<AxisArg>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct MirrorArgs {
    /// Root joints of the chains to mirror; the current selection when absent.
    nodes: Option<Vec<u64>>,
    /// Mirror plane through the world origin (default `yz`).
    plane: Option<PlaneArg>,
    /// Append this suffix to the names instead of swapping left and right.
    suffix: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct AttachArgs {
    /// Tag type id, e.g. `forge.ik_tag`, `forge.weight_tag`, `forge.pose_morph_tag`.
    #[serde(rename = "type")]
    type_id: String,
    /// The object that gets the tag.
    node: u64,
}

/// Registers the joint commands.
pub fn register(r: &mut CommandRegistry) -> Result<(), CommandError> {
    r.register_fn(
        CommandSpec {
            id: "tag.attach",
            label: "Attach Tag",
            undoable: true,
            sample: || json!({"type": "forge.ik_tag", "node": 1}),
        },
        attach,
    )?;
    r.register_fn(
        CommandSpec {
            id: "joint.create_chain",
            label: "Create Joint Chain",
            undoable: true,
            sample: || json!({"points": [[0.0, 0.0, 0.0], [0.0, 100.0, 0.0]], "align": "y"}),
        },
        create,
    )?;
    r.register_fn(
        CommandSpec {
            id: "joint.align",
            label: "Align Joints",
            undoable: true,
            sample: || json!({"axis": "z"}),
        },
        align,
    )?;
    r.register_fn(
        CommandSpec {
            id: "joint.mirror",
            label: "Mirror Joints",
            undoable: true,
            sample: || json!({"plane": "yz"}),
        },
        mirror,
    )
}

fn joints_of(
    ctx: &dyn CommandContext,
    nodes: Option<Vec<u64>>,
) -> Result<Vec<NodeId>, CommandError> {
    let ids: Vec<NodeId> = match nodes {
        Some(n) => n.into_iter().map(NodeId).collect(),
        None => ctx.selection().nodes.iter().copied().collect(),
    };
    if ids.is_empty() {
        return Err(CommandError::failed("select a joint"));
    }
    let scene = ctx.doc().scene();
    if let Some(bad) = ids.iter().find(|i| !is_joint(scene, **i)) {
        return Err(CommandError::failed(format!(
            "node {} is not a joint",
            bad.0
        )));
    }
    Ok(ids)
}

fn ids_json(ids: &[NodeId]) -> Vec<u64> {
    ids.iter().map(|i| i.0).collect()
}

fn create(ctx: &mut dyn CommandContext, a: ChainArgs) -> Result<Value, CommandError> {
    if a.points.is_empty() || a.points.iter().flatten().any(|v| !v.is_finite()) {
        return Err(CommandError::InvalidArgs(
            "points needs at least one finite [x, y, z]".into(),
        ));
    }
    let parent = a.parent.map(NodeId);
    if let Some(p) = parent
        && !ctx.doc().scene().contains(p)
    {
        return Err(CommandError::failed(format!("unknown node {}", p.0)));
    }
    let points: Vec<DVec3> = a.points.iter().map(|p| DVec3::from_array(*p)).collect();
    let prefix = a.prefix.unwrap_or_else(|| "Joint".into());
    let align = a.align.map(AxisArg::axis);
    let ids = ctx.doc_mut().transact("Create Joint Chain", |tx| {
        create_chain(tx, parent, &points, &prefix, align)
    })?;
    if let Some(root) = ids.first() {
        ctx.selection_mut().click(*root);
    }
    Ok(json!({"nodes": ids_json(&ids)}))
}

fn align(ctx: &mut dyn CommandContext, a: AlignArgs) -> Result<Value, CommandError> {
    let ids = joints_of(ctx, a.nodes)?;
    let axis = a.axis.map_or(Axis::Z, AxisArg::axis);
    // parents first: aligning a parent moves nothing that was already aligned below it
    let scene = ctx.doc().scene();
    let mut ordered = ids.clone();
    ordered.sort_by_key(|i| {
        let mut depth = 0;
        let mut at = scene.node(*i).and_then(|n| n.parent);
        while let Some(p) = at {
            depth += 1;
            at = scene.node(p).and_then(|n| n.parent);
        }
        depth
    });
    ctx.doc_mut().transact("Align Joints", |tx| {
        for id in &ordered {
            align_joint(tx, *id, axis)?;
        }
        Ok(())
    })?;
    Ok(json!({"nodes": ids_json(&ids)}))
}

fn mirror(ctx: &mut dyn CommandContext, a: MirrorArgs) -> Result<Value, CommandError> {
    let ids = joints_of(ctx, a.nodes)?;
    let plane = match a.plane.unwrap_or_default() {
        PlaneArg::Yz => Plane::Yz,
        PlaneArg::Xz => Plane::Xz,
        PlaneArg::Xy => Plane::Xy,
    };
    let rule = match a.suffix {
        Some(s) => RenameRule::Suffix(s),
        None => RenameRule::LeftRight,
    };
    // a joint under another listed joint is mirrored with its chain, not twice
    let scene = ctx.doc().scene();
    let roots: Vec<NodeId> = ids
        .iter()
        .copied()
        .filter(|i| {
            let mut at = scene.node(*i).and_then(|n| n.parent);
            while let Some(p) = at {
                if ids.contains(&p) {
                    return false;
                }
                at = scene.node(p).and_then(|n| n.parent);
            }
            true
        })
        .collect();
    let made = ctx.doc_mut().transact("Mirror Joints", |tx| {
        roots
            .iter()
            .map(|r| mirror_chain(tx, *r, plane, &rule))
            .collect::<Result<Vec<_>, _>>()
    })?;
    let sel = ctx.selection_mut();
    sel.clear();
    for id in &made {
        sel.nodes.insert(*id);
    }
    sel.active = made.last().copied();
    Ok(json!({"nodes": ids_json(&made)}))
}

/// `tag.attach`: a tag of any registered tag type on a named object (unlike `object.create`,
/// which uses the selection).
fn attach(ctx: &mut dyn CommandContext, a: AttachArgs) -> Result<Value, CommandError> {
    let node = NodeId(a.node);
    if !ctx.doc().scene().contains(node) {
        return Err(CommandError::failed(format!("unknown node {}", a.node)));
    }
    let tag = ctx
        .doc_mut()
        .transact("Attach Tag", |tx| tx.attach_tag(&a.type_id, node, None))?;
    Ok(json!({"node": tag.0}))
}
