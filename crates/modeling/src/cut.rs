//! Loop cut and Knife (docs/specs/MODELING.md §5.1, §5.2). Own design, parity-pending.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use forge_geometry::mesh::poly_edges;
use forge_geometry::selection::IndexSet;
use forge_geometry::{Edge, PolyMesh};
use glam::DVec3;

use crate::ToolError;

/// The most cuts one loop cut makes.
pub const MAX_CUTS: usize = 64;

/// A cut mesh and the edges the cut created (as point pairs of the new mesh).
#[derive(Debug, Clone, PartialEq)]
pub struct CutOutput {
    pub mesh: PolyMesh,
    pub new_edges: Vec<(u32, u32)>,
}

impl CutOutput {
    /// The ids (in the new mesh) of the created edges.
    pub fn new_edge_ids(&self) -> Vec<u32> {
        let ids = self.mesh.edge_ids();
        let mut out: Vec<u32> = self
            .new_edges
            .iter()
            .filter_map(|(a, b)| ids.get(&Edge::new(*a, *b)).copied())
            .collect();
        out.sort_unstable();
        out.dedup();
        out
    }
}

/// `count` evenly spaced offsets `i / (count + 1)`.
pub fn even_offsets(count: usize) -> Vec<f64> {
    (1..=count).map(|i| i as f64 / (count + 1) as f64).collect()
}

fn invalid<T>(msg: &str) -> Result<T, ToolError> {
    Err(ToolError::Invalid(msg.to_string()))
}

fn check_offsets(offsets: &[f64]) -> Result<Vec<f64>, ToolError> {
    if offsets.is_empty() {
        return invalid("loop cut: give at least one offset");
    }
    if offsets.len() > MAX_CUTS {
        return invalid("loop cut: at most 64 cuts");
    }
    let mut sorted = offsets.to_vec();
    if sorted
        .iter()
        .any(|t| !t.is_finite() || *t <= 0.0 || *t >= 1.0)
    {
        return invalid("loop cut: offsets must lie between 0 and 1 (exclusive)");
    }
    sorted.sort_by(f64::total_cmp);
    if sorted.windows(2).any(|w| w[1] - w[0] < 1e-9) {
        return invalid("loop cut: offsets must differ");
    }
    Ok(sorted)
}

struct Step {
    face: u32,
    from: Edge,
    to: Edge,
}

fn position_of(poly: &[u32], e: Edge) -> Option<usize> {
    poly_edges(poly).position(|x| x == e)
}

/// The orientation of the edge opposite `e` in a quad, parallel to `e` running `(s, t)`.
fn parallel(poly: &[u32], j: usize, s: u32) -> (u32, u32) {
    if poly[j] == s {
        (poly[(j + 3) % 4], poly[(j + 2) % 4])
    } else {
        (poly[(j + 2) % 4], poly[(j + 3) % 4])
    }
}

/// Walks the quads from `first` away from `start`; stops at a border, a non-quad polygon, a
/// non-manifold edge, a visited polygon or the start edge (`closed`).
fn walk(
    mesh: &PolyMesh,
    faces: &HashMap<Edge, Vec<u32>>,
    start: Edge,
    first: u32,
    visited: &mut BTreeSet<u32>,
) -> (Vec<Step>, bool) {
    let (mut e, mut f) = (start, first);
    let mut steps = Vec::new();
    loop {
        let poly = &mesh.polys[f as usize];
        if poly.len() != 4 || !visited.insert(f) {
            return (steps, false);
        }
        let Some(j) = position_of(poly, e) else {
            return (steps, false);
        };
        let o = Edge::new(poly[(j + 2) % 4], poly[(j + 3) % 4]);
        steps.push(Step {
            face: f,
            from: e,
            to: o,
        });
        if o == start {
            return (steps, true);
        }
        match faces.get(&o).map(Vec::as_slice) {
            Some([a, b]) => f = if *a == f { *b } else { *a },
            _ => return (steps, false),
        }
        e = o;
    }
}

