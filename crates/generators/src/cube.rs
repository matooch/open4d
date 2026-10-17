//! Cube (docs/specs/PRIMITIVES.md §2; docs/parity/cube.md).

use forge_core::{Category, NodeType, ParamDesc, Unit};
use forge_geometry::PolyMesh;
use glam::DVec3;

use crate::{BuildError, Params};

pub(crate) fn node_type() -> NodeType {
    let size = |id: &str, label: &str| {
        ParamDesc::float(id, label, 200.0)
            .range(0.0, f64::MAX)
            .unit(Unit::Distance)
    };
    let segs = |id: &str, label: &str| ParamDesc::int(id, label, 1).range(1, 1000);
    NodeType::new("forge.cube", "Cube", Category::Generator)
        .param(size("size_x", "Size X"))
        .param(size("size_y", "Size Y"))
        .param(size("size_z", "Size Z"))
        .param(segs("segments_x", "Segments X"))
        .param(segs("segments_y", "Segments Y"))
        .param(segs("segments_z", "Segments Z"))
        .param(ParamDesc::bool(
            "separate_surfaces",
            "Separate surfaces",
            false,
        ))
        .param(ParamDesc::bool("fillet", "Fillet", false))
        .param(
            ParamDesc::float("fillet_radius", "Fillet radius", 40.0)
                .range(0.0, f64::MAX)
                .unit(Unit::Distance),
        )
        .param(ParamDesc::int("fillet_segments", "Fillet segments", 5).range(1, 1000))
}

pub(crate) fn build(p: &Params) -> Result<PolyMesh, BuildError> {
    if p.bool("fillet") {
        return Err(BuildError::Unsupported("cube fillet"));
    }
    if p.bool("separate_surfaces") {
        return Err(BuildError::Unsupported("cube separate surfaces"));
    }
    let (hx, hy, hz) = (
        p.float("size_x") / 2.0,
        p.float("size_y") / 2.0,
        p.float("size_z") / 2.0,
    );
    let (sx, sy, sz) = (
        p.int("segments_x"),
        p.int("segments_y"),
        p.int("segments_z"),
    );
    let x = |i: usize| -hx + i as f64 * 2.0 * hx / sx as f64;
    let y = |k: usize| -hy + k as f64 * 2.0 * hy / sy as f64;
    let z = |j: usize| -hz + j as f64 * 2.0 * hz / sz as f64;

    // Perimeter grid points (i, j) in walk order: -Z side, +X side, +Z side, -X side.
    let mut ring: Vec<(usize, usize)> = Vec::with_capacity(2 * (sx + sz));
    ring.extend((0..sx).map(|i| (i, 0)));
    ring.extend((0..sz).map(|j| (sx, j)));
    ring.extend((1..=sx).rev().map(|i| (i, sz)));
    ring.extend((1..=sz).rev().map(|j| (0, j)));
    let n = ring.len();
    let ring_index = |i: usize, j: usize| -> usize {
        if j == 0 {
            i
        } else if i == sx {
            sx + j
        } else if j == sz {
            sx + sz + (sx - i)
        } else {
            (2 * sx + sz + (sz - j)) % n
        }
    };
    let column = (sy + 1) as u32;

    let mut points = Vec::new();
    for &(i, j) in &ring {
        points.extend((0..=sy).map(|k| DVec3::new(x(i), y(k), z(j))));
    }
    let top_base = points.len();
    for i in 1..sx {
        points.extend((1..sz).map(|j| DVec3::new(x(i), hy, z(j))));
    }
    let bottom_base = points.len();
    // Bottom grid counts j from z = +hz.
    let zb = |jb: usize| hz - jb as f64 * 2.0 * hz / sz as f64;
    for i in 1..sx {
        points.extend((1..sz).map(|jb| DVec3::new(x(i), -hy, zb(jb))));
    }
    let interior = |base: usize, i: usize, j: usize| (base + (i - 1) * (sz - 1) + (j - 1)) as u32;
    let boundary = |i: usize, j: usize| i == 0 || i == sx || j == 0 || j == sz;
    let top = |i: usize, j: usize| {
        if boundary(i, j) {
            ring_index(i, j) as u32 * column + sy as u32
        } else {
            interior(top_base, i, j)
        }
    };
    let bottom = |i: usize, jb: usize| {
        if boundary(i, jb) {
            ring_index(i, sz - jb) as u32 * column
        } else {
            interior(bottom_base, i, jb)
        }
    };

    let mut polys = Vec::new();
    let stretches = [(0, sx), (sx, sz), (sx + sz, sx), (2 * sx + sz, sz)];
    for (start, len) in stretches {
        for k in 0..sy as u32 {
            for r in start..start + len {
                let (c0, c1) = (r as u32 * column, ((r + 1) % n) as u32 * column);
                polys.push(vec![c0 + k, c0 + k + 1, c1 + k + 1, c1 + k]);
            }
        }
    }
    for grid in [&top as &dyn Fn(usize, usize) -> u32, &bottom] {
        for j in 0..sz {
            for i in 0..sx {
                polys.push(vec![
                    grid(i, j),
                    grid(i, j + 1),
                    grid(i + 1, j + 1),
                    grid(i + 1, j),
                ]);
            }
        }
    }
    Ok(PolyMesh { points, polys })
}
