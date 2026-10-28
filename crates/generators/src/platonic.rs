//! Platonic sphere types (docs/specs/PRIMITIVES.md §3.1; docs/parity/sphere.md): a
//! platonic solid with its faces subdivided flat at frequency `n`, projected onto the
//! sphere.

use forge_geometry::PolyMesh;
use glam::DVec3;

/// The type's subdivision frequency for a Segments value (observed at 8 and 24, and for
/// the icosahedron also at 16: `r25-drift/sphere-type-icosahedron`).
pub(crate) fn frequency(kind: &str, segments: usize) -> usize {
    let per = match kind {
        "tetrahedron" => 2,
        "hexahedron" => 3,
        "octahedron" => 4,
        _ => 5, // icosahedron: 6 would give n = 2 at 16, observed 3
    };
    (segments / per).max(1)
}

/// Base vertices (unit circumradius) and faces `(A, B, C)` of a triangle type, 0-based.
fn triangle_solid(kind: &str) -> (Vec<DVec3>, Vec<[u32; 3]>) {
    match kind {
        "tetrahedron" => {
            let (s, c) = (8f64.sqrt() / 3.0, -1.0 / 3.0);
            let (sin60, cos60) = (3f64.sqrt() / 2.0, 0.5);
            let pts = vec![
                DVec3::new(-s * sin60, c, -s * cos60),
                DVec3::new(s * sin60, c, -s * cos60),
                DVec3::new(0.0, c, s),
                DVec3::Y,
            ];
            (pts, vec![[0, 1, 2], [0, 3, 1], [1, 3, 2], [2, 3, 0]])
        }
        "octahedron" => {
            let pts = vec![
                -DVec3::X,
                -DVec3::Z,
                DVec3::X,
                DVec3::Z,
                DVec3::Y,
                -DVec3::Y,
            ];
            let faces = vec![
                [4, 1, 0],
                [4, 2, 1],
                [4, 3, 2],
                [4, 0, 3],
                [5, 0, 1],
                [5, 1, 2],
                [5, 2, 3],
                [5, 3, 0],
            ];
            (pts, faces)
        }
        _ => {
            let phi = (1.0 + 5f64.sqrt()) / 2.0;
            let a = 1.0 / (1.0 + phi * phi).sqrt();
            let b = phi * a;
            let pts = vec![
                DVec3::new(0.0, a, -b),
                DVec3::new(0.0, -a, -b),
                DVec3::new(b, 0.0, -a),
                DVec3::new(b, 0.0, a),
                DVec3::new(0.0, a, b),
                DVec3::new(0.0, -a, b),
                DVec3::new(-b, 0.0, a),
                DVec3::new(-b, 0.0, -a),
                DVec3::new(a, b, 0.0),
                DVec3::new(-a, b, 0.0),
                DVec3::new(-a, -b, 0.0),
                DVec3::new(a, -b, 0.0),
            ];
            let faces = vec![
                [9, 8, 0],
                [8, 2, 0],
                [8, 3, 2],
                [8, 4, 3],
                [8, 9, 4],
                [4, 9, 6],
                [6, 9, 7],
                [9, 0, 7],
                [11, 10, 1],
                [2, 11, 1],
                [2, 3, 11],
                [3, 5, 11],
                [5, 10, 11],
                [5, 6, 10],
                [6, 7, 10],
                [7, 1, 10],
                [1, 7, 0],
                [2, 1, 0],
                [4, 5, 3],
                [4, 6, 5],
            ];
            (pts, faces)
        }
    }
}

/// Tetrahedron, octahedron or icosahedron at frequency `n`, radius `r`.
pub(crate) fn triangles(kind: &str, n: usize, r: f64) -> PolyMesh {
    let (base, faces) = triangle_solid(kind);
    let faces: Vec<Vec<u32>> = faces.iter().map(|f| f.to_vec()).collect();
    flat_subdivision(&base, &faces, n, |p| p.normalize() * r)
}

