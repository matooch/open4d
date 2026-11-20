//! Mirroring joint chains and weights (docs/specs/RIG.md §1.3, §5).

use forge_core::{EditError, NodeId, Scene, Tx, Value};
use glam::{DMat4, DVec3};

use crate::joint::is_joint;
use crate::tools::{decompose, set_transform};
use crate::weights::{WeightMap, read_weights, tag_joints};

/// The mirror plane through the world origin.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Plane {
    /// Reflects x.
    Yz,
    /// Reflects y.
    Xz,
    /// Reflects z.
    Xy,
}

impl Plane {
    /// The reflection matrix `S`.
    pub fn reflection(self) -> DMat4 {
        DMat4::from_scale(match self {
            Plane::Yz => DVec3::new(-1.0, 1.0, 1.0),
            Plane::Xz => DVec3::new(1.0, -1.0, 1.0),
            Plane::Xy => DVec3::new(1.0, 1.0, -1.0),
        })
    }
}

/// How the mirrored joints are named.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenameRule {
    /// Swap left and right (see [`mirror_name`]); append `_mirror` when the name has neither.
    LeftRight,
    /// Append the suffix.
    Suffix(String),
}

fn swap_word(s: &str) -> Option<String> {
    const PAIRS: [(&str, &str); 6] = [
        ("Left", "Right"),
        ("left", "right"),
        ("LEFT", "RIGHT"),
        ("Right", "Left"),
        ("right", "left"),
        ("RIGHT", "LEFT"),
    ];
    PAIRS
        .iter()
        .find(|(a, _)| s.contains(a))
        .map(|(a, b)| s.replacen(a, b, 1))
}

fn swap_token(s: &str) -> Option<String> {
    let is_delim = |c: char| matches!(c, '_' | '.' | '-' | ' ');
    let mut out = String::new();
    let mut token = String::new();
    let mut done = false;
    let flush = |token: &mut String, out: &mut String, done: &mut bool| {
        if !*done {
            let swapped = match token.as_str() {
                "L" => Some("R"),
                "R" => Some("L"),
                "l" => Some("r"),
                "r" => Some("l"),
                _ => None,
            };
            if let Some(w) = swapped {
                out.push_str(w);
                *done = true;
                token.clear();
                return;
            }
        }
        out.push_str(token);
        token.clear();
    };
    for c in s.chars() {
        if is_delim(c) {
            flush(&mut token, &mut out, &mut done);
            out.push(c);
        } else {
            token.push(c);
        }
    }
    flush(&mut token, &mut out, &mut done);
    done.then_some(out)
}

/// The mirrored name: a `Left`/`Right` word is swapped (case kept); otherwise a lone `L` / `R`
/// token between `_`, `.`, `-` or spaces is swapped; otherwise `_mirror` is appended.
pub fn mirror_name(name: &str, rule: &RenameRule) -> String {
    match rule {
        RenameRule::Suffix(s) => format!("{name}{s}"),
        RenameRule::LeftRight => swap_word(name)
            .or_else(|| swap_token(name))
            .unwrap_or_else(|| format!("{name}_mirror")),
    }
}

/// Copies the joint subtree of `root` next to it, reflected through `plane`: every copy has the
/// world matrix `S · M · S`, so rotations stay proper (no negative scale). Other children
/// (non-joints) are not copied. Returns the new root.
///
/// # Errors
/// Fails for a node that is not a joint.
pub fn mirror_chain(
    tx: &mut Tx<'_>,
    root: NodeId,
    plane: Plane,
    rename: &RenameRule,
) -> Result<NodeId, EditError> {
    if !is_joint(tx.scene(), root) {
        return Err(EditError::UnknownNode(root));
    }
    let s = plane.reflection();
    let parent = tx.scene().node(root).and_then(|n| n.parent);
    let siblings = match parent {
        Some(p) => tx.scene().node(p).map(|n| n.children.clone()),
        None => Some(tx.scene().roots().to_vec()),
    }
    .unwrap_or_default();
    let index = siblings.iter().position(|c| *c == root).map(|i| i + 1);
    let parent_world = tx
        .scene()
        .parent_world_matrix(root)
        .unwrap_or(DMat4::IDENTITY);
    mirror_node(tx, root, parent, index, parent_world, &s, rename)
}

fn mirror_node(
    tx: &mut Tx<'_>,
    src: NodeId,
    parent: Option<NodeId>,
    index: Option<usize>,
    parent_world: DMat4,
    s: &DMat4,
    rename: &RenameRule,
) -> Result<NodeId, EditError> {
    let scene = tx.scene();
    let node = scene.node(src).ok_or(EditError::UnknownNode(src))?;
    let ty = node.type_id.clone();
    let name = mirror_name(&node.name, rename);
    let params: Vec<(String, Value)> = node
        .params
        .iter()
        .filter(|(k, _)| !k.starts_with(forge_core::transform::PREFIX))
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    let world = scene.world_matrix(src).unwrap_or(DMat4::IDENTITY);
    let kids: Vec<NodeId> = node
        .children
        .iter()
        .copied()
        .filter(|c| is_joint(scene, *c))
        .collect();
    let new_world = *s * world * *s;
    let id = tx.create(&ty, parent, index)?;
    tx.rename(id, &name)?;
    for (k, v) in params {
        tx.set_param(id, &k, v)?;
    }
    set_transform(tx, id, &decompose(&(parent_world.inverse() * new_world)))?;
    for k in kids {
        mirror_node(tx, k, Some(id), None, new_world, s, rename)?;
    }
    Ok(id)
}

