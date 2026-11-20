//! The joint object (docs/specs/RIG.md §1).

use forge_core::{Category, NodeId, NodeType, ParamDesc, Scene, Unit, Value};
use glam::{DMat4, DVec3};

/// Type id of the joint object.
pub const JOINT: &str = "forge.joint";

/// A local axis of a joint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Axis {
    X,
    Y,
    Z,
}

impl Axis {
    pub fn unit(self) -> DVec3 {
        match self {
            Axis::X => DVec3::X,
            Axis::Y => DVec3::Y,
            Axis::Z => DVec3::Z,
        }
    }

    /// The enum option id (`"x"`, `"y"`, `"z"`).
    pub fn id(self) -> &'static str {
        match self {
            Axis::X => "x",
            Axis::Y => "y",
            Axis::Z => "z",
        }
    }

    pub fn from_id(id: &str) -> Option<Axis> {
        match id {
            "x" => Some(Axis::X),
            "y" => Some(Axis::Y),
            "z" => Some(Axis::Z),
            _ => None,
        }
    }
}

pub(crate) fn joint_type() -> NodeType {
    NodeType::new(JOINT, "Joint", Category::Object)
        .param(ParamDesc::float("length", "Length", 150.0).unit(Unit::Distance))
        .param(ParamDesc::enumeration(
            "bone",
            "Bone",
            "to_child",
            &[
                ("null", "Null"),
                ("axis", "Axis"),
                ("from_parent", "From Parent"),
                ("to_child", "To Child"),
            ],
        ))
        .param(ParamDesc::enumeration(
            "axis",
            "Axis",
            "z",
            &[("x", "X"), ("y", "Y"), ("z", "Z")],
        ))
        .param(ParamDesc::enumeration(
            "display_object",
            "Display (object)",
            "standard",
            &[
                ("none", "None"),
                ("standard", "Standard"),
                ("box", "Box"),
                ("line", "Line"),
                ("stick", "Stick"),
                ("polygon", "Polygon"),
            ],
        ))
        .param(ParamDesc::enumeration(
            "size_object",
            "Size (object)",
            "length",
            &[("custom", "Custom"), ("length", "Length")],
        ))
        .param(ParamDesc::enumeration(
            "display_joint",
            "Display (joint)",
            "axis",
            &[
                ("none", "None"),
                ("axis", "Axis"),
                ("sphere_wire", "Sphere (Wire)"),
                ("circle", "Circle"),
                ("sphere", "Sphere"),
            ],
        ))
        .param(ParamDesc::enumeration(
            "size_joint",
            "Size (joint)",
            "bone",
            &[("custom", "Custom"), ("bone", "Bone")],
        ))
        .param(ParamDesc::float("kinematics_weight", "Weight", 1.0))
        .param(ParamDesc::float("stretch", "Stretch", 1.0))
        .param(ParamDesc::bool("enabled", "Enabled", true))
}

/// True for a joint node.
pub fn is_joint(scene: &Scene, id: NodeId) -> bool {
    scene.node(id).is_some_and(|n| n.type_id == JOINT)
}

fn enum_param<'a>(scene: &'a Scene, id: NodeId, key: &str) -> &'a str {
    match scene.param(id, key) {
        Some(Value::Enum(s)) => s,
        _ => "",
    }
}

/// The joint's `axis` param (z when missing).
pub fn joint_axis(scene: &Scene, id: NodeId) -> Axis {
    Axis::from_id(enum_param(scene, id, "axis")).unwrap_or(Axis::Z)
}

/// The first child that is a joint.
pub fn first_child_joint(scene: &Scene, id: NodeId) -> Option<NodeId> {
    scene
        .node(id)?
        .children
        .iter()
        .copied()
        .find(|c| is_joint(scene, *c))
}

/// The bone in the joint's local space before its scale (RIG.md §1.1).
pub fn bone_vector(scene: &Scene, id: NodeId) -> DVec3 {
    if enum_param(scene, id, "bone") == "to_child"
        && let Some(c) = first_child_joint(scene, id)
        && let Some(t) = scene.local_transform(c)
    {
        return t.pos;
    }
    let length = match scene.param(id, "length") {
        Some(Value::Float(l)) => *l,
        _ => 150.0,
    };
    joint_axis(scene, id).unit() * length
}

/// Length of [`bone_vector`].
pub fn joint_length(scene: &Scene, id: NodeId) -> f64 {
    bone_vector(scene, id).length()
}

/// `(joint, world matrix)` for `root` and its joint descendants, depth first.
pub fn chain_world_matrices(scene: &Scene, root: NodeId) -> Vec<(NodeId, DMat4)> {
    let mut out = Vec::new();
    if !is_joint(scene, root) {
        return out;
    }
    let mut ids = vec![root];
    ids.extend(
        scene
            .descendants(root)
            .into_iter()
            .filter(|d| is_joint(scene, *d)),
    );
    for id in ids {
        if let Some(m) = scene.world_matrix(id) {
            out.push((id, m));
        }
    }
    out
}
