//! object.duplicate, object.arrange, object.randomize, object.transfer
//! (docs/specs/MODELING.md §9). Each is one registry command with explicit numeric arguments
//! and one undo step; the arguments are the tool descriptors' settings (flat) plus a node list.

use std::collections::{BTreeMap, BTreeSet};

use forge_core::transform::{POS, ROT, SCALE, Transform, hpb_from_rotation, rotation_part};
use forge_core::{EditError, NodeId, Track, Tx, Value};
use forge_modeling::{
    Settings, ToolDesc, arrange_positions, object_descs, randomize_draw, step_of,
};
use glam::DVec3;
use serde_json::{Map, Value as Json, json};

use super::object::{siblings, top_level};
use super::target_nodes;
use super::tools::setting_schema;
use crate::context::CommandContext;
use crate::error::CommandError;
use crate::registry::{CommandHandler, CommandRegistry};

type Run = fn(&mut dyn CommandContext, &Settings, Extras) -> Result<Json, CommandError>;

/// The arguments that are not settings.
struct Extras {
    nodes: Option<Vec<u64>>,
    source: Option<u64>,
}

struct ObjectCommand {
    /// The registry id (`desc.command`).
    command: &'static str,
    desc: ToolDesc,
    run: Run,
}

pub(super) fn register(r: &mut CommandRegistry) -> Result<(), CommandError> {
    for desc in object_descs() {
        let run: Run = match desc.id {
            "duplicate" => duplicate,
            "arrange" => arrange,
            "randomize" => randomize,
            "transfer" => transfer,
            other => {
                return Err(CommandError::Failed(format!("tool {other} has no command")));
            }
        };
        r.register(Box::new(ObjectCommand {
            command: desc.command,
            desc,
            run,
        }))?;
    }
    Ok(())
}

impl CommandHandler for ObjectCommand {
    fn id(&self) -> &str {
        self.command
    }

    fn label(&self) -> &str {
        self.desc.label
    }

    fn args_schema(&self) -> Json {
        let mut properties: Map<String, Json> = self
            .desc
            .params
            .iter()
            .map(|p| (p.id.clone(), setting_schema(p)))
            .collect();
        if self.desc.id == "transfer" {
            properties.insert(
                "source".into(),
                json!({"description": "Node to copy from; the active node when absent.",
                       "type": ["integer", "null"]}),
            );
        } else {
            properties.insert(
                "nodes".into(),
                json!({"description": "Nodes to work on; the current selection when absent.",
                       "type": ["array", "null"], "items": {"type": "integer", "minimum": 0}}),
            );
        }
        json!({"type": "object", "additionalProperties": false, "properties": properties})
    }

    fn run(&self, ctx: &mut dyn CommandContext, args: Json) -> Result<Json, CommandError> {
        let id = self.command;
        let args = if args.is_null() { json!({}) } else { args };
        let mut obj = args.as_object().cloned().ok_or_else(|| {
            CommandError::InvalidArgs(format!("{id}: arguments must be an object"))
        })?;
        let bad = |what: &str| CommandError::InvalidArgs(format!("{id}: bad {what}"));
        let transfer = self.desc.id == "transfer";
        let nodes = match if transfer { None } else { obj.remove("nodes") } {
            None | Some(Json::Null) => None,
            Some(Json::Array(items)) => Some(
                items
                    .iter()
                    .map(|v| v.as_u64().ok_or_else(|| bad("nodes")))
                    .collect::<Result<Vec<_>, _>>()?,
            ),
            Some(_) => return Err(bad("nodes")),
        };
        let source = match if transfer { obj.remove("source") } else { None } {
            None | Some(Json::Null) => None,
            Some(Json::Number(n)) => Some(n.as_u64().ok_or_else(|| bad("source"))?),
            Some(_) => return Err(bad("source")),
        };
        let settings = Settings::from_json(&self.desc, &obj)
            .map_err(|e| CommandError::InvalidArgs(format!("{id}: {e}")))?;
        (self.run)(ctx, &settings, Extras { nodes, source })
    }

    fn undoable(&self) -> bool {
        true
    }

    fn sample_args(&self) -> Json {
        match self.desc.id {
            "duplicate" => json!({"nodes": [1], "copies": 2, "offset": [200.0, 0.0, 0.0]}),
            "arrange" => json!({"nodes": [1, 2], "mode": "line", "spacing": [200.0, 0.0, 0.0]}),
            "randomize" => json!({"nodes": [1], "seed": 7, "position": [50.0, 50.0, 50.0]}),
            _ => json!({"source": 1, "target": 2, "space": "world"}),
        }
    }
}