/// Cuts the ring of quads through `start_edge` (an id of [`PolyMesh::edges`]) at the fractions
/// `offsets` of the start edge, which runs from its lower to its higher point (§5.1).
pub fn loop_cut(mesh: &PolyMesh, start_edge: u32, offsets: &[f64]) -> Result<CutOutput, ToolError> {
    let offsets = check_offsets(offsets)?;
    let start = *mesh
        .edges()
        .get(start_edge as usize)
        .ok_or(ToolError::BadEdge(start_edge))?;
    let mut faces: HashMap<Edge, Vec<u32>> = HashMap::new();
    for (f, poly) in mesh.polys.iter().enumerate() {
        for e in poly_edges(poly) {
            let list = faces.entry(e).or_default();
            if !list.contains(&(f as u32)) {
                list.push(f as u32);
            }
        }
    }
    let start_faces = faces.get(&start).cloned().unwrap_or_default();
    if start_faces.len() > 2 {
        return invalid("loop cut: the edge is shared by more than two polygons");
    }
    let mut visited = BTreeSet::new();
    let (forward, closed) = match start_faces.first() {
        Some(f) => walk(mesh, &faces, start, *f, &mut visited),
        None => (Vec::new(), false),
    };
    let backward = match start_faces.get(1) {
        Some(f) if !closed => walk(mesh, &faces, start, *f, &mut visited).0,
        _ => Vec::new(),
    };
    if forward.is_empty() && backward.is_empty() {
        return invalid("loop cut: the edge borders no quad");
    }

    // the ring: backward (far end first), the start edge, forward; closed rings end at the start
    let mut ring: Vec<Edge> = backward.iter().rev().map(|s| s.to).collect();
    ring.push(start);
    ring.extend(
        forward
            .iter()
            .map(|s| s.to)
            .take(forward.len() - usize::from(closed)),
    );
    // orientations, propagated from the start edge through the quads
    let mut orient: HashMap<Edge, (u32, u32)> = HashMap::new();
    orient.insert(start, (start.0, start.1));
    for steps in [&forward, &backward] {
        for s in steps {
            let (a, _) = orient[&s.from];
            let poly = &mesh.polys[s.face as usize];
            let j = position_of(poly, s.from).unwrap_or(0);
            orient.entry(s.to).or_insert_with(|| parallel(poly, j, a));
        }
    }
    // the new points
    let mut out = mesh.clone();
    let mut cuts: HashMap<Edge, Vec<u32>> = HashMap::new();
    for e in &ring {
        let (from, to) = orient[e];
        let (pa, pb) = (mesh.points[from as usize], mesh.points[to as usize]);
        let ids = offsets
            .iter()
            .map(|t| {
                out.points.push(pa + (pb - pa) * *t);
                (out.points.len() - 1) as u32
            })
            .collect();
        cuts.insert(*e, ids);
    }
    // points along `u -> v` of an edge, in that direction
    let along = |e: Edge, u: u32| -> Vec<u32> {
        let mut v = cuts[&e].clone();
        if orient[&e].0 != u {
            v.reverse();
        }
        v
    };
    let mut appended: Vec<Vec<u32>> = Vec::new();
    let mut new_edges = Vec::new();
    let mut in_strip = BTreeSet::new();
    let steps = backward.iter().rev().chain(forward.iter());
    // strip order: backward walk (far end first), then forward
    let mut replaced: Vec<(u32, Vec<Vec<u32>>)> = Vec::new();
    for s in steps {
        let poly = &mesh.polys[s.face as usize];
        let j = position_of(poly, s.from).unwrap_or(0);
        let (u, v, w, x) = (
            poly[j],
            poly[(j + 1) % 4],
            poly[(j + 2) % 4],
            poly[(j + 3) % 4],
        );
        let mut l1 = vec![u];
        l1.extend(along(s.from, u));
        l1.push(v);
        let mut l2 = vec![x];
        l2.extend(along(s.to, x));
        l2.push(w);
        let quads: Vec<Vec<u32>> = (0..l1.len() - 1)
            .map(|k| vec![l1[k], l1[k + 1], l2[k + 1], l2[k]])
            .collect();
        for k in 1..l1.len() - 1 {
            new_edges.push((l1[k], l2[k]));
        }
        in_strip.insert(s.face);
        replaced.push((s.face, quads));
    }
    for (face, quads) in replaced {
        let mut it = quads.into_iter();
        out.polys[face as usize] = it.next().unwrap_or_default();
        appended.extend(it);
    }
    // neighbours outside the strip get the points in their edge (no T-junctions)
    for (f, poly) in mesh.polys.iter().enumerate() {
        if in_strip.contains(&(f as u32)) {
            continue;
        }
        let mut next = Vec::with_capacity(poly.len());
        for (i, &c) in poly.iter().enumerate() {
            next.push(c);
            let e = Edge::new(c, poly[(i + 1) % poly.len()]);
            if cuts.contains_key(&e) {
                next.extend(along(e, c));
            }
        }
        out.polys[f] = next;
    }
    out.polys.extend(appended);
    Ok(CutOutput {
        mesh: out,
        new_edges,
    })
}

// ---------------------------------------------------------------- Knife

/// The cutting surface of a plane knife: the plane through `point` with `normal`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Plane {
    pub point: DVec3,
    pub normal: DVec3,
}

/// Where the viewer is: a point (perspective) or a view direction (parallel projection).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum View {
    Eye(DVec3),
    Direction(DVec3),
}

