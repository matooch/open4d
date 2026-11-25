//! `weights.*`: the weight tools of forge-rig as editor commands (docs/specs/RIG.md section 9).

use forge_command::{CommandContext, CommandError, CommandRegistry, CommandSpec};
use forge_core::{EvalCtx, NodeId};
use forge_geometry::PolyMesh;
use forge_rig::mirror::{Plane, RenameRule, Side, mirror_tag_weights};
use forge_rig::paint::{Brush, BrushMode, Falloff, point_adjacency, smooth_weights, stroke};
use forge_rig::weights::{find_weight_tag, read_weights, tag_joints};
use glam::DVec3;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};

#[derive(Deserialize, JsonSchema, Clone, Copy, Default, PartialEq)]
#[serde(rename_all = "snake_case")]
enum ModeArg {
    #[default]
    Add,
    Subtract,
    Set,
    Smooth,
}

#[derive(Deserialize, JsonSchema, Clone, Copy, Default, PartialEq)]
#[serde(rename_all = "snake_case")]
enum FalloffArg {
    Constant,
    Linear,
    #[default]
    Smooth,
}

#[derive(Deserialize, JsonSchema, Clone, Copy, Default, PartialEq)]
#[serde(rename_all = "snake_case")]
enum PlaneArg {
    #[default]
    Yz,
    Xz,
    Xy,
}

