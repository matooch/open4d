//! Point/edge/polygon selections and conversions (docs/specs/GEOMETRY.md §4).
//! Edge ids are those of [`PolyMesh::edges`]. Out-of-range input indices are ignored.

use std::collections::{BTreeMap, BTreeSet};

use crate::PolyMesh;
use crate::mesh::poly_edges;

/// Sorted set of element indices.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct IndexSet(BTreeSet<u32>);

impl IndexSet {
    pub fn new() -> IndexSet {
        IndexSet::default()
    }

    pub fn insert(&mut self, i: u32) -> bool {
        self.0.insert(i)
    }

    pub fn remove(&mut self, i: u32) -> bool {
        self.0.remove(&i)
    }

    pub fn contains(&self, i: u32) -> bool {
        self.0.contains(&i)
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Ascending.
    pub fn iter(&self) -> impl Iterator<Item = u32> + '_ {
        self.0.iter().copied()
    }

    pub fn union(&self, other: &IndexSet) -> IndexSet {
        IndexSet(&self.0 | &other.0)
    }

    pub fn intersection(&self, other: &IndexSet) -> IndexSet {
        IndexSet(&self.0 & &other.0)
    }

    pub fn difference(&self, other: &IndexSet) -> IndexSet {
        IndexSet(&self.0 - &other.0)
    }

    /// Complement within `0..count`.
    pub fn invert(&self, count: u32) -> IndexSet {
        (0..count).filter(|i| !self.contains(*i)).collect()
    }
}

impl FromIterator<u32> for IndexSet {
    fn from_iter<I: IntoIterator<Item = u32>>(iter: I) -> Self {
        IndexSet(iter.into_iter().collect())
    }
}

/// The three selection sets an editable mesh carries.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Selection {
    pub points: IndexSet,
    pub edges: IndexSet,
    pub polys: IndexSet,
}

/// Whether a converted element needs all or any of its parts selected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    All,
    Any,
}

fn selected_polys<'a>(
    mesh: &'a PolyMesh,
    polys: &'a IndexSet,
) -> impl Iterator<Item = &'a Vec<u32>> {
    polys.iter().filter_map(|f| mesh.polys.get(f as usize))
}

fn matches(mode: Mode, mut verts: impl Iterator<Item = u32>, points: &IndexSet) -> bool {
    match mode {
        Mode::All => verts.all(|v| points.contains(v)),
        Mode::Any => verts.any(|v| points.contains(v)),
    }
}

pub fn polys_to_points(mesh: &PolyMesh, polys: &IndexSet) -> IndexSet {
    selected_polys(mesh, polys).flatten().copied().collect()
}

pub fn polys_to_edges(mesh: &PolyMesh, polys: &IndexSet) -> IndexSet {
    let ids = mesh.edge_ids();
    selected_polys(mesh, polys)
        .flat_map(|p| poly_edges(p).map(|e| ids[&e]).collect::<Vec<_>>())
        .collect()
}

pub fn points_to_polys(mesh: &PolyMesh, points: &IndexSet, mode: Mode) -> IndexSet {
    (0..mesh.polys.len() as u32)
        .filter(|&f| matches(mode, mesh.polys[f as usize].iter().copied(), points))
        .collect()
}

pub fn points_to_edges(mesh: &PolyMesh, points: &IndexSet, mode: Mode) -> IndexSet {
    (0u32..)
        .zip(mesh.edges())
        .filter(|(_, e)| matches(mode, [e.0, e.1].into_iter(), points))
        .map(|(i, _)| i)
        .collect()
}

pub fn edges_to_points(mesh: &PolyMesh, edges: &IndexSet) -> IndexSet {
    let all = mesh.edges();
    edges
        .iter()
        .filter_map(|i| all.get(i as usize))
        .flat_map(|e| [e.0, e.1])
        .collect()
}

/// Edge-connected neighbours of each point.
fn point_neighbors(mesh: &PolyMesh) -> Vec<BTreeSet<u32>> {
    let mut n = vec![BTreeSet::new(); mesh.points.len()];
    for e in mesh.edges() {
        n[e.0 as usize].insert(e.1);
        n[e.1 as usize].insert(e.0);
    }
    n
}

/// Polygons sharing an edge with each polygon.
fn poly_neighbors(mesh: &PolyMesh) -> Vec<BTreeSet<u32>> {
    let mut by_edge: BTreeMap<crate::Edge, Vec<u32>> = BTreeMap::new();
    for (f, p) in mesh.polys.iter().enumerate() {
        for e in poly_edges(p) {
            by_edge.entry(e).or_default().push(f as u32);
        }
    }
    let mut n = vec![BTreeSet::new(); mesh.polys.len()];
    for fs in by_edge.values() {
        for &a in fs {
            for &b in fs {
                if a != b {
                    n[a as usize].insert(b);
                }
            }
        }
    }
    n
}

fn grow(neighbors: &[BTreeSet<u32>], sel: &IndexSet) -> IndexSet {
    let mut out = sel.clone();
    for i in sel.iter() {
        if let Some(ns) = neighbors.get(i as usize) {
            out.0.extend(ns);
        }
    }
    out
}

fn shrink(neighbors: &[BTreeSet<u32>], sel: &IndexSet) -> IndexSet {
    sel.iter()
        .filter(|&i| {
            neighbors
                .get(i as usize)
                .is_some_and(|ns| ns.iter().all(|n| sel.contains(*n)))
        })
        .collect()
}

pub fn grow_points(mesh: &PolyMesh, points: &IndexSet) -> IndexSet {
    grow(&point_neighbors(mesh), points)
}

pub fn shrink_points(mesh: &PolyMesh, points: &IndexSet) -> IndexSet {
    shrink(&point_neighbors(mesh), points)
}

pub fn grow_polys(mesh: &PolyMesh, polys: &IndexSet) -> IndexSet {
    grow(&poly_neighbors(mesh), polys)
}

pub fn shrink_polys(mesh: &PolyMesh, polys: &IndexSet) -> IndexSet {
    shrink(&poly_neighbors(mesh), polys)
}
