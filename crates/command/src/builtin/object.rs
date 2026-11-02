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
use crate::state::{
    CameraSet, ConvertMode, GizmoModeArg, GizmoSpaceArg, LayoutArg, LightingArg, ModeArg, PaneArg,
    SelectExtra, SelectOp, ShadingMode, UiRequest, ViewArg,
};

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
struct MoveArgs {
    /// Nodes to move; the current selection when absent.
    nodes: Option<Vec<u64>>,
    /// New parent; null or absent moves to the root level.
    parent: Option<u64>,
    /// Position among the parent's children without the moved nodes; absent = last.
    index: Option<usize>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct GroupArgs {
    /// Nodes to group; the current selection when absent.
    nodes: Option<Vec<u64>>,
    /// Name of the new group (default "Null").
    name: Option<String>,
}

/// A visibility setting.
#[derive(Deserialize, JsonSchema, Clone, Copy)]
#[serde(rename_all = "snake_case")]
enum VisArg {
    /// Follow the parent.
    Default,
    On,
    Off,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct VisibilityArgs {
    /// Nodes to change; the current selection when absent.
    nodes: Option<Vec<u64>>,
    /// Editor (viewport) visibility.
    editor: Option<VisArg>,
    /// Render visibility.
    render: Option<VisArg>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct EnableArgs {
    /// Nodes to change; the current selection when absent.
    nodes: Option<Vec<u64>>,
    enabled: bool,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct ShadingArgs {
    mode: ShadingMode,
    /// Viewport pane; absent = the active pane.
    pane: Option<PaneArg>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct ViewArgs {
    view: ViewArg,
    /// Viewport pane; absent = the active pane.
    pane: Option<PaneArg>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct LayoutArgs {
    /// `single` or `four`; absent toggles.
    layout: Option<LayoutArg>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct MaximizeArgs {
    /// Viewport pane; absent = the active pane.
    pane: Option<PaneArg>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct LightingArgs {
    mode: LightingArg,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct CloneArgs {
    /// Objects that become the cloner's children; the current selection when absent.
    nodes: Option<Vec<u64>>,
    /// Name of the new cloner (default: the type's name).
    name: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct CloneDisplayArgs {
    /// Show the index of each clone of the selected cloners.
    indices: Option<bool>,
    /// Show the axes of each clone of the selected cloners.
    axes: Option<bool>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct EnabledArgs {
    enabled: bool,
}

/// Checks the numbers of `view.set_camera` (docs/specs/COMMANDS.md §6).
fn check_camera(a: &CameraSet) -> Result<(), CommandError> {
    let finite = |name: &str, v: Option<f64>| match v {
        Some(v) if !v.is_finite() => Err(CommandError::Failed(format!("{name} must be finite"))),
        _ => Ok(()),
    };
    for (name, v) in [("distance", a.distance), ("yaw", a.yaw), ("pitch", a.pitch)] {
        finite(name, v)?;
    }
    finite("fov", a.fov)?;
    finite("height", a.height)?;
    if let Some(t) = a.target
        && t.iter().any(|v| !v.is_finite())
    {
        return Err(CommandError::Failed("target must be finite".into()));
    }
    if a.distance.is_some_and(|d| d <= 0.0) {
        return Err(CommandError::Failed(
            "distance must be greater than 0".into(),
        ));
    }
    if a.height.is_some_and(|h| h <= 0.0) {
        return Err(CommandError::Failed("height must be greater than 0".into()));
    }
    if a.fov.is_some_and(|f| !(1.0..170.0).contains(&f)) {
        return Err(CommandError::Failed(
            "fov must be between 1 and 170 degrees".into(),
        ));
    }
    Ok(())
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct GizmoModeArgs {
    mode: GizmoModeArg,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct GizmoSpaceArgs {
    /// `object` or `world`; absent toggles.
    space: Option<GizmoSpaceArg>,
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
struct ConvertArgs {
    /// Element kind to convert to: points, edges or polys (not model).
    to: ModeArg,
    /// `strict` (default): all parts selected; `tolerant`: any part.
    #[serde(default)]
    mode: ConvertMode,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct AcrossArgs {
    /// Polygons only: which edge pair of a quad the strip crosses (0 or 1, default 0).
    #[serde(default)]
    across: u8,
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
    r.register_fn(
        spec(
            "object.move",
            "Move Objects",
            true,
            || json!({"nodes": [2, 3], "parent": 1, "index": 0}),
        ),
        move_nodes,
    )?;
    r.register_fn(
        spec(
            "object.group",
            "Group Objects",
            true,
            || json!({"nodes": [2, 3], "name": "Group"}),
        ),
        group,
    )?;
    r.register_fn(
        spec(
            "object.create_cloner",
            "Create Cloner from Selection",
            true,
            || json!({"nodes": [2, 3], "name": "Cloner"}),
        ),
        create_cloner,
    )?;
    r.register_fn(
        spec(
            "view.clone_display",
            "Clone Display",
            false,
            || json!({"indices": true, "axes": false}),
        ),
        |c, a: CloneDisplayArgs| {
            if a.indices.is_none() && a.axes.is_none() {
                return Err(CommandError::InvalidArgs(
                    "view.clone_display needs `indices` and/or `axes`".into(),
                ));
            }
            c.set_clone_display(a.indices, a.axes);
            Ok(Value::Null)
        },
    )?;
    r.register_fn(
        spec(
            "object.visibility",
            "Visibility",
            true,
            || json!({"nodes": [1], "editor": "off", "render": "default"}),
        ),
        visibility,
    )?;
    r.register_fn(
        spec(
            "object.enable",
            "Enable",
            true,
            || json!({"nodes": [1], "enabled": false}),
        ),
        enable,
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
            c.set_shading(a.mode, a.pane);
            Ok(Value::Null)
        },
    )?;
    r.register_fn(
        spec(
            "view.layout",
            "Viewport Layout",
            false,
            || json!({"layout": "four"}),
        ),
        |c, a: LayoutArgs| {
            c.set_layout(a.layout);
            Ok(Value::Null)
        },
    )?;
    r.register_fn(
        spec(
            "view.maximize",
            "Maximize Pane",
            false,
            || json!({"pane": "top"}),
        ),
        |c, a: MaximizeArgs| {
            c.maximize(a.pane);
            Ok(Value::Null)
        },
    )?;
    r.register_fn(
        spec(
            "view.lighting",
            "Viewport Lighting",
            false,
            || json!({"mode": "rig"}),
        ),
        |c, a: LightingArgs| {
            c.set_lighting(a.mode);
            Ok(Value::Null)
        },
    )?;
    r.register_fn(
        spec(
            "view.shadows",
            "Viewport Shadows",
            false,
            || json!({"enabled": true}),
        ),
        |c, a: EnabledArgs| {
            c.set_shadows(a.enabled);
            Ok(Value::Null)
        },
    )?;
    r.register_fn(
        spec(
            "view.antialias",
            "Viewport Anti-aliasing",
            false,
            || json!({"enabled": true}),
        ),
        |c, a: EnabledArgs| {
            c.set_antialias(a.enabled);
            Ok(Value::Null)
        },
    )?;
    r.register_fn(
        spec(
            "view.gizmo_mode",
            "Gizmo Mode",
            false,
            || json!({"mode": "move"}),
        ),
        |c, a: GizmoModeArgs| {
            c.set_gizmo_mode(a.mode);
            Ok(Value::Null)
        },
    )?;
    r.register_fn(
        spec(
            "view.gizmo_space",
            "Gizmo Space",
            false,
            || json!({"space": "world"}),
        ),
        |c, a: GizmoSpaceArgs| {
            c.set_gizmo_space(a.space);
            Ok(Value::Null)
        },
    )?;
    register_plain(r, "object.begin_rename", "Rename Active", false, |c| {
        c.request_ui(UiRequest::RenameActive)
            .map_err(CommandError::Failed)?;
        Ok(Value::Null)
    })?;
    register_plain(r, "view.frame_all", "Frame All", false, |c| {
        c.frame_all();
        Ok(Value::Null)
    })?;
    register_plain(r, "view.frame_selected", "Frame Selected", false, |c| {
        c.frame_selected();
        Ok(Value::Null)
    })?;
    r.register_fn(
        spec(
            "view.set_view",
            "Set View",
            false,
            || json!({"view": "top"}),
        ),
        |c, a: ViewArgs| {
            c.set_view(a.view, a.pane);
            Ok(Value::Null)
        },
    )?;
    r.register_fn(
        spec(
            "view.set_camera",
            "Set Camera",
            false,
            || json!({"target": [0.0, 0.0, 0.0], "distance": 500.0, "yaw": 30.0, "pitch": 20.0}),
        ),
        |c, a: CameraSet| {
            check_camera(&a)?;
            c.set_camera(a);
            Ok(Value::Null)
        },
    )?;
    register_plain(r, "view.get_camera", "Get Camera", false, |c| {
        Ok(c.camera_info())
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
    register_plain(r, "select.connected", "Select Connected", true, |c| {
        select_extra(c, SelectExtra::Connected)
    })?;
    r.register_fn(
        spec(
            "select.convert",
            "Convert Selection",
            true,
            || json!({"to": "polys", "mode": "strict"}),
        ),
        |c, a: ConvertArgs| {
            if a.to == ModeArg::Model {
                return Err(CommandError::InvalidArgs(
                    "select.convert needs `to` points, edges or polys".into(),
                ));
            }
            select_extra(
                c,
                SelectExtra::Convert {
                    to: a.to,
                    mode: a.mode,
                },
            )
        },
    )?;
    r.register_fn(
        spec(
            "select.loop",
            "Loop Selection",
            true,
            || json!({"across": 0}),
        ),
        |c, a: AcrossArgs| select_extra(c, SelectExtra::Loop { across: a.across }),
    )?;
    r.register_fn(
        spec(
            "select.ring",
            "Ring Selection",
            true,
            || json!({"across": 0}),
        ),
        |c, a: AcrossArgs| select_extra(c, SelectExtra::Ring { across: a.across }),
    )?;
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

/// The type id of the cloner (the command crate does not depend on the clone engine).
const CLONER: &str = "forge.cloner";

/// `object.create_cloner` (docs/specs/UI.md §16.2): a cloner where the first chosen object
/// was, the chosen objects as its children; chosen effectors are linked in its effector list
/// instead (they are not prototypes). One undo step.
fn create_cloner(ctx: &mut dyn CommandContext, a: CloneArgs) -> Result<Value, CommandError> {
    let ids = target_nodes(ctx, a.nodes);
    let scene = ctx.doc().scene();
    if scene.registry().get(CLONER).is_none() {
        return Err(CommandError::failed("the clone toolset is not installed"));
    }
    let category = |id: NodeId| scene.node_type(id).map(|t| t.category);
    let effectors: Vec<NodeId> = top_level(scene, &ids)
        .into_iter()
        .filter(|id| category(*id) == Some(Category::Effector))
        .collect();
    let nodes: Vec<NodeId> = top_level(scene, &ids)
        .into_iter()
        .filter(|id| !matches!(category(*id), Some(Category::Effector | Category::Field)))
        .collect();
    let (parent, index) = match nodes.first().copied() {
        Some(first) => {
            let parent = scene.node(first).and_then(|n| n.parent);
            let index = siblings(scene, parent).iter().position(|s| *s == first);
            (parent, index)
        }
        None => (None, None),
    };
    let smooth = scene.registry().get(forge_core::smoothing::TAG).is_some();
    let cloner = ctx.doc_mut().transact("Create Cloner", |tx| {
        let c = tx.create(CLONER, parent, index)?;
        if smooth {
            tx.attach_tag(forge_core::smoothing::TAG, c, None)?;
        }
        if let Some(name) = &a.name {
            tx.rename(c, name)?;
        }
        for id in &nodes {
            tx.move_node(*id, Some(c), None)?;
        }
        if !effectors.is_empty() {
            tx.set_param(c, "effectors", Param::LinkList(effectors.clone()))?;
        }
        Ok(c)
    })?;
    ctx.selection_mut().click(cloner);
    Ok(json!({"node": cloner.0}))
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

/// The hierarchy nodes of `ids`, without duplicates and without nodes below another listed
/// node, in the order given.
fn top_level(scene: &forge_core::Scene, ids: &[NodeId]) -> Vec<NodeId> {
    let chosen: BTreeSet<NodeId> = ids.iter().copied().collect();
    let mut seen = BTreeSet::new();
    ids.iter()
        .copied()
        .filter(|id| {
            let Some(n) = scene.node(*id) else {
                return false;
            };
            if n.is_tag() || !seen.insert(*id) {
                return false;
            }
            let mut p = n.parent;
            while let Some(pid) = p {
                if chosen.contains(&pid) {
                    return false;
                }
                p = scene.node(pid).and_then(|n| n.parent);
            }
            true
        })
        .collect()
}

fn siblings(scene: &forge_core::Scene, parent: Option<NodeId>) -> Vec<NodeId> {
    match parent {
        Some(p) => scene
            .node(p)
            .map(|n| n.children.clone())
            .unwrap_or_default(),
        None => scene.roots().to_vec(),
    }
}

fn move_nodes(ctx: &mut dyn CommandContext, a: MoveArgs) -> Result<Value, CommandError> {
    let ids = target_nodes(ctx, a.nodes);
    let parent = a.parent.map(NodeId);
    let moved = top_level(ctx.doc().scene(), &ids);
    if moved.is_empty() {
        return Ok(json!({"moved": 0}));
    }
    let base_len = siblings(ctx.doc().scene(), parent)
        .iter()
        .filter(|s| !moved.contains(s))
        .count();
    ctx.doc_mut().transact("Move", |tx| {
        // Append all first, so `index` counts the list without them, then place them.
        for id in &moved {
            tx.move_node(*id, parent, None)?;
        }
        if let Some(index) = a.index {
            let index = index.min(base_len);
            for (k, id) in moved.iter().enumerate() {
                tx.move_node(*id, parent, Some(index + k))?;
            }
        }
        Ok(())
    })?;
    Ok(json!({"moved": moved.len()}))
}

fn group(ctx: &mut dyn CommandContext, a: GroupArgs) -> Result<Value, CommandError> {
    let ids = target_nodes(ctx, a.nodes);
    let nodes = top_level(ctx.doc().scene(), &ids);
    let Some(first) = nodes.first().copied() else {
        return Err(CommandError::failed("select objects to group"));
    };
    let (parent, index) = {
        let scene = ctx.doc().scene();
        let parent = scene.node(first).and_then(|n| n.parent);
        let index = siblings(scene, parent).iter().position(|s| *s == first);
        (parent, index)
    };
    let name = a.name.unwrap_or_else(|| "Null".into());
    let group = ctx.doc_mut().transact("Group Objects", |tx| {
        let g = tx.create("forge.null", parent, index)?;
        tx.rename(g, &name)?;
        for id in &nodes {
            tx.move_node(*id, Some(g), None)?;
        }
        Ok(g)
    })?;
    ctx.selection_mut().click(group);
    Ok(json!({"node": group.0}))
}

fn visibility(ctx: &mut dyn CommandContext, a: VisibilityArgs) -> Result<Value, CommandError> {
    use forge_core::visibility::{EDITOR_VIS, RENDER_VIS, Vis};
    if a.editor.is_none() && a.render.is_none() {
        return Err(CommandError::InvalidArgs(
            "object.visibility needs `editor` and/or `render`".into(),
        ));
    }
    let vis = |v: VisArg| match v {
        VisArg::Default => Vis::Default,
        VisArg::On => Vis::On,
        VisArg::Off => Vis::Off,
    };
    let ids = hierarchy_targets(ctx, a.nodes);
    ctx.doc_mut().transact("Visibility", |tx| {
        for id in &ids {
            if let Some(v) = a.editor {
                tx.set_param(*id, EDITOR_VIS, Param::Enum(vis(v).id().into()))?;
            }
            if let Some(v) = a.render {
                tx.set_param(*id, RENDER_VIS, Param::Enum(vis(v).id().into()))?;
            }
        }
        Ok(())
    })?;
    Ok(Value::Null)
}

fn enable(ctx: &mut dyn CommandContext, a: EnableArgs) -> Result<Value, CommandError> {
    let ids = hierarchy_targets(ctx, a.nodes);
    ctx.doc_mut().transact("Enable", |tx| {
        ids.iter().try_for_each(|id| {
            tx.set_param(*id, forge_core::visibility::ENABLED, Param::Bool(a.enabled))
        })
    })?;
    Ok(Value::Null)
}

/// Existing hierarchy nodes among the targets (tags have no object state).
fn hierarchy_targets(ctx: &dyn CommandContext, nodes: Option<Vec<u64>>) -> Vec<NodeId> {
    let scene = ctx.doc().scene();
    target_nodes(ctx, nodes)
        .into_iter()
        .filter(|id| scene.node(*id).is_some_and(|n| !n.is_tag()))
        .collect()
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

fn select_extra(ctx: &mut dyn CommandContext, cmd: SelectExtra) -> Result<Value, CommandError> {
    ctx.select_extra(cmd).map_err(CommandError::Failed)?;
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
