//! Subdivision surface generator (docs/specs/SUBDIVISION.md; docs/parity/subdivision.md).

use forge_core::{Category, NodeId, NodeType, ParamDesc, Scene};
use forge_geometry::PolyMesh;
use std::collections::BTreeSet;

use forge_geometry::Edge;
use forge_geometry::subdivide::{subdivide_levels_with_creases, subdivide_osd};

use crate::{BuildError, Params, evaluate, evaluate_geometry};

pub const TYPE: &str = "forge.subdivision";

pub(crate) fn node_type() -> NodeType {
    NodeType::new(TYPE, "Subdivision", Category::Generator)
        .param(ParamDesc::int("level_view", "Viewport level", 2).range(0, 6))
        .param(ParamDesc::int("level_render", "Render level", 3).range(0, 6))
        .param(ParamDesc::enumeration(
            "scheme",
            "Scheme",
            "catmull_clark_ngons",
            &[
                ("catmull_clark", "Catmull-Clark"),
                ("catmull_clark_ngons", "Catmull-Clark (n-gons)"),
                ("osd_catmull_clark", "OpenSubdiv Catmull-Clark"),
                (
                    "osd_catmull_clark_adaptive",
                    "OpenSubdiv Catmull-Clark (adaptive)",
                ),
                ("osd_loop", "OpenSubdiv Loop"),
                ("osd_bilinear", "OpenSubdiv bilinear"),
            ],
        ))
        .param(ParamDesc::enumeration(
            "subdivide_uvs",
            "Subdivide UVs",
            "standard",
            &[
                ("standard", "Standard"),
                ("boundary", "Boundary"),
                ("edge", "Edge"),
            ],
        ))
}

/// `level` steps of the scheme (§3).
pub fn subdivide(mesh: &PolyMesh, level: usize, scheme: &str) -> Result<PolyMesh, BuildError> {
    subdivide_with_creases(mesh, level, scheme, &BTreeSet::new())
}

/// [`subdivide`] with sharp input edges (SUBDIVISION.md §2).
pub fn subdivide_with_creases(
    mesh: &PolyMesh,
    level: usize,
    scheme: &str,
    creases: &BTreeSet<Edge>,
) -> Result<PolyMesh, BuildError> {
    let smooth = match scheme {
        "catmull_clark" | "catmull_clark_ngons" => true,
        // §3.1: OpenSubdiv numbers its output level by level.
        "osd_catmull_clark" => return Ok(subdivide_osd(mesh, level, true, creases)),
        "osd_bilinear" => return Ok(subdivide_osd(mesh, level, false, creases)),
        // Observed: the export ignores the adaptive level and keeps the input.
        "osd_catmull_clark_adaptive" => return Ok(mesh.clone()),
        "osd_loop" => return Err(BuildError::Unsupported("Loop subdivision")),
        _ => return Err(BuildError::Unsupported("unknown subdivision scheme")),
    };
    Ok(subdivide_levels_with_creases(mesh, level, smooth, creases))
}

/// The child the generator subdivides: its first hierarchy child with a mesh.
pub fn input_of(scene: &Scene, id: NodeId) -> Option<NodeId> {
    let node = scene.node(id)?;
    node.children
        .iter()
        .copied()
        .find(|c| matches!(evaluate_geometry(scene, *c), Ok(Some(_)) | Err(_)))
}

pub(crate) fn evaluate_subdivision(
    scene: &Scene,
    id: NodeId,
    p: &Params,
) -> Result<Option<PolyMesh>, BuildError> {
    let Some(child) = input_of(scene, id) else {
        return Ok(None);
    };
    let Some(mut mesh) = evaluate(scene, child)? else {
        return Ok(None);
    };
    let m = scene.local_transform(child).expect("child exists").matrix();
    for pt in &mut mesh.points {
        *pt = m.transform_point3(*pt);
    }
    subdivide(&mesh, p.int("level_view"), &p.enumeration("scheme")).map(Some)
}
