//! Catmull–Clark and linear subdivision steps (docs/specs/SUBDIVISION.md §2).
//! Ref: Catmull & Clark, CAD 10(6) (1978). Vertex and face orders follow
//! docs/parity/subdivision.md.

use std::collections::{BTreeMap, BTreeSet};

use glam::DVec3;

use crate::PolyMesh;
use crate::mesh::Edge;

fn average(points: impl IntoIterator<Item = DVec3>) -> DVec3 {
    let (sum, n) = points
        .into_iter()
        .fold((DVec3::ZERO, 0usize), |(s, n), p| (s + p, n + 1));
    if n == 0 { DVec3::ZERO } else { sum / n as f64 }
}

/// One step; `creases` are sharp input edges (their halves are returned as the next
/// level's creases).
fn step(mesh: &PolyMesh, smooth: bool, creases: &BTreeSet<Edge>) -> (PolyMesh, BTreeSet<Edge>) {
    let pts = &mesh.points;
    let face_points: Vec<DVec3> = mesh
        .polys
        .iter()
        .map(|p| average(p.iter().map(|&v| pts[v as usize])))
        .collect();

    // Edge → adjacent faces; vertex → incident faces and edges.
    let mut edge_faces: BTreeMap<Edge, Vec<usize>> = BTreeMap::new();
    let mut vert_faces = vec![Vec::new(); pts.len()];
    let mut vert_edges: Vec<Vec<Edge>> = vec![Vec::new(); pts.len()];
    for (f, poly) in mesh.polys.iter().enumerate() {
        for (i, &v) in poly.iter().enumerate() {
            vert_faces[v as usize].push(f);
            let e = Edge::new(v, poly[(i + 1) % poly.len()]);
            let faces = edge_faces.entry(e).or_default();
            if faces.is_empty() {
                vert_edges[e.0 as usize].push(e);
                vert_edges[e.1 as usize].push(e);
            }
            faces.push(f);
        }
    }

    // New indices: originals, then per face its face point and its new edge points.
    let mut points: Vec<DVec3> =
        Vec::with_capacity(pts.len() + face_points.len() + edge_faces.len());
    points.extend(pts.iter().copied());
    let mut face_index = Vec::with_capacity(mesh.polys.len());
    let mut edge_index: BTreeMap<Edge, u32> = BTreeMap::new();
    for (f, poly) in mesh.polys.iter().enumerate() {
        face_index.push(points.len() as u32);
        points.push(face_points[f]);
        for i in 0..poly.len() {
            let e = Edge::new(poly[i], poly[(i + 1) % poly.len()]);
            if let std::collections::btree_map::Entry::Vacant(slot) = edge_index.entry(e) {
                slot.insert(points.len() as u32);
                let mid = (pts[e.0 as usize] + pts[e.1 as usize]) / 2.0;
                let faces = &edge_faces[&e];
                let p = if smooth && faces.len() >= 2 && !creases.contains(&e) {
                    let fp = average(faces.iter().map(|&g| face_points[g]));
                    (mid + fp) / 2.0
                } else {
                    mid
                };
                points.push(p);
            }
        }
    }

    if smooth {
        for v in 0..pts.len() {
            let faces = &vert_faces[v];
            let edges = &vert_edges[v];
            if faces.is_empty() {
                continue;
            }
            let p = pts[v];
            let open = edges.iter().filter(|e| edge_faces[e].len() == 1).count();
            let sharp: Vec<&Edge> = edges
                .iter()
                .filter(|e| edge_faces[e].len() == 1 || creases.contains(e))
                .collect();
            points[v] = if sharp.len() >= 3 {
                p // three or more sharp edges: a corner that stays put
            } else if sharp.len() == 2 {
                let other = |e: &Edge| pts[if e.0 as usize == v { e.1 } else { e.0 } as usize];
                (6.0 * p + other(sharp[0]) + other(sharp[1])) / 8.0
            } else if open == 0 {
                let n = edges.len() as f64;
                let q = average(faces.iter().map(|&f| face_points[f]));
                let r = average(
                    edges
                        .iter()
                        .map(|e| (pts[e.0 as usize] + pts[e.1 as usize]) / 2.0),
                );
                (q + 2.0 * r + (n - 3.0) * p) / n
            } else {
                p // non-manifold boundary: keep
            };
        }
    }

    let mut polys = Vec::with_capacity(mesh.polys.iter().map(Vec::len).sum());
    for (f, poly) in mesh.polys.iter().enumerate() {
        let n = poly.len();
        let e = |a: usize, b: usize| edge_index[&Edge::new(poly[a % n], poly[b % n])];
        for k in (1..n).chain(std::iter::once(0)) {
            polys.push(vec![poly[k], e(k, k + 1), face_index[f], e(k + n - 1, k)]);
        }
    }
    let next_creases = creases
        .iter()
        .filter_map(|e| {
            edge_index
                .get(e)
                .map(|&m| [Edge::new(e.0, m), Edge::new(m, e.1)])
        })
        .flatten()
        .collect();
    (PolyMesh { points, polys }, next_creases)
}

