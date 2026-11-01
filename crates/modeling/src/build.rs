//! Topology tools: Bridge, Close Polygon Hole, Stitch and Sew, Polygon Pen
//! (docs/specs/MODELING.md §6). Own design, parity-pending.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use forge_geometry::mesh::poly_edges;
use forge_geometry::{Edge, PolyMesh};
use glam::DVec3;

use crate::ToolError;
use crate::util::{clean_poly, remove_points, remove_polys, swap_removed};

/// The most segments one bridge makes.
pub const MAX_SEGMENTS: usize = 64;

/// A built mesh with the polygons and points the tool created (ids in the new mesh).
#[derive(Debug, Clone, PartialEq)]
pub struct BuildOutput {
    pub mesh: PolyMesh,
    pub new_polys: Vec<u32>,
    pub new_points: Vec<u32>,
}

/// One end of a bridge: the border loop through a border edge, or a polygon that is removed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BridgeEnd {
    Edge(u32),
    Poly(u32),
}

/// Where merged points of Stitch go.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StitchTo {
    /// The average of the merged points.
    #[default]
    Middle,
    /// The lowest index keeps its position.
    First,
}

/// A Polygon Pen corner: an existing point (the snap) or a new position.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PenCorner {
    Point(u32),
    New(DVec3),
}

fn invalid<T>(msg: impl Into<String>) -> Result<T, ToolError> {
    Err(ToolError::Invalid(msg.into()))
}

/// The border loop through border edge `edge`, in the direction of its polygon (§6.1).
pub fn border_loop(mesh: &PolyMesh, edge: u32) -> Result<Vec<u32>, ToolError> {
    let edges = mesh.edges();
    let Some(e) = edges.get(edge as usize) else {
        return Err(ToolError::BadEdge(edge));
    };
    let mut uses: HashMap<Edge, usize> = HashMap::new();
    for poly in &mesh.polys {
        for x in poly_edges(poly) {
            *uses.entry(x).or_default() += 1;
        }
    }
    // directed border edges: the way their one polygon runs them
    let mut next: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
    let mut start: Option<(u32, u32)> = None;
    for poly in &mesh.polys {
        for i in 0..poly.len() {
            let (a, b) = (poly[i], poly[(i + 1) % poly.len()]);
            if uses[&Edge::new(a, b)] == 1 {
                next.entry(a).or_default().push(b);
                if Edge::new(a, b) == *e {
                    start = Some((a, b));
                }
            }
        }
    }
    let Some((u, v)) = start else {
        return invalid(format!("edge {edge} is not a border edge"));
    };
    let mut points = vec![u];
    let mut cur = v;
    while cur != u {
        if points.contains(&cur) || points.len() > mesh.points.len() {
            return invalid("the border is pinched (a point is visited twice)");
        }
        points.push(cur);
        cur = match next.get(&cur).map(Vec::as_slice) {
            Some([only]) => *only,
            _ => return invalid(format!("the border is pinched at point {cur}")),
        };
    }
    // a point where two border loops touch has two outgoing border edges
    if let Some(p) = points.iter().find(|p| next[*p].len() != 1) {
        return invalid(format!("the border is pinched at point {p}"));
    }
    Ok(points)
}

/// The loop of a bridge end, in the direction the neighbours give the border.
fn end_loop(mesh: &PolyMesh, end: BridgeEnd) -> Result<Vec<u32>, ToolError> {
    match end {
        BridgeEnd::Edge(e) => border_loop(mesh, e),
        BridgeEnd::Poly(p) => {
            let poly = mesh.polys.get(p as usize).ok_or(ToolError::BadPolygon(p))?;
            Ok(poly.iter().rev().copied().collect())
        }
    }
}

