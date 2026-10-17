//! Plane (docs/specs/PRIMITIVES.md §1; docs/parity/plane.md).

use forge_core::{Category, NodeType, ParamDesc, Unit};
use forge_geometry::PolyMesh;
use glam::DVec3;

use crate::Params;

pub(crate) fn node_type() -> NodeType {
    NodeType::new("forge.plane", "Plane", Category::Generator)
        .param(
            ParamDesc::float("width", "Width", 400.0)
                .range(0.0, f64::MAX)
                .unit(Unit::Distance),
        )
        .param(
            ParamDesc::float("height", "Height", 400.0)
                .range(0.0, f64::MAX)
                .unit(Unit::Distance),
        )
        .param(ParamDesc::int("segments_w", "Width segments", 20).range(1, 1000))
        .param(ParamDesc::int("segments_h", "Height segments", 20).range(1, 1000))
        .param(ParamDesc::enumeration(
            "orientation",
            "Orientation",
            "pos_y",
            &[
                ("pos_x", "+X"),
                ("neg_x", "-X"),
                ("pos_y", "+Y"),
                ("neg_y", "-Y"),
                ("pos_z", "+Z"),
                ("neg_z", "-Z"),
            ],
        ))
}

pub(crate) fn build(p: &Params) -> PolyMesh {
    let (w, h) = (p.float("width"), p.float("height"));
    let (sw, sh) = (p.int("segments_w"), p.int("segments_h"));
    let place: fn(f64, f64) -> DVec3 = match p.enumeration("orientation").as_str() {
        "pos_x" => |u, v| DVec3::new(0.0, -u, v),
        "neg_x" => |u, v| DVec3::new(0.0, u, v),
        "neg_y" => |u, v| DVec3::new(-u, 0.0, v),
        "pos_z" => |u, v| DVec3::new(u, -v, 0.0),
        "neg_z" => |u, v| DVec3::new(u, v, 0.0),
        _ => |u, v| DVec3::new(u, 0.0, v),
    };
    let mut points = Vec::with_capacity((sw + 1) * (sh + 1));
    for j in 0..=sh {
        let v = -h / 2.0 + j as f64 * h / sh as f64;
        for i in 0..=sw {
            let u = -w / 2.0 + i as f64 * w / sw as f64;
            points.push(place(u, v));
        }
    }
    let row = (sw + 1) as u32;
    let mut polys = Vec::with_capacity(sw * sh);
    for j in 0..sh as u32 {
        for i in 0..sw as u32 {
            let a = j * row + i;
            polys.push(vec![a, a + row, a + row + 1, a + 1]);
        }
    }
    PolyMesh { points, polys }
}
