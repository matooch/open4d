//! Expression-style motion tags: Target, Align to Spline, Vibrate, Look at Camera,
//! Protection, Display and Compositing (docs/specs/RIG.md §13, brief 106).
//!
//! These tags run in the tag stage (after tracks, before generators). They are registered
//! and evaluated by the app crate because Align to Spline needs `forge-generators` spline
//! evaluation, and the dependency graph cannot place generators below `forge-rig`.

use forge_core::transform::{POS, ROT, SCALE};
use forge_core::{Category, NodeId, NodeType, ParamDesc, Scene, Unit, Value};
use forge_generators::spline_sampling::arc_length_parameter;
use forge_generators::splines::{Curve, world_curve};
use forge_noise::Noise;
use forge_rig::tags::priority_params;
use glam::{DMat4, DVec3};

pub const TARGET_TAG: &str = "forge.target_tag";
pub const ALIGN_TAG: &str = "forge.align_to_spline_tag";
pub const VIBRATE_TAG: &str = "forge.vibrate_tag";
pub const LOOK_TAG: &str = "forge.look_at_camera_tag";
pub use forge_rig::PROTECTION_TAG;
pub const DISPLAY_TAG: &str = "forge.display_tag";
pub const COMPOSITING_TAG: &str = "forge.compositing_tag";

/// All motion tag type ids.
pub const TAGS: &[&str] = &[
    TARGET_TAG,
    ALIGN_TAG,
    VIBRATE_TAG,
    LOOK_TAG,
    PROTECTION_TAG,
    DISPLAY_TAG,
    COMPOSITING_TAG,
];

fn bool_or(scene: &Scene, id: NodeId, key: &str, default: bool) -> bool {
    match scene.param(id, key) {
        Some(Value::Bool(b)) => *b,
        _ => default,
    }
}

fn float_or(scene: &Scene, id: NodeId, key: &str, default: f64) -> f64 {
    match scene.param(id, key) {
        Some(Value::Float(v)) => *v,
        _ => default,
    }
}

fn vec3_or(scene: &Scene, id: NodeId, key: &str, default: DVec3) -> DVec3 {
    match scene.param(id, key) {
        Some(Value::Vec3(v)) => DVec3::from_array(*v),
        _ => default,
    }
}

fn link_or(scene: &Scene, id: NodeId, key: &str) -> Option<NodeId> {
    match scene.param(id, key) {
        Some(Value::Link(Some(t))) => Some(*t),
        _ => None,
    }
}

/// The HPB rotation (radians) that turns the object's local +Z axis toward `dir`.
/// See docs/parity/animation-2.md target-tag and align-to-spline cases.
fn look_at_hpb(dir: DVec3) -> DVec3 {
    let horizontal = dir.x.hypot(dir.z);
    if dir.y != 0.0 && horizontal <= 1e-5 * dir.length() {
        return DVec3::new(0.0, std::f64::consts::FRAC_PI_2.copysign(dir.y), 0.0);
    }
    let h = (-dir.x).atan2(dir.z);
    let p = dir.y.atan2(horizontal);
    DVec3::new(h, p, 0.0)
}

fn world_to_parent(scene: &Scene, host: NodeId) -> Option<DMat4> {
    let parent = scene.parent_world_matrix(host)?;
    (parent.determinant() != 0.0).then(|| parent.inverse())
}

// ---------------------------------------------------------------------------
// Target tag
// ---------------------------------------------------------------------------

fn target_type() -> NodeType {
    let mut ty = NodeType::new(TARGET_TAG, "Target", Category::Tag)
        .param(ParamDesc::bool("enabled", "Enabled", true))
        .param(ParamDesc::link(
            "target",
            "Target Object",
            &[Category::Object],
        ));
    for p in priority_params() {
        ty = ty.param(p);
    }
    ty
}

fn target_writes(scene: &Scene, tag: NodeId) -> Vec<(NodeId, String, Value)> {
    if !bool_or(scene, tag, "enabled", true) {
        return Vec::new();
    }
    let Some(host) = scene.node(tag).and_then(|n| n.host) else {
        return Vec::new();
    };
    let Some(target) = link_or(scene, tag, "target") else {
        return Vec::new();
    };
    let Some(host_pos) = scene.world_matrix(host).map(|m| m.w_axis.truncate()) else {
        return Vec::new();
    };
    let Some(target_pos) = scene.world_matrix(target).map(|m| m.w_axis.truncate()) else {
        return Vec::new();
    };
    let Some(local) = world_to_parent(scene, host) else {
        return Vec::new();
    };
    let dir = local.transform_vector3(target_pos - host_pos);
    if dir == DVec3::ZERO {
        return Vec::new();
    }
    let hpb = look_at_hpb(dir);
    vec![(host, ROT.into(), Value::Vec3(hpb.to_array()))]
}

