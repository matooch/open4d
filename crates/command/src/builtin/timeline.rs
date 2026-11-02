//! timeline.*, key.*

use forge_core::time::{self, DEFAULT_MARKER_COLOR, color_hex, parse_color};
use forge_core::{
    Key, Marker, NodeId, PlayMode, Scene, SceneSettings, Target, TimeConfig, TimeDisplay,
    parse_target,
};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};

use super::{register_plain, spec};
use crate::context::CommandContext;
use crate::error::CommandError;
use crate::registry::CommandRegistry;
use crate::timeline::{Playback, Timeline};

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

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct KeySetArgs {
    node: u64,
    /// Track: an animatable float parameter id, or `<vec3 param>.<x|y|z>`
    /// (`transform.pos.x`, `transform.rot.y`, `transform.scale.z`).
    track: String,
    /// Frame of the key (any integer; fps comes from the project).
    frame: i64,
    /// Value in display units (degrees for angles, percent as shown).
    value: f64,
}

fn key_set(ctx: &mut dyn CommandContext, a: KeySetArgs) -> Result<Value, CommandError> {
    let id = NodeId(a.node);
    if !a.value.is_finite() {
        return Err(CommandError::InvalidArgs("value must be finite".into()));
    }
    let scene = ctx.doc().scene();
    let node = scene
        .node(id)
        .ok_or_else(|| CommandError::Failed(format!("unknown node {id}")))?;
    let ty = scene
        .registry()
        .get(&node.type_id)
        .ok_or_else(|| CommandError::Failed(format!("unknown type {}", node.type_id)))?;
    let param = match parse_target(&a.track, |p| ty.find_param(p)).map_err(CommandError::failed)? {
        Target::Float(p) | Target::Component(p, _) => p,
    };
    let unit = ty.find_param(&param).map(|d| d.ui.unit).unwrap_or_default();
    let time = a.frame as f64 / f64::from(scene.settings().fps);
    let key = Key::new(time, super::param::stored(unit, a.value));
    ctx.doc_mut()
        .transact("Set Key", |tx| tx.set_key(id, &a.track, key))?;
    Ok(Value::Null)
}

