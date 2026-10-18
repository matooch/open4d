//! Cube (docs/specs/PRIMITIVES.md §2; docs/parity/cube.md).

use forge_core::{Category, NodeType, ParamDesc, Unit};
use forge_geometry::PolyMesh;
use glam::{DVec2, DVec3};

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
        // Observed: Separate Surfaces is disabled while Fillet is on.
        return Ok(fillet(p));
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

/// Index of perimeter grid point `(i, j)` in the walk −Z, +X, +Z, −X from (0, 0).
fn perimeter_index(sx: usize, sz: usize, i: usize, j: usize) -> usize {
    if j == 0 {
        i
    } else if i == sx {
        sx + j
    } else if j == sz {
        sx + sz + (sx - i)
    } else {
        (2 * sx + sz + (sz - j)) % (2 * (sx + sz))
    }
}

/// Filleted cube (docs/specs/PRIMITIVES.md §2.1): rings around Y between two caps.
fn fillet(p: &Params) -> PolyMesh {
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
    let f = p.float("fillet_radius").clamp(0.0, hx.min(hy).min(hz));
    let s = p.int("fillet_segments");
    let (ix, iy, iz) = (hx - f, hy - f, hz - f);
    let quarter = std::f64::consts::FRAC_PI_2 / s as f64;

    // Sides: start, step direction, outward normal, segments.
    let sides = [
        (
            DVec2::new(-ix, -iz),
            DVec2::new(2.0 * ix, 0.0),
            -DVec2::Y,
            sx,
        ),
        (DVec2::new(ix, -iz), DVec2::new(0.0, 2.0 * iz), DVec2::X, sz),
        (DVec2::new(ix, iz), DVec2::new(-2.0 * ix, 0.0), DVec2::Y, sx),
        (
            DVec2::new(-ix, iz),
            DVec2::new(0.0, -2.0 * iz),
            -DVec2::X,
            sz,
        ),
    ];
    let cap_count = 2 * (sx + sz);
    // Ring entries: (inner point, outward direction, cap boundary point below/above).
    let mut cap = Vec::with_capacity(cap_count);
    let mut ring = Vec::new();
    for k in 0..4 {
        let (start, step, normal, segs) = sides[k];
        for i in 0..segs {
            let q = start + step * (i as f64 / segs as f64);
            ring.push((q, normal, cap.len()));
            cap.push(q);
        }
        let (corner, _, next, _) = sides[(k + 1) % 4];
        let corner_cap = cap.len() % cap_count;
        for t in 0..s {
            let a = t as f64 * quarter;
            ring.push((corner, normal * a.cos() + next * a.sin(), corner_cap));
        }
    }
    // Rows bottom to top: (y, distance from the inner box).
    let mut rows: Vec<(f64, f64)> = (1..s)
        .map(|j| {
            (
                -iy - f * (j as f64 * quarter).cos(),
                f * (j as f64 * quarter).sin(),
            )
        })
        .collect();
    rows.extend((0..=sy).map(|m| (-iy + m as f64 * 2.0 * iy / sy as f64, f)));
    rows.extend((1..s).rev().map(|j| {
        (
            iy + f * (j as f64 * quarter).cos(),
            f * (j as f64 * quarter).sin(),
        )
    }));

    let at = |q: DVec2, y: f64| DVec3::new(q.x, y, q.y);
    let mut points: Vec<DVec3> = cap.iter().map(|&q| at(q, -hy)).collect();
    for &(y, rho) in &rows {
        points.extend(ring.iter().map(|&(q, d, _)| at(q + d * rho, y)));
    }
    let top_cap = points.len();
    points.extend(cap.iter().map(|&q| at(q, hy)));
    let x = |i: usize| -ix + i as f64 * 2.0 * ix / sx as f64;
    let z = |k: usize| -iz + k as f64 * 2.0 * iz / sz as f64;
    let top_interior = points.len();
    for y in [hy, -hy] {
        for i in 1..sx {
            points.extend((1..sz).map(|k| DVec3::new(x(i), y, z(k))));
        }
    }
    let bottom_interior = top_interior + (sx - 1) * (sz - 1);

    let n = ring.len();
    let r = |row: usize, i: usize| (cap_count + row * n + i % n) as u32;
    let last = rows.len() - 1;
    let mut polys = Vec::new();
    for i in 0..n {
        let (a, b) = (ring[i].2, ring[(i + 1) % n].2);
        let (ta, tb) = ((top_cap + a) as u32, (top_cap + b) as u32);
        polys.push(if a == b {
            vec![a as u32, r(0, i), r(0, i + 1)]
        } else {
            vec![a as u32, r(0, i), r(0, i + 1), b as u32]
        });
        for row in 0..last {
            polys.push(vec![
                r(row, i),
                r(row + 1, i),
                r(row + 1, i + 1),
                r(row, i + 1),
            ]);
        }
        polys.push(if a == b {
            vec![r(last, i), ta, r(last, i + 1)]
        } else {
            vec![r(last, i), ta, tb, r(last, i + 1)]
        });
    }
    let grid = |boundary: usize, interior: usize, i: usize, k: usize| -> u32 {
        if i == 0 || i == sx || k == 0 || k == sz {
            (boundary + perimeter_index(sx, sz, i, k)) as u32
        } else {
            (interior + (i - 1) * (sz - 1) + k - 1) as u32
        }
    };
    let t = |i: usize, k: usize| grid(top_cap, top_interior, i, k);
    let bt = |i: usize, k: usize| grid(0, bottom_interior, i, k);
    for i in 0..sx {
        for k in 0..sz {
            polys.push(vec![t(i, k), t(i, k + 1), t(i + 1, k + 1), t(i + 1, k)]);
        }
    }
    for i in 0..sx {
        for k in 0..sz {
            polys.push(vec![bt(i, k), bt(i + 1, k), bt(i + 1, k + 1), bt(i, k + 1)]);
        }
    }
    PolyMesh { points, polys }
}
