//! Selection commands on a mesh: edge grow/shrink, convert, connected, loop and ring
//! (docs/specs/EDITABLE.md §2.8). Edge ids are those of [`PolyMesh::edges`]. Out-of-range
//! input indices are ignored. Loop and ring are own design (parity-pending).

use std::collections::{BTreeMap, BTreeSet};

use crate::PolyMesh;
use crate::mesh::{Edge, poly_edges};
use crate::selection::{
    IndexSet, Mode, edges_to_points, points_to_edges, points_to_polys, polys_to_edges,
    polys_to_points,
};

/// The kind of element a set holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Points,
    Edges,
    Polys,
}

fn selected_edges(mesh: &PolyMesh, edges: &IndexSet) -> Vec<Edge> {
    let all = mesh.edges();
    edges
        .iter()
        .filter_map(|i| all.get(i as usize))
        .copied()
        .collect()
}

/// Edges sharing a point with each edge.
fn edge_neighbors(mesh: &PolyMesh) -> Vec<BTreeSet<u32>> {
    let edges = mesh.edges();
    let mut at_point: Vec<Vec<u32>> = vec![Vec::new(); mesh.points.len()];
    for (i, e) in edges.iter().enumerate() {
        at_point[e.0 as usize].push(i as u32);
        at_point[e.1 as usize].push(i as u32);
    }
    edges
        .iter()
        .enumerate()
        .map(|(i, e)| {
            [e.0, e.1]
                .iter()
                .flat_map(|p| at_point[*p as usize].iter().copied())
                .filter(|n| *n as usize != i)
                .collect()
        })
        .collect()
}

/// Adds every edge that shares a point with a selected edge.
pub fn grow_edges(mesh: &PolyMesh, sel: &IndexSet) -> IndexSet {
    let n = edge_neighbors(mesh);
    let mut out = sel.clone();
    for i in sel.iter() {
        for j in n.get(i as usize).into_iter().flatten() {
            out.insert(*j);
        }
    }
    out
}

/// Keeps the selected edges whose point-sharing neighbours are all selected.
pub fn shrink_edges(mesh: &PolyMesh, sel: &IndexSet) -> IndexSet {
    let n = edge_neighbors(mesh);
    sel.iter()
        .filter(|i| {
            n.get(*i as usize)
                .is_some_and(|ns| ns.iter().all(|j| sel.contains(*j)))
        })
        .collect()
}

/// Polygons whose edges are all (`All`) or any (`Any`) selected.
pub fn edges_to_polys(mesh: &PolyMesh, edges: &IndexSet, mode: Mode) -> IndexSet {
    let ids = mesh.edge_ids();
    (0..mesh.polys.len() as u32)
        .filter(|&f| {
            let mut it = poly_edges(&mesh.polys[f as usize]).map(|e| edges.contains(ids[&e]));
            match mode {
                Mode::All => it.all(|b| b),
                Mode::Any => it.any(|b| b),
            }
        })
        .collect()
}

/// Converts `sel` of kind `from` to kind `to`. `Mode::All` is the strict rule, `Mode::Any`
/// the tolerant one; it only matters when going from smaller to larger elements.
pub fn convert(mesh: &PolyMesh, sel: &IndexSet, from: Kind, to: Kind, mode: Mode) -> IndexSet {
    match (from, to) {
        (a, b) if a == b => sel.clone(),
        (Kind::Polys, Kind::Points) => polys_to_points(mesh, sel),
        (Kind::Polys, Kind::Edges) => polys_to_edges(mesh, sel),
        (Kind::Points, Kind::Edges) => points_to_edges(mesh, sel, mode),
        (Kind::Points, Kind::Polys) => points_to_polys(mesh, sel, mode),
        (Kind::Edges, Kind::Points) => edges_to_points(mesh, sel),
        (Kind::Edges, Kind::Polys) => edges_to_polys(mesh, sel, mode),
        _ => unreachable!("equal kinds are handled above"),
    }
}

