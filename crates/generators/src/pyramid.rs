//! Pyramid (docs/specs/PRIMITIVES2.md §9; docs/parity/pyramid.md).

use forge_core::{Category, NodeType, ParamDesc, Unit};
use forge_geometry::PolyMesh;
use glam::DVec3;

use crate::lathe::{orientation_param, oriented};
use crate::platonic::flat_subdivision;
use crate::{BuildError, Params};

pub(crate) fn node_type() -> NodeType {
    let size = |id: &str, label: &str| {
        ParamDesc::float(id, label, 200.0)
            .range(0.0, f64::MAX)
            .unit(Unit::Distance)
    };
    NodeType::new("forge.pyramid", "Pyramid", Category::Generator)
        .param(size("size_x", "Size X"))
        .param(size("size_y", "Size Y"))
        .param(size("size_z", "Size Z"))
        .param(ParamDesc::int("segments", "Segments", 1).range(1, 1000))
        .param(orientation_param())
}

pub(crate) fn build(p: &Params) -> Result<PolyMesh, BuildError> {
    let (x, y, z) = (
        p.float("size_x") / 2.0,
        p.float("size_y") / 2.0,
        p.float("size_z") / 2.0,
    );
    let base = [
        DVec3::new(-x, -y, -z),
        DVec3::new(x, -y, -z),
        DVec3::new(x, -y, z),
        DVec3::new(-x, -y, z),
        DVec3::new(0.0, y, 0.0),
    ];
    let faces = [
        vec![0, 1, 2, 3],
        vec![4, 2, 1],
        vec![4, 3, 2],
        vec![4, 0, 3],
        vec![4, 1, 0],
    ];
    let mesh = flat_subdivision(&base, &faces, p.int("segments"), |q| q);
    Ok(oriented(p, mesh))
}
