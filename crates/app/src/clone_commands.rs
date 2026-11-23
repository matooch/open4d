//! Bake evaluated Cloner data into a native cache tag (clone-toolset spec O5).

use forge_clone_engine::{CLONER, cache};
use forge_command::{CommandContext, CommandError, CommandRegistry, CommandSpec};
use forge_core::{EvalCtx, NodeId, Value};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value as Json, json};

#[derive(Deserialize, JsonSchema)]
struct BakeArgs {
    /// Cloner node ID.
    node: u64,
    /// First frame to bake (inclusive).
    start: i64,
    /// Last frame to bake (inclusive).
    end: i64,
}

pub fn register(commands: &mut CommandRegistry) -> Result<(), CommandError> {
    commands.register_fn(
        CommandSpec {
            id: "cloner.bake",
            label: "Bake Clone Cache",
            undoable: true,
            sample: || json!({"node": 1, "start": 0, "end": 30}),
        },
        bake,
    )
}

fn bake(ctx: &mut dyn CommandContext, args: BakeArgs) -> Result<Json, CommandError> {
    let node = NodeId(args.node);
    let source = ctx.doc().scene();
    if !source.node(node).is_some_and(|n| n.type_id == CLONER) {
        return Err(CommandError::failed("expected a Cloner node"));
    }
    let fps = source.settings().fps;
    let mut writer = cache::Writer::new(args.start, args.end, fps)
        .ok_or_else(|| CommandError::failed("invalid bake range (maximum 10000 frames)"))?;
    let existing = cache::tag(source, node).map(|tag| tag.id);
    let (source, _) = source
        .with_params(existing.map(|tag| (tag, "frames".into(), Value::FloatList(Vec::new()))));
    let eval = crate::animate::Evaluated::default();
    for frame in args.start..=args.end {
        let scene = eval.get(&source, frame);
        let clones =
            forge_generators::clone_data(&scene, node, &EvalCtx::at_frame(frame, f64::from(fps)))
                .map_err(|err| CommandError::failed(format!("frame {frame}: {err}")))?
                .ok_or_else(|| CommandError::failed("the evaluated node was not a Cloner"))?;
        writer.push(&clones).ok_or_else(|| {
            CommandError::failed(format!("frame {frame}: invalid or oversized clone cache"))
        })?;
    }
    let frames = writer.finish().expect("every requested frame was recorded");
    let tag = ctx.doc_mut().transact("Bake Clone Cache", |tx| {
        let tag = match existing {
            Some(tag) => tag,
            None => tx.attach_tag(cache::CLONE_CACHE, node, None)?,
        };
        tx.set_param(tag, "frames", Value::FloatList(frames))?;
        tx.set_param(tag, "enabled", Value::Bool(true))?;
        Ok(tag)
    })?;
    Ok(json!({"tag": tag.0}))
}