/// A drawn knife line: the segment `from -> to` seen from `view`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KnifeLine {
    pub from: DVec3,
    pub to: DVec3,
    pub view: View,
    /// Polygons facing away from the viewer are not cut.
    pub visible_only: bool,
}

impl KnifeLine {
    /// The plane through the segment and the viewer.
    pub fn plane(&self) -> Result<Plane, ToolError> {
        let v = self.to - self.from;
        let normal = match self.view {
            View::Eye(e) => (self.from - e).cross(self.to - e),
            View::Direction(d) => v.cross(d),
        };
        if v.length() < 1e-12 || normal.length() < 1e-12 * v.length().max(1.0) {
            return invalid("knife: the line is degenerate (zero length or through the eye)");
        }
        Ok(Plane {
            point: self.from,
            normal,
        })
    }

    /// Position `s` of `p` along the segment (0 at `from`, 1 at `to`) seen from the viewer.
    fn param(&self, p: DVec3) -> f64 {
        let v = self.to - self.from;
        match self.view {
            View::Eye(e) => {
                let (w, r) = (p - e, self.from - e);
                let vw = v.cross(w);
                -(r.cross(w)).dot(vw) / vw.length_squared().max(1e-300)
            }
            View::Direction(d) => {
                let vd = v.cross(d);
                (p - self.from).cross(d).dot(vd) / vd.length_squared().max(1e-300)
            }
        }
    }

    fn faces_away(&self, normal: DVec3, centroid: DVec3) -> bool {
        match self.view {
            View::Eye(e) => normal.dot(centroid - e) >= 0.0,
            View::Direction(d) => normal.dot(d) >= 0.0,
        }
    }
}

fn newell(mesh: &PolyMesh, poly: &[u32]) -> DVec3 {
    let mut n = DVec3::ZERO;
    for i in 0..poly.len() {
        let (a, b) = (
            mesh.points[poly[i] as usize],
            mesh.points[poly[(i + 1) % poly.len()] as usize],
        );
        n += a.cross(b);
    }
    n
}

#[derive(Clone, Copy)]
enum Vertex {
    Corner(usize),
    OnEdge(Edge),
}

fn bbox_diagonal(mesh: &PolyMesh) -> f64 {
    let (mut lo, mut hi) = (DVec3::splat(f64::MAX), DVec3::splat(f64::MIN));
    for p in &mesh.points {
        lo = lo.min(*p);
        hi = hi.max(*p);
    }
    if mesh.points.is_empty() {
        0.0
    } else {
        (hi - lo).length()
    }
}

/// Splits `poly` along the chords that join two of its non-adjacent corners; the first part
/// keeps the position, the other parts follow (§5.2).
fn split_by_chords(poly: Vec<u32>, chords: &[(u32, u32)]) -> Vec<Vec<u32>> {
    let n = poly.len();
    for (k, &(a, b)) in chords.iter().enumerate() {
        let (Some(i), Some(j)) = (
            poly.iter().position(|c| *c == a),
            poly.iter().position(|c| *c == b),
        ) else {
            continue;
        };
        let (i, j) = (i.min(j), i.max(j));
        if j - i < 2 || (i == 0 && j == n - 1) {
            continue;
        }
        let rest: Vec<(u32, u32)> = chords
            .iter()
            .enumerate()
            .filter(|(m, _)| *m != k)
            .map(|(_, c)| *c)
            .collect();
        let mut kept: Vec<u32> = poly[..=i].to_vec();
        kept.extend_from_slice(&poly[j..]);
        let other: Vec<u32> = poly[i..=j].to_vec();
        let mut parts = split_by_chords(kept, &rest);
        parts.extend(split_by_chords(other, &rest));
        return parts;
    }
    vec![poly]
}