/// Bridges two ends with `segments` rows of quads (§6.2).
pub fn bridge(
    mesh: &PolyMesh,
    a: BridgeEnd,
    b: BridgeEnd,
    segments: usize,
) -> Result<BuildOutput, ToolError> {
    if segments == 0 || segments > MAX_SEGMENTS {
        return invalid("bridge: segments is 1..64");
    }
    let (la, lb) = (end_loop(mesh, a)?, end_loop(mesh, b)?);
    if la.len() != lb.len() {
        return invalid(format!(
            "bridge: the loops have {} and {} points",
            la.len(),
            lb.len()
        ));
    }
    if la.iter().any(|p| lb.contains(p)) {
        return invalid("bridge: the loops share points (or are the same loop)");
    }
    let n = la.len();
    let rev: Vec<u32> = lb.iter().rev().copied().collect();
    let cost = |k: usize| -> f64 {
        (0..n)
            .map(|i| {
                (mesh.points[la[i] as usize] - mesh.points[rev[(i + k) % n] as usize]).length()
            })
            .sum()
    };
    let mut best = (0, cost(0));
    for k in 1..n {
        let c = cost(k);
        if c < best.1 - 1e-9 * (1.0 + best.1) {
            best = (k, c);
        }
    }
    let partner: Vec<u32> = (0..n).map(|i| rev[(i + best.0) % n]).collect();

    let mut out = mesh.clone();
    let removed: BTreeSet<u32> = [a, b]
        .iter()
        .filter_map(|e| match e {
            BridgeEnd::Poly(p) => Some(*p),
            BridgeEnd::Edge(_) => None,
        })
        .collect();
    remove_polys(&mut out, &removed);

    let mut rings: Vec<Vec<u32>> = vec![la.clone()];
    for j in 1..segments {
        let t = j as f64 / segments as f64;
        let mut ring = Vec::with_capacity(n);
        for i in 0..n {
            let (p, q) = (
                mesh.points[la[i] as usize],
                mesh.points[partner[i] as usize],
            );
            ring.push(out.points.len() as u32);
            out.points.push(p + (q - p) * t);
        }
        rings.push(ring);
    }
    rings.push(partner);
    let first_new = out.polys.len() as u32;
    for j in 0..segments {
        for i in 0..n {
            let i1 = (i + 1) % n;
            out.polys.push(vec![
                rings[j][i1],
                rings[j][i],
                rings[j + 1][i],
                rings[j + 1][i1],
            ]);
        }
    }
    let new_polys = (first_new..out.polys.len() as u32).collect();
    let new_points = (mesh.points.len() as u32..out.points.len() as u32).collect();
    Ok(BuildOutput {
        mesh: out,
        new_polys,
        new_points,
    })
}

/// Closes the hole bounded by the border loop through `edge` with one n-gon (§6.3).
pub fn close_hole(mesh: &PolyMesh, edge: u32) -> Result<BuildOutput, ToolError> {
    let lp = border_loop(mesh, edge)?;
    let mut out = mesh.clone();
    out.polys.push(lp.into_iter().rev().collect());
    // the reverse starts at the last point; rotate so that it starts at the edge's start
    let poly = out.polys.last_mut().expect("just pushed");
    poly.rotate_right(1);
    let id = out.polys.len() as u32 - 1;
    Ok(BuildOutput {
        mesh: out,
        new_polys: vec![id],
        new_points: Vec::new(),
    })
}

/// The point pairs that sew border edge pairs together (§6.4): the border edge `u -> v` meets
/// the border edge `x -> y` with `u` joined to `y` and `v` to `x`.
pub fn stitch_edges(mesh: &PolyMesh, pairs: &[(u32, u32)]) -> Result<Vec<(u32, u32)>, ToolError> {
    let edges = mesh.edges();
    let mut uses: HashMap<Edge, usize> = HashMap::new();
    let mut directed: HashMap<Edge, (u32, u32)> = HashMap::new();
    for poly in &mesh.polys {
        for i in 0..poly.len() {
            let (a, b) = (poly[i], poly[(i + 1) % poly.len()]);
            *uses.entry(Edge::new(a, b)).or_default() += 1;
            directed.insert(Edge::new(a, b), (a, b));
        }
    }
    let border = |id: u32| -> Result<(u32, u32), ToolError> {
        let e = *edges.get(id as usize).ok_or(ToolError::BadEdge(id))?;
        if uses[&e] != 1 {
            return invalid(format!("edge {id} is not a border edge"));
        }
        Ok(directed[&e])
    };
    let mut out = Vec::new();
    for &(ea, eb) in pairs {
        if ea == eb {
            return invalid("stitch: an edge cannot be sewn to itself");
        }
        let ((u, v), (x, y)) = (border(ea)?, border(eb)?);
        for (p, q) in [(u, y), (v, x)] {
            if p != q {
                out.push((p, q));
            }
        }
    }
    if out.is_empty() {
        return invalid("stitch: nothing to sew (the edges already share their points)");
    }
    Ok(out)
}