#[derive(Deserialize, JsonSchema, Clone, Copy, Default, PartialEq)]
#[serde(rename_all = "snake_case")]
enum FromArg {
    #[default]
    Positive,
    Negative,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct PaintArgs {
    /// The skinned object (default: the active node).
    object: Option<u64>,
    /// A joint of the weight tag's list.
    joint: u64,
    /// Stroke centre in object space.
    center: [f64; 3],
    radius: f64,
    /// 0 to 1 (default 1).
    strength: Option<f64>,
    mode: Option<ModeArg>,
    falloff: Option<FalloffArg>,
    /// Value `set` moves towards, 0 to 1 (default 1).
    target: Option<f64>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct ObjectArgs {
    /// The skinned object (default: the active node).
    object: Option<u64>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct PruneArgs {
    object: Option<u64>,
    /// Weights below this are dropped (default 0.01).
    threshold: Option<f64>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct SmoothArgs {
    object: Option<u64>,
    /// Default 1.
    iterations: Option<usize>,
    /// 0 to 1 (default 0.5).
    factor: Option<f64>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct MirrorArgs {
    object: Option<u64>,
    plane: Option<PlaneArg>,
    from: Option<FromArg>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct AutoBindArgs {
    /// The mesh object (default: the active node). It gets a weight tag and a skin deformer
    /// when it has none.
    object: Option<u64>,
    /// The joints to bind to, in slot order.
    joints: Vec<u64>,
}

/// Registers the weight commands.
pub fn register(r: &mut CommandRegistry) -> Result<(), CommandError> {
    r.register_fn(
        CommandSpec {
            id: "weights.auto_bind",
            label: "Auto Bind",
            undoable: true,
            sample: || json!({"joints": [1, 2, 3]}),
        },
        auto_bind_command,
    )?;
    r.register_fn(
        CommandSpec {
            id: "weights.paint",
            label: "Paint Weights",
            undoable: true,
            sample: || json!({"joint": 1, "center": [0.0, 0.0, 0.0], "radius": 50.0}),
        },
        paint,
    )?;
    r.register_fn(
        CommandSpec {
            id: "weights.normalise",
            label: "Normalise Weights",
            undoable: true,
            sample: || json!({}),
        },
        normalise,
    )?;
    r.register_fn(
        CommandSpec {
            id: "weights.prune",
            label: "Prune Weights",
            undoable: true,
            sample: || json!({"threshold": 0.01}),
        },
        prune,
    )?;
    r.register_fn(
        CommandSpec {
            id: "weights.smooth",
            label: "Smooth Weights",
            undoable: true,
            sample: || json!({"iterations": 1, "factor": 0.5}),
        },
        smooth,
    )?;
    r.register_fn(
        CommandSpec {
            id: "weights.mirror",
            label: "Mirror Weights",
            undoable: true,
            sample: || json!({"plane": "yz", "from": "positive"}),
        },
        mirror,
    )
}

/// The object and its weight tag.
fn target(ctx: &dyn CommandContext, object: Option<u64>) -> Result<(NodeId, NodeId), CommandError> {
    let obj = match object {
        Some(o) => NodeId(o),
        None => ctx
            .selection()
            .active
            .ok_or_else(|| CommandError::failed("select a skinned object"))?,
    };
    let scene = ctx.doc().scene();
    if !scene.contains(obj) {
        return Err(CommandError::failed(format!("unknown node {}", obj.0)));
    }
    let tag = find_weight_tag(scene, obj)
        .ok_or_else(|| CommandError::failed(format!("node {} has no weight tag", obj.0)))?;
    Ok((obj, tag))
}

fn mesh_of(ctx: &dyn CommandContext, obj: NodeId) -> Result<PolyMesh, CommandError> {
    forge_generators::evaluate(ctx.doc().scene(), obj, &EvalCtx::default())
        .map_err(|e| CommandError::failed(e.to_string()))?
        .ok_or_else(|| CommandError::failed("the object has no polygon geometry"))
}

fn finite(name: &str, v: f64) -> Result<(), CommandError> {
    if v.is_finite() {
        Ok(())
    } else {
        Err(CommandError::InvalidArgs(format!("{name} must be finite")))
    }
}

fn paint(ctx: &mut dyn CommandContext, a: PaintArgs) -> Result<Value, CommandError> {
    finite("radius", a.radius)?;
    if a.radius <= 0.0 {
        return Err(CommandError::InvalidArgs("radius must be positive".into()));
    }
    for c in a.center {
        finite("center", c)?;
    }
    let (obj, tag) = target(ctx, a.object)?;
    let joint = NodeId(a.joint);
    let slot = tag_joints(ctx.doc().scene(), tag)
        .iter()
        .position(|j| *j == joint)
        .ok_or_else(|| CommandError::failed(format!("joint {} is not in the tag", a.joint)))?;
    let mesh = mesh_of(ctx, obj)?;
    let adj = point_adjacency(&mesh.polys, mesh.points.len());
    let brush = Brush {
        mode: match a.mode.unwrap_or_default() {
            ModeArg::Add => BrushMode::Add,
            ModeArg::Subtract => BrushMode::Subtract,
            ModeArg::Set => BrushMode::Set,
            ModeArg::Smooth => BrushMode::Smooth,
        },
        radius: a.radius,
        strength: a.strength.unwrap_or(1.0).clamp(0.0, 1.0),
        falloff: match a.falloff.unwrap_or_default() {
            FalloffArg::Constant => Falloff::Constant,
            FalloffArg::Linear => Falloff::Linear,
            FalloffArg::Smooth => Falloff::Smooth,
        },
        target: a.target.unwrap_or(1.0).clamp(0.0, 1.0),
    };
    let mut weights = read_weights(ctx.doc().scene(), tag);
    let n = stroke(
        &mut weights,
        &mesh.points,
        &adj,
        slot as u32,
        DVec3::from_array(a.center),
        &brush,
    );
    ctx.doc_mut().transact("Paint Weights", |tx| {
        tx.set_param(tag, "weights", weights.to_value())
    })?;
    Ok(json!({"points": n}))
}

fn normalise(ctx: &mut dyn CommandContext, a: ObjectArgs) -> Result<Value, CommandError> {
    let (_, tag) = target(ctx, a.object)?;
    let mut w = read_weights(ctx.doc().scene(), tag);
    w.normalise();
    ctx.doc_mut().transact("Normalise Weights", |tx| {
        tx.set_param(tag, "weights", w.to_value())
    })?;
    Ok(json!({}))
}

fn prune(ctx: &mut dyn CommandContext, a: PruneArgs) -> Result<Value, CommandError> {
    let threshold = a.threshold.unwrap_or(0.01);
    finite("threshold", threshold)?;
    let (_, tag) = target(ctx, a.object)?;
    let mut w = read_weights(ctx.doc().scene(), tag);
    w.prune(threshold);
    ctx.doc_mut().transact("Prune Weights", |tx| {
        tx.set_param(tag, "weights", w.to_value())
    })?;
    Ok(json!({}))
}

fn smooth(ctx: &mut dyn CommandContext, a: SmoothArgs) -> Result<Value, CommandError> {
    let factor = a.factor.unwrap_or(0.5);
    finite("factor", factor)?;
    let (obj, tag) = target(ctx, a.object)?;
    let mesh = mesh_of(ctx, obj)?;
    let adj = point_adjacency(&mesh.polys, mesh.points.len());
    let mut w = read_weights(ctx.doc().scene(), tag);
    smooth_weights(&mut w, &adj, a.iterations.unwrap_or(1), factor);
    ctx.doc_mut().transact("Smooth Weights", |tx| {
        tx.set_param(tag, "weights", w.to_value())
    })?;
    Ok(json!({}))
}

fn mirror(ctx: &mut dyn CommandContext, a: MirrorArgs) -> Result<Value, CommandError> {
    let (obj, tag) = target(ctx, a.object)?;
    let mesh = mesh_of(ctx, obj)?;
    let plane = match a.plane.unwrap_or_default() {
        PlaneArg::Yz => Plane::Yz,
        PlaneArg::Xz => Plane::Xz,
        PlaneArg::Xy => Plane::Xy,
    };
    let from = match a.from.unwrap_or_default() {
        FromArg::Positive => Side::Positive,
        FromArg::Negative => Side::Negative,
    };
    let report = ctx.doc_mut().transact("Mirror Weights", |tx| {
        mirror_tag_weights(tx, tag, &mesh, plane, &RenameRule::LeftRight, from)
    })?;
    Ok(json!({"copied": report.copied, "unmatched": report.unmatched}))
}

/// `weights.auto_bind` (docs/specs/RIG.md section 4): bone heat weights for the given joints on
/// the object's rest mesh, the current joint pose stored as the bind pose, one undo step.
fn auto_bind_command(ctx: &mut dyn CommandContext, a: AutoBindArgs) -> Result<Value, CommandError> {
    if a.joints.is_empty() {
        return Err(CommandError::InvalidArgs(
            "joints needs at least one joint".into(),
        ));
    }
    let obj = match a.object {
        Some(o) => NodeId(o),
        None => ctx
            .selection()
            .active
            .ok_or_else(|| CommandError::failed("select a mesh object"))?,
    };
    let scene = ctx.doc().scene();
    if !scene.contains(obj) {
        return Err(CommandError::failed(format!("unknown node {}", obj.0)));
    }
    let joints: Vec<NodeId> = a.joints.iter().map(|j| NodeId(*j)).collect();
    if let Some(bad) = joints
        .iter()
        .find(|j| !forge_rig::joint::is_joint(scene, **j))
    {
        return Err(CommandError::failed(format!(
            "node {} is not a joint",
            bad.0
        )));
    }
    let has_skin = scene.node(obj).is_some_and(|n| {
        n.children
            .iter()
            .any(|c| scene.node(*c).is_some_and(|c| c.type_id == forge_rig::SKIN))
    });
    let existing = find_weight_tag(scene, obj);
    let mesh = mesh_of(ctx, obj)?;
    let tag = ctx.doc_mut().transact("Auto Bind", |tx| {
        let tag = match existing {
            Some(t) => t,
            None => tx.attach_tag(forge_rig::WEIGHT_TAG, obj, None)?,
        };
        tx.set_param(tag, "joints", forge_core::Value::LinkList(joints.clone()))?;
        forge_rig::auto_weights::auto_bind(
            tx,
            tag,
            &mesh,
            &forge_rig::auto_weights::AutoWeightOptions::default(),
        )?;
        if !has_skin {
            tx.create(forge_rig::SKIN, Some(obj), None)?;
        }
        Ok(tag)
    })?;
    Ok(json!({"tag": tag.0}))
}
