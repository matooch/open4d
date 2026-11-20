//! Mirroring joint chains and weights (docs/specs/RIG.md §1.3, §5).

use forge_core::{EditError, NodeId, Tx, Value};
use glam::{DMat4, DVec3};

use crate::joint::is_joint;
use crate::tools::{decompose, set_transform};

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
