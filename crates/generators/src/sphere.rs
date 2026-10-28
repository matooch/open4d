//! Sphere (docs/specs/PRIMITIVES.md §3; docs/parity/sphere.md).

use std::f64::consts::PI;

use forge_core::{Category, NodeType, ParamDesc, Unit};
use forge_geometry::PolyMesh;
use glam::DVec3;

use crate::{BuildError, Params, platonic};

pub(crate) fn node_type() -> NodeType {
    NodeType::new("forge.sphere", "Sphere", Category::Generator)
        .param(
            ParamDesc::float("radius", "Radius", 100.0)
                .range(0.0, f64::MAX)
                .unit(Unit::Distance),
        )
        .param(ParamDesc::int("segments", "Segments", 16).range(3, 1000))
        .param(ParamDesc::enumeration(
            "type",
            "Type",
            "standard",
            &[
                ("standard", "Standard"),
                ("tetrahedron", "Tetrahedron"),
                ("hexahedron", "Hexahedron"),
                ("octahedron", "Octahedron"),
                ("icosahedron", "Icosahedron"),
                ("hemisphere", "Hemisphere"),
            ],
        ))
        .param(ParamDesc::bool("render_perfect", "Render perfect", true))
}

pub(crate) fn build(p: &Params) -> Result<PolyMesh, BuildError> {
    let r = p.float("radius");
    let bands = p.int("segments").div_ceil(2);
    let meridians = 2 * bands;
    let hemisphere = match p.enumeration("type").as_str() {
        "standard" => false,
        "hemisphere" if bands.is_multiple_of(2) => true,
        "hemisphere" => return Err(BuildError::Unsupported("hemisphere with an odd band count")),
        kind @ ("tetrahedron" | "octahedron" | "icosahedron") => {
            let n = platonic::frequency(kind, p.int("segments"));
            return Ok(platonic::triangles(kind, n, r));
        }
        "hexahedron" => {
            let n = platonic::frequency("hexahedron", p.int("segments"));
            return Ok(platonic::hexahedron(n, r));
        }
        _ => return Err(BuildError::Unsupported("unknown sphere type")),
    };
    let ring_point = |ring: usize, k: usize| {
        let theta = ring as f64 * PI / bands as f64;
        let phi = 2.0 * PI * k as f64 / meridians as f64;
        let rr = r * theta.sin();
        DVec3::new(rr * phi.cos(), -r * theta.cos(), rr * phi.sin())
    };
    let first_ring = if hemisphere { bands / 2 } else { 1 };
    let mut points = Vec::new();
    if !hemisphere {
        points.push(DVec3::new(0.0, -r, 0.0));
    }
    points.push(DVec3::new(0.0, r, 0.0));
    let ring_base = points.len();
    for ring in first_ring..bands {
        points.extend((0..meridians).map(|k| ring_point(ring, k)));
    }
    let v = |ring: usize, k: usize| {
        (ring_base + (ring - first_ring) * meridians + k % meridians) as u32
    };
    let (south, north) = if hemisphere {
        (None, 0)
    } else {
        (Some(0u32), 1)
    };

    let mut polys = Vec::new();
    for ring in first_ring..bands - 1 {
        for k in 0..meridians {
            polys.push(vec![
                v(ring, k),
                v(ring + 1, k),
                v(ring + 1, k + 1),
                v(ring, k + 1),
            ]);
        }
    }
    let top = bands - 1;
    for k in 0..meridians {
        if let Some(s) = south {
            polys.push(vec![s, v(1, k), v(1, k + 1)]);
        }
        polys.push(vec![v(top, k + 1), v(top, k), north]);
    }
    Ok(PolyMesh { points, polys })
}
