//! Coordinates manager maths and the `transform.edit` command (docs/specs/MANIPULATE.md §4,
//! docs/specs/UI.md §20, COMMANDS.md §6). Pure functions first, the command at the end.

use forge_command::{CommandContext, CommandError, CommandRegistry, CommandSpec};
use forge_core::transform::{self, Transform, hpb_from_rotation, rotation_part};
use forge_core::{EditError, NodeId, Scene, Tx, Value};
use glam::{DMat4, DVec3};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value as Json, json};

use crate::animate::{AnimCtx, AnimEdit, unchanged, write_param};

/// `Local` corresponds to the reference coordinate manager's "Object (Rel)".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Space {
    #[default]
    Local,
    World,
}

/// What the third row shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Third {
    #[default]
    Scale,
    Size,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Row {
    Position,
    Rotation,
    Third(Third),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CoordView {
    pub pos: DVec3,
    /// `(h, p, b)` radians.
    pub rot: DVec3,
    /// `None` for Size when the node has no mesh.
    pub third: Option<DVec3>,
}

/// Per row and axis: true when the selected objects differ (UI.md §20.2).
pub type Mixed = [[bool; 3]; 3];

fn column_lengths(m: &DMat4) -> DVec3 {
    DVec3::new(
        m.x_axis.truncate().length(),
        m.y_axis.truncate().length(),
        m.z_axis.truncate().length(),
    )
}

/// Bounding-box size of a generator node's local mesh.
pub fn mesh_size(scene: &Scene, id: NodeId) -> Option<DVec3> {
    let mesh = forge_generators::evaluate(scene, id, &forge_core::EvalCtx::default()).ok()??;
    let first = *mesh.points.first()?;
    let (lo, hi) = mesh
        .points
        .iter()
        .fold((first, first), |(lo, hi), p| (lo.min(*p), hi.max(*p)));
    Some(hi - lo)
}

/// The rows of a node with parent matrix `parent`, local transform `local` and local mesh
/// size `size`, in `space`.
pub fn view_of(
    parent: &DMat4,
    local: &Transform,
    size: Option<DVec3>,
    space: Space,
    third: Third,
) -> CoordView {
    let (pos, rot, scale) = match space {
        Space::Local => (local.pos, local.rot, local.scale),
        Space::World => {
            let world = *parent * local.matrix();
            (
                world.w_axis.truncate(),
                hpb_from_rotation(rotation_part(&world)),
                column_lengths(&world),
            )
        }
    };
    let third = match third {
        Third::Scale => Some(scale),
        Third::Size => size.map(|s| s * scale),
    };
    CoordView { pos, rot, third }
}

pub fn coord_view(scene: &Scene, id: NodeId, space: Space, third: Third) -> Option<CoordView> {
    let local = scene.local_transform(id)?;
    let parent = scene.parent_world_matrix(id)?;
    let size = (third == Third::Size)
        .then(|| mesh_size(scene, id))
        .flatten();
    Some(view_of(&parent, &local, size, space, third))
}

/// The view of several objects: the first id's values plus the axes that differ among them.
pub fn coord_view_multi(
    scene: &Scene,
    ids: &[NodeId],
    space: Space,
    third: Third,
) -> Option<(CoordView, Mixed)> {
    let views: Vec<CoordView> = ids
        .iter()
        .filter_map(|id| coord_view(scene, *id, space, third))
        .collect();
    let first = *views.first()?;
    let differs = |a: f64, b: f64| (a - b).abs() > 1e-9 * (1.0 + a.abs().max(b.abs()));
    let mut mixed: Mixed = [[false; 3]; 3];
    for v in &views[1..] {
        for k in 0..3 {
            mixed[0][k] |= differs(first.pos[k], v.pos[k]);
            mixed[1][k] |= differs(first.rot[k], v.rot[k]);
            mixed[2][k] |= match (first.third, v.third) {
                (Some(a), Some(b)) => differs(a[k], b[k]),
                (None, None) => false,
                _ => true,
            };
        }
    }
    Some((first, mixed))
}

/// The local transform that gives `row` the value `value` in `space`, starting from `start`
/// below `parent`; `size` is the local mesh bounding box (MANIPULATE.md §4).
pub fn next_transform(
    parent: &DMat4,
    start: &Transform,
    size: Option<DVec3>,
    space: Space,
    row: Row,
    value: DVec3,
) -> Transform {
    let start = *start;
    let parent_scale = column_lengths(parent);
    // Per-axis divide that keeps the current value where the divisor is zero.
    let ratio = |num: DVec3, den: DVec3, keep: DVec3| {
        DVec3::from_array(std::array::from_fn(|k| {
            if den[k].abs() > 1e-12 {
                num[k] / den[k]
            } else {
                keep[k]
            }
        }))
    };
    match (space, row) {
        (Space::Local, Row::Position) => Transform {
            pos: value,
            ..start
        },
        (Space::Local, Row::Rotation) => Transform {
            rot: value,
            ..start
        },
        (Space::Local, Row::Third(Third::Scale)) => Transform {
            scale: value,
            ..start
        },
        (Space::World, Row::Position) => Transform {
            pos: parent.inverse().transform_point3(value),
            ..start
        },
        (Space::World, Row::Rotation) => {
            let wanted = Transform {
                rot: value,
                ..Transform::IDENTITY
            }
            .rotation();
            Transform {
                rot: hpb_from_rotation(rotation_part(parent).inverse() * wanted),
                ..start
            }
        }
        (Space::World, Row::Third(Third::Scale)) => Transform {
            scale: ratio(value, parent_scale, start.scale),
            ..start
        },
        (_, Row::Third(Third::Size)) => {
            let size = size.unwrap_or(DVec3::ZERO);
            let scale = ratio(value, size, start.scale);
            let scale = if space == Space::World {
                ratio(scale, parent_scale, start.scale)
            } else {
                scale
            };
            Transform { scale, ..start }
        }
    }
}

/// Writes the three transform params. With `anim` the params that already hold the
/// shown value are skipped and animated ones get keys (docs/specs/ANIM.md §2.4); without
/// it all three are set.
pub fn write_transform(
    tx: &mut Tx,
    id: NodeId,
    t: &Transform,
    anim: Option<&AnimEdit>,
) -> Result<(), EditError> {
    for (param, v) in [
        (transform::POS, t.pos),
        (transform::ROT, t.rot),
        (transform::SCALE, t.scale),
    ] {
        let value = Value::Vec3(v.to_array());
        if unchanged(anim, id, param, &value) {
            continue;
        }
        write_param(tx, id, param, value, anim.map(|a| &a.ctx))?;
    }
    Ok(())
}

type Axes = [Option<f64>; 3];

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct TransformEditArgs {
    /// Objects to edit; absent means the current selection.
    nodes: Option<Vec<u64>>,
    /// `local` (default) or `world`.
    space: Option<String>,
    /// Position `[x, y, z]`; a `null` axis keeps the shown value.
    pos: Option<Axes>,
    /// Rotation `[h, p, b]` in degrees; `null` keeps an axis.
    rot: Option<Axes>,
    /// Scale factors; `null` keeps an axis. Not together with `size`.
    scale: Option<Axes>,
    /// Size (mesh bounding box times scale); `null` keeps an axis.
    size: Option<Axes>,
    /// Edits with the same gesture number and nodes merge into one undo step.
    gesture: Option<u64>,
}

/// Registers `transform.edit`.
pub fn register(r: &mut CommandRegistry) -> Result<(), CommandError> {
    r.register_fn(
        CommandSpec {
            id: "transform.edit",
            label: "Set Coordinates",
            undoable: true,
            sample: || json!({"nodes": [1], "space": "world", "pos": [0.0, 100.0, null], "gesture": 1}),
        },
        edit,
    )
}

fn edit(ctx: &mut dyn CommandContext, a: TransformEditArgs) -> Result<Json, CommandError> {
    let space = match a.space.as_deref() {
        None | Some("local") => Space::Local,
        Some("world") => Space::World,
        Some(o) => {
            return Err(CommandError::InvalidArgs(format!(
                "space must be local or world, not {o}"
            )));
        }
    };
    if a.scale.is_some() && a.size.is_some() {
        return Err(CommandError::InvalidArgs(
            "give scale or size, not both".into(),
        ));
    }
    if a.pos.is_none() && a.rot.is_none() && a.scale.is_none() && a.size.is_none() {
        return Err(CommandError::InvalidArgs(
            "give at least one of pos, rot, scale, size".into(),
        ));
    }
    let ids: Vec<NodeId> = match &a.nodes {
        Some(n) => n.iter().map(|i| NodeId(*i)).collect(),
        None => ctx.selection().nodes.iter().copied().collect(),
    };
    let eval = ctx.evaluated_scene();
    let third = if a.size.is_some() {
        Third::Size
    } else {
        Third::Scale
    };
    let mut results: Vec<(NodeId, Transform)> = Vec::new();
    for id in ids {
        let (Some(start), Some(parent)) = (eval.local_transform(id), eval.parent_world_matrix(id))
        else {
            continue;
        };
        let size = (third == Third::Size)
            .then(|| mesh_size(&eval, id))
            .flatten();
        let mut t = start;
        let rows = [
            (Row::Position, a.pos, 1.0),
            (Row::Rotation, a.rot, std::f64::consts::PI / 180.0),
            (Row::Third(third), a.scale.or(a.size), 1.0),
        ];
        for (row, axes, unit) in rows {
            let Some(axes) = axes else { continue };
            let shown = view_of(&parent, &t, size, space, third);
            let current = match row {
                Row::Position => Some(shown.pos),
                Row::Rotation => Some(shown.rot),
                Row::Third(_) => shown.third,
            };
            let Some(current) = current else { continue };
            let value = DVec3::from_array(std::array::from_fn(|k| match axes[k] {
                Some(x) if x.is_finite() => x * unit,
                Some(_) => f64::NAN,
                None => current[k],
            }));
            if !value.is_finite() {
                return Err(CommandError::InvalidArgs("values must be finite".into()));
            }
            t = next_transform(&parent, &t, size, space, row, value);
        }
        results.push((id, t));
    }
    if results.is_empty() {
        return Err(CommandError::Failed("no object with a transform".into()));
    }
    let anim = AnimCtx {
        frame: ctx.timeline().current,
        fps: ctx.doc().scene().settings().fps,
        autokey: ctx.autokey(),
    };
    let anim_edit = AnimEdit {
        ctx: anim,
        eval: &eval,
    };
    let n = results.len();
    let run = |tx: &mut Tx| {
        for (id, t) in &results {
            write_transform(tx, *id, t, Some(&anim_edit))?;
        }
        Ok(())
    };
    match a.gesture {
        Some(g) => {
            let ids = results
                .iter()
                .map(|(i, _)| i.to_string())
                .collect::<Vec<_>>()
                .join(",");
            ctx.doc_mut()
                .transact_merged("Set Coordinates", &format!("coords:{ids}:{g}"), run)?;
        }
        None => ctx.doc_mut().transact("Set Coordinates", run)?,
    }
    Ok(json!({"changed": n}))
}