/// The hierarchy nodes among the targets, nested ones dropped; `Failed` when none is left.
fn objects(
    ctx: &dyn CommandContext,
    nodes: Option<Vec<u64>>,
    label: &str,
) -> Result<Vec<NodeId>, CommandError> {
    let ids = target_nodes(ctx, nodes);
    let scene = ctx.doc().scene();
    if let Some(missing) = ids.iter().find(|i| !scene.contains(**i)) {
        return Err(CommandError::Failed(format!(
            "{label}: node {missing} does not exist"
        )));
    }
    let top = top_level(scene, &ids);
    if top.is_empty() {
        return Err(CommandError::Failed(format!("{label}: select objects")));
    }
    Ok(top)
}

fn vec3_of(scene: &forge_core::Scene, id: NodeId, key: &str) -> DVec3 {
    match scene.param(id, key) {
        Some(Value::Vec3(v)) => DVec3::from_array(*v),
        _ if key == SCALE => DVec3::ONE,
        _ => DVec3::ZERO,
    }
}

// ---------- Duplicate ----------

/// The ids of a node, its tags, and everything below, with their tags.
fn subtree_ids(scene: &forge_core::Scene, root: NodeId) -> BTreeSet<NodeId> {
    let mut out = BTreeSet::new();
    for id in std::iter::once(root).chain(scene.descendants(root)) {
        out.insert(id);
        if let Some(n) = scene.node(id) {
            out.extend(n.tags.iter().copied());
        }
    }
    out
}

/// Links that point into the copied subtree are set after all copies exist.
struct Pending {
    node: NodeId,
    key: String,
    value: Value,
}

fn points_inside(v: &Value, inside: &BTreeSet<NodeId>) -> bool {
    match v {
        Value::Link(Some(t)) => inside.contains(t),
        Value::LinkList(l) => l.iter().any(|t| inside.contains(t)),
        Value::LinkRows(rows) => rows.iter().any(|r| inside.contains(&r.target)),
        _ => false,
    }
}

fn remap(v: &Value, map: &BTreeMap<NodeId, NodeId>) -> Value {
    let to = |t: &NodeId| map.get(t).copied().unwrap_or(*t);
    match v {
        Value::Link(Some(t)) => Value::Link(Some(to(t))),
        Value::LinkList(l) => Value::LinkList(l.iter().map(to).collect()),
        Value::LinkRows(rows) => Value::LinkRows(
            rows.iter()
                .map(|r| forge_core::LinkRow {
                    target: to(&r.target),
                    ..r.clone()
                })
                .collect(),
        ),
        other => other.clone(),
    }
}

struct Copier<'a> {
    inside: &'a BTreeSet<NodeId>,
    map: BTreeMap<NodeId, NodeId>,
    pending: Vec<Pending>,
    /// Skip the transform tracks of the root copy: its placement is the step.
    root: NodeId,
}

impl Copier<'_> {
    fn fill(&mut self, tx: &mut Tx, src: NodeId, new: NodeId) -> Result<(), EditError> {
        let node = tx.scene().node(src).ok_or(EditError::UnknownNode(src))?;
        let (name, params, tracks) = (node.name.clone(), node.params.clone(), node.tracks.clone());
        tx.rename(new, &name)?;
        for (key, value) in params {
            if points_inside(&value, self.inside) {
                self.pending.push(Pending {
                    node: new,
                    key,
                    value,
                });
            } else {
                tx.set_param(new, &key, value)?;
            }
        }
        let skip_transform = src == self.root;
        for (key, track) in tracks {
            if skip_transform && key.starts_with(forge_core::transform::PREFIX) {
                continue;
            }
            tx.set_track(new, &key, Some(Track::clone(&track)))?;
        }
        self.map.insert(src, new);
        Ok(())
    }

    fn copy(
        &mut self,
        tx: &mut Tx,
        src: NodeId,
        parent: Option<NodeId>,
        index: Option<usize>,
    ) -> Result<NodeId, EditError> {
        let node = tx.scene().node(src).ok_or(EditError::UnknownNode(src))?;
        let (type_id, tags, children) = (
            node.type_id.clone(),
            node.tags.clone(),
            node.children.clone(),
        );
        let new = tx.create(&type_id, parent, index)?;
        self.fill(tx, src, new)?;
        for tag in tags {
            let t_type = tx
                .scene()
                .node(tag)
                .ok_or(EditError::UnknownNode(tag))?
                .type_id
                .clone();
            let t = tx.attach_tag(&t_type, new, None)?;
            self.fill(tx, tag, t)?;
        }
        for child in children {
            self.copy(tx, child, Some(new), None)?;
        }
        Ok(new)
    }

    fn finish(self, tx: &mut Tx) -> Result<(), EditError> {
        for p in self.pending {
            tx.set_param(p.node, &p.key, remap(&p.value, &self.map))?;
        }
        Ok(())
    }
}