fn start_playback(ctx: &mut dyn CommandContext) {
    let scene = ctx.doc().scene();
    let playback = Playback::of(&scene.settings(), scene.time());
    ctx.timeline_mut().play_in(&playback);
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
        spec(
            "key.set",
            "Set Key",
            true,
            || json!({"node": 1, "track": "transform.pos.x", "frame": 10, "value": 100.0}),
        ),
        key_set,
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
        start_playback(c);
        Ok(Value::Null)
    })?;
    register_plain(r, "timeline.toggle_play", "Play / Stop", false, |c| {
        if c.timeline().playing {
            c.timeline_mut().stop();
        } else {
            start_playback(c);
        }
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
    )?;
    register_time(r)?;
    register_markers(r)
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct FpsArgs {
    /// New frames per second, 1 to 240. Keys keep their seconds; the ranges, markers and the
    /// current frame are converted (ANIM.md §2.9).
    fps: u32,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct PreviewArgs {
    /// First frame of the preview range, inside the project range.
    start: i64,
    /// Last frame of the preview range, inside the project range.
    end: i64,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct ModeArgs {
    /// `once`, `loop` or `ping_pong`.
    mode: String,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct DroppingArgs {
    /// true: real-time playback that drops frames; false: every frame is shown.
    enabled: bool,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct DisplayArgs {
    /// `frames`, `smpte` or `seconds`.
    format: String,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct MarkerAddArgs {
    /// Frame of the marker (default: the current frame), inside the project range.
    frame: Option<i64>,
    /// Name (default `Marker <id>`).
    name: Option<String>,
    /// Colour as `#rrggbb` (default `#e0a030`).
    color: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct MarkerIdArgs {
    id: u64,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct MarkerRenameArgs {
    id: u64,
    name: String,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct MarkerMoveArgs {
    id: u64,
    /// New frame, inside the project range.
    frame: i64,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct MarkerColorArgs {
    id: u64,
    /// `#rrggbb`.
    color: String,
}

fn marker_json(m: &Marker) -> Value {
    json!({"id": m.id, "frame": m.frame, "name": m.name, "color": color_hex(m.color)})
}

/// The timeline state as the commands and RPC report it (ANIM.md §2.9).
pub fn timeline_state(scene: &Scene, timeline: &Timeline) -> Value {
    let (s, t) = (scene.settings(), scene.time());
    json!({
        "frame": timeline.current,
        "fps": s.fps,
        "start": s.start,
        "end": s.end,
        "playing": timeline.playing,
        "preview": t.preview.map(|(a, b)| json!([a, b])),
        "mode": t.mode.id(),
        "drop_frames": t.drop_frames,
        "display": t.display.id(),
        "text": time::format_time(timeline.current, s.fps, t.display),
        "markers": t.markers.iter().map(marker_json).collect::<Vec<_>>(),
    })
}

/// Applies `f` to a copy of the time configuration and stores it as one undo step (steps with
/// the same `merge` key merge). Nothing is recorded when `f` changes nothing.
fn edit_time<R>(
    ctx: &mut dyn CommandContext,
    label: &str,
    merge: Option<String>,
    f: impl FnOnce(&mut TimeConfig, &SceneSettings) -> Result<R, CommandError>,
) -> Result<R, CommandError> {
    let settings = ctx.doc().scene().settings();
    let before = ctx.doc().scene().time().clone();
    let mut time = before.clone();
    let out = f(&mut time, &settings)?;
    if time != before {
        let run = |tx: &mut forge_core::Tx| tx.set_time(time);
        match merge {
            Some(key) => ctx.doc_mut().transact_merged(label, &key, run)?,
            None => ctx.doc_mut().transact(label, run)?,
        }
        ctx.timeline_mut().reanchor();
    }
    Ok(out)
}

fn in_range(frame: i64, s: &SceneSettings) -> Result<(), CommandError> {
    if (s.start..=s.end).contains(&frame) {
        Ok(())
    } else {
        Err(CommandError::Failed(format!(
            "frame {frame} is outside the project range {}..{}",
            s.start, s.end
        )))
    }
}

fn color_arg(text: &str) -> Result<[u8; 3], CommandError> {
    parse_color(text)
        .ok_or_else(|| CommandError::InvalidArgs(format!("color must be #rrggbb, got {text:?}")))
}

fn marker_mut(time: &mut TimeConfig, id: u64) -> Result<&mut Marker, CommandError> {
    time.markers
        .iter_mut()
        .find(|m| m.id == id)
        .ok_or_else(|| CommandError::Failed(format!("unknown marker {id}")))
}

/// Converts every frame-based setting to `fps`; keys are in seconds and stay untouched.
fn set_fps(ctx: &mut dyn CommandContext, a: FpsArgs) -> Result<Value, CommandError> {
    let old = ctx.doc().scene().settings();
    let conv = |f: i64| time::convert_frame(f, old.fps, a.fps);
    let settings = SceneSettings {
        fps: a.fps,
        start: conv(old.start),
        end: conv(old.end),
    };
    let mut config = ctx.doc().scene().time().clone();
    config.preview = config.preview.map(|(s, e)| (conv(s), conv(e)));
    for m in &mut config.markers {
        m.frame = conv(m.frame);
    }
    let current = conv(ctx.timeline().current);
    if settings != old || config != *ctx.doc().scene().time() {
        ctx.doc_mut()
            .transact("Set FPS", |tx| tx.set_settings_and_time(settings, config))?;
    }
    ctx.timeline_mut().set_current(current, &settings);
    Ok(
        json!({"fps": settings.fps, "start": settings.start, "end": settings.end,
        "frame": ctx.timeline().current}),
    )
}

fn register_time(r: &mut CommandRegistry) -> Result<(), CommandError> {
    r.register_fn(
        spec("timeline.set_fps", "Set FPS", true, || json!({"fps": 24})),
        set_fps,
    )?;
    r.register_fn(
        spec(
            "timeline.set_preview",
            "Set Preview Range",
            true,
            || json!({"start": 10, "end": 40}),
        ),
        |c, a: PreviewArgs| {
            edit_time(
                c,
                "Set Preview Range",
                Some("timeline-preview".into()),
                |t, s| {
                    if a.start > a.end {
                        return Err(CommandError::InvalidArgs(format!(
                            "start {} is after end {}",
                            a.start, a.end
                        )));
                    }
                    in_range(a.start, s)?;
                    in_range(a.end, s)?;
                    t.preview = Some((a.start, a.end));
                    Ok(Value::Null)
                },
            )
        },
    )?;
    register_plain(
        r,
        "timeline.clear_preview",
        "Clear Preview Range",
        true,
        |c| {
            edit_time(c, "Clear Preview Range", None, |t, _| {
                t.preview = None;
                Ok(Value::Null)
            })
        },
    )?;
    r.register_fn(
        spec(
            "timeline.set_mode",
            "Set Playback Mode",
            true,
            || json!({"mode": "loop"}),
        ),
        |c, a: ModeArgs| {
            let mode = PlayMode::from_id(&a.mode).ok_or_else(|| {
                CommandError::InvalidArgs(format!(
                    "mode must be once, loop or ping_pong, got {:?}",
                    a.mode
                ))
            })?;
            edit_time(c, "Set Playback Mode", None, |t, _| {
                t.mode = mode;
                Ok(Value::Null)
            })
        },
    )?;
    r.register_fn(
        spec(
            "timeline.set_frame_dropping",
            "Set Frame Dropping",
            true,
            || json!({"enabled": true}),
        ),
        |c, a: DroppingArgs| {
            edit_time(c, "Set Frame Dropping", None, |t, _| {
                t.drop_frames = a.enabled;
                Ok(Value::Null)
            })
        },
    )?;
    r.register_fn(
        spec(
            "timeline.set_display",
            "Set Time Display",
            true,
            || json!({"format": "frames"}),
        ),
        |c, a: DisplayArgs| {
            let display = TimeDisplay::from_id(&a.format).ok_or_else(|| {
                CommandError::InvalidArgs(format!(
                    "format must be frames, smpte or seconds, got {:?}",
                    a.format
                ))
            })?;
            edit_time(c, "Set Time Display", None, |t, _| {
                t.display = display;
                Ok(Value::Null)
            })
        },
    )?;
    register_plain(r, "timeline.info", "Timeline Info", false, |c| {
        Ok(timeline_state(c.doc().scene(), c.timeline()))
    })
}

fn register_markers(r: &mut CommandRegistry) -> Result<(), CommandError> {
    r.register_fn(
        spec(
            "marker.add",
            "Add Marker",
            true,
            || json!({"frame": 10, "name": "Hit", "color": "#e0a030"}),
        ),
        |c, a: MarkerAddArgs| {
            let frame = a.frame.unwrap_or(c.timeline().current);
            let color = a
                .color
                .as_deref()
                .map_or(Ok(DEFAULT_MARKER_COLOR), color_arg)?;
            edit_time(c, "Add Marker", None, |t, s| {
                in_range(frame, s)?;
                let id = t.next_marker_id();
                t.markers.push(Marker {
                    id,
                    frame,
                    name: a.name.unwrap_or_else(|| format!("Marker {id}")),
                    color,
                });
                Ok(json!({"id": id, "frame": frame}))
            })
        },
    )?;
    r.register_fn(
        spec("marker.remove", "Remove Marker", true, || json!({"id": 1})),
        |c, a: MarkerIdArgs| {
            edit_time(c, "Remove Marker", None, |t, _| {
                marker_mut(t, a.id)?;
                t.markers.retain(|m| m.id != a.id);
                Ok(Value::Null)
            })
        },
    )?;
    r.register_fn(
        spec(
            "marker.rename",
            "Rename Marker",
            true,
            || json!({"id": 1, "name": "Hit"}),
        ),
        |c, a: MarkerRenameArgs| {
            edit_time(
                c,
                "Rename Marker",
                Some(format!("marker-name-{}", a.id)),
                |t, _| {
                    marker_mut(t, a.id)?.name = a.name;
                    Ok(Value::Null)
                },
            )
        },
    )?;
    r.register_fn(
        spec(
            "marker.move",
            "Move Marker",
            true,
            || json!({"id": 1, "frame": 20}),
        ),
        |c, a: MarkerMoveArgs| {
            edit_time(
                c,
                "Move Marker",
                Some(format!("marker-move-{}", a.id)),
                |t, s| {
                    in_range(a.frame, s)?;
                    marker_mut(t, a.id)?.frame = a.frame;
                    Ok(Value::Null)
                },
            )
        },
    )?;
    r.register_fn(
        spec(
            "marker.set_color",
            "Set Marker Color",
            true,
            || json!({"id": 1, "color": "#38c6e8"}),
        ),
        |c, a: MarkerColorArgs| {
            let color = color_arg(&a.color)?;
            edit_time(c, "Set Marker Color", None, |t, _| {
                marker_mut(t, a.id)?.color = color;
                Ok(Value::Null)
            })
        },
    )?;
    register_plain(r, "marker.list", "List Markers", false, |c| {
        let markers: Vec<Value> = c
            .doc()
            .scene()
            .time()
            .markers
            .iter()
            .map(marker_json)
            .collect();
        Ok(json!({"markers": markers}))
    })?;
    r.register_fn(
        spec(
            "marker.jump",
            "Jump to Marker",
            false,
            || json!({"forward": true}),
        ),
        |c, a: JumpArgs| {
            let scene = c.doc().scene();
            let target =
                scene
                    .time()
                    .jump_target(c.timeline().current, a.forward, &scene.settings());
            if let Some(f) = target {
                go_to(c, f);
            }
            Ok(json!({"frame": c.timeline().current, "found": target.is_some()}))
        },
    )
}
