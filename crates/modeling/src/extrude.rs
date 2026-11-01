//! Extrude and Extrude Inner (docs/specs/MODELING.md §1–2).

use std::collections::{BTreeMap, BTreeSet, HashMap};

use forge_geometry::normals::poly_normal;
use forge_geometry::selection::{IndexSet, Selection};
use forge_geometry::{Edge, PolyMesh, mesh::poly_edges};
use glam::DVec3;

use crate::ToolError;
use crate::settings::Settings;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Extrude,
    Inset,
}

struct Run {
    kind: Kind,
    offset: f64,
    rows: usize,
    preserve_groups: bool,
    max_angle: f64,
}

/// Extrudes `polys` along their normals by `offset`, with side faces (MODELING.md §1).
/// Returns the new mesh and the selection of the cap polygons.
pub fn extrude(
    mesh: &PolyMesh,
    polys: &IndexSet,
    s: &Settings,
) -> Result<(PolyMesh, Selection), ToolError> {
    if s.float("offset_variation") != 0.0 {
        return Err(ToolError::Unsupported("Offset Variation".into()));
    }
    if s.float("rotation_deg") != 0.0 {
        return Err(ToolError::Unsupported("Rotation".into()));
    }
    if s.bool("create_caps") {
        return Err(ToolError::Unsupported("Create Caps".into()));
    }
    run(mesh, polys, &params(Kind::Extrude, s))
}

/// Insets `polys` inside their outline by `offset` (MODELING.md §2).
pub fn extrude_inner(
    mesh: &PolyMesh,
    polys: &IndexSet,
    s: &Settings,
) -> Result<(PolyMesh, Selection), ToolError> {
    if s.float("offset_variation") != 0.0 {
        return Err(ToolError::Unsupported("Offset Variation".into()));
    }
    run(mesh, polys, &params(Kind::Inset, s))
}

fn params(kind: Kind, s: &Settings) -> Run {
    Run {
        kind,
        offset: s.float("offset"),
        rows: s.int("subdivision").max(0) as usize + 1,
        preserve_groups: s.bool("preserve_groups"),
        max_angle: s.float("maximum_angle_deg").to_radians(),
    }
}

fn run(mesh: &PolyMesh, polys: &IndexSet, p: &Run) -> Result<(PolyMesh, Selection), ToolError> {
    if polys.is_empty() {
        return Err(ToolError::EmptySelection);
    }
    let chosen: Vec<u32> = polys.iter().collect();
    if let Some(bad) = chosen.iter().find(|i| **i as usize >= mesh.polys.len()) {
        return Err(ToolError::BadPolygon(*bad));
    }
    let mut out = mesh.clone();
    for group in groups(mesh, &chosen, p) {
        apply_group(mesh, &group, p, &mut out);
    }
    let selection = Selection {
        polys: chosen.iter().copied().collect(),
        ..Selection::default()
    };
    Ok((out, selection))
}

/// Selected polygons that share an edge and bend by at most the maximum angle form a
/// group (Preserve Groups); otherwise each polygon is a group. Ordered by smallest index.
fn groups(mesh: &PolyMesh, chosen: &[u32], p: &Run) -> Vec<Vec<u32>> {
    if !p.preserve_groups {
        return chosen.iter().map(|i| vec![*i]).collect();
    }
    let slot: HashMap<u32, usize> = chosen.iter().enumerate().map(|(k, i)| (*i, k)).collect();
    let mut parent: Vec<usize> = (0..chosen.len()).collect();
    fn find(parent: &mut [usize], mut x: usize) -> usize {
        while parent[x] != x {
            parent[x] = parent[parent[x]];
            x = parent[x];
        }
        x
    }
    let normals: Vec<DVec3> = chosen
        .iter()
        .map(|i| poly_normal(&mesh.points, &mesh.polys[*i as usize]))
        .collect();
    let mut by_edge: HashMap<Edge, Vec<usize>> = HashMap::new();
    for i in chosen {
        for e in poly_edges(&mesh.polys[*i as usize]) {
            by_edge.entry(e).or_default().push(slot[i]);
        }
    }
    let limit = p.max_angle.cos() - 1e-9;
    for users in by_edge.values() {
        for pair in users.windows(2) {
            if normals[pair[0]].dot(normals[pair[1]]) >= limit {
                let (a, b) = (find(&mut parent, pair[0]), find(&mut parent, pair[1]));
                parent[a.max(b)] = a.min(b);
            }
        }
    }
    let mut by_root: BTreeMap<usize, Vec<u32>> = BTreeMap::new();
    for (k, i) in chosen.iter().enumerate() {
        let root = find(&mut parent, k);
        by_root.entry(root).or_default().push(*i);
    }
    // roots are the smallest slot of their group, so the map order is by smallest index
    by_root.into_values().collect()
}

/// A boundary edge `a -> b` of a group, with the polygon that owns it.
struct Side {
    a: u32,
    b: u32,
    poly: u32,
}

