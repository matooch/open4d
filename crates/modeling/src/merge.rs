//! Merging polygons: Melt, Dissolve, Untriangulate share `merge_groups`
//! (docs/specs/MODELING.md §4.5).

use std::collections::{BTreeSet, HashMap, HashSet};

use forge_geometry::PolyMesh;
use forge_geometry::mesh::poly_edges;
use forge_geometry::selection::Selection;

use crate::ToolError;
use crate::settings::Settings;
use crate::util::{
    ToolResult, edge_groups, groups_of, polys_required, polys_selection, remove_points,
    remove_polys, union,
};

fn unsupported<T>(what: &str) -> Result<T, ToolError> {
    Err(ToolError::Unsupported(what.to_string()))
}

/// The boundary loop of a group of polygons: the directed edges whose reverse is not in the
/// group, chained; it starts at the first boundary edge met in polygon and corner order. The
/// points in `drop` are left out of the loop.
pub(crate) fn loop_of(
    mesh: &PolyMesh,
    group: &[u32],
    drop: &BTreeSet<u32>,
) -> Result<Vec<u32>, ToolError> {
    let mut directed: HashSet<(u32, u32)> = HashSet::new();
    for &p in group {
        let poly = &mesh.polys[p as usize];
        for k in 0..poly.len() {
            if !directed.insert((poly[k], poly[(k + 1) % poly.len()])) {
                return unsupported("polygons with inconsistent winding cannot be merged");
            }
        }
    }
    let mut next: HashMap<u32, u32> = HashMap::new();
    let mut first = None;
    for &p in group {
        let poly = &mesh.polys[p as usize];
        for k in 0..poly.len() {
            let (a, b) = (poly[k], poly[(k + 1) % poly.len()]);
            if directed.contains(&(b, a)) {
                continue;
            }
            first.get_or_insert(a);
            if next.insert(a, b).is_some() {
                return unsupported("the boundary of the polygons touches itself");
            }
        }
    }
    let Some(start) = first else {
        return unsupported("the polygons have no boundary (a closed surface)");
    };
    let mut walk = vec![start];
    let mut at = next[&start];
    while at != start {
        walk.push(at);
        match next.get(&at) {
            Some(&n) if walk.len() <= next.len() => at = n,
            _ => return unsupported("the boundary of the polygons is not one closed loop"),
        }
    }
    if walk.len() != next.len() {
        return unsupported("the polygons have a hole or several boundary loops");
    }
    walk.retain(|c| !drop.contains(c));
    if walk.len() < 3 {
        return unsupported("the merged polygon would have fewer than three points");
    }
    Ok(walk)
}

/// Replaces each group by its boundary loop (placed at the group's lowest polygon), removes the
/// other polygons and the points no polygon uses any more. Returns the new indices of the
/// merged polygons. `drop` points are removed from the loops.
pub(crate) fn merge_groups(
    mesh: &PolyMesh,
    groups: &[Vec<u32>],
    drop: &BTreeSet<u32>,
) -> Result<(PolyMesh, Vec<u32>), ToolError> {
    let mut loops: HashMap<u32, Vec<u32>> = HashMap::new();
    let mut removed: HashSet<u32> = HashSet::new();
    let mut touched: BTreeSet<u32> = BTreeSet::new();
    for g in groups {
        let walk = loop_of(mesh, g, drop)?;
        loops.insert(g[0], walk);
        removed.extend(g[1..].iter().copied());
        for &p in g {
            touched.extend(mesh.polys[p as usize].iter().copied());
        }
    }
    let mut out = mesh.clone();
    let mut first_polys = Vec::new();
    for (i, poly) in out.polys.iter_mut().enumerate() {
        if let Some(walk) = loops.remove(&(i as u32)) {
            *poly = walk;
            first_polys.push(i);
        }
    }
    let gone: BTreeSet<u32> = removed.into_iter().collect();
    let at = remove_polys(&mut out, &gone);
    let merged = first_polys.into_iter().map(|i| at[i] as u32).collect();
    let used: HashSet<u32> = out.polys.iter().flatten().copied().collect();
    let unused: BTreeSet<u32> = touched.into_iter().filter(|p| !used.contains(p)).collect();
    remove_points(&mut out, &unused);
    Ok((out, merged))
}

