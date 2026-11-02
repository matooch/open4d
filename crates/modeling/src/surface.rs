//! Subdivide, Triangulate, Untriangulate, Reverse Normals, Optimize
//! (docs/specs/MODELING.md §4.1–4.4, §4.9).

use std::collections::{BTreeSet, HashMap, HashSet};

use forge_geometry::mesh::poly_edges;
use forge_geometry::normals::poly_normal;
use forge_geometry::selection::Selection;
use forge_geometry::subdivide::{catmull_clark, linear, subdivide_levels};
use forge_geometry::triangulate::triangulate_poly;
use forge_geometry::{Edge, PolyMesh};
use glam::DVec3;

use crate::ToolError;
use crate::merge::merge_where_possible;
use crate::settings::Settings;
use crate::util::{
    ToolResult, clean_poly, polys_or_all, polys_selection, remove_points, remove_polys,
};

/// One level for meshes that are not all quads: originals, edge points in `edges()` order,
/// face points; polygon `i` becomes its last corner's quad, the other quads are appended.
fn subdivide_once(mesh: &PolyMesh, smooth: bool) -> PolyMesh {
    let generated = if smooth {
        catmull_clark(mesh)
    } else {
        linear(mesh)
    };
    let n0 = mesh.points.len();
    let ids = mesh.edge_ids();
    let ne = ids.len();
    let mut points = vec![DVec3::ZERO; n0 + ne + mesh.polys.len()];
    points[..n0].copy_from_slice(&generated.points[..n0]);
    // the generator makes, face by face, the face point and then the edge points not seen yet
    let mut cursor = n0;
    let mut seen: HashSet<Edge> = HashSet::new();
    for (f, poly) in mesh.polys.iter().enumerate() {
        points[n0 + ne + f] = generated.points[cursor];
        cursor += 1;
        for e in poly_edges(poly) {
            if seen.insert(e) {
                points[n0 + ids[&e] as usize] = generated.points[cursor];
                cursor += 1;
            }
        }
    }
    let mut polys = mesh.polys.clone();
    let mut appended = Vec::new();
    for (f, poly) in mesh.polys.iter().enumerate() {
        let n = poly.len();
        let face = (n0 + ne + f) as u32;
        let edge = |a: u32, b: u32| (n0 + ids[&Edge::new(a, b)] as usize) as u32;
        let mut quads: Vec<Vec<u32>> = (0..n)
            .map(|k| {
                vec![
                    poly[k],
                    edge(poly[k], poly[(k + 1) % n]),
                    face,
                    edge(poly[(k + n - 1) % n], poly[k]),
                ]
            })
            .collect();
        polys[f] = quads.pop().unwrap_or_default();
        appended.extend(quads);
    }
    polys.extend(appended);
    PolyMesh { points, polys }
}

/// Lattice coordinates `(R, C)` (R along `c0 -> c3`, C along `c0 -> c1`, both `0..=2m`) of the
/// generator's patch grid point `g(i, j)` of patch `k` (SUBDIVISION.md §2).
fn lattice_of(k: usize, i: usize, j: usize, m: usize) -> (usize, usize) {
    match k {
        0 => (j, i),
        1 => (i, 2 * m - j),
        2 => (2 * m - j, 2 * m - i),
        _ => (2 * m - i, j),
    }
}

