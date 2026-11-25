//! The Pose Morph tag: a base pose plus named point targets with animatable strengths
//! (docs/specs/RIG.md section 15; own design D20, parity-pending O37).

use forge_core::{Category, EditError, NodeId, NodeType, ParamDesc, Scene, Tx, Unit, Value};
use glam::DVec3;

/// Type id of the Pose Morph tag.
pub const POSE_MORPH_TAG: &str = "forge.pose_morph_tag";

/// Number of target slots of one tag.
pub const MAX_TARGETS: usize = 16;

/// `name_i`
pub fn name_id(slot: usize) -> String {
    format!("name_{slot}")
}

/// `target_i`
pub fn target_id(slot: usize) -> String {
    format!("target_{slot}")
}

/// `strength_i`
pub fn strength_id(slot: usize) -> String {
    format!("strength_{slot}")
}

pub(crate) fn pose_morph_tag_type() -> NodeType {
    let mut ty = NodeType::new(POSE_MORPH_TAG, "Pose Morph", Category::Tag)
        .param(ParamDesc::bool("enabled", "Enabled", true))
        .param(ParamDesc::enumeration(
            "mode",
            "Mode",
            "relative",
            &[("relative", "Relative"), ("absolute", "Absolute")],
        ))
        .param(ParamDesc::vec3_list("base", "Base pose").hidden());
    for i in 0..MAX_TARGETS {
        let group = format!("Morph {}", i + 1);
        ty = ty
            .param(
                ParamDesc::string(&name_id(i), "Name", &format!("Morph {}", i + 1)).group(&group),
            )
            .param(ParamDesc::vec3_list(&target_id(i), "Target points").hidden())
            .param(
                ParamDesc::float(&strength_id(i), "Strength", 0.0)
                    .unit(Unit::Percent)
                    .group(&group),
            );
    }
    ty
}

/// How the targets combine (RIG.md section 15.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Relative,
    Absolute,
}

/// The morph itself, independent of the scene: `points` become `c + Σ s_i · (t_i − b)`
/// (relative) or `c + Σ s_i · (t_i − c)` (absolute). `targets` pairs each target with its strength;
/// targets of another point count and zero strengths are skipped, and nothing happens when `base`
/// has another point count than `points`.
pub fn morph_points(points: &mut [DVec3], base: &[DVec3], targets: &[(&[DVec3], f64)], mode: Mode) {
    if base.len() != points.len() {
        return;
    }
    let active: Vec<&(&[DVec3], f64)> = targets
        .iter()
        .filter(|(t, s)| t.len() == points.len() && *s != 0.0 && s.is_finite())
        .collect();
    if active.is_empty() {
        return;
    }
    for (k, p) in points.iter_mut().enumerate() {
        let c = *p;
        let mut sum = DVec3::ZERO;
        for (t, s) in &active {
            sum += match mode {
                Mode::Relative => (t[k] - base[k]) * *s,
                Mode::Absolute => (t[k] - c) * *s,
            };
        }
        *p = c + sum;
    }
}

fn list(scene: &Scene, tag: NodeId, id: &str) -> Vec<DVec3> {
    match scene.param(tag, id) {
        Some(Value::Vec3List(l)) => l.iter().map(|p| DVec3::from_array(*p)).collect(),
        _ => Vec::new(),
    }
}

/// The first Pose Morph tag of `object`.
pub fn find_pose_morph_tag(scene: &Scene, object: NodeId) -> Option<NodeId> {
    scene
        .node(object)?
        .tags
        .iter()
        .copied()
        .find(|t| scene.node(*t).is_some_and(|n| n.type_id == POSE_MORPH_TAG))
}

/// True once the tag has a base pose.
pub fn has_base(scene: &Scene, tag: NodeId) -> bool {
    !list(scene, tag, "base").is_empty()
}

/// The slots of `tag` that hold a target, ascending.
pub fn used_slots(scene: &Scene, tag: NodeId) -> Vec<usize> {
    (0..MAX_TARGETS)
        .filter(|i| {
            matches!(scene.param(tag, &target_id(*i)), Some(Value::Vec3List(l)) if !l.is_empty())
        })
        .collect()
}

/// The strength of one slot as stored (a fraction, 1.0 = 100 %).
pub fn strength(scene: &Scene, tag: NodeId, slot: usize) -> f64 {
    match scene.param(tag, &strength_id(slot)) {
        Some(Value::Float(s)) => *s,
        _ => 0.0,
    }
}