/// `<base>.N` with the lowest free N over the whole scene (the Split rule, MODELING.md §4.7).
fn free_name(scene: &forge_core::Scene, base: &str, taken_extra: &BTreeSet<String>) -> String {
    let taken: BTreeSet<String> = scene
        .roots()
        .iter()
        .flat_map(|r| std::iter::once(*r).chain(scene.descendants(*r)))
        .filter_map(|n| scene.node(n).map(|n| n.name.clone()))
        .chain(taken_extra.iter().cloned())
        .collect();
    (1..)
        .map(|n| format!("{base}.{n}"))
        .find(|c| !taken.contains(c))
        .unwrap_or_else(|| base.to_string())
}

fn duplicate(
    ctx: &mut dyn CommandContext,
    s: &Settings,
    extras: Extras,
) -> Result<Json, CommandError> {
    let sources = objects(ctx, extras.nodes, "Duplicate")?;
    let copies = s.int("copies").max(1) as usize;
    let created = ctx.doc_mut().transact("Duplicate", |tx| {
        let mut created = Vec::new();
        for src in &sources {
            let (parent, index, base_name, base) = {
                let scene = tx.scene();
                let parent = scene.node(*src).and_then(|n| n.parent);
                let index = siblings(scene, parent)
                    .iter()
                    .position(|c| c == src)
                    .ok_or(EditError::UnknownNode(*src))?;
                let t = scene
                    .local_transform(*src)
                    .ok_or(EditError::UnknownNode(*src))?;
                let name = scene.node(*src).map(|n| n.name.clone()).unwrap_or_default();
                (parent, index, name, t)
            };
            let inside = subtree_ids(tx.scene(), *src);
            let mut used = BTreeSet::new();
            for k in 1..=copies {
                let step = step_of(k, s);
                let mut copier = Copier {
                    inside: &inside,
                    map: BTreeMap::new(),
                    pending: Vec::new(),
                    root: *src,
                };
                let new = copier.copy(tx, *src, parent, Some(index + k))?;
                copier.finish(tx)?;
                let name = free_name(tx.scene(), &base_name, &used);
                used.insert(name.clone());
                tx.rename(new, &name)?;
                let t = Transform {
                    pos: base.pos + step.pos,
                    rot: base.rot + step.rot_deg.map(f64::to_radians),
                    scale: base.scale * step.scale,
                };
                tx.set_param(new, POS, Value::Vec3(t.pos.to_array()))?;
                tx.set_param(new, ROT, Value::Vec3(t.rot.to_array()))?;
                tx.set_param(new, SCALE, Value::Vec3(t.scale.to_array()))?;
                created.push(new);
            }
        }
        Ok(created)
    })?;
    let sel = ctx.selection_mut();
    sel.nodes = created.iter().copied().collect();
    sel.active = created.last().copied();
    sel.anchor = sel.active;
    Ok(json!({"created": created.iter().map(|i| i.0).collect::<Vec<_>>(), "count": created.len()}))
}

// ---------- Arrange ----------

fn arrange(
    ctx: &mut dyn CommandContext,
    s: &Settings,
    extras: Extras,
) -> Result<Json, CommandError> {
    let nodes = objects(ctx, extras.nodes, "Arrange")?;
    let scene = ctx.doc().scene();
    let start: Vec<DVec3> = nodes.iter().map(|n| vec3_of(scene, *n, POS)).collect();
    let target = arrange_positions(&start, s);
    ctx.doc_mut().transact("Arrange", |tx| {
        for (n, p) in nodes.iter().zip(&target) {
            tx.set_param(*n, POS, Value::Vec3(p.to_array()))?;
        }
        Ok(())
    })?;
    Ok(json!({"arranged": nodes.len()}))
}