/// Levels of an all-quad mesh in the command's lattice order (§4.1): each quad is a
/// `2^L x 2^L` lattice; points are the originals, the edge points along each edge in its
/// first-seen direction, then each polygon's interior points; cells row by row.
fn subdivide_quads(mesh: &PolyMesh, levels: usize, smooth: bool) -> PolyMesh {
    let generated = subdivide_levels(mesh, levels, smooth);
    let m = 1usize << (levels - 1);
    let w = 2 * m + 1;
    // generator vertex id at lattice point (R, C) of polygon f
    let mut lat = vec![u32::MAX; mesh.polys.len() * w * w];
    for f in 0..mesh.polys.len() {
        for (pp, k) in [1usize, 2, 3, 0].into_iter().enumerate() {
            for j in 0..m {
                for i in 0..m {
                    let face = &generated.polys[f * 4 * m * m + pp * m * m + j * m + i];
                    for (v, (di, dj)) in face.iter().zip([(0, 0), (1, 0), (1, 1), (0, 1)]) {
                        let (r, c) = lattice_of(k, i + di, j + dj, m);
                        lat[(f * w + r) * w + c] = *v;
                    }
                }
            }
        }
    }
    let at = |f: usize, r: usize, c: usize| lat[(f * w + r) * w + c];
    let n0 = mesh.points.len();
    let mut order: Vec<u32> = (0..n0 as u32).collect();
    let mut seen: HashSet<Edge> = HashSet::new();
    let mut edge_points: Vec<u32> = Vec::new();
    for (f, poly) in mesh.polys.iter().enumerate() {
        for k in 0..4 {
            if seen.insert(Edge::new(poly[k], poly[(k + 1) % 4])) {
                for s in 1..2 * m {
                    let (r, c) = match k {
                        0 => (0, s),
                        1 => (s, 2 * m),
                        2 => (2 * m, 2 * m - s),
                        _ => (2 * m - s, 0),
                    };
                    edge_points.push(at(f, r, c));
                }
            }
        }
    }
    order.extend(edge_points);
    for f in 0..mesh.polys.len() {
        for r in 1..2 * m {
            for c in 1..2 * m {
                order.push(at(f, r, c));
            }
        }
    }
    let mut mine = vec![0u32; generated.points.len()];
    for (slot, id) in order.iter().enumerate() {
        mine[*id as usize] = slot as u32;
    }
    let points = order
        .iter()
        .map(|id| generated.points[*id as usize])
        .collect();
    let cell = |f: usize, r: usize, c: usize| -> Vec<u32> {
        [(r, c), (r, c + 1), (r + 1, c + 1), (r + 1, c)]
            .iter()
            .map(|(r, c)| mine[at(f, *r, *c) as usize])
            .collect()
    };
    let mut polys = Vec::new();
    let mut appended = Vec::new();
    for f in 0..mesh.polys.len() {
        for r in 0..2 * m {
            for c in 0..2 * m {
                if (r, c) == (2 * m - 1, 2 * m - 1) {
                    polys.push(cell(f, r, c));
                } else {
                    appended.push(cell(f, r, c));
                }
            }
        }
    }
    polys.extend(appended);
    PolyMesh { points, polys }
}

/// Subdivide (§4.1).
pub fn subdivide(mesh: &PolyMesh, sel: &Selection, s: &Settings) -> ToolResult {
    let chosen = polys_or_all(mesh, &sel.polys)?;
    if chosen.len() != mesh.polys.len() {
        return Err(ToolError::Unsupported(
            "Subdivide of a part of the polygons (no golden data)".to_string(),
        ));
    }
    let smooth = s.bool("hyper");
    let levels = s.int("subdivisions").max(1) as usize;
    let out = if mesh.polys.iter().all(|p| p.len() == 4) {
        subdivide_quads(mesh, levels, smooth)
    } else {
        let mut out = mesh.clone();
        for _ in 0..levels {
            out = subdivide_once(&out, smooth);
        }
        out
    };
    let all = 0..out.polys.len() as u32;
    Ok((out, polys_selection(all)))
}

/// The fan from the first corner, or `None` if a triangle is turned against the polygon.
fn fan(points: &[DVec3], poly: &[u32]) -> Option<Vec<[u32; 3]>> {
    let n = poly_normal(points, poly);
    let tris: Vec<[u32; 3]> = (1..poly.len() - 1)
        .map(|i| [poly[0], poly[i], poly[i + 1]])
        .collect();
    let turned = |t: &[u32; 3]| {
        let [a, b, c] = t.map(|i| points[i as usize]);
        (b - a).cross(c - a).dot(n) <= 0.0
    };
    (n == DVec3::ZERO || !tris.iter().any(turned)).then_some(tris)
}