/// Faces (quads `[c0, c1, c2, c3]` and triangles `(A, B, C)`) cut flat at frequency `n`,
/// every point then mapped by `map` (PRIMITIVES.md §3.1, PRIMITIVES2.md §9).
pub(crate) fn flat_subdivision(
    base: &[DVec3],
    faces: &[Vec<u32>],
    n: usize,
    map: impl Fn(DVec3) -> DVec3,
) -> PolyMesh {
    let mut points: Vec<DVec3> = base.iter().map(|&p| map(p)).collect();

    // Edge points: edges in first-appearance order over the face cycles, each walked
    // from its lower-index vertex.
    let mut edge_start = std::collections::BTreeMap::new();
    for f in faces {
        for i in 0..f.len() {
            let (a, b) = (
                f[i].min(f[(i + 1) % f.len()]),
                f[i].max(f[(i + 1) % f.len()]),
            );
            edge_start.entry((a, b)).or_insert_with(|| {
                let start = points.len() as u32;
                let (pa, pb) = (base[a as usize], base[b as usize]);
                points.extend((1..n).map(|t| map(pa.lerp(pb, t as f64 / n as f64))));
                start
            });
        }
    }
    // Point t (0..=n) along edge a→b.
    let on_edge = |a: u32, b: u32, t: usize| -> u32 {
        if t == 0 {
            a
        } else if t == n {
            b
        } else if a < b {
            edge_start[&(a, b)] + (t - 1) as u32
        } else {
            edge_start[&(b, a)] + (n - t - 1) as u32
        }
    };

    let mut polys = Vec::new();
    for f in faces {
        let interior = points.len() as u32;
        let corner = |i: usize| base[f[i] as usize];
        if let &[c0, c1, c2, c3] = f.as_slice() {
            // Quad grid b(i, k): i along c0→c1, k along c0→c3; interior i outer, k inner.
            for i in 1..n {
                for k in 1..n {
                    let (u, v) = (i as f64 / n as f64, k as f64 / n as f64);
                    let p = corner(0) * (1.0 - u) * (1.0 - v)
                        + corner(1) * u * (1.0 - v)
                        + corner(2) * u * v
                        + corner(3) * (1.0 - u) * v;
                    points.push(map(p));
                }
            }
            let b = |i: usize, k: usize| -> u32 {
                if k == 0 {
                    on_edge(c0, c1, i)
                } else if i == n {
                    on_edge(c1, c2, k)
                } else if k == n {
                    on_edge(c3, c2, i)
                } else if i == 0 {
                    on_edge(c0, c3, k)
                } else {
                    interior + ((i - 1) * (n - 1) + k - 1) as u32
                }
            };
            for i in 0..n {
                for k in 0..n {
                    polys.push(vec![b(i, k), b(i + 1, k), b(i + 1, k + 1), b(i, k + 1)]);
                }
            }
            continue;
        }
        let (a, b, c) = (f[0], f[1], f[2]);
        let (pa, pb, pc) = (corner(0), corner(1), corner(2));
        // Rows r = 0..=n from apex A; P(r, 0) on A→B, P(r, r) on A→C, interior per face.
        for row in 2..n {
            for i in 1..row {
                let flat = pa
                    + (pb - pa) * ((row - i) as f64 / n as f64)
                    + (pc - pa) * (i as f64 / n as f64);
                points.push(map(flat));
            }
        }
        let p = |row: usize, i: usize| -> u32 {
            if i == 0 {
                on_edge(a, b, row)
            } else if i == row {
                on_edge(a, c, row)
            } else if row == n {
                on_edge(b, c, i)
            } else {
                // rows 2..row−1 hold (row'−1) points each before this row
                interior + ((row - 1) * (row - 2) / 2 + i - 1) as u32
            }
        };
        for row in 0..n {
            for i in 0..=row {
                polys.push(vec![p(row, i), p(row + 1, i), p(row + 1, i + 1)]);
                if i < row {
                    polys.push(vec![p(row + 1, i + 1), p(row, i + 1), p(row, i)]);
                }
            }
        }
    }
    PolyMesh { points, polys }
}

/// Hexahedron: a cube grid of `n × n` quads per face, projected onto the sphere.
pub(crate) fn hexahedron(n: usize, r: f64) -> PolyMesh {
    let at = |i: usize| -1.0 + 2.0 * i as f64 / n as f64;
    let on_sphere = |x: f64, y: f64, z: f64| DVec3::new(x, y, z).normalize() * r;
    // Side ring: 4n columns around Y, each n+1 points from y = −1 to +1.
    let column = |c: usize| -> (f64, f64) {
        let (s, t) = (c / n, at(c % n));
        match s {
            0 => (t, -1.0),
            1 => (1.0, t),
            2 => (-t, 1.0),
            _ => (-1.0, -t),
        }
    };
    let mut points = Vec::new();
    for c in 0..4 * n {
        let (x, z) = column(c);
        points.extend((0..=n).map(|j| on_sphere(x, at(j), z)));
    }
    let ring = |c: usize, j: usize| ((c % (4 * n)) * (n + 1) + j) as u32;
    // The ring column through cap-boundary point (x, z) on the square's edge.
    let ring_at = |i: usize, k: usize, flip: bool| -> Option<usize> {
        let k = if flip { n - k } else { k };
        match (i, k) {
            (_, 0) => Some(i),
            (i, _) if i == n => Some(n + k),
            (_, k) if k == n => Some(3 * n - i),
            (0, _) => Some((4 * n - k) % (4 * n)),
            _ => None,
        }
    };

    let mut polys = Vec::with_capacity(6 * n * n);
    for s in 0..4 {
        for j in 0..n {
            for i in 0..n {
                let c = s * n + i;
                polys.push(vec![
                    ring(c, j),
                    ring(c, j + 1),
                    ring(c + 1, j + 1),
                    ring(c + 1, j),
                ]);
            }
        }
    }
    // Caps: top (y = +1, z index k from −1), bottom (y = −1, z index k from +1).
    for (y, row, flip) in [(1.0, n, false), (-1.0, 0, true)] {
        let interior = points.len() as u32;
        for i in 1..n {
            for k in 1..n {
                let z = if flip { -at(k) } else { at(k) };
                points.push(on_sphere(at(i), y, z));
            }
        }
        let t = |i: usize, k: usize| match ring_at(i, k, flip) {
            Some(c) => ring(c, row),
            None => interior + ((i - 1) * (n - 1) + k - 1) as u32,
        };
        for k in 0..n {
            for i in 0..n {
                polys.push(vec![t(i, k), t(i, k + 1), t(i + 1, k + 1), t(i + 1, k)]);
            }
        }
    }
    PolyMesh { points, polys }
}