/// The cut of the polygons `polys` (all when empty) along `plane`; with `line` only the part
/// inside the segment, and only on polygons facing the viewer when `visible_only` (§5.2).
fn knife(
    mesh: &PolyMesh,
    polys: &IndexSet,
    plane: Plane,
    line: Option<&KnifeLine>,
) -> Result<CutOutput, ToolError> {
    if let Some(p) = polys.iter().find(|p| *p as usize >= mesh.polys.len()) {
        return Err(ToolError::BadPolygon(p));
    }
    let len = plane.normal.length();
    if !(len.is_finite() && len > 1e-12) {
        return invalid("knife: the plane normal is zero");
    }
    let n = plane.normal / len;
    let eps = 1e-9 * (1.0 + bbox_diagonal(mesh));
    let dist = |p: DVec3| n.dot(p - plane.point);
    let targets: Vec<u32> = if polys.is_empty() {
        (0..mesh.polys.len() as u32).collect()
    } else {
        polys.iter().collect()
    };
    let edge_point = |e: Edge| {
        let (pa, pb) = (mesh.points[e.0 as usize], mesh.points[e.1 as usize]);
        let (da, db) = (dist(pa), dist(pb));
        pa + (pb - pa) * (da / (da - db))
    };
    let mut chords: Vec<(u32, Vec<(Vertex, Vertex)>)> = Vec::new();
    for &f in &targets {
        let poly = &mesh.polys[f as usize];
        let m = poly.len();
        let centroid = poly.iter().map(|c| mesh.points[*c as usize]).sum::<DVec3>() / m as f64;
        let normal = newell(mesh, poly);
        if let Some(l) = line
            && l.visible_only
            && l.faces_away(normal, centroid)
        {
            continue;
        }
        let d: Vec<f64> = poly
            .iter()
            .map(|c| dist(mesh.points[*c as usize]))
            .collect();
        let sign: Vec<i8> = d
            .iter()
            .map(|x| {
                if x.abs() <= eps {
                    0
                } else if *x > 0.0 {
                    1
                } else {
                    -1
                }
            })
            .collect();
        let mut found: Vec<(Vertex, DVec3)> = Vec::new();
        for i in 0..m {
            let (prev, next) = (sign[(i + m - 1) % m], sign[(i + 1) % m]);
            if sign[i] == 0 && prev != 0 && next != 0 && prev != next {
                found.push((Vertex::Corner(i), mesh.points[poly[i] as usize]));
            }
            if sign[i] * sign[(i + 1) % m] == -1 {
                let e = Edge::new(poly[i], poly[(i + 1) % m]);
                found.push((Vertex::OnEdge(e), edge_point(e)));
            }
        }
        let along = n.cross(normal);
        if found.len() < 2 || along.length() < 1e-12 * normal.length().max(1.0) {
            continue;
        }
        found.sort_by(|a, b| a.1.dot(along).total_cmp(&b.1.dot(along)));
        let pairs: Vec<(Vertex, Vertex)> = found
            .as_chunks::<2>()
            .0
            .iter()
            .filter(|c| {
                line.is_none_or(|l| {
                    c.iter()
                        .all(|(_, p)| (-1e-9..=1.0 + 1e-9).contains(&l.param(*p)))
                })
            })
            .map(|c| (c[0].0, c[1].0))
            .collect();
        if !pairs.is_empty() {
            chords.push((f, pairs));
        }
    }
    if chords.is_empty() {
        return invalid("knife: the cut does not cross any polygon");
    }
    // new points on the crossed edges, ascending by edge
    let used: BTreeSet<Edge> = chords
        .iter()
        .flat_map(|(_, pairs)| pairs.iter().flat_map(|(a, b)| [*a, *b]))
        .filter_map(|v| match v {
            Vertex::OnEdge(e) => Some(e),
            Vertex::Corner(_) => None,
        })
        .collect();
    let mut out = mesh.clone();
    let mut new_at: BTreeMap<Edge, u32> = BTreeMap::new();
    for e in &used {
        out.points.push(edge_point(*e));
        new_at.insert(*e, (out.points.len() - 1) as u32);
    }
    for poly in &mut out.polys {
        if new_at.is_empty() {
            break;
        }
        let old = std::mem::take(poly);
        for (i, &c) in old.iter().enumerate() {
            poly.push(c);
            if let Some(p) = new_at.get(&Edge::new(c, old[(i + 1) % old.len()])) {
                poly.push(*p);
            }
        }
    }
    let mut new_edges = Vec::new();
    let mut appended = Vec::new();
    for (f, pairs) in chords {
        let corners = &mesh.polys[f as usize];
        let point_of = |v: Vertex| match v {
            Vertex::Corner(i) => corners[i],
            Vertex::OnEdge(e) => new_at[&e],
        };
        let cs: Vec<(u32, u32)> = pairs
            .iter()
            .map(|(a, b)| (point_of(*a), point_of(*b)))
            .collect();
        new_edges.extend(cs.iter().copied());
        let mut parts =
            split_by_chords(std::mem::take(&mut out.polys[f as usize]), &cs).into_iter();
        out.polys[f as usize] = parts.next().unwrap_or_default();
        appended.extend(parts);
    }
    out.polys.extend(appended);
    Ok(CutOutput {
        mesh: out,
        new_edges,
    })
}

/// Knife in plane mode (§5.2).
pub fn knife_plane(
    mesh: &PolyMesh,
    polys: &IndexSet,
    plane: Plane,
) -> Result<CutOutput, ToolError> {
    knife(mesh, polys, plane, None)
}

/// Knife in line mode (§5.2).
pub fn knife_line(
    mesh: &PolyMesh,
    polys: &IndexSet,
    line: &KnifeLine,
) -> Result<CutOutput, ToolError> {
    knife(mesh, polys, line.plane()?, Some(line))
}