/// Melt (§4.5): edge-connected selected polygons become one polygon each.
pub fn melt(mesh: &PolyMesh, sel: &Selection, _s: &Settings) -> ToolResult {
    let chosen = polys_required(mesh, &sel.polys)?;
    let groups: Vec<Vec<u32>> = edge_groups(mesh, &chosen)
        .into_iter()
        .filter(|g| g.len() > 1)
        // a group without one simple boundary loop (a ring around a hole, a closed
        // surface) is left as it is (`melt-ring`)
        .filter(|g| loop_of(mesh, g, &BTreeSet::new()).is_ok())
        .collect();
    if groups.is_empty() {
        // single polygons stay as they are (`melt-single`, `dissolve-poly`)
        return Ok((mesh.clone(), polys_selection(chosen)));
    }
    let (out, merged) = merge_groups(mesh, &groups, &BTreeSet::new())?;
    Ok((out, polys_selection(merged)))
}

/// Dissolve (§4.5): edges, else points, else polygons (as Melt).
pub fn dissolve(mesh: &PolyMesh, sel: &Selection, s: &Settings) -> ToolResult {
    if !sel.edges.is_empty() {
        let edges = mesh.edges();
        if let Some(e) = sel.edges.iter().find(|e| *e as usize >= edges.len()) {
            return Err(ToolError::BadEdge(e));
        }
        let mut users: HashMap<forge_geometry::Edge, Vec<u32>> = HashMap::new();
        for (i, poly) in mesh.polys.iter().enumerate() {
            for e in poly_edges(poly) {
                users.entry(e).or_default().push(i as u32);
            }
        }
        let all: Vec<u32> = (0..mesh.polys.len() as u32).collect();
        let mut parent: Vec<usize> = (0..all.len()).collect();
        let mut any = false;
        for e in sel.edges.iter() {
            if let Some([a, b]) = users
                .get(&edges[e as usize])
                .map(Vec::as_slice)
                .and_then(|u| <&[u32; 2]>::try_from(u).ok())
            {
                union(&mut parent, *a as usize, *b as usize);
                any = true;
            }
        }
        if !any {
            return Err(ToolError::Invalid(
                "no selected edge is shared by two polygons".to_string(),
            ));
        }
        let groups: Vec<Vec<u32>> = groups_of(&mut parent, &all)
            .into_iter()
            .filter(|g| g.len() > 1)
            .collect();
        let (out, merged) = merge_groups(mesh, &groups, &BTreeSet::new())?;
        return Ok((out, polys_selection(merged)));
    }
    if !sel.points.is_empty() {
        crate::util::check_points(mesh, &sel.points)?;
        let all: Vec<u32> = (0..mesh.polys.len() as u32).collect();
        let mut parent: Vec<usize> = (0..all.len()).collect();
        let mut involved: BTreeSet<usize> = BTreeSet::new();
        for v in sel.points.iter() {
            let users: Vec<usize> = mesh
                .polys
                .iter()
                .enumerate()
                .filter(|(_, p)| p.contains(&v))
                .map(|(i, _)| i)
                .collect();
            for w in users.windows(2) {
                union(&mut parent, w[0], w[1]);
            }
            involved.extend(users);
        }
        let groups: Vec<Vec<u32>> = groups_of(&mut parent, &all)
            .into_iter()
            .filter(|g| g.iter().any(|p| involved.contains(&(*p as usize))))
            .collect();
        if groups.is_empty() {
            return Err(ToolError::Invalid(
                "the selected points belong to no polygon".to_string(),
            ));
        }
        let drop: BTreeSet<u32> = sel.points.iter().collect();
        let (out, merged) = merge_groups(mesh, &groups, &drop)?;
        return Ok((out, polys_selection(merged)));
    }
    melt(mesh, sel, s)
}

/// Used by Untriangulate: merges the groups that have one simple boundary loop and skips the
/// others. Returns the new mesh and the merged polygons' new indices.
pub(crate) fn merge_where_possible(
    mesh: &PolyMesh,
    groups: Vec<Vec<u32>>,
) -> Result<(PolyMesh, Vec<u32>), ToolError> {
    let none = BTreeSet::new();
    let ok: Vec<Vec<u32>> = groups
        .into_iter()
        .filter(|g| loop_of(mesh, g, &none).is_ok())
        .collect();
    merge_groups(mesh, &ok, &none)
}