fn apply_group(mesh: &PolyMesh, group: &[u32], p: &Run, out: &mut PolyMesh) {
    let pts = &mesh.points;
    let mut uses: HashMap<Edge, u32> = HashMap::new();
    for i in group {
        for e in poly_edges(&mesh.polys[*i as usize]) {
            *uses.entry(e).or_default() += 1;
        }
    }
    let mut sides = Vec::new();
    for i in group {
        let poly = &mesh.polys[*i as usize];
        for (k, a) in poly.iter().enumerate() {
            let b = poly[(k + 1) % poly.len()];
            if uses[&Edge::new(*a, b)] == 1 {
                sides.push(Side { a: *a, b, poly: *i });
            }
        }
    }
    let boundary: BTreeSet<u32> = sides.iter().flat_map(|s| [s.a, s.b]).collect();
    let moved: BTreeSet<u32> = match p.kind {
        Kind::Extrude => group
            .iter()
            .flat_map(|i| mesh.polys[*i as usize].iter().copied())
            .collect(),
        Kind::Inset => boundary,
    };
    let target = match p.kind {
        Kind::Extrude => extrude_targets(mesh, group, &moved, p.offset),
        Kind::Inset => inset_targets(mesh, &sides, &moved, p.offset),
    };
    // new points, ascending original index
    let mut new_of: HashMap<u32, u32> = HashMap::new();
    for old in &moved {
        new_of.insert(*old, out.points.len() as u32);
        out.points.push(target[old]);
    }
    let map = |v: u32| new_of.get(&v).copied().unwrap_or(v);
    // caps replace the polygons in place
    for i in group {
        out.polys[*i as usize] = mesh.polys[*i as usize].iter().map(|v| map(*v)).collect();
    }
    // columns of intermediate points: one per edge start point, then stragglers
    let n = p.rows - 1;
    let mut columns: HashMap<u32, Vec<u32>> = HashMap::new();
    for point in sides.iter().map(|s| s.a).chain(sides.iter().map(|s| s.b)) {
        if columns.contains_key(&point) {
            continue;
        }
        let (top, bottom) = (out.points[map(point) as usize], pts[point as usize]);
        let ids = (1..=n)
            .map(|k| {
                let t = k as f64 / p.rows as f64;
                out.points.push(top + (bottom - top) * t);
                (out.points.len() - 1) as u32
            })
            .collect();
        columns.insert(point, ids);
    }
    let rail = |v: u32| -> Vec<u32> {
        let mut r = vec![map(v)];
        r.extend(&columns[&v]);
        r.push(v);
        r
    };
    for side in &sides {
        let (l, r) = (rail(side.a), rail(side.b));
        for j in 0..p.rows {
            let quad = if (1..p.rows - 1).contains(&j) {
                vec![r[j + 1], r[j], l[j], l[j + 1]]
            } else {
                vec![l[j], l[j + 1], r[j + 1], r[j]]
            };
            out.polys.push(quad);
        }
    }
}

/// Extrude: every group point moves along the mean of the unit normals of the group's
/// polygons that use it.
fn extrude_targets(
    mesh: &PolyMesh,
    group: &[u32],
    moved: &BTreeSet<u32>,
    offset: f64,
) -> HashMap<u32, DVec3> {
    let mut sum: HashMap<u32, DVec3> = moved.iter().map(|v| (*v, DVec3::ZERO)).collect();
    for i in group {
        let poly = &mesh.polys[*i as usize];
        let n = poly_normal(&mesh.points, poly);
        for v in poly {
            if let Some(s) = sum.get_mut(v) {
                *s += n;
            }
        }
    }
    sum.into_iter()
        .map(|(v, s)| {
            let dir = if s.length() < 1e-12 { s } else { s.normalize() };
            (v, mesh.points[v as usize] + dir * offset)
        })
        .collect()
}

/// Inset: every boundary point moves in its polygon plane to `offset` from both adjacent
/// boundary edges (a mitre).
fn inset_targets(
    mesh: &PolyMesh,
    sides: &[Side],
    moved: &BTreeSet<u32>,
    offset: f64,
) -> HashMap<u32, DVec3> {
    let pts = &mesh.points;
    let inward: Vec<DVec3> = sides
        .iter()
        .map(|s| {
            let n = poly_normal(pts, &mesh.polys[s.poly as usize]);
            let d = pts[s.b as usize] - pts[s.a as usize];
            let v = n.cross(d);
            if v.length() < 1e-12 {
                DVec3::ZERO
            } else {
                v.normalize()
            }
        })
        .collect();
    moved
        .iter()
        .map(|v| {
            let incoming: Vec<DVec3> = (0..sides.len())
                .filter(|k| sides[*k].b == *v)
                .map(|k| inward[k])
                .collect();
            let outgoing: Vec<DVec3> = (0..sides.len())
                .filter(|k| sides[*k].a == *v)
                .map(|k| inward[k])
                .collect();
            let shift = if let ([i1], [i2]) = (incoming.as_slice(), outgoing.as_slice()) {
                let denom = 1.0 + i1.dot(*i2);
                if denom < 1e-9 {
                    *i1 * offset
                } else {
                    (*i1 + *i2) / denom * offset
                }
            } else {
                let sum: DVec3 = incoming.iter().chain(&outgoing).sum();
                if sum.length() < 1e-12 {
                    sum
                } else {
                    sum.normalize() * offset
                }
            };
            (*v, pts[*v as usize] + shift)
        })
        .collect()
}
