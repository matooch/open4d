//! The Polygon primitive (docs/specs/PRIMITIVES3.md §3; docs/parity/primitives-3.md).

use forge_core::{Category, NodeType, ParamDesc};
use forge_geometry::PolyMesh;
use glam::DVec3;

use crate::lathe::{orientation_param, oriented};
use crate::platonic::flat_subdivision;
use crate::round::{count, distance};
use crate::{BuildError, Params};

pub(crate) fn node_type() -> NodeType {
    NodeType::new("forge.polygon_prim", "Polygon", Category::Generator)
        .param(distance("width", "Width", 100.0))
        .param(distance("height", "Height", 100.0))
        .param(count("segments", "Segments", 1, 1))
        .param(ParamDesc::bool("triangle", "Triangle", false))
        .param(orientation_param())
}

pub(crate) fn build(p: &Params) -> Result<PolyMesh, BuildError> {
    let (x, z) = (p.float("width") / 2.0, p.float("height") / 2.0);
    let (base, face) = if p.bool("triangle") {
        (
            vec![
                DVec3::new(-x, 0.0, -z),
                DVec3::new(0.0, 0.0, z),
                DVec3::new(x, 0.0, -z),
            ],
            vec![0, 1, 2],
        )
    } else {
        (
            vec![
                DVec3::new(-x, 0.0, -z),
                DVec3::new(-x, 0.0, z),
                DVec3::new(x, 0.0, z),
                DVec3::new(x, 0.0, -z),
            ],
            vec![0, 1, 2, 3],
        )
    };
    let mesh = flat_subdivision(&base, &[face], p.int("segments"), |q| q);
    Ok(oriented(p, mesh))
}