/// Piece id of each point: points are in one piece when a polygon joins them.
fn point_pieces(mesh: &PolyMesh) -> Vec<usize> {
    let mut parent: Vec<usize> = (0..mesh.points.len()).collect();
    fn find(parent: &mut [usize], mut x: usize) -> usize {
        while parent[x] != x {
            parent[x] = parent[parent[x]];
            x = parent[x];
        }
        x
    }
    for poly in &mesh.polys {
        let Some(&first) = poly.first() else { continue };
        for &v in &poly[1..] {
            let (a, b) = (
                find(&mut parent, first as usize),
                find(&mut parent, v as usize),
            );
            if a != b {
                parent[a.max(b)] = a.min(b);
            }
        }
    }
    (0..parent.len()).map(|i| find(&mut parent, i)).collect()
}

/// Every element of the pieces (points joined through polygons) that hold a selected
/// element of kind `kind`.
pub fn connected(mesh: &PolyMesh, kind: Kind, sel: &IndexSet) -> IndexSet {
    let piece = point_pieces(mesh);
    let of_point = |p: u32| piece.get(p as usize).copied();
    let wanted: BTreeSet<usize> = match kind {
        Kind::Points => sel.iter().filter_map(of_point).collect(),
        Kind::Edges => selected_edges(mesh, sel)
            .iter()
            .filter_map(|e| of_point(e.0))
            .collect(),
        Kind::Polys => sel
            .iter()
            .filter_map(|f| mesh.polys.get(f as usize)?.first().copied())
            .filter_map(of_point)
            .collect(),
    };
    match kind {
        Kind::Points => (0..mesh.points.len() as u32)
            .filter(|p| of_point(*p).is_some_and(|c| wanted.contains(&c)))
            .collect(),
        Kind::Edges => (0u32..)
            .zip(mesh.edges())
            .filter(|(_, e)| of_point(e.0).is_some_and(|c| wanted.contains(&c)))
            .map(|(i, _)| i)
            .collect(),
        Kind::Polys => (0..mesh.polys.len() as u32)
            .filter(|f| {
                mesh.polys[*f as usize]
                    .first()
                    .and_then(|p| of_point(*p))
                    .is_some_and(|c| wanted.contains(&c))
            })
            .collect(),
    }
}

/// Edge and polygon adjacency used by loop and ring.
struct Adjacency<'a> {
    mesh: &'a PolyMesh,
    edges: Vec<Edge>,
    ids: BTreeMap<Edge, u32>,
    /// Polygons of each edge, ascending.
    faces: Vec<Vec<u32>>,
    /// Edges at each point.
    at_point: Vec<Vec<u32>>,
    /// Polygons at each point.
    polys_at: Vec<Vec<u32>>,
}

