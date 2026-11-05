//! Boolean generator (docs/specs/BOOLE.md; docs/parity/boole.md).

use std::collections::BTreeMap;

use forge_core::{Category, EvalCtx, NodeId, NodeType, ParamDesc, Scene, Unit, Value};
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
        .param(ParamDesc::bool("hide_new_edges", "Hide new edges", true))
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
                .range(0.0001, f64::MAX)
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
        .filter(|c| {
            matches!(
                evaluate_geometry(scene, *c, &EvalCtx::default()),
                Ok(Some(_)) | Err(_)
            )
        })
        .take(2)
        .collect()
}

/// Boolean of two meshes already in the generator's space, with the generator's
/// param values (§2–5), as one welded mesh.
pub fn boole(
    a: &PolyMesh,
    b: &PolyMesh,
    values: &BTreeMap<String, Value>,
) -> Result<PolyMesh, BuildError> {
    Ok(run(a, b, values)?.mesh)
}

/// [`boole`] in the R25 export layout (docs/specs/BOOLE.md §5a): A's polygons with their
/// own points, then B's polygons with their own copies of the points.
pub fn boole_objects(
    a: &PolyMesh,
    b: &PolyMesh,
    values: &BTreeMap<String, Value>,
) -> Result<PolyMesh, BuildError> {
    let r = run(a, b, values)?;
    Ok(two_objects(&r.mesh, &r.from_b))
}

fn run(
    a: &PolyMesh,
    b: &PolyMesh,
    values: &BTreeMap<String, Value>,
) -> Result<forge_geometry::boolean::BoolResult, BuildError> {
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
    Ok(boolean(a, b, &opts))
}

/// The R25 layout (docs/specs/BOOLE.md §5a): A's polygons with their own points, then B's
/// polygons with their own copies of the points. Within each object the points keep the
/// welded result's order (surviving input points first, then new points by first use).
fn two_objects(welded: &PolyMesh, from_b: &[bool]) -> PolyMesh {
    let mut out = PolyMesh::default();
    for want_b in [false, true] {
        let mut map = vec![u32::MAX; welded.points.len()];
        let mut used: Vec<u32> = Vec::new();
        for (poly, _) in welded
            .polys
            .iter()
            .zip(from_b)
            .filter(|(_, b)| **b == want_b)
        {
            for &v in poly {
                if map[v as usize] == u32::MAX {
                    map[v as usize] = 0;
                    used.push(v);
                }
            }
        }
        used.sort_unstable();
        let base = out.points.len() as u32;
        for (i, &v) in used.iter().enumerate() {
            map[v as usize] = base + i as u32;
            out.points.push(welded.points[v as usize]);
        }
        for (poly, _) in welded
            .polys
            .iter()
            .zip(from_b)
            .filter(|(_, b)| **b == want_b)
        {
            out.polys
                .push(poly.iter().map(|&v| map[v as usize]).collect());
        }
    }
    out
}

pub(crate) fn evaluate_boole(
    scene: &Scene,
    id: NodeId,
    ctx: &EvalCtx,
) -> Result<Option<PolyMesh>, BuildError> {
    let values = &scene.node(id).expect("boole exists").params;
    let mut meshes = Vec::new();
    for child in inputs_of(scene, id) {
        let Some(mut mesh) = evaluate(scene, child, ctx)? else {
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