/// One Catmull–Clark step.
pub fn catmull_clark(mesh: &PolyMesh) -> PolyMesh {
    step(mesh, true, &BTreeSet::new()).0
}

/// One linear (bilinear) step: Catmull–Clark topology, positions unchanged in shape.
pub fn linear(mesh: &PolyMesh) -> PolyMesh {
    step(mesh, false, &BTreeSet::new()).0
}

/// Where a level-L point sits relative to the input mesh (docs/specs/SUBDIVISION.md §2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Addr {
    Orig(u32),
    Face(usize),
    /// On an input edge, at `s / (2m)` of its length from its lower-index vertex.
    Edge(Edge, u32),
    /// Inside face `f`'s patch `k` at grid `(i, j)` (canonical owner).
    Patch(usize, usize, u32, u32),
}

/// Canonical address of grid point `(u, v)` of patch `k` of input face `f`.
fn canon(input: &PolyMesh, m: u32, f: usize, k: usize, u: u32, v: u32) -> Addr {
    let poly = &input.polys[f];
    let n = poly.len();
    let (c, next, prev) = (poly[k], poly[(k + 1) % n], poly[(k + n - 1) % n]);
    let on_edge = |from: u32, to: u32, dist: u32| {
        Addr::Edge(
            Edge::new(from, to),
            if from < to { dist } else { 2 * m - dist },
        )
    };
    match (u, v) {
        (0, 0) => Addr::Orig(c),
        (u, v) if u == m && v == m => Addr::Face(f),
        (u, 0) => on_edge(c, next, u),
        (0, v) => on_edge(c, prev, v),
        // row j = m belongs to the previous patch's column i = m
        (u, v) if v == m => Addr::Patch(f, (k + n - 1) % n, m, u),
        (u, v) => Addr::Patch(f, k, u, v),
    }
}

/// A face's place in the input: (input face, patch, corner grid coordinates).
type Frame = (usize, usize, Vec<(u32, u32)>);

/// Patch (corner) order within a face: 1, 2, …, n−1, 0.
fn patch_order(n: usize) -> impl Iterator<Item = usize> {
    (1..n).chain(std::iter::once(0))
}

/// `levels` subdivision steps in the reference app's vertex and face order.
pub fn subdivide_levels(input: &PolyMesh, levels: usize, smooth: bool) -> PolyMesh {
    subdivide_levels_with_creases(input, levels, smooth, &BTreeSet::new())
}