// ---------------------------------------------------------------------------
// Align to Spline tag
// ---------------------------------------------------------------------------

fn align_type() -> NodeType {
    let mut ty = NodeType::new(ALIGN_TAG, "Align to Spline", Category::Tag)
        .param(ParamDesc::bool("enabled", "Enabled", true))
        .param(ParamDesc::link("spline", "Spline", &[Category::Generator]))
        .param(ParamDesc::float("position", "Position", 0.0).range(0.0, 1.0))
        .param(ParamDesc::bool("tangential", "Tangential", false));
    for p in priority_params() {
        ty = ty.param(p);
    }
    ty
}

/// Point and tangent on `curve` at position `u` in `[0,1]`.
///
/// Align to Spline caps the distance just short of the endpoint (RIG.md §13).
fn spline_pos_tan(curve: &Curve, u: f64) -> Option<(DVec3, DVec3)> {
    let s = arc_length_parameter(curve, u.min(0.9999))?;
    let i = (s as usize).min(curve.segments.len() - 1);
    let segment = &curve.segments[i];
    let tangent = Curve::tangent(segment, s - i as f64);
    let tangent = if tangent == DVec3::ZERO {
        segment[3] - segment[0]
    } else {
        tangent
    };
    Some((curve.at(s), tangent))
}

fn align_writes(scene: &Scene, tag: NodeId) -> Vec<(NodeId, String, Value)> {
    if !bool_or(scene, tag, "enabled", true) {
        return Vec::new();
    }
    let Some(host) = scene.node(tag).and_then(|n| n.host) else {
        return Vec::new();
    };
    let Some(spline) = link_or(scene, tag, "spline") else {
        return Vec::new();
    };
    let Some(curve) = world_curve(scene, spline) else {
        return Vec::new();
    };
    let u = float_or(scene, tag, "position", 0.0);
    let Some((pos, tan)) = spline_pos_tan(&curve, u) else {
        return Vec::new();
    };
    let Some(local) = world_to_parent(scene, host) else {
        return Vec::new();
    };
    let pos = local.transform_point3(pos);
    let mut out = vec![(host, POS.into(), Value::Vec3(pos.to_array()))];
    if bool_or(scene, tag, "tangential", false) && tan != DVec3::ZERO {
        let hpb = look_at_hpb(local.transform_vector3(tan));
        out.push((host, ROT.into(), Value::Vec3(hpb.to_array())));
    }
    out
}

// ---------------------------------------------------------------------------
// Vibrate tag
// ---------------------------------------------------------------------------

fn vibrate_type() -> NodeType {
    let mut ty = NodeType::new(VIBRATE_TAG, "Vibrate", Category::Tag)
        .param(ParamDesc::bool("enabled", "Enabled", true))
        .param(ParamDesc::bool("enable_position", "Position", true))
        .param(ParamDesc::bool("enable_rotation", "Rotation", false))
        .param(ParamDesc::bool("enable_scale", "Scale", false))
        .param(ParamDesc::vec3(
            "position_amplitude",
            "Position Amplitude",
            [50.0; 3],
        ))
        .param(ParamDesc::vec3(
            "rotation_amplitude",
            "Rotation Amplitude",
            [50.0; 3],
        ))
        .param(ParamDesc::vec3(
            "scale_amplitude",
            "Scale Amplitude",
            [50.0; 3],
        ))
        .param(ParamDesc::float(
            "position_frequency",
            "Position Frequency",
            2.0,
        ))
        .param(ParamDesc::float(
            "rotation_frequency",
            "Rotation Frequency",
            2.0,
        ))
        .param(ParamDesc::float("scale_frequency", "Scale Frequency", 2.0))
        .param(ParamDesc::float("seed", "Seed", 0.0))
        .param(ParamDesc::bool("regular_pulse", "Regular Pulse", false))
        .param(ParamDesc::bool("relative", "Relative", true));
    for p in priority_params() {
        ty = ty.param(p);
    }
    ty
}

/// Deterministic smooth noise in `[-1, 1]` for the given seed, frequency and time.
fn vibrate_noise(seed: u32, freq: f64, t: f64, axis: usize) -> f64 {
    let noise = Noise {
        seed,
        octaves: 1,
        ..Noise::default()
    };
    let p = DVec3::new(
        seed as f64 + axis as f64 * 17.0,
        freq * t * 0.5,
        seed as f64 * 0.25,
    );
    noise.sample3(p).clamp(-1.0, 1.0)
}