// ---------- Randomize ----------

fn randomize(
    ctx: &mut dyn CommandContext,
    s: &Settings,
    extras: Extras,
) -> Result<Json, CommandError> {
    let ids = target_nodes(ctx, extras.nodes);
    let scene = ctx.doc().scene();
    let nodes: Vec<NodeId> = ids
        .into_iter()
        .filter(|i| scene.node(*i).is_some_and(|n| !n.is_tag()))
        .collect();
    if nodes.is_empty() {
        return Err(CommandError::failed("Randomize: select objects"));
    }
    let (pos_r, rot_r, scale_r) = (s.vec3("position"), s.vec3("rotation_deg"), s.vec3("scale"));
    let (seed, uniform) = (s.int("seed"), s.bool("uniform_scale"));
    let edits: Vec<(NodeId, [DVec3; 3])> = nodes
        .iter()
        .map(|n| {
            let u = randomize_draw(seed, n.0);
            let (du, dr) = (DVec3::from_slice(&u[0..3]), DVec3::from_slice(&u[3..6]));
            let ds = if uniform {
                DVec3::splat(u[6])
            } else {
                DVec3::from_slice(&u[6..9])
            };
            let pos = vec3_of(scene, *n, POS) + du * pos_r;
            let rot = vec3_of(scene, *n, ROT) + (dr * rot_r).map(f64::to_radians);
            let scale = vec3_of(scene, *n, SCALE) * (DVec3::ONE + ds * scale_r);
            (*n, [pos, rot, scale])
        })
        .collect();
    ctx.doc_mut().transact("Randomize", |tx| {
        for (n, [pos, rot, scale]) in &edits {
            tx.set_param(*n, POS, Value::Vec3(pos.to_array()))?;
            tx.set_param(*n, ROT, Value::Vec3(rot.to_array()))?;
            tx.set_param(*n, SCALE, Value::Vec3(scale.to_array()))?;
        }
        Ok(())
    })?;
    Ok(json!({"randomized": edits.len()}))
}

// ---------- Transfer ----------

fn transfer(
    ctx: &mut dyn CommandContext,
    s: &Settings,
    extras: Extras,
) -> Result<Json, CommandError> {
    let fail = |m: &str| CommandError::Failed(format!("Transfer: {m}"));
    if s.int("target") < 0 {
        return Err(CommandError::InvalidArgs(
            "object.transfer needs `target` (a node id)".into(),
        ));
    }
    let target = NodeId(s.int("target") as u64);
    let source = extras
        .source
        .map(NodeId)
        .or(ctx.selection().active)
        .ok_or_else(|| fail("no source object"))?;
    let scene = ctx.doc().scene();
    for (what, id) in [("source", source), ("target", target)] {
        match scene.node(id) {
            None => return Err(fail(&format!("{what} {id} does not exist"))),
            Some(n) if n.is_tag() => return Err(fail(&format!("{what} {id} is a tag"))),
            Some(_) => {}
        }
    }
    if source == target {
        return Err(fail("source and target are the same object"));
    }
    let wanted: Transform = if s.choice("space") == "local" {
        scene
            .local_transform(source)
            .ok_or_else(|| fail("no source transform"))?
    } else {
        let world = scene
            .world_matrix(source)
            .ok_or_else(|| fail("no source transform"))?;
        let parent = scene
            .parent_world_matrix(target)
            .ok_or_else(|| fail("no target parent"))?;
        let local = parent.inverse() * world;
        Transform {
            pos: local.w_axis.truncate(),
            rot: hpb_from_rotation(rotation_part(&local)),
            scale: DVec3::new(
                local.x_axis.truncate().length(),
                local.y_axis.truncate().length(),
                local.z_axis.truncate().length(),
            ),
        }
    };
    ctx.doc_mut().transact("Transfer", |tx| {
        if s.bool("position") {
            tx.set_param(target, POS, Value::Vec3(wanted.pos.to_array()))?;
        }
        if s.bool("rotation") {
            tx.set_param(target, ROT, Value::Vec3(wanted.rot.to_array()))?;
        }
        if s.bool("scale") {
            tx.set_param(target, SCALE, Value::Vec3(wanted.scale.to_array()))?;
        }
        Ok(())
    })?;
    Ok(json!({"node": target.0}))
}