/// Triangulate (§4.2).
pub fn triangulate(mesh: &PolyMesh, sel: &Selection, _s: &Settings) -> ToolResult {
    let chosen = polys_or_all(mesh, &sel.polys)?;
    let mut polys = mesh.polys.clone();
    let mut appended: Vec<Vec<u32>> = Vec::new();
    for &i in &chosen {
        let poly = &mesh.polys[i as usize];
        if poly.len() == 3 {
            continue;
        }
        let tris = fan(&mesh.points, poly).unwrap_or_else(|| triangulate_poly(&mesh.points, poly));
        polys[i as usize] = tris[0].to_vec();
        appended.extend(tris[1..].iter().map(|t| t.to_vec()));
    }
    let base = polys.len() as u32;
    let new_count = appended.len() as u32;
    polys.extend(appended);
    let selected = chosen.into_iter().chain(base..base + new_count);
    Ok((
        PolyMesh {
            points: mesh.points.clone(),
            polys,
        },
        polys_selection(selected),
    ))
}

/// Strictly convex quad `[a_k, d, a_k+1, a_k+2]` from triangle `a` and the far point `d` of
/// the triangle across edge `k`.
fn convex(points: &[DVec3], quad: &[u32; 4], normal: DVec3) -> bool {
    (0..4).all(|k| {
        let (p0, p1, p2) = (
            points[quad[k] as usize],
            points[quad[(k + 1) % 4] as usize],
            points[quad[(k + 2) % 4] as usize],
        );
        let (e1, e2) = (p1 - p0, p2 - p1);
        e1.cross(e2).dot(normal) > 1e-9 * e1.length() * e2.length()
    })
}

/// Untriangulate (§4.3).
pub fn untriangulate(mesh: &PolyMesh, sel: &Selection, s: &Settings) -> ToolResult {
    let chosen = polys_or_all(mesh, &sel.polys)?;
    let tris: Vec<u32> = chosen
        .into_iter()
        .filter(|i| mesh.polys[*i as usize].len() == 3)
        .collect();
    let normal = |t: u32| poly_normal(&mesh.points, &mesh.polys[t as usize]);
    let limit = s.float("angle_deg").to_radians().cos() - 1e-12;
    let coplanar = |a: u32, b: u32| normal(a).dot(normal(b)) >= limit;
    let mut by_edge: HashMap<Edge, Vec<u32>> = HashMap::new();
    for &t in &tris {
        for e in poly_edges(&mesh.polys[t as usize]) {
            by_edge.entry(e).or_default().push(t);
        }
    }
    let neighbours = |t: u32| -> Vec<u32> {
        let mut out: Vec<u32> = poly_edges(&mesh.polys[t as usize])
            .flat_map(|e| by_edge[&e].iter().copied())
            .filter(|u| *u != t)
            .collect();
        out.sort_unstable();
        out.dedup();
        out
    };
    let mut used: HashSet<u32> = HashSet::new();
    let mut groups: Vec<Vec<u32>> = Vec::new();
    for &t in &tris {
        if used.contains(&t) {
            continue;
        }
        if s.bool("ngons") {
            let mut group = vec![t];
            let mut queue = vec![t];
            used.insert(t);
            while let Some(c) = queue.pop() {
                for u in neighbours(c) {
                    if !used.contains(&u) && coplanar(t, u) {
                        used.insert(u);
                        group.push(u);
                        queue.push(u);
                    }
                }
            }
            group.sort_unstable();
            if group.len() > 1 {
                groups.push(group);
            }
            continue;
        }
        let partner = neighbours(t).into_iter().find(|&u| {
            if used.contains(&u) || !coplanar(t, u) {
                return false;
            }
            let (a, b) = (&mesh.polys[t as usize], &mesh.polys[u as usize]);
            (0..3).any(|k| {
                let (x, y) = (a[k], a[(k + 1) % 3]);
                let Some(far) = (0..3)
                    .find(|j| b[*j] == y && b[(j + 1) % 3] == x)
                    .map(|j| b[(j + 2) % 3])
                else {
                    return false;
                };
                convex(&mesh.points, &[x, far, y, a[(k + 2) % 3]], normal(t))
            })
        });
        if let Some(u) = partner {
            used.insert(t);
            used.insert(u);
            groups.push(vec![t.min(u), t.max(u)]);
        }
    }
    groups.sort();
    let (out, merged) = merge_where_possible(mesh, groups)?;
    Ok((out, polys_selection(merged)))
}