/// Applies the object's Pose Morph tag to `points` (the object's own points, before its
/// deformers; RIG.md section 15.3). Does nothing without an enabled tag.
pub fn apply_pose_morph(scene: &Scene, object: NodeId, points: &mut [DVec3]) {
    let Some(tag) = find_pose_morph_tag(scene, object) else {
        return;
    };
    if matches!(scene.param(tag, "enabled"), Some(Value::Bool(false))) {
        return;
    }
    let mode = match scene.param(tag, "mode") {
        Some(Value::Enum(m)) if m == "absolute" => Mode::Absolute,
        _ => Mode::Relative,
    };
    let base = list(scene, tag, "base");
    let stored: Vec<(Vec<DVec3>, f64)> = used_slots(scene, tag)
        .into_iter()
        .map(|i| (list(scene, tag, &target_id(i)), strength(scene, tag, i)))
        .collect();
    let targets: Vec<(&[DVec3], f64)> = stored.iter().map(|(t, s)| (t.as_slice(), *s)).collect();
    morph_points(points, &base, &targets, mode);
}

fn entries(points: &[DVec3]) -> Value {
    Value::Vec3List(points.iter().map(|p| p.to_array()).collect())
}

fn require_tag(tx: &Tx<'_>, tag: NodeId) -> Result<(), EditError> {
    match tx.scene().node(tag) {
        Some(n) if n.type_id == POSE_MORPH_TAG => Ok(()),
        _ => Err(EditError::UnknownNode(tag)),
    }
}

fn require_slot(slot: usize) -> Result<(), EditError> {
    if slot < MAX_TARGETS {
        Ok(())
    } else {
        Err(EditError::InvalidLink(format!(
            "slot {slot} is out of range (0..{MAX_TARGETS})"
        )))
    }
}

/// Stores `points` as the base pose.
///
/// # Errors
/// Fails for a node that is not a Pose Morph tag.
pub fn capture_base(tx: &mut Tx<'_>, tag: NodeId, points: &[DVec3]) -> Result<(), EditError> {
    require_tag(tx, tag)?;
    tx.set_param(tag, "base", entries(points))
}

/// Makes a target from `points` in the first free slot and returns the slot.
///
/// # Errors
/// Fails for a node that is not a Pose Morph tag, a tag without a base pose
/// ([`capture_base`] first), with every slot in use, or when `points` has another count than
/// the base.
pub fn add_target(
    tx: &mut Tx<'_>,
    tag: NodeId,
    name: Option<&str>,
    points: &[DVec3],
) -> Result<usize, EditError> {
    require_tag(tx, tag)?;
    let scene = tx.scene();
    let base_len = list(scene, tag, "base").len();
    let slot = (0..MAX_TARGETS)
        .find(|i| !used_slots(scene, tag).contains(i))
        .ok_or_else(|| EditError::InvalidLink(format!("all {MAX_TARGETS} targets are in use")))?;
    if base_len == 0 {
        return Err(EditError::InvalidLink(
            "the tag has no base pose yet".into(),
        ));
    }
    if base_len != points.len() {
        return Err(EditError::InvalidLink(format!(
            "the target has {} points, the base {base_len}",
            points.len()
        )));
    }
    tx.set_param(tag, &target_id(slot), entries(points))?;
    if let Some(n) = name {
        tx.set_param(tag, &name_id(slot), Value::String(n.to_string()))?;
    }
    Ok(slot)
}

/// Replaces the points of target `slot`.
///
/// # Errors
/// Fails for a bad slot, an unused slot, or a point count that differs from the base.
pub fn set_target(
    tx: &mut Tx<'_>,
    tag: NodeId,
    slot: usize,
    points: &[DVec3],
) -> Result<(), EditError> {
    require_tag(tx, tag)?;
    require_slot(slot)?;
    if !used_slots(tx.scene(), tag).contains(&slot) {
        return Err(EditError::InvalidLink(format!("slot {slot} is empty")));
    }
    if list(tx.scene(), tag, "base").len() != points.len() {
        return Err(EditError::InvalidLink(
            "the target must have as many points as the base".into(),
        ));
    }
    tx.set_param(tag, &target_id(slot), entries(points))
}

/// Empties target `slot` and resets its name and strength.
///
/// # Errors
/// Fails for a bad slot or a node that is not a Pose Morph tag.
pub fn clear_target(tx: &mut Tx<'_>, tag: NodeId, slot: usize) -> Result<(), EditError> {
    require_tag(tx, tag)?;
    require_slot(slot)?;
    tx.set_param(tag, &target_id(slot), Value::Vec3List(Vec::new()))?;
    tx.set_param(
        tag,
        &name_id(slot),
        Value::String(format!("Morph {}", slot + 1)),
    )?;
    tx.set_param(tag, &strength_id(slot), Value::Float(0.0))
}
