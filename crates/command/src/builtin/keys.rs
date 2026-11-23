//! key.move, key.scale, key.delete, key.set_interp, key.set_tangent, key.paste,
//! key.set_speed, key.euler_filter, track.set_behavior: dope sheet and curve graph edits
//! (docs/specs/ANIM.md section 2b). Keys are addressed as `{node, track, frame}`; every command
//! is one undo step.

use std::collections::BTreeMap;

use forge_anim::{SpeedSide, keyedit, tangent};
use forge_core::{
    Behavior, Interp, KeyTangent, NodeId, Scene, TangentMode, Track, TrackTarget, Tx, Unit,
    parse_track_target,
};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};

use super::spec;
use crate::context::CommandContext;
use crate::error::CommandError;
use crate::registry::CommandRegistry;

#[derive(Deserialize, JsonSchema, Clone)]
#[serde(deny_unknown_fields)]
struct KeyRef {
    node: u64,
    /// Track key (`transform.pos.x`, a float parameter id, ...).
    track: String,
    /// Frame of the key (fractional frames allowed).
    frame: f64,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct MoveArgs {
    keys: Vec<KeyRef>,
    /// Frames to move by.
    delta_frames: f64,
    /// Value change in display units (degrees for angles, percent as shown).
    #[serde(default)]
    delta_value: f64,
    /// Also move every later key of the same tracks in time.
    #[serde(default)]
    ripple: bool,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct ScaleArgs {
    keys: Vec<KeyRef>,
    /// The frame that stays where it is.
    pivot_frame: f64,
    /// Time scale factor (not zero; negative mirrors).
    factor: f64,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct KeysArgs {
    keys: Vec<KeyRef>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct InterpArgs {
    keys: Vec<KeyRef>,
    /// `spline`, `linear` or `step`.
    interp: String,
}

#[derive(Deserialize, JsonSchema, Default)]
#[serde(deny_unknown_fields)]
struct HandleArgs {
    /// Handle end relative to the key: frames (negative on the left side) ...
    dt_frames: f64,
    /// ... and value, in display units.
    dv: f64,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct TangentArgs {
    node: u64,
    track: String,
    frame: f64,
    /// `auto`, `linear`, `ease`, `ease_in`, `ease_out`, `zero_angle`, `zero_length` or `user`.
    preset: String,
    /// New handles for `user`.
    left: Option<HandleArgs>,
    right: Option<HandleArgs>,
    broken: Option<bool>,
    weighted: Option<bool>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct SpeedArgs {
    node: u64,
    track: String,
    frame: f64,
    /// Spline segment side: `left` (incoming) or `right` (outgoing).
    side: String,
    /// Signed target speed in display units per second.
    speed: f64,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct PasteKey {
    node: u64,
    track: String,
    frame: f64,
    /// Value in display units.
    value: f64,
    /// `spline`, `linear` or `step`.
    interp: String,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct PasteArgs {
    keys: Vec<PasteKey>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct BehaviorArgs {
    node: u64,
    track: String,
    /// `constant`, `continue`, `repeat`, `offset_repeat` or `oscillate`; unchanged when absent.
    before: Option<String>,
    after: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct FilterArgs {
    node: u64,
}

fn interp_of(name: &str) -> Result<Interp, CommandError> {
    match name {
        "spline" => Ok(Interp::Spline),
        "linear" => Ok(Interp::Linear),
        "step" => Ok(Interp::Step),
        other => Err(CommandError::InvalidArgs(format!(
            "interp must be spline, linear or step, not {other:?}"
        ))),
    }
}

fn behavior_of(name: &str) -> Result<Behavior, CommandError> {
    match name {
        "constant" => Ok(Behavior::Constant),
        "continue" => Ok(Behavior::Continue),
        "repeat" => Ok(Behavior::Repeat),
        "offset_repeat" => Ok(Behavior::OffsetRepeat),
        "oscillate" => Ok(Behavior::Oscillate),
        other => Err(CommandError::InvalidArgs(format!(
            "unknown behaviour {other:?}"
        ))),
    }
}

/// The display-to-stored factor of a track: degrees to radians, percent to fractions.
fn unit_of(scene: &Scene, id: NodeId, track: &str) -> Result<Unit, CommandError> {
    let node = scene
        .node(id)
        .ok_or_else(|| CommandError::Failed(format!("unknown node {id}")))?;
    let ty = scene
        .registry()
        .get(&node.type_id)
        .ok_or_else(|| CommandError::Failed(format!("unknown type {}", node.type_id)))?;
    let target = parse_track_target(track, |p| ty.find_param(p)).map_err(CommandError::failed)?;
    let param = match target {
        TrackTarget::Float(p)
        | TrackTarget::Component(p, _)
        | TrackTarget::Int(p)
        | TrackTarget::Points(p) => p,
        _ => return Ok(Unit::None),
    };
    Ok(ty.find_param(&param).map(|d| d.ui.unit).unwrap_or_default())
}

/// Keys grouped by track as `(node, track) -> times in seconds`.
fn group(
    scene: &Scene,
    keys: &[KeyRef],
) -> Result<BTreeMap<(u64, String), Vec<f64>>, CommandError> {
    let fps = f64::from(scene.settings().fps);
    let mut out: BTreeMap<(u64, String), Vec<f64>> = BTreeMap::new();
    for k in keys {
        if !k.frame.is_finite() {
            return Err(CommandError::InvalidArgs("frame must be finite".into()));
        }
        out.entry((k.node, k.track.clone()))
            .or_default()
            .push(k.frame / fps);
    }
    if out.is_empty() {
        return Err(CommandError::InvalidArgs("no keys given".into()));
    }
    Ok(out)
}

fn track_of(scene: &Scene, node: u64, track: &str) -> Result<Track, CommandError> {
    scene
        .node(NodeId(node))
        .and_then(|n| n.tracks.get(track))
        .cloned()
        .ok_or_else(|| CommandError::Failed(format!("node {node} has no track {track:?}")))
}

/// Applies `edit` to every group's track in one transaction. `edit` returns the new track
/// (`None`: nothing to change for that track, an error if no track changed).
fn edit_tracks(
    ctx: &mut dyn CommandContext,
    label: &str,
    keys: &[KeyRef],
    edit: impl Fn(&Track, &[f64], Unit) -> Option<Track>,
) -> Result<Value, CommandError> {
    let scene = ctx.doc().scene();
    let groups = group(scene, keys)?;
    let mut changes = Vec::new();
    for ((node, track), times) in &groups {
        let t = track_of(scene, *node, track)?;
        let unit = unit_of(scene, NodeId(*node), track)?;
        if let Some(n) = edit(&t, times, unit)
            && n != t
        {
            changes.push((NodeId(*node), track.clone(), n));
        }
    }
    if changes.is_empty() {
        return Err(CommandError::Failed("no key changed".into()));
    }
    let count = changes.len();
    ctx.doc_mut().transact(label, |tx: &mut Tx| {
        for (id, key, t) in &changes {
            tx.set_track(*id, key, Some(t.clone()))?;
        }
        Ok(())
    })?;
    Ok(json!({"tracks": count}))
}

fn stored(unit: Unit, v: f64) -> f64 {
    super::param::stored(unit, v)
}

fn set_tangent(ctx: &mut dyn CommandContext, a: TangentArgs) -> Result<Value, CommandError> {
    let key = KeyRef {
        node: a.node,
        track: a.track.clone(),
        frame: a.frame,
    };
    const PRESETS: [&str; 8] = [
        "auto",
        "linear",
        "ease",
        "ease_in",
        "ease_out",
        "zero_angle",
        "zero_length",
        "user",
    ];
    if !PRESETS.contains(&a.preset.as_str()) {
        return Err(CommandError::InvalidArgs(format!(
            "unknown preset {:?}",
            a.preset
        )));
    }
    let fps_inv = 1.0 / f64::from(ctx.doc().scene().settings().fps);
    edit_tracks(ctx, "Set Tangent", &[key], |t, times, unit| {
        let i = t
            .keys
            .iter()
            .position(|k| (k.time - times[0]).abs() < 1e-9)?;
        let cur = t.tangents.get(i).copied().unwrap_or_default();
        let handle = |h: &Option<HandleArgs>, side: f64, old: forge_core::Handle| match h {
            Some(h) => forge_core::Handle {
                mode: TangentMode::User,
                dt: side * (h.dt_frames * fps_inv).abs(),
                dv: stored(unit, h.dv),
            },
            None => old,
        };
        let mut new = match a.preset.as_str() {
            "auto" => KeyTangent::default(),
            "linear" => KeyTangent::chord(),
            "ease" => tangent::ease(&t.keys, i),
            "ease_in" => tangent::ease_in(&t.keys, i, cur),
            "ease_out" => tangent::ease_out(&t.keys, i, cur),
            "zero_angle" => tangent::zero_angle(&t.keys, i, cur),
            "zero_length" => tangent::zero_length(cur),
            "user" => KeyTangent {
                left: handle(&a.left, -1.0, cur.left),
                right: handle(&a.right, 1.0, cur.right),
                ..cur
            },
            _ => return None,
        };
        new.broken = a.broken.unwrap_or(cur.broken);
        new.weighted = a.weighted.unwrap_or(cur.weighted);
        let mut out = t.clone();
        out.set_tangent(i, new);
        Some(out)
    })
}

fn set_speed(ctx: &mut dyn CommandContext, a: SpeedArgs) -> Result<Value, CommandError> {
    let side = match a.side.as_str() {
        "left" => SpeedSide::Left,
        "right" => SpeedSide::Right,
        _ => {
            return Err(CommandError::InvalidArgs(
                "side must be left or right".into(),
            ));
        }
    };
    if !a.speed.is_finite() {
        return Err(CommandError::InvalidArgs("speed must be finite".into()));
    }
    let scene = ctx.doc().scene();
    let ty = scene
        .node_type(NodeId(a.node))
        .ok_or_else(|| CommandError::failed("unknown node type"))?;
    if !matches!(
        parse_track_target(&a.track, |k| ty.find_param(k)),
        Ok(TrackTarget::Float(_) | TrackTarget::Component(..) | TrackTarget::Channel(..))
    ) {
        return Err(CommandError::failed(
            "speed editing needs a continuous scalar track",
        ));
    }
    let key = KeyRef {
        node: a.node,
        track: a.track,
        frame: a.frame,
    };
    edit_tracks(ctx, "Set Key Speed", &[key], |track, times, unit| {
        if track.quaternion {
            return None;
        }
        let i = track
            .keys
            .iter()
            .position(|k| (k.time - times[0]).abs() < 1e-9)?;
        let new = tangent::set_speed(track, i, side, stored(unit, a.speed))?;
        let mut out = track.clone();
        out.set_tangent(i, new);
        Some(out)
    })
}

pub(super) fn register(r: &mut CommandRegistry) -> Result<(), CommandError> {
    r.register_fn(
        spec("key.move", "Move Keys", true, || {
            json!({"keys": [{"node": 1, "track": "transform.pos.x", "frame": 10.0}], "delta_frames": 5.0})
        }),
        |c, a: MoveArgs| {
            if !a.delta_frames.is_finite() || !a.delta_value.is_finite() {
                return Err(CommandError::InvalidArgs("deltas must be finite".into()));
            }
            let fps = f64::from(c.doc().scene().settings().fps);
            edit_tracks(c, "Move Keys", &a.keys, |t, times, unit| {
                keyedit::move_keys(t, times, a.delta_frames / fps, stored(unit, a.delta_value), a.ripple)
            })
        },
    )?;
    r.register_fn(
        spec("key.scale", "Scale Keys", true, || {
            json!({"keys": [{"node": 1, "track": "transform.pos.x", "frame": 10.0}], "pivot_frame": 0.0, "factor": 2.0})
        }),
        |c, a: ScaleArgs| {
            if !a.pivot_frame.is_finite() || !a.factor.is_finite() || a.factor == 0.0 {
                return Err(CommandError::InvalidArgs("pivot and factor must be finite, factor not 0".into()));
            }
            let fps = f64::from(c.doc().scene().settings().fps);
            edit_tracks(c, "Scale Keys", &a.keys, |t, times, _| {
                keyedit::scale_keys(t, times, a.pivot_frame / fps, a.factor)
            })
        },
    )?;
    r.register_fn(
        spec(
            "key.delete",
            "Delete Selected Keys",
            true,
            || json!({"keys": [{"node": 1, "track": "transform.pos.x", "frame": 10.0}]}),
        ),
        |c, a: KeysArgs| delete(c, a),
    )?;
    r.register_fn(
        spec("key.set_interp", "Set Key Interpolation", true, || {
            json!({"keys": [{"node": 1, "track": "transform.pos.x", "frame": 10.0}], "interp": "linear"})
        }),
        |c, a: InterpArgs| {
            let i = interp_of(&a.interp)?;
            edit_tracks(c, "Key Interpolation", &a.keys, |t, times, _| {
                keyedit::set_interp(t, times, i)
            })
        },
    )?;
    r.register_fn(
        spec(
            "key.set_tangent",
            "Set Key Tangent",
            true,
            || json!({"node": 1, "track": "transform.pos.x", "frame": 10.0, "preset": "ease"}),
        ),
        set_tangent,
    )?;
    r.register_fn(
        spec("key.set_speed", "Set Key Speed", true, || {
            json!({"node": 1, "track": "transform.pos.x", "frame": 10.0, "side": "right", "speed": 20.0})
        }),
        set_speed,
    )?;
    r.register_fn(
        spec("key.paste", "Paste Keys", true, || {
            json!({"keys": [{"node": 1, "track": "transform.pos.x", "frame": 20.0, "value": 5.0, "interp": "spline"}]})
        }),
        paste,
    )?;
    r.register_fn(
        spec(
            "track.set_behavior",
            "Track Behavior",
            true,
            || json!({"node": 1, "track": "transform.pos.x", "after": "repeat"}),
        ),
        |c, a: BehaviorArgs| {
            let mut t = track_of(c.doc().scene(), a.node, &a.track)?;
            if let Some(b) = &a.before {
                t.before = behavior_of(b)?;
            }
            if let Some(b) = &a.after {
                t.after = behavior_of(b)?;
            }
            let id = NodeId(a.node);
            c.doc_mut().transact("Track Behavior", |tx| {
                tx.set_track(id, &a.track, Some(t.clone()))
            })?;
            Ok(Value::Null)
        },
    )?;
    r.register_fn(
        spec(
            "key.euler_filter",
            "Euler Filter",
            true,
            || json!({"node": 1}),
        ),
        |c, a: FilterArgs| {
            let id = NodeId(a.node);
            let scene = c.doc().scene();
            let node = scene
                .node(id)
                .ok_or_else(|| CommandError::Failed(format!("unknown node {id}")))?;
            let mut changed = Vec::new();
            for (key, t) in &node.tracks {
                if !key.starts_with("transform.rot.") {
                    continue;
                }
                let mut n = t.clone();
                if forge_anim::euler_filter(&mut n) {
                    changed.push((key.clone(), n));
                }
            }
            if changed.is_empty() {
                return Err(CommandError::Failed("nothing to filter".into()));
            }
            let count = changed.len();
            c.doc_mut().transact("Euler Filter", |tx| {
                for (k, t) in &changed {
                    tx.set_track(id, k, Some(t.clone()))?;
                }
                Ok(())
            })?;
            Ok(json!({"tracks": count}))
        },
    )?;
    Ok(())
}

fn delete(ctx: &mut dyn CommandContext, a: KeysArgs) -> Result<Value, CommandError> {
    let scene = ctx.doc().scene();
    let groups = group(scene, &a.keys)?;
    // a track that loses every key goes away with it
    let mut edits: Vec<(NodeId, String, Option<Track>)> = Vec::new();
    for ((node, track), times) in &groups {
        let t = track_of(scene, *node, track)?;
        let mut rest = t.clone();
        for time in times {
            if let Some(k) = rest
                .keys
                .iter()
                .find(|k| (k.time - time).abs() < 1e-9)
                .map(|k| k.time)
            {
                rest.remove_key(k);
            }
        }
        if rest == t {
            continue;
        }
        edits.push((
            NodeId(*node),
            track.clone(),
            (!rest.keys.is_empty()).then_some(rest),
        ));
    }
    if edits.is_empty() {
        return Err(CommandError::Failed("no key changed".into()));
    }
    let count = edits.len();
    ctx.doc_mut().transact("Delete Keys", |tx| {
        for (id, key, t) in &edits {
            tx.set_track(*id, key, t.clone())?;
        }
        Ok(())
    })?;
    Ok(json!({"tracks": count}))
}

fn paste(ctx: &mut dyn CommandContext, a: PasteArgs) -> Result<Value, CommandError> {
    let scene = ctx.doc().scene();
    let fps = f64::from(scene.settings().fps);
    let mut by_track: BTreeMap<(u64, String), Vec<keyedit::PastedKey>> = BTreeMap::new();
    for k in &a.keys {
        if !k.frame.is_finite() || !k.value.is_finite() {
            return Err(CommandError::InvalidArgs(
                "frame and value must be finite".into(),
            ));
        }
        let unit = unit_of(scene, NodeId(k.node), &k.track)?;
        by_track
            .entry((k.node, k.track.clone()))
            .or_default()
            .push((
                k.frame / fps,
                stored(unit, k.value),
                interp_of(&k.interp)?,
                None,
            ));
    }
    if by_track.is_empty() {
        return Err(CommandError::InvalidArgs("no keys given".into()));
    }
    let mut edits = Vec::new();
    for ((node, track), keys) in by_track {
        let base = scene
            .node(NodeId(node))
            .and_then(|n| n.tracks.get(&track))
            .cloned()
            .unwrap_or_default();
        let t = keyedit::paste_keys(&base, &keys)
            .ok_or_else(|| CommandError::Failed(format!("cannot paste into {track:?}")))?;
        edits.push((NodeId(node), track, t));
    }
    let count = edits.len();
    ctx.doc_mut().transact("Paste Keys", |tx| {
        for (id, key, t) in &edits {
            tx.set_track(*id, key, Some(t.clone()))?;
        }
        Ok(())
    })?;
    Ok(json!({"tracks": count}))
}