/// Reverse Normals (§4.4): the first corner stays.
pub fn reverse_normals(mesh: &PolyMesh, sel: &Selection, _s: &Settings) -> ToolResult {
    let chosen = polys_or_all(mesh, &sel.polys)?;
    let mut out = mesh.clone();
    for &i in &chosen {
        out.polys[i as usize][1..].reverse();
    }
    Ok((out, polys_selection(chosen)))
}

/// The same corner cycle in a canonical form: the smallest rotation of both directions.
fn canonical(poly: &[u32]) -> Vec<u32> {
    let n = poly.len();
    let rotations = |p: &[u32]| -> Vec<Vec<u32>> {
        (0..n)
            .map(|k| (0..n).map(|j| p[(k + j) % n]).collect())
            .collect()
    };
    let mut reversed = poly.to_vec();
    reversed.reverse();
    let mut all = rotations(poly);
    all.extend(rotations(&reversed));
    all.into_iter().min().unwrap_or_default()
}

fn degenerate(poly: &[u32]) -> bool {
    let distinct: BTreeSet<u32> = poly.iter().copied().collect();
    poly.len() < 3 || distinct.len() != poly.len()
}

/// Optimize (§4.9).
pub fn optimize(mesh: &PolyMesh, _sel: &Selection, s: &Settings) -> ToolResult {
    let mut out = mesh.clone();
    let n = out.points.len();
    let mut rep: Vec<u32> = (0..n as u32).collect();
    if s.bool("optimize_points") {
        let tol = s.float("tolerance");
        let cell = tol.max(1e-9);
        let key = |p: DVec3| {
            let c = (p / cell).floor();
            (c.x as i64, c.y as i64, c.z as i64)
        };
        let mut grid: HashMap<(i64, i64, i64), Vec<u32>> = HashMap::new();
        for (i, slot) in rep.iter_mut().enumerate() {
            let p = out.points[i];
            let (kx, ky, kz) = key(p);
            let mut best: Option<u32> = None;
            for dx in -1..=1 {
                for dy in -1..=1 {
                    for dz in -1..=1 {
                        let Some(list) = grid.get(&(kx + dx, ky + dy, kz + dz)) else {
                            continue;
                        };
                        for &j in list {
                            if out.points[j as usize].distance(p) <= tol {
                                best = Some(best.map_or(j, |b| b.min(j)));
                            }
                        }
                    }
                }
            }
            match best {
                Some(j) => *slot = j,
                None => grid.entry((kx, ky, kz)).or_default().push(i as u32),
            }
        }
    }
    let mut seen: HashSet<Vec<u32>> = HashSet::new();
    let mut dropped = BTreeSet::new();
    for (i, poly) in out.polys.iter_mut().enumerate() {
        let mapped: Vec<u32> = poly.iter().map(|c| rep[*c as usize]).collect();
        let keep = if mapped == *poly {
            !(s.bool("optimize_polygons") && degenerate(poly))
        } else {
            match clean_poly(&mapped) {
                Some(p) => {
                    *poly = p;
                    true
                }
                None => false,
            }
        };
        if !keep || (s.bool("optimize_polygons") && !seen.insert(canonical(poly))) {
            dropped.insert(i as u32);
        }
    }
    remove_polys(&mut out, &dropped);
    if s.bool("delete_unused_points") {
        let used: HashSet<u32> = out.polys.iter().flatten().copied().collect();
        let unused: BTreeSet<u32> = (0..n as u32).filter(|p| !used.contains(p)).collect();
        remove_points(&mut out, &unused);
    }
    Ok((out, Selection::default()))
}
