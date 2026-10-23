//! Boolean generator (docs/specs/BOOLE.md; docs/parity/boole.md).

use std::collections::BTreeMap;

use forge_core::{Category, NodeId, NodeType, ParamDesc, Scene, Unit, Value};
use forge_geometry::PolyMesh;
use forge_geometry::boolean::{BoolOp, BoolOptions, boolean};

use crate::{BuildError, Params, evaluate, evaluate_geometry};

pub const TYPE: &str = "forge.boole";

pub(crate) fn node_type() -> NodeType {
    NodeType::new(TYPE, "Boole", Category::Generator)
        .param(ParamDesc::enumeration(
            "mode",
            "Boolean Type",
            "a_minus_b",
            &[
                ("union", "A union B"),
                ("a_minus_b", "A subtract B"),
                ("intersect", "A intersect B"),
                ("a_without_b", "A without B"),
            ],
        ))
        .param(ParamDesc::bool("high_quality", "High Quality", true))
        .param(ParamDesc::bool("hide_new_edges", "Hide new edges", false))
        .param(ParamDesc::bool(
            "single_object",
            "Create single object",
            false,
        ))
        .param(ParamDesc::bool(
            "phong_breaks",
            "Create Phong breaks at intersections",
            false,
        ))
        .param(ParamDesc::bool(
            "select_intersections",
            "Select intersections",
            false,
        ))
        .param(
            ParamDesc::float("optimize_points", "Optimize Points", 0.01)
                .range(0.0, f64::MAX)
                .unit(Unit::Distance),
        )
}

/// The first two hierarchy children with a mesh: A, then B (§1).
pub fn inputs_of(scene: &Scene, id: NodeId) -> Vec<NodeId> {
    let Some(node) = scene.node(id) else {
        return Vec::new();
    };
    node.children
        .iter()
        .copied()
        .filter(|c| matches!(evaluate_geometry(scene, *c), Ok(Some(_)) | Err(_)))
        .take(2)
        .collect()
}

/// Boolean of two meshes already in the generator's space, with the generator's
/// param values (§2–5).
pub fn boole(
    a: &PolyMesh,
    b: &PolyMesh,
    values: &BTreeMap<String, Value>,
) -> Result<PolyMesh, BuildError> {
    let ty = node_type();
    let p = Params { ty: &ty, values };
    if !p.bool("high_quality") {
        return Err(BuildError::Unsupported("boolean with High Quality off"));
    }
    let op = match p.enumeration("mode").as_str() {
        "union" => BoolOp::Union,
        "intersect" => BoolOp::Intersect,
        "a_without_b" => BoolOp::Without,
        _ => BoolOp::Subtract,
    };
    let opts = BoolOptions {
        op,
        hide_new_edges: p.bool("hide_new_edges"),
    };
    Ok(boolean(a, b, &opts).mesh)
}

pub(crate) fn evaluate_boole(scene: &Scene, id: NodeId) -> Result<Option<PolyMesh>, BuildError> {
    let values = &scene.node(id).expect("boole exists").params;
    let mut meshes = Vec::new();
    for child in inputs_of(scene, id) {
        let Some(mut mesh) = evaluate(scene, child)? else {
            continue;
        };
        let m = scene.local_transform(child).expect("child exists").matrix();
        for pt in &mut mesh.points {
            *pt = m.transform_point3(*pt);
        }
        meshes.push(mesh);
    }
    match meshes.as_slice() {
        [] => Ok(None),
        [a] => Ok(Some(a.clone())),
        [a, b, ..] => boole(a, b, values).map(Some),
    }
}
