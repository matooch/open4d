//! Weld, Split, Disconnect, Connect Points, Mirror, Center Axis
//! (docs/specs/MODELING.md §4.6–4.8, §4.10–4.11).

use std::collections::{BTreeMap, BTreeSet, HashSet};

use forge_geometry::PolyMesh;
use forge_geometry::selection::Selection;
use glam::DVec3;

use crate::ToolError;
use crate::settings::Settings;
use crate::util::{
    ToolResult, check_points, clean_poly, edge_groups, polys_or_all, polys_required,
    polys_selection, remove_points, remove_polys,
};

/// Weld (§4.6): the selected points become one point at their average, at the lowest index.
pub fn weld(mesh: &PolyMesh, sel: &Selection, _s: &Settings) -> ToolResult {
    check_points(mesh, &sel.points)?;
    if sel.points.is_empty() {
        return Err(ToolError::EmptySelection);
    }
    if sel.points.len() < 2 {
        return Err(ToolError::Invalid("select at least two points".to_string()));
    }
    let chosen: Vec<u32> = sel.points.iter().collect();
    let target = chosen[0];
    let centre = chosen
        .iter()
        .map(|p| mesh.points[*p as usize])
        .sum::<DVec3>()
        / chosen.len() as f64;
    let mut out = mesh.clone();
    out.points[target as usize] = centre;
    let merged: HashSet<u32> = chosen.iter().copied().collect();
    let mut dropped = BTreeSet::new();
    for (i, poly) in out.polys.iter_mut().enumerate() {
        let mapped: Vec<u32> = poly
            .iter()
            .map(|c| if merged.contains(c) { target } else { *c })
            .collect();
        match clean_poly(&mapped) {
            Some(p) => *poly = p,
            None => {
                dropped.insert(i as u32);
            }
        }
    }
    remove_polys(&mut out, &dropped);
    let gone: BTreeSet<u32> = chosen[1..].iter().copied().collect();
    remove_points(&mut out, &gone);
    Ok((out, Selection::default()))
}

/// The copy Split puts in a new object (§4.7): the selected polygons with their points in
/// ascending original index.
pub fn split_copy(mesh: &PolyMesh, sel: &Selection) -> Result<PolyMesh, ToolError> {
    let chosen = polys_required(mesh, &sel.polys)?;
    let used: BTreeSet<u32> = chosen
        .iter()
        .flat_map(|p| mesh.polys[*p as usize].iter().copied())
        .collect();
    let slot: BTreeMap<u32, u32> = used
        .iter()
        .enumerate()
        .map(|(k, p)| (*p, k as u32))
        .collect();
    Ok(PolyMesh {
        points: used.iter().map(|p| mesh.points[*p as usize]).collect(),
        polys: chosen
            .iter()
            .map(|p| mesh.polys[*p as usize].iter().map(|c| slot[c]).collect())
            .collect(),
    })
}

/// Split as a tool function: the source keeps all its polygons (§4.7); the command adds the
/// copy.
pub fn split(mesh: &PolyMesh, sel: &Selection, _s: &Settings) -> ToolResult {
    split_copy(mesh, sel)?;
    Ok((mesh.clone(), polys_selection(sel.polys.iter())))
}

/// Disconnect (§4.7).
pub fn disconnect(mesh: &PolyMesh, sel: &Selection, s: &Settings) -> ToolResult {
    let chosen = polys_required(mesh, &sel.polys)?;
    let groups: Vec<Vec<u32>> = if s.bool("preserve_groups") {
        edge_groups(mesh, &chosen)
    } else {
        chosen.iter().map(|p| vec![*p]).collect()
    };
    let mut total = vec![0u32; mesh.points.len()];
    for poly in &mesh.polys {
        for c in poly {
            total[*c as usize] += 1;
        }
    }
    let mut out = mesh.clone();
    for group in &groups {
        let mut inside: BTreeMap<u32, u32> = BTreeMap::new();
        for p in group {
            for c in &mesh.polys[*p as usize] {
                *inside.entry(*c).or_default() += 1;
            }
        }
        let mut copy: BTreeMap<u32, u32> = BTreeMap::new();
        for (point, count) in &inside {
            if total[*point as usize] > *count {
                copy.insert(*point, out.points.len() as u32);
                out.points.push(mesh.points[*point as usize]);
            }
        }
        // the group no longer uses the originals it copied: later groups see only the
        // polygons still on them (`disconnect-all`: the last polygons keep the originals)
        for point in copy.keys() {
            total[*point as usize] -= inside[point];
        }
        for p in group {
            for c in out.polys[*p as usize].iter_mut() {
                if let Some(n) = copy.get(c) {
                    *c = *n;
                }
            }
        }
    }
    Ok((out, polys_selection(chosen)))
}

