//! Slide (docs/specs/MODELING.md §5.3). Own design, parity-pending.

use std::collections::{BTreeMap, VecDeque};

use forge_geometry::selection::{IndexSet, Selection};
use forge_geometry::{Edge, PolyMesh};
use glam::DVec3;

use crate::util::ToolResult;
use crate::{Settings, ToolError};

fn invalid<T>(msg: &str) -> Result<T, ToolError> {
    Err(ToolError::Invalid(msg.to_string()))
}

/// Orients every selected edge: the lowest id of a chain runs low to high, and the direction
/// continues through points where exactly two selected edges meet.
fn orient_chains(edges: &[Edge], chosen: &[u32]) -> BTreeMap<u32, (u32, u32)> {
    let mut at: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
    for &id in chosen {
        let e = edges[id as usize];
        at.entry(e.0).or_default().push(id);
        at.entry(e.1).or_default().push(id);
    }
    let mut dir: BTreeMap<u32, (u32, u32)> = BTreeMap::new();
    for &seed in chosen {
        if dir.contains_key(&seed) {
            continue;
        }
        let e = edges[seed as usize];
        dir.insert(seed, (e.0, e.1));
        let mut queue = VecDeque::from([seed]);
        while let Some(id) = queue.pop_front() {
            let (u, w) = dir[&id];
            for v in [u, w] {
                let list = &at[&v];
                if list.len() != 2 {
                    continue;
                }
                let other = if list[0] == id { list[1] } else { list[0] };
                if dir.contains_key(&other) {
                    continue;
                }
                let oe = edges[other as usize];
                let far = if oe.0 == v { oe.1 } else { oe.0 };
                // an edge ending at v continues with one starting at v, and the reverse
                dir.insert(other, if v == w { (v, far) } else { (far, v) });
                queue.push_back(other);
            }
        }
    }
    dir
}

/// The far end of the rail at `v` on the left (`left`) or right side of the edge `u -> w`.
fn rail(mesh: &PolyMesh, (u, w): (u32, u32), v: u32, left: bool) -> Option<u32> {
    let (a, b) = if left { (u, w) } else { (w, u) };
    for poly in &mesh.polys {
        let n = poly.len();
        let Some(i) = (0..n).find(|&i| poly[i] == a && poly[(i + 1) % n] == b) else {
            continue;
        };
        // the polygon runs a -> b; the rail at a is the previous corner, at b the corner after
        return Some(if v == a {
            poly[(i + n - 1) % n]
        } else {
            poly[(i + 2) % n]
        });
    }
    None
}

fn slide_edges(mesh: &PolyMesh, selected: &IndexSet, amount: f64) -> Result<PolyMesh, ToolError> {
    let edges = mesh.edges();
    if let Some(e) = selected.iter().find(|e| *e as usize >= edges.len()) {
        return Err(ToolError::BadEdge(e));
    }
    let chosen: Vec<u32> = selected.iter().collect();
    let dir = orient_chains(&edges, &chosen);
    let left = amount > 0.0;
    let mut target: BTreeMap<u32, u32> = BTreeMap::new();
    for &id in &chosen {
        let (u, w) = dir[&id];
        for v in [u, w] {
            let q = rail(mesh, (u, w), v, left).ok_or_else(|| {
                ToolError::Invalid(format!("slide: edge {id} has no polygon on that side"))
            })?;
            target.entry(v).or_insert(q);
        }
    }
    let mut out = mesh.clone();
    for (v, q) in target {
        let (p, t): (DVec3, DVec3) = (mesh.points[v as usize], mesh.points[q as usize]);
        out.points[v as usize] = p + (t - p) * amount.abs();
    }
    Ok(out)
}

fn slide_points(
    mesh: &PolyMesh,
    points: &IndexSet,
    toward: i64,
    amount: f64,
) -> Result<PolyMesh, ToolError> {
    if let Some(p) = points.iter().find(|p| *p as usize >= mesh.points.len()) {
        return Err(ToolError::BadPoint(p));
    }
    let q = u32::try_from(toward)
        .map_err(|_| ToolError::Invalid("slide: set `toward`, the point to slide to".into()))?;
    if q as usize >= mesh.points.len() {
        return Err(ToolError::BadPoint(q));
    }
    if amount <= 0.0 {
        return invalid("slide: a point slides with a positive amount");
    }
    let ids = mesh.edge_ids();
    let mut out = mesh.clone();
    for p in points.iter().filter(|p| *p != q) {
        if !ids.contains_key(&Edge::new(p, q)) {
            return Err(ToolError::Invalid(format!(
                "slide: points {p} and {q} do not share an edge"
            )));
        }
        let (a, b) = (mesh.points[p as usize], mesh.points[q as usize]);
        out.points[p as usize] = a + (b - a) * amount.min(1.0);
    }
    Ok(out)
}

/// Slide the selected edges sideways along their neighbours, or the selected points toward a
/// point (§5.3). The selection is returned unchanged.
pub fn slide(mesh: &PolyMesh, sel: &Selection, s: &Settings) -> ToolResult {
    let amount = s.float("amount").clamp(-1.0, 1.0);
    if amount == 0.0 {
        return invalid("slide: the amount is 0");
    }
    let moved = if !sel.edges.is_empty() {
        slide_edges(mesh, &sel.edges, amount)?
    } else if !sel.points.is_empty() {
        slide_points(mesh, &sel.points, s.int("toward"), amount)?
    } else {
        return Err(ToolError::EmptySelection);
    };
    Ok((moved, sel.clone()))
}