/// The merged points (ids in the new mesh) and the mesh after Stitch and Sew (§6.4).
pub fn stitch(
    mesh: &PolyMesh,
    pairs: &[(u32, u32)],
    to: StitchTo,
) -> Result<BuildOutput, ToolError> {
    if pairs.is_empty() {
        return invalid("stitch: give at least one pair of points");
    }
    let n = mesh.points.len();
    let mut root: Vec<u32> = (0..n as u32).collect();
    fn find(root: &mut [u32], mut x: u32) -> u32 {
        while root[x as usize] != x {
            root[x as usize] = root[root[x as usize] as usize];
            x = root[x as usize];
        }
        x
    }
    for &(a, b) in pairs {
        for p in [a, b] {
            if p as usize >= n {
                return Err(ToolError::BadPoint(p));
            }
        }
        if a == b {
            return invalid("stitch: a point cannot be joined to itself");
        }
        let (ra, rb) = (find(&mut root, a), find(&mut root, b));
        // the lowest index is the root of its set
        root[ra.max(rb) as usize] = ra.min(rb);
    }
    let rep: Vec<u32> = (0..n as u32).map(|p| find(&mut root, p)).collect();
    let mut sets: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
    for (p, r) in rep.iter().enumerate() {
        if *r != p as u32 {
            sets.entry(*r).or_insert_with(|| vec![*r]).push(p as u32);
        }
    }
    let mut out = mesh.clone();
    if to == StitchTo::Middle {
        for (r, members) in &sets {
            let sum: DVec3 = members.iter().map(|m| mesh.points[*m as usize]).sum();
            out.points[*r as usize] = sum / members.len() as f64;
        }
    }
    let mut dropped = BTreeSet::new();
    for (i, poly) in out.polys.iter_mut().enumerate() {
        let mapped: Vec<u32> = poly.iter().map(|c| rep[*c as usize]).collect();
        match clean_poly(&mapped) {
            Some(p) => *poly = p,
            None => {
                dropped.insert(i as u32);
            }
        }
    }
    remove_polys(&mut out, &dropped);
    let gone: BTreeSet<u32> = sets.values().flat_map(|m| m[1..].iter().copied()).collect();
    let (_, at) = swap_removed(n, &gone);
    remove_points(&mut out, &gone);
    let mut merged: Vec<u32> = sets.keys().map(|r| at[*r as usize] as u32).collect();
    merged.sort_unstable();
    Ok(BuildOutput {
        mesh: out,
        new_polys: Vec::new(),
        new_points: merged,
    })
}

/// Adds a point or a polygon from Polygon Pen corners (§6.5).
pub fn pen(mesh: &PolyMesh, corners: &[PenCorner]) -> Result<BuildOutput, ToolError> {
    if corners.is_empty() {
        return Err(ToolError::EmptySelection);
    }
    let mut out = mesh.clone();
    let mut poly: Vec<u32> = Vec::new();
    for c in corners {
        let id = match *c {
            PenCorner::Point(p) => {
                if p as usize >= mesh.points.len() {
                    return Err(ToolError::BadPoint(p));
                }
                p
            }
            PenCorner::New(pos) => {
                if !pos.is_finite() {
                    return invalid("pen: positions must be finite");
                }
                out.points.push(pos);
                out.points.len() as u32 - 1
            }
        };
        if poly.contains(&id) {
            return invalid(format!("pen: point {id} is used twice"));
        }
        poly.push(id);
    }
    let new_points: Vec<u32> = (mesh.points.len() as u32..out.points.len() as u32).collect();
    match poly.len() {
        1 => {
            if new_points.is_empty() {
                return invalid("pen: that point exists already");
            }
            Ok(BuildOutput {
                mesh: out,
                new_polys: Vec::new(),
                new_points,
            })
        }
        2 => invalid("pen: a polygon needs three corners (one corner adds a point)"),
        _ => {
            // Newell normal: zero for a degenerate outline
            let mut normal = DVec3::ZERO;
            for i in 0..poly.len() {
                let (a, b) = (
                    out.points[poly[i] as usize],
                    out.points[poly[(i + 1) % poly.len()] as usize],
                );
                normal += a.cross(b);
            }
            if normal.length() <= 1e-12 {
                return invalid("pen: the outline has no area");
            }
            out.polys.push(poly);
            Ok(BuildOutput {
                new_polys: vec![out.polys.len() as u32 - 1],
                mesh: out,
                new_points,
            })
        }
    }
}
