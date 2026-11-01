//! timeline.*, key.*

use forge_core::SceneSettings;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};

use super::{register_plain, spec};
use crate::context::CommandContext;
use crate::error::CommandError;
use crate::registry::CommandRegistry;

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct FrameArgs {
    /// Frame number, clamped to the project range.
    frame: i64,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct StepArgs {
    /// Frames to move by (negative goes back).
    delta: i64,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct SettingsArgs {
    /// Frames per second, 1 to 240.
    fps: u32,
    /// First frame of the project range.
    start: i64,
    /// Last frame of the project range.
    end: i64,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct JumpArgs {
    /// true: next key, false: previous key.
    forward: bool,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct AutokeyArgs {
    /// New state; the current one is flipped when absent.
    enabled: Option<bool>,
}

fn frame(ctx: &dyn CommandContext) -> Value {
    json!({"frame": ctx.timeline().current})
}

fn go_to(ctx: &mut dyn CommandContext, frame: i64) {
    let settings = ctx.doc().scene().settings();
    ctx.timeline_mut().set_current(frame, &settings);
}

pub(super) fn register(r: &mut CommandRegistry) -> Result<(), CommandError> {
    register_plain(r, "key.record", "Record", true, |c| {
        let keys = c.record_keys().map_err(CommandError::Failed)?;
        Ok(json!({"keys": keys}))
    })?;
    register_plain(r, "key.delete_at_frame", "Delete Keys", true, |c| {
        let keys = c.delete_keys().map_err(CommandError::Failed)?;
        Ok(json!({"keys": keys}))
    })?;
    r.register_fn(
        spec(
            "key.jump",
            "Jump to Key",
            false,
            || json!({"forward": true}),
        ),
        |c, a: JumpArgs| {
            if let Some(f) = c.key_frame_near(a.forward) {
                go_to(c, f);
            }
            Ok(frame(c))
        },
    )?;
    r.register_fn(
        spec("key.autokey", "Autokey", false, || json!({"enabled": true})),
        |c, a: AutokeyArgs| {
            let on = a.enabled.unwrap_or(!c.autokey());
            c.set_autokey(on);
            Ok(json!({"enabled": on}))
        },
    )?;
    r.register_fn(
        spec(
            "timeline.set_frame",
            "Go to Frame",
            false,
            || json!({"frame": 10}),
        ),
        |c, a: FrameArgs| {
            go_to(c, a.frame);
            Ok(frame(c))
        },
    )?;
    r.register_fn(
        spec(
            "timeline.step",
            "Step Frames",
            false,
            || json!({"delta": 1}),
        ),
        |c, a: StepArgs| {
            let target = c.timeline().current.saturating_add(a.delta);
            go_to(c, target);
            Ok(frame(c))
        },
    )?;
    register_plain(r, "timeline.play", "Play", false, |c| {
        c.timeline_mut().play();
        Ok(Value::Null)
    })?;
    register_plain(r, "timeline.stop", "Stop", false, |c| {
        c.timeline_mut().stop();
        Ok(Value::Null)
    })?;
    r.register_fn(
        spec(
            "timeline.settings",
            "Timeline Settings",
            true,
            || json!({"fps": 30, "start": 0, "end": 90}),
        ),
        |c, a: SettingsArgs| {
            let settings = SceneSettings {
                fps: a.fps,
                start: a.start,
                end: a.end,
            };
            c.doc_mut()
                .transact_merged("Set Timeline", "timeline-settings", |tx| {
                    tx.set_settings(settings)
                })?;
            c.timeline_mut().reanchor();
            c.sync_timeline();
            Ok(Value::Null)
        },
    )
}
