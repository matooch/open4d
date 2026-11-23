//! Bake dynamic bodies and edit the scene's solver settings (DYNAMICS.md §5.2).

use std::collections::BTreeSet;

use forge_command::{CommandContext, CommandError, CommandRegistry, CommandSpec};
use forge_core::{EditError, NodeId, Scene, Tx, Value};
use forge_dynamics::{BodyKind, DynamicsSettings, SETTINGS, archive, build, clones};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value as Json, json};

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct Settings {
    enabled: Option<bool>,
    /// Downward acceleration in cm/s².
    gravity: Option<f64>,
    time_scale: Option<f64>,
    steps_per_frame: Option<u32>,
}

#[derive(Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum Mode {
    Keys,
    Cache,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct Bake {
    /// Dynamic objects to bake.
    nodes: Vec<u64>,
    /// First frame to record (inclusive).
    start: i64,
    /// Final frame to record (inclusive).
    end: i64,
    mode: Mode,
}

pub fn register(r: &mut CommandRegistry) -> Result<(), CommandError> {
    r.register_fn(
        CommandSpec {
            id: "dynamics.bake",
            label: "Bake Dynamics",
            undoable: true,
            sample: || json!({"nodes": [1], "start": 0, "end": 30, "mode": "cache"}),
        },
        bake,
    )?;
    r.register_fn(
        CommandSpec {
            id: "dynamics.settings",
            label: "Set Dynamics Settings",
            undoable: true,
            sample: || json!({"gravity": 981, "steps_per_frame": 4}),
        },
        settings,
    )
}

fn settings(ctx: &mut dyn CommandContext, a: Settings) -> Result<Json, CommandError> {
    let current = DynamicsSettings::node(ctx.doc().scene());
    let values = [
        ("enabled", a.enabled.map(Value::Bool)),
        ("gravity", a.gravity.map(Value::Float)),
        ("time_scale", a.time_scale.map(Value::Float)),
        (
            "steps_per_frame",
            a.steps_per_frame.map(|n| Value::Int(i64::from(n))),
        ),
    ];
    if values.iter().all(|(_, v)| v.is_none()) {
        return Err(CommandError::failed("no dynamics settings given"));
    }
    let id = ctx.doc_mut().transact("Set Dynamics Settings", |tx| {
        let node = match current {
            Some(node) => node,
            None => tx.create(SETTINGS, None, None)?,
        };
        for (param, value) in values {
            if let Some(value) = value
                && tx.scene().param(node, param) != Some(&value)
            {
                tx.set_param(node, param, value)?;
            }
        }
        Ok(node)
    })?;
    Ok(json!({"node": id.0}))
}

struct Target {
    node: NodeId,
    body: NodeId,
    cache: Option<NodeId>,
    kind: archive::Kind,
    width: usize,
    samples: Vec<f64>,
}

impl Target {
    fn new(scene: &Scene, node: NodeId, mode: &Mode) -> Result<Self, CommandError> {
        let body = build::body_tag(scene, node)
            .filter(|tag| build::kind_of(scene, *tag) == Some(BodyKind::Dynamic))
            .ok_or_else(|| CommandError::failed(format!("node {node} is not a dynamic body")))?;
        let kind = if build::is_soft(scene, body) {
            archive::Kind::Soft
        } else if clones::is_cloner(scene, node) {
            archive::Kind::Clones
        } else {
            archive::Kind::Rigid
        };
        if *mode == Mode::Keys && kind != archive::Kind::Rigid {
            return Err(CommandError::failed(
                "clones and soft bodies must bake to a cache tag",
            ));
        }
        Ok(Self {
            node,
            body,
            cache: archive::tag(scene, node),
            kind,
            width: 0,
            samples: Vec::new(),
        })
    }

    fn write_keys(self, tx: &mut Tx, start: i64, fps: u32) -> Result<(), EditError> {
        const PATHS: [&str; 6] = [
            "transform.pos.x",
            "transform.pos.y",
            "transform.pos.z",
            "transform.rot.x",
            "transform.rot.y",
            "transform.rot.z",
        ];
        let (rows, _) = self.samples.as_chunks::<6>();
        for (axis, path) in PATHS.into_iter().enumerate() {
            let track = forge_anim::bake_samples(
                rows.iter()
                    .enumerate()
                    .map(|(i, row)| ((start + i as i64) as f64 / f64::from(fps), row[axis])),
            );
            tx.set_track(self.node, path, track)?;
        }
        tx.set_param(self.body, "enabled", Value::Bool(false))?;
        if let Some(cache) = self.cache {
            tx.set_param(cache, "enabled", Value::Bool(false))?;
        }
        Ok(())
    }

    fn write_cache(
        self,
        tx: &mut Tx,
        start: i64,
        count: i64,
        fps: u32,
    ) -> Result<NodeId, EditError> {
        let id = match self.cache {
            Some(id) => id,
            None => tx.attach_tag(archive::TYPE, self.node, None)?,
        };
        for (name, value) in [
            ("enabled", Value::Bool(true)),
            ("kind", Value::Enum(self.kind.id().into())),
            ("start_frame", Value::Int(start)),
            ("count", Value::Int(count)),
            ("fps", Value::Int(i64::from(fps))),
            ("samples", Value::FloatList(self.samples)),
        ] {
            tx.set_param(id, name, value)?;
        }
        Ok(id)
    }
}

fn bake(ctx: &mut dyn CommandContext, a: Bake) -> Result<Json, CommandError> {
    let original = ctx.doc().scene();
    let settings = original.settings();
    if a.start < settings.start
        || a.end > settings.end
        || a.start > a.end
        || a.end.saturating_sub(settings.start) >= archive::MAX_FRAMES
    {
        return Err(CommandError::failed(
            "bake range must be within the first 10000 project frames",
        ));
    }
    if a.nodes.is_empty() {
        return Err(CommandError::failed("select at least one dynamic body"));
    }
    let mut targets: Vec<_> = a
        .nodes
        .into_iter()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .map(|node| Target::new(original, NodeId(node), &a.mode))
        .collect::<Result<_, _>>()?;
    let (source, _) = original.with_params(
        targets
            .iter()
            .filter_map(|t| t.cache)
            .map(|id| (id, "enabled".to_string(), Value::Bool(false))),
    );
    let eval = crate::animate::Evaluated::default();
    let mut total = 0;
    for frame in a.start..=a.end {
        let scene = eval.get(&source, frame);
        for t in &mut targets {
            let values = archive::sample(&scene, t.node, t.body, t.kind).ok_or_else(|| {
                CommandError::failed(format!(
                    "node {} has no simulation samples at frame {frame}",
                    t.node
                ))
            })?;
            if t.width != 0 && values.len() != t.width {
                return Err(CommandError::failed(
                    "changing clone/point counts cannot be baked",
                ));
            }
            if values.len() > archive::MAX_VALUES - total {
                return Err(CommandError::failed("dynamics bake exceeds sample limit"));
            }
            t.width = values.len();
            total += t.width;
            t.samples.extend(values);
        }
    }
    let count = a.end - a.start + 1;
    let nodes = targets.len();
    let tags = ctx.doc_mut().transact("Bake Dynamics", |tx| {
        let mut tags = Vec::new();
        for target in targets {
            match a.mode {
                Mode::Keys => target.write_keys(tx, a.start, settings.fps)?,
                Mode::Cache => tags.push(target.write_cache(tx, a.start, count, settings.fps)?.0),
            }
        }
        Ok(tags)
    })?;
    Ok(json!({"nodes": nodes, "frames": count, "tags": tags}))
}