impl<'a> Adjacency<'a> {
    fn new(mesh: &'a PolyMesh) -> Adjacency<'a> {
        let edges = mesh.edges();
        let ids = mesh.edge_ids();
        let mut faces = vec![Vec::new(); edges.len()];
        for (f, p) in mesh.polys.iter().enumerate() {
            for e in poly_edges(p) {
                let list: &mut Vec<u32> = &mut faces[ids[&e] as usize];
                if !list.contains(&(f as u32)) {
                    list.push(f as u32);
                }
            }
        }
        let mut polys_at: Vec<Vec<u32>> = vec![Vec::new(); mesh.points.len()];
        for (f, p) in mesh.polys.iter().enumerate() {
            for &v in p {
                let list = &mut polys_at[v as usize];
                if !list.contains(&(f as u32)) {
                    list.push(f as u32);
                }
            }
        }
        let mut at_point = vec![Vec::new(); mesh.points.len()];
        for (i, e) in edges.iter().enumerate() {
            at_point[e.0 as usize].push(i as u32);
            at_point[e.1 as usize].push(i as u32);
        }
        Adjacency {
            mesh,
            edges,
            ids,
            faces,
            at_point,
            polys_at,
        }
    }

    fn is_border(&self, e: u32) -> bool {
        self.faces[e as usize].len() == 1
    }

    /// The edge a loop continues on after arriving along `e` at point `v`.
    fn loop_next(&self, e: u32, v: u32) -> Option<u32> {
        let around = &self.at_point[v as usize];
        if self.is_border(e) {
            let border: Vec<u32> = around
                .iter()
                .copied()
                .filter(|x| self.is_border(*x))
                .collect();
            return (border.len() == 2 && around.len() == 3)
                .then(|| border.into_iter().find(|x| *x != e))
                .flatten();
        }
        let interior = around.iter().all(|x| self.faces[*x as usize].len() == 2);
        if around.len() != 4
            || !interior
            || self.polys_at[v as usize]
                .iter()
                .any(|f| self.mesh.polys[*f as usize].len() != 4)
        {
            return None;
        }
        let mine = &self.faces[e as usize];
        let mut opposite = around
            .iter()
            .copied()
            .filter(|x| *x != e && self.faces[*x as usize].iter().all(|f| !mine.contains(f)));
        let found = opposite.next()?;
        opposite.next().is_none().then_some(found)
    }

    fn walk_loop(&self, start: u32, out: &mut BTreeSet<u32>) {
        out.insert(start);
        let Edge(a, b) = self.edges[start as usize];
        for end in [b, a] {
            let (mut e, mut v) = (start, end);
            while let Some(next) = self.loop_next(e, v) {
                if !out.insert(next) {
                    break;
                }
                let Edge(x, y) = self.edges[next as usize];
                v = if x == v { y } else { x };
                e = next;
            }
        }
    }

    /// The edge of a quad opposite to local edge `j`.
    fn opposite_in(&self, f: u32, e: u32) -> Option<u32> {
        let poly = &self.mesh.polys[f as usize];
        if poly.len() != 4 {
            return None;
        }
        let j = poly_edges(poly).position(|x| self.ids[&x] == e)?;
        Some(self.ids[&Edge::new(poly[(j + 2) % 4], poly[(j + 3) % 4])])
    }

    fn walk_ring(&self, start: u32, out: &mut BTreeSet<u32>) {
        out.insert(start);
        for first in self.faces[start as usize].clone() {
            let (mut e, mut f) = (start, first);
            while let Some(o) = self.opposite_in(f, e) {
                if !out.insert(o) {
                    break;
                }
                match self.faces[o as usize].iter().copied().find(|g| *g != f) {
                    Some(g) => {
                        e = o;
                        f = g;
                    }
                    None => break,
                }
            }
        }
    }

    /// The strip of quads through `start`, crossing its edge pair `across` (0 or 1).
    fn walk_strip(&self, start: u32, across: u8, out: &mut BTreeSet<u32>) {
        out.insert(start);
        let poly = &self.mesh.polys[start as usize];
        if poly.len() != 4 {
            return;
        }
        let local = usize::from(across & 1);
        for exit in [local, local + 2] {
            let mut e = self.ids[&Edge::new(poly[exit], poly[(exit + 1) % 4])];
            let mut from = start;
            while let Some(g) = self.faces[e as usize].iter().copied().find(|g| *g != from) {
                if !out.insert(g) {
                    break;
                }
                let Some(o) = self.opposite_in(g, e) else {
                    break;
                };
                e = o;
                from = g;
            }
        }
    }
}

/// Edge loops through the selected edges (see EDITABLE.md §2.8).
pub fn edge_loop(mesh: &PolyMesh, edges: &IndexSet) -> IndexSet {
    let adj = Adjacency::new(mesh);
    let mut out = BTreeSet::new();
    for e in edges.iter().filter(|e| (*e as usize) < adj.edges.len()) {
        adj.walk_loop(e, &mut out);
    }
    out.into_iter().collect()
}

/// Edge rings through the selected edges.
pub fn edge_ring(mesh: &PolyMesh, edges: &IndexSet) -> IndexSet {
    let adj = Adjacency::new(mesh);
    let mut out = BTreeSet::new();
    for e in edges.iter().filter(|e| (*e as usize) < adj.edges.len()) {
        adj.walk_ring(e, &mut out);
    }
    out.into_iter().collect()
}

/// Quad strips through the selected polygons, crossing each one's edge pair `across`.
pub fn poly_strip(mesh: &PolyMesh, polys: &IndexSet, across: u8) -> IndexSet {
    let adj = Adjacency::new(mesh);
    let mut out = BTreeSet::new();
    for f in polys.iter().filter(|f| (*f as usize) < mesh.polys.len()) {
        adj.walk_strip(f, across, &mut out);
    }
    out.into_iter().collect()
}