/// Like [`subdivide_levels`] with sharp input edges (docs/specs/SUBDIVISION.md §2).
pub fn subdivide_levels_with_creases(
    input: &PolyMesh,
    levels: usize,
    smooth: bool,
    creases: &BTreeSet<Edge>,
) -> PolyMesh {
    if levels <= 1 {
        return if levels == 0 {
            input.clone()
        } else {
            step(input, smooth, creases).0
        };
    }
    let m: u32 = 1 << (levels - 1);

    // Level 1, with each vertex's address and each face's patch frame.
    let (mut cur, mut sharp) = step(input, smooth, creases);
    let mut addr: Vec<Addr> = (0..input.points.len() as u32).map(Addr::Orig).collect();
    let mut seen = std::collections::BTreeSet::new();
    for (f, poly) in input.polys.iter().enumerate() {
        addr.push(Addr::Face(f));
        for i in 0..poly.len() {
            let e = Edge::new(poly[i], poly[(i + 1) % poly.len()]);
            if seen.insert(e) {
                addr.push(Addr::Edge(e, m));
            }
        }
    }
    // (input face, patch, corner coordinates) per current face
    let mut frames: Vec<Frame> = Vec::new();
    for (f, poly) in input.polys.iter().enumerate() {
        for k in patch_order(poly.len()) {
            frames.push((f, k, vec![(0, 0), (m, 0), (m, m), (0, m)]));
        }
    }

    // Further steps: new points get addresses from their face's frame.
    for _ in 1..levels {
        let (next, next_sharp) = step(&cur, smooth, &sharp);
        sharp = next_sharp;
        let mut next_addr = addr.clone();
        let mut next_frames = Vec::with_capacity(next.polys.len());
        let mut seen = std::collections::BTreeSet::new();
        for (poly, (f, k, q)) in cur.polys.iter().zip(&frames) {
            let n = poly.len();
            let mid = |a: (u32, u32), b: (u32, u32)| ((a.0 + b.0) / 2, (a.1 + b.1) / 2);
            let centre = (
                q.iter().map(|c| c.0).sum::<u32>() / n as u32,
                q.iter().map(|c| c.1).sum::<u32>() / n as u32,
            );
            next_addr.push(canon(input, m, *f, *k, centre.0, centre.1));
            for i in 0..n {
                if seen.insert(Edge::new(poly[i], poly[(i + 1) % n])) {
                    let p = mid(q[i], q[(i + 1) % n]);
                    next_addr.push(canon(input, m, *f, *k, p.0, p.1));
                }
            }
            for c in patch_order(n) {
                let corners = vec![
                    q[c],
                    mid(q[c], q[(c + 1) % n]),
                    centre,
                    mid(q[(c + n - 1) % n], q[c]),
                ];
                next_frames.push((*f, *k, corners));
            }
        }
        cur = next;
        addr = next_addr;
        frames = next_frames;
    }
    let position: BTreeMap<Addr, DVec3> = addr
        .iter()
        .copied()
        .zip(cur.points.iter().copied())
        .collect();

    // Emit in reference order.
    let mut order: Vec<Addr> = (0..input.points.len() as u32).map(Addr::Orig).collect();
    let mut first_dir: Vec<(u32, u32)> = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    for (f, poly) in input.polys.iter().enumerate() {
        order.push(Addr::Face(f));
        for i in 0..poly.len() {
            let (a, b) = (poly[i], poly[(i + 1) % poly.len()]);
            if seen.insert(Edge::new(a, b)) {
                order.push(Addr::Edge(Edge::new(a, b), m));
                first_dir.push((a, b));
            }
        }
    }
    for &(a, b) in &first_dir {
        // the half next to b (from b towards the middle), then the half next to a
        for t in ((m + 1)..2 * m).rev().chain(1..m) {
            order.push(Addr::Edge(
                Edge::new(a, b),
                if a < b { t } else { 2 * m - t },
            ));
        }
    }
    for (f, poly) in input.polys.iter().enumerate() {
        for k in patch_order(poly.len()) {
            for j in 1..m {
                for i in 1..=m {
                    order.push(canon(input, m, f, k, i, j));
                }
            }
        }
    }
    let index: BTreeMap<Addr, u32> = order
        .iter()
        .enumerate()
        .map(|(i, a)| (*a, i as u32))
        .collect();
    let points = order.iter().map(|a| position[a]).collect();
    let mut polys = Vec::new();
    for (f, poly) in input.polys.iter().enumerate() {
        for k in patch_order(poly.len()) {
            let g = |i: u32, j: u32| index[&canon(input, m, f, k, i, j)];
            for j in 0..m {
                for i in 0..m {
                    polys.push(vec![g(i, j), g(i + 1, j), g(i + 1, j + 1), g(i, j + 1)]);
                }
            }
        }
    }
    PolyMesh { points, polys }
}
