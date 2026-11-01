//! Helpers shared by the mesh commands (docs/specs/MODELING.md §4).

use std::collections::{BTreeMap, BTreeSet, HashMap};

use forge_geometry::mesh::poly_edges;
use forge_geometry::selection::{IndexSet, Selection};
use forge_geometry::{Edge, PolyMesh};

use crate::ToolError;

pub(crate) type ToolResult = Result<(PolyMesh, Selection), ToolError>;

/// A selection holding only these polygons.
pub(crate) fn polys_selection(polys: impl IntoIterator<Item = u32>) -> Selection {
    let mut sel = Selection::default();
    for p in polys {
        sel.polys.insert(p);
    }
    sel
}

fn check_polys(mesh: &PolyMesh, polys: &IndexSet) -> Result<(), ToolError> {
    match polys.iter().find(|p| *p as usize >= mesh.polys.len()) {
        Some(p) => Err(ToolError::BadPolygon(p)),
        None => Ok(()),
    }
}

/// The selected polygons, ascending; an empty selection means every polygon (§4).
pub(crate) fn polys_or_all(mesh: &PolyMesh, polys: &IndexSet) -> Result<Vec<u32>, ToolError> {
    check_polys(mesh, polys)?;
    if polys.is_empty() {
        Ok((0..mesh.polys.len() as u32).collect())
    } else {
        Ok(polys.iter().collect())
    }
}

/// The selected polygons, ascending; an empty selection is an error.
pub(crate) fn polys_required(mesh: &PolyMesh, polys: &IndexSet) -> Result<Vec<u32>, ToolError> {
    check_polys(mesh, polys)?;
    if polys.is_empty() {
        return Err(ToolError::EmptySelection);
    }
    Ok(polys.iter().collect())
}

pub(crate) fn check_points(mesh: &PolyMesh, points: &IndexSet) -> Result<(), ToolError> {
    match points.iter().find(|p| *p as usize >= mesh.points.len()) {
        Some(p) => Err(ToolError::BadPoint(p)),
        None => Ok(()),
    }
}

/// Removes consecutive duplicate corners (also across the wrap). `None` when fewer than
/// three corners are left or a corner repeats.
pub(crate) fn clean_poly(poly: &[u32]) -> Option<Vec<u32>> {
    let mut out: Vec<u32> = Vec::with_capacity(poly.len());
    for &c in poly {
        if out.last() != Some(&c) {
            out.push(c);
        }
    }
    while out.len() > 1 && out.first() == out.last() {
        out.pop();
    }
    let distinct: BTreeSet<u32> = out.iter().copied().collect();
    (out.len() >= 3 && distinct.len() == out.len()).then_some(out)
}

/// The ids left after removing `remove` one by one in ascending order by swap-remove (the last
/// item moves into the freed slot), as the id at each final slot; and where each id ended up.
pub(crate) fn swap_removed(n: usize, remove: &BTreeSet<u32>) -> (Vec<u32>, Vec<usize>) {
    let mut slots: Vec<u32> = (0..n as u32).collect();
    let mut at: Vec<usize> = (0..n).collect();
    for &id in remove {
        let slot = at[id as usize];
        slots.swap_remove(slot);
        if slot < slots.len() {
            at[slots[slot] as usize] = slot;
        }
    }
    (slots, at)
}

/// Removes points one by one in ascending index by swap-remove: the last point moves into the
/// freed slot (docs/specs/MODELING.md §4). No polygon may use a removed point.
pub(crate) fn remove_points(mesh: &mut PolyMesh, remove: &BTreeSet<u32>) {
    if remove.is_empty() {
        return;
    }
    let (slots, at) = swap_removed(mesh.points.len(), remove);
    let points: Vec<_> = slots.iter().map(|&id| mesh.points[id as usize]).collect();
    mesh.points = points;
    for poly in &mut mesh.polys {
        for c in poly.iter_mut() {
            *c = at[*c as usize] as u32;
        }
    }
}

/// Removes polygons by swap-remove (§4); returns where each original polygon ended up.
pub(crate) fn remove_polys(mesh: &mut PolyMesh, remove: &BTreeSet<u32>) -> Vec<usize> {
    let (slots, at) = swap_removed(mesh.polys.len(), remove);
    mesh.polys = slots
        .iter()
        .map(|&id| std::mem::take(&mut mesh.polys[id as usize]))
        .collect();
    at
}

/// Edge-connected components of `chosen` (polygons sharing an edge), each ascending, ordered by
/// their smallest polygon.
pub(crate) fn edge_groups(mesh: &PolyMesh, chosen: &[u32]) -> Vec<Vec<u32>> {
    let mut parent: Vec<usize> = (0..chosen.len()).collect();
    let mut by_edge: HashMap<Edge, usize> = HashMap::new();
    for (k, i) in chosen.iter().enumerate() {
        for e in poly_edges(&mesh.polys[*i as usize]) {
            if let Some(&other) = by_edge.get(&e) {
                union(&mut parent, k, other);
            } else {
                by_edge.insert(e, k);
            }
        }
    }
    groups_of(&mut parent, chosen)
}

pub(crate) fn find(parent: &mut [usize], mut x: usize) -> usize {
    while parent[x] != x {
        parent[x] = parent[parent[x]];
        x = parent[x];
    }
    x
}

pub(crate) fn union(parent: &mut [usize], a: usize, b: usize) {
    let (a, b) = (find(parent, a), find(parent, b));
    parent[a.max(b)] = a.min(b);
}

/// The union-find classes over `items`, ordered by smallest member (items are ascending).
pub(crate) fn groups_of(parent: &mut [usize], items: &[u32]) -> Vec<Vec<u32>> {
    let mut by_root: BTreeMap<usize, Vec<u32>> = BTreeMap::new();
    for (k, i) in items.iter().enumerate() {
        by_root.entry(find(parent, k)).or_default().push(*i);
    }
    by_root.into_values().collect()
}