fn vibrate_writes(scene: &Scene, tag: NodeId, time: f64) -> Vec<(NodeId, String, Value)> {
    if !bool_or(scene, tag, "enabled", true) {
        return Vec::new();
    }
    let Some(host) = scene.node(tag).and_then(|n| n.host) else {
        return Vec::new();
    };
    let Some(base) = scene.local_transform(host) else {
        return Vec::new();
    };
    let seed = float_or(scene, tag, "seed", 0.0) as u32;
    let relative = bool_or(scene, tag, "relative", true);
    let regular = bool_or(scene, tag, "regular_pulse", false);
    let offset = |amp: DVec3, freq: f64, axis: usize| {
        if regular {
            amp * (std::f64::consts::TAU * freq * time).sin()
        } else {
            amp * DVec3::from_array(std::array::from_fn(|i| {
                vibrate_noise(seed, freq, time, axis + i)
            }))
        }
    };
    let mut out = Vec::new();
    if bool_or(scene, tag, "enable_position", false) {
        let amp = vec3_or(scene, tag, "position_amplitude", DVec3::ZERO);
        let freq = float_or(scene, tag, "position_frequency", 1.0);
        let off = offset(amp, freq, 0) + if relative { base.pos } else { DVec3::ZERO };
        out.push((host, POS.into(), Value::Vec3(off.to_array())));
    }
    if bool_or(scene, tag, "enable_rotation", false) {
        let amp = vec3_or(scene, tag, "rotation_amplitude", DVec3::ZERO);
        let freq = float_or(scene, tag, "rotation_frequency", 1.0);
        let off = offset(amp, freq, 3) + if relative { base.rot } else { DVec3::ZERO };
        out.push((host, ROT.into(), Value::Vec3(off.to_array())));
    }
    if bool_or(scene, tag, "enable_scale", false) {
        let amp = vec3_or(scene, tag, "scale_amplitude", DVec3::ZERO);
        let freq = float_or(scene, tag, "scale_frequency", 1.0);
        let off =
            (DVec3::ONE + offset(amp, freq, 6)) * if relative { base.scale } else { DVec3::ONE };
        out.push((host, SCALE.into(), Value::Vec3(off.to_array())));
    }
    out
}

// ---------------------------------------------------------------------------
// Look at Camera tag
// ---------------------------------------------------------------------------

fn look_type() -> NodeType {
    let mut ty = NodeType::new(LOOK_TAG, "Look at Camera", Category::Tag)
        .param(ParamDesc::bool("enabled", "Enabled", true))
        .param(ParamDesc::bool(
            "change_pitch",
            "Change Pitch Rotation",
            true,
        ));
    for p in priority_params() {
        ty = ty.param(p);
    }
    ty
}

fn first_camera(scene: &Scene) -> Option<NodeId> {
    for root in scene.roots().iter().copied() {
        for id in std::iter::once(root).chain(scene.descendants(root)) {
            if scene.node(id).is_some_and(|n| n.type_id == "forge.camera") {
                return Some(id);
            }
        }
    }
    None
}

fn look_writes(scene: &Scene, tag: NodeId) -> Vec<(NodeId, String, Value)> {
    if !bool_or(scene, tag, "enabled", true) {
        return Vec::new();
    }
    let Some(host) = scene.node(tag).and_then(|n| n.host) else {
        return Vec::new();
    };
    let Some(camera) = first_camera(scene) else {
        return Vec::new();
    };
    let Some(host_pos) = scene.world_matrix(host).map(|m| m.w_axis.truncate()) else {
        return Vec::new();
    };
    let Some(cam_pos) = scene.world_matrix(camera).map(|m| m.w_axis.truncate()) else {
        return Vec::new();
    };
    let Some(local) = world_to_parent(scene, host) else {
        return Vec::new();
    };
    let dir = local.transform_vector3(cam_pos - host_pos);
    if dir == DVec3::ZERO {
        return Vec::new();
    }
    let mut hpb = look_at_hpb(dir);
    if !bool_or(scene, tag, "change_pitch", true)
        && let Some(t) = scene.local_transform(host)
    {
        hpb.y = t.rot.y;
    }
    vec![(host, ROT.into(), Value::Vec3(hpb.to_array()))]
}

// ---------------------------------------------------------------------------
// Protection tag
// ---------------------------------------------------------------------------

fn protection_type() -> NodeType {
    let mut ty = NodeType::new(PROTECTION_TAG, "Protection", Category::Tag)
        .param(ParamDesc::bool("enabled", "Enabled", true))
        .param(ParamDesc::enumeration(
            "position",
            "Position",
            "lock",
            &[("none", "None"), ("lock", "Lock"), ("limit", "Limit")],
        ))
        .param(ParamDesc::bool("pos_x", "X", true))
        .param(ParamDesc::bool("pos_y", "Y", true))
        .param(ParamDesc::bool("pos_z", "Z", true))
        .param(ParamDesc::float("min_x", "Min X", 0.0).unit(Unit::Distance))
        .param(ParamDesc::float("min_y", "Min Y", 0.0).unit(Unit::Distance))
        .param(ParamDesc::float("min_z", "Min Z", 0.0).unit(Unit::Distance))
        .param(ParamDesc::float("max_x", "Max X", 0.0).unit(Unit::Distance))
        .param(ParamDesc::float("max_y", "Max Y", 0.0).unit(Unit::Distance))
        .param(ParamDesc::float("max_z", "Max Z", 0.0).unit(Unit::Distance));
    for p in priority_params() {
        ty = ty.param(p);
    }
    ty
}