/// Connect Points (§4.8): polygons with exactly two selected, non-neighbouring points are cut.
pub fn connect_points(mesh: &PolyMesh, sel: &Selection, _s: &Settings) -> ToolResult {
    check_points(mesh, &sel.points)?;
    if sel.points.is_empty() {
        if sel.edges.is_empty() {
            return Err(ToolError::EmptySelection);
        }
        return Err(ToolError::Unsupported(
            "Connect on edges (the reference connects edge midpoints; not observed)".to_string(),
        ));
    }
    let mut polys = mesh.polys.clone();
    let mut appended = Vec::new();
    let mut cut = Vec::new();
    for (i, poly) in mesh.polys.iter().enumerate() {
        let at: Vec<usize> = (0..poly.len())
            .filter(|k| sel.points.contains(poly[*k]))
            .collect();
        match at.as_slice() {
            [] | [_] => {}
            [a, b] => {
                let n = poly.len();
                if b - a == 1 || (*a == 0 && *b == n - 1) {
                    continue;
                }
                let mut kept = poly[..=*a].to_vec();
                kept.extend_from_slice(&poly[*b..]);
                appended.push(poly[*a..=*b].to_vec());
                polys[i] = kept;
                cut.push(i as u32);
            }
            _ => {
                return Err(ToolError::Unsupported(
                    "three or more selected points in one polygon (no golden data)".to_string(),
                ));
            }
        }
    }
    if cut.is_empty() {
        return Err(ToolError::Invalid(
            "no polygon holds two selected points that are not neighbours".to_string(),
        ));
    }
    let base = polys.len() as u32;
    let count = appended.len() as u32;
    polys.extend(appended);
    let selected = cut.into_iter().chain(base..base + count);
    Ok((
        PolyMesh {
            points: mesh.points.clone(),
            polys,
        },
        polys_selection(selected),
    ))
}

/// Mirror (§4.10).
pub fn mirror(mesh: &PolyMesh, sel: &Selection, s: &Settings) -> ToolResult {
    let chosen = polys_or_all(mesh, &sel.polys)?;
    let n = s.vec3("plane_normal");
    if n.length() < 1e-12 {
        return Err(ToolError::Invalid("the plane normal is zero".to_string()));
    }
    let n = n.normalize();
    let q = s.vec3("plane_point");
    let reflect = |p: DVec3| p - 2.0 * (p - q).dot(n) * n;
    let used: BTreeSet<u32> = chosen
        .iter()
        .flat_map(|p| mesh.polys[*p as usize].iter().copied())
        .collect();
    let mut out = mesh.clone();
    if !s.bool("duplicate") {
        for &p in &used {
            out.points[p as usize] = reflect(mesh.points[p as usize]);
        }
        for &i in &chosen {
            out.polys[i as usize][1..].reverse();
        }
        return Ok((out, polys_selection(chosen)));
    }
    let tol = s.float("tolerance");
    let mut copy: BTreeMap<u32, u32> = BTreeMap::new();
    for &p in &used {
        let at = mesh.points[p as usize];
        if s.bool("weld") && (at - q).dot(n).abs() <= tol {
            copy.insert(p, p);
        } else {
            copy.insert(p, out.points.len() as u32);
            out.points.push(reflect(at));
        }
    }
    let base = out.polys.len() as u32;
    for &i in &chosen {
        let mut poly: Vec<u32> = mesh.polys[i as usize].iter().map(|c| copy[c]).collect();
        poly[1..].reverse();
        out.polys.push(poly);
    }
    let end = out.polys.len() as u32;
    Ok((out, polys_selection(base..end)))
}

/// Center Axis (§4.11): the mesh moved by `-c` and the centre `c` (object space).
pub fn center_axis(mesh: &PolyMesh, s: &Settings) -> Result<(PolyMesh, DVec3), ToolError> {
    if mesh.points.is_empty() {
        return Err(ToolError::EmptySelection);
    }
    let c = match s.choice("mode") {
        "point_average" => mesh.points.iter().sum::<DVec3>() / mesh.points.len() as f64,
        _ => {
            let (lo, hi) = mesh.points.iter().fold(
                (DVec3::splat(f64::MAX), DVec3::splat(f64::MIN)),
                |(lo, hi), p| (lo.min(*p), hi.max(*p)),
            );
            (lo + hi) * 0.5
        }
    };
    let out = PolyMesh {
        points: mesh.points.iter().map(|p| *p - c).collect(),
        polys: mesh.polys.clone(),
    };
    Ok((out, c))
}

/// Center Axis as a tool function (the preview): the re-centred mesh.
pub fn center_axis_tool(mesh: &PolyMesh, _sel: &Selection, s: &Settings) -> ToolResult {
    Ok((center_axis(mesh, s)?.0, Selection::default()))
}