/// The side of the plane weights are copied from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    /// Points with a positive coordinate along the plane normal (x for `Yz`).
    Positive,
    /// Points with a negative coordinate.
    Negative,
}

impl Plane {
    /// Index of the reflected coordinate (0 = x, 1 = y, 2 = z).
    fn axis(self) -> usize {
        match self {
            Plane::Yz => 0,
            Plane::Xz => 1,
            Plane::Xy => 2,
        }
    }
}

/// For every slot of `joints`, the slot of its mirror twin: the joint named
/// `mirror_name(name, rename)`. Joints without a twin (centre joints) map to themselves.
pub fn joint_mirror_map(scene: &Scene, joints: &[NodeId], rename: &RenameRule) -> Vec<usize> {
    let names: Vec<&str> = joints
        .iter()
        .map(|j| scene.node(*j).map_or("", |n| n.name.as_str()))
        .collect();
    names
        .iter()
        .enumerate()
        .map(|(slot, n)| {
            let twin = mirror_name(n, rename);
            names
                .iter()
                .position(|m| *m == twin && *m != *n)
                .unwrap_or(slot)
        })
        .collect()
}

/// What [`mirror_weights`] did.
#[derive(Debug, Clone, PartialEq)]
pub struct MirrorReport {
    /// The weights after mirroring.
    pub weights: WeightMap,
    /// Target-side points that received a copy.
    pub copied: usize,
    /// Target-side points without a source point within `max_distance` of their reflection;
    /// their weights are unchanged.
    pub unmatched: usize,
}

/// Copies weights across `plane`: every point on the side opposite to `from` takes the weights
/// of the nearest point (to its reflection) on the `from` side, with joint slots renamed by
/// `slot_map` ([`joint_mirror_map`]). Points within `centre_tolerance` of the plane keep their
/// weights. A reflection with no source point within `max_distance` leaves the point alone.
/// Nearest-point search is brute force (`O(n²)`).
pub fn mirror_weights(
    points: &[DVec3],
    weights: &WeightMap,
    slot_map: &[usize],
    plane: Plane,
    from: Side,
    centre_tolerance: f64,
    max_distance: f64,
) -> MirrorReport {
    let axis = plane.axis();
    let sign = match from {
        Side::Positive => 1.0,
        Side::Negative => -1.0,
    };
    let source: Vec<usize> = (0..points.len())
        .filter(|i| sign * points[*i][axis] > centre_tolerance)
        .collect();
    let mut out = weights.clone();
    let (mut copied, mut unmatched) = (0, 0);
    for (i, p) in points.iter().enumerate() {
        if sign * p[axis] >= -centre_tolerance {
            continue; // source side or on the plane
        }
        let mut reflected = *p;
        reflected[axis] = -reflected[axis];
        let nearest = source
            .iter()
            .map(|s| ((points[*s] - reflected).length(), *s))
            .min_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
        match nearest {
            Some((d, s)) if d <= max_distance => {
                let row: Vec<(u32, f64)> = weights
                    .get(s as u32)
                    .iter()
                    .map(|(slot, w)| {
                        (
                            slot_map
                                .get(*slot as usize)
                                .copied()
                                .unwrap_or(*slot as usize) as u32,
                            *w,
                        )
                    })
                    .collect();
                out.set(i as u32, row);
                copied += 1;
            }
            _ => unmatched += 1,
        }
    }
    MirrorReport {
        weights: out,
        copied,
        unmatched,
    }
}

/// Mirrors the weights stored on `tag` across `plane` for the host's unskinned `mesh`; the twin
/// joints are found by name among the tag's joints. Stores the result and returns the report.
///
/// # Errors
/// Fails for a node that is not a weight tag with a host.
pub fn mirror_tag_weights(
    tx: &mut Tx<'_>,
    tag: NodeId,
    mesh: &forge_geometry::PolyMesh,
    plane: Plane,
    rename: &RenameRule,
    from: Side,
) -> Result<MirrorReport, EditError> {
    let scene = tx.scene();
    if scene.node(tag).and_then(|n| n.host).is_none() {
        return Err(EditError::UnknownNode(tag));
    }
    let joints = tag_joints(scene, tag);
    let map = joint_mirror_map(scene, &joints, rename);
    let weights = read_weights(scene, tag);
    let (lo, hi) = mesh.points.iter().fold(
        (DVec3::splat(f64::MAX), DVec3::splat(f64::MIN)),
        |(l, h), p| (l.min(*p), h.max(*p)),
    );
    let diag = (hi - lo).length().max(1e-9);
    let report = mirror_weights(
        &mesh.points,
        &weights,
        &map,
        plane,
        from,
        1e-6 * diag,
        1e-3 * diag,
    );
    tx.set_param(tag, "weights", report.weights.to_value())?;
    Ok(report)
}

/// Appends `joints` to the tag's joint list (skipping ones already listed); returns their
/// slots.
///
/// # Errors
/// Fails when the tag does not exist.
pub fn add_joints_to_tag(
    tx: &mut Tx<'_>,
    tag: NodeId,
    joints: &[NodeId],
) -> Result<Vec<usize>, EditError> {
    let mut list = tag_joints(tx.scene(), tag);
    let slots = joints
        .iter()
        .map(|j| match list.iter().position(|x| x == j) {
            Some(s) => s,
            None => {
                list.push(*j);
                list.len() - 1
            }
        })
        .collect();
    tx.set_param(tag, "joints", Value::LinkList(list))?;
    Ok(slots)
}