// ---------------------------------------------------------------------------
// Display tag
// ---------------------------------------------------------------------------

fn display_type() -> NodeType {
    let mut ty = NodeType::new(DISPLAY_TAG, "Display", Category::Tag)
        .param(ParamDesc::bool("enabled", "Enabled", true))
        .param(ParamDesc::bool(
            "use_shading_mode",
            "Use Shading Mode",
            false,
        ))
        .param(ParamDesc::enumeration(
            "shading_mode",
            "Shading Mode",
            "gouraud",
            &[
                ("gouraud", "Gouraud Shading"),
                ("gouraud_lines", "Gouraud Shading (Lines)"),
                ("quick", "Quick Shading"),
                ("quick_lines", "Quick Shading (Lines)"),
                ("constant", "Constant Shading"),
                ("constant_lines", "Constant Shading (Lines)"),
            ],
        ))
        .param(ParamDesc::enumeration(
            "style",
            "Style",
            "wireframe",
            &[
                ("wireframe", "Wireframe"),
                ("isoparms", "Isoparms"),
                ("box", "Box"),
                ("skeleton", "Skeleton"),
            ],
        ))
        .param(ParamDesc::enumeration(
            "volume_vector",
            "Volume Vector",
            "direction",
            &[("direction", "Direction"), ("length", "Length")],
        ))
        .param(ParamDesc::bool("use_lod", "Use Level of Detail", false))
        .param(ParamDesc::float("lod", "Level of Detail", 1.0))
        .param(ParamDesc::bool("use_visibility", "Use Visibility", false))
        .param(ParamDesc::float("visibility", "Visibility", 1.0).range(0.0, 1.0))
        .param(ParamDesc::bool(
            "use_backface_culling",
            "Use Backface Culling",
            false,
        ));
    for p in priority_params() {
        ty = ty.param(p);
    }
    ty
}

// ---------------------------------------------------------------------------
// Compositing tag
// ---------------------------------------------------------------------------

fn compositing_type() -> NodeType {
    let mut ty = NodeType::new(COMPOSITING_TAG, "Compositing", Category::Tag)
        .param(ParamDesc::bool("enabled", "Enabled", true))
        .param(ParamDesc::bool("cast_shadows", "Cast Shadows", true))
        .param(ParamDesc::bool("receive_shadows", "Receive Shadows", true))
        .param(ParamDesc::bool("self_shadowing", "Self Shadowing", true))
        .param(ParamDesc::bool("seen_by_camera", "Seen by Camera", true))
        .param(ParamDesc::bool("seen_by_rays", "Seen by Rays", true))
        .param(ParamDesc::bool("seen_by_gi", "Seen by GI", true))
        .param(ParamDesc::bool(
            "compositing_background",
            "Compositing Background",
            false,
        ))
        .param(ParamDesc::bool(
            "compositing_background_hdr",
            "Compositing Background for HDR Maps",
            false,
        ))
        .param(ParamDesc::bool(
            "seen_by_transparency",
            "Seen by Transparency",
            true,
        ));
    for p in priority_params() {
        ty = ty.param(p);
    }
    ty
}

// ---------------------------------------------------------------------------
// Public dispatch
// ---------------------------------------------------------------------------

pub fn tag_types() -> Vec<NodeType> {
    vec![
        target_type(),
        align_type(),
        vibrate_type(),
        look_type(),
        protection_type(),
        display_type(),
        compositing_type(),
    ]
}

/// Transform writes of one motion tag.
pub fn evaluate_tag(scene: &Scene, tag: NodeId, time: f64) -> Vec<(NodeId, String, Value)> {
    if !forge_core::layers::expressions_on(scene, tag) {
        return Vec::new();
    }
    let Some(ty) = scene.node(tag).map(|n| n.type_id.as_str()) else {
        return Vec::new();
    };
    match ty {
        TARGET_TAG => target_writes(scene, tag),
        ALIGN_TAG => align_writes(scene, tag),
        VIBRATE_TAG => vibrate_writes(scene, tag, time),
        LOOK_TAG => look_writes(scene, tag),
        PROTECTION_TAG | DISPLAY_TAG | COMPOSITING_TAG => Vec::new(),
        _ => Vec::new(),
    }
}
