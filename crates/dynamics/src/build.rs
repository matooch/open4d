//! Reading bodies out of a scene (docs/specs/DYNAMICS.md §1.2).

use forge_core::{NodeId, Scene};
use forge_geometry::PolyMesh;
use glam::DVec3;

use crate::nodes::{self, RIGID_BODY};
use crate::sim::{BodyKey, BodyKind, BodySpec, MassSpec, ShapeKind};

/// True when the tag makes its object a soft body (Soft Body: Made of Polygons/Lines on a
/// dynamic body).
pub fn is_soft(scene: &Scene, tag: NodeId) -> bool {
    nodes::choice(scene, tag, "soft_body") == "polygons"
        && kind_of(scene, tag) == Some(BodyKind::Dynamic)
}

/// The first rigid body tag of `host`, including a disabled tag.
pub fn body_tag(scene: &Scene, host: NodeId) -> Option<NodeId> {
    scene
        .node(host)?
        .tags
        .iter()
        .copied()
        .find(|t| scene.node(*t).is_some_and(|n| n.type_id == RIGID_BODY))
}

/// Every object in document order (depth first).
pub fn objects_in_order(scene: &Scene) -> Vec<NodeId> {
    let mut out = Vec::new();
    let mut stack: Vec<NodeId> = scene.roots().iter().rev().copied().collect();
    while let Some(id) = stack.pop() {
        if let Some(n) = scene.node(id) {
            out.push(id);
            stack.extend(n.children.iter().rev());
        }
    }
    out
}

/// The kind a tag asks for; `None` for Off-with-disabled, Ghost and a disabled tag (no body).
pub fn kind_of(scene: &Scene, tag: NodeId) -> Option<BodyKind> {
    if !nodes::boolean(scene, tag, "enabled") {
        return None;
    }
    match nodes::choice(scene, tag, "dynamic") {
        "off" => Some(BodyKind::Static),
        "on" => Some(BodyKind::Dynamic),
        "kinematic" => Some(BodyKind::Kinematic),
        // Ghost is not derived (DYNAMICS.md §1.3): it takes no part in the simulation.
        _ => None,
    }
}

pub(crate) fn shape_of(scene: &Scene, tag: NodeId) -> ShapeKind {
    match nodes::choice(scene, tag, "shape") {
        "box" => ShapeKind::Box,
        "sphere" => ShapeKind::Sphere,
        "convex_hull" => ShapeKind::ConvexHull,
        "static_mesh" => ShapeKind::StaticMesh,
        _ => ShapeKind::Automatic,
    }
}

/// The body description a tag gives `mesh` at `world`.
pub fn spec_from_tag(
    scene: &Scene,
    tag: NodeId,
    key: BodyKey,
    kind: BodyKind,
    world: glam::DMat4,
    mesh: PolyMesh,
) -> BodySpec {
    let mass = match nodes::choice(scene, tag, "mass_mode") {
        "custom_density" => MassSpec::Density(nodes::float(scene, tag, "density")),
        "custom_mass" => MassSpec::Mass(nodes::float(scene, tag, "mass")),
        _ => MassSpec::Density(1.0),
    };
    let custom = nodes::boolean(scene, tag, "custom_initial_velocity");
    let vel = |k: &str| {
        if custom {
            DVec3::from_array(nodes::vec3(scene, tag, k))
        } else {
            DVec3::ZERO
        }
    };
    BodySpec {
        key,
        world,
        kind,
        shape: shape_of(scene, tag),
        mesh,
        size_increment: nodes::float(scene, tag, "size_increment"),
        bounce: nodes::float(scene, tag, "bounce"),
        friction: nodes::float(scene, tag, "friction"),
        mass,
        linear_damping: nodes::float(scene, tag, "linear_damping"),
        angular_damping: nodes::float(scene, tag, "angular_damping"),
        linear_velocity: vel("initial_linear_velocity"),
        angular_velocity: vel("initial_angular_velocity"),
    }
}

/// The bodies of the scene in document order. `mesh_of` gives an object's evaluated mesh in its
/// own space (the caller owns evaluation, so this crate needs no generators). Objects without
/// geometry are skipped.
pub fn bodies_of_scene(
    scene: &Scene,
    mesh_of: &dyn Fn(NodeId) -> Option<PolyMesh>,
) -> Vec<BodySpec> {
    let mut out = Vec::new();
    for id in objects_in_order(scene) {
        if crate::clones::is_cloner(scene, id) {
            continue; // its clones are the bodies (clones.rs)
        }
        let Some(tag) = body_tag(scene, id) else {
            continue;
        };
        let Some(kind) = kind_of(scene, tag) else {
            continue;
        };
        if is_soft(scene, tag) {
            continue; // simulated by the soft body solver (soft_of_scene)
        }
        let Some(mesh) = mesh_of(id) else { continue };
        let Some(world) = scene.world_matrix(id) else {
            continue;
        };
        let mut spec = spec_from_tag(scene, tag, BodyKey::Node(id), kind, world, mesh);
        // Automatic shape on a sphere primitive is an exact ball (DYNAMICS.md §1.4).
        if spec.shape == ShapeKind::Automatic
            && scene.node(id).is_some_and(|n| n.type_id == "forge.sphere")
        {
            spec.shape = ShapeKind::Sphere;
        }
        out.push(spec);
    }
    out
}

/// The soft bodies of the scene in document order: every object whose tag is a dynamic soft
/// body, with its evaluated mesh placed in world space.
pub fn soft_of_scene(
    scene: &Scene,
    mesh_of: &dyn Fn(NodeId) -> Option<PolyMesh>,
) -> Vec<crate::soft::SoftSpec> {
    let mut out = Vec::new();
    for id in objects_in_order(scene) {
        let Some(tag) = body_tag(scene, id) else {
            continue;
        };
        if !is_soft(scene, tag) || crate::clones::is_cloner(scene, id) {
            continue;
        }
        let (Some(mesh), Some(world)) = (mesh_of(id), scene.world_matrix(id)) else {
            continue;
        };
        let f = |k: &str| nodes::float(scene, tag, k);
        out.push(crate::soft::SoftSpec {
            key: BodyKey::Node(id),
            points: mesh
                .points
                .iter()
                .map(|p| world.transform_point3(*p))
                .collect(),
            polys: mesh.polys,
            params: crate::soft::SoftParams {
                structural: f("structural"),
                structural_damping: f("structural_damping"),
                shear: f("shear"),
                shear_damping: f("shear_damping"),
                flexion: f("flexion"),
                flexion_damping: f("flexion_damping"),
                keep_shape: nodes::boolean(scene, tag, "keep_shape"),
                friction: f("friction"),
                bounce: f("bounce"),
                linear_damping: f("linear_damping"),
            },
        });
    }
    out
}
