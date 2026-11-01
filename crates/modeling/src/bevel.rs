//! Bevel (docs/specs/MODELING.md §3): polygon mode (inset plus lift) and edge mode (chamfer
//! and solid) on closed meshes whose beveled vertices have three polygons.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use forge_geometry::PolyMesh;
use forge_geometry::normals::poly_normal;
use forge_geometry::selection::{IndexSet, Selection};
use glam::DVec3;

use crate::ToolError;
use crate::extrude::bevel_polys;
use crate::settings::Settings;

fn unsupported<T>(what: &str) -> Result<T, ToolError> {
    Err(ToolError::Unsupported(what.to_string()))
}

/// The checked settings of a run.
struct Options {
    solid: bool,
    proportional: bool,
    offset: f64,
    levels: usize,
    extrusion: f64,
}

impl Options {
    fn read(s: &Settings) -> Result<Options, ToolError> {
        let off = |what: &str| format!("{what} (no reference data)");
        if s.choice("shape") != "round" {
            return unsupported("Shape User/Profile");
        }
        if s.choice("offset_mode") == "radial" {
            return unsupported(&off("Offset Mode Radial"));
        }
        if s.float("tension") != 1.0 {
            return unsupported(&off("Tension other than 1"));
        }
        if s.float("depth") != 1.0 {
            return unsupported(&off("Depth other than 1"));
        }
        if matches!(s.choice("mitering"), "radial" | "patch") {
            return unsupported(&off("Mitering Radial/Patch"));
        }
        if s.choice("ending") != "default" {
            return unsupported(&off("Ending Extend/Inset"));
        }
        if s.choice("partial_rounding") != "none" {
            return unsupported(&off("Partial Rounding"));
        }
        if s.bool("corner_ngons") || s.bool("rounding_ngons") {
            return unsupported(&off("Corner/Rounding N-gons"));
        }
        Ok(Options {
            solid: s.choice("bevel_mode") == "solid",
            proportional: s.choice("offset_mode") == "proportional",
            offset: s.float("offset"),
            levels: s.int("subdivision").max(0) as usize,
            extrusion: s.float("extrusion"),
        })
    }
}

/// Bevels the selection: edges, else polygons, else points (point mode is unsupported).
pub fn bevel(
    mesh: &PolyMesh,
    sel: &Selection,
    s: &Settings,
) -> Result<(PolyMesh, Selection), ToolError> {
    let o = Options::read(s)?;
    if !sel.edges.is_empty() {
        bevel_edges(mesh, &sel.edges, &o)
    } else if !sel.polys.is_empty() {
        if o.solid {
            return unsupported("Bevel Mode Solid in polygon mode (no reference data)");
        }
        if o.levels > 0 {
            return unsupported("Subdivision in polygon mode (no reference data)");
        }
        if o.proportional {
            return unsupported("Offset Mode Proportional in polygon mode (no reference data)");
        }
        bevel_polys(mesh, &sel.polys, o.offset, o.extrusion)
    } else if !sel.points.is_empty() {
        unsupported("point mode (no reference data)")
    } else {
        Err(ToolError::EmptySelection)
    }
}

/// A directed edge `u -> w`: the polygon and the corner of `u`.
type DirMap = HashMap<(u32, u32), (u32, usize)>;

/// One beveled edge `a -> b` with its two polygons and its profile.
struct Strip {
    a: u32,
    b: u32,
    /// The polygon that holds `b -> a` (level 0).
    f_a: u32,
    half: usize,
    dir: DVec3,
    /// The level points of the cross-section through `a`.
    profile: Vec<DVec3>,
}

/// What a beveled vertex needs.
struct Vertex {
    strips: Vec<usize>,
    /// Creation ids by native level (one selected edge: level of strip 0; two: from `f0`).
    ids: Vec<u32>,
    /// One selected edge: the neighbours of the two end points and their levels.
    n1: u32,
    n2: u32,
    cap: u32,
    /// Two selected edges: the polygon between them.
    f0: u32,
}

fn next_of(mesh: &PolyMesh, dir: &DirMap, from: u32, v: u32) -> Option<(u32, u32)> {
    let (p, k) = *dir.get(&(from, v))?;
    let poly = &mesh.polys[p as usize];
    Some((p, poly[(k + 2) % poly.len()]))
}

fn bevel_edges(
    mesh: &PolyMesh,
    edges: &IndexSet,
    o: &Options,
) -> Result<(PolyMesh, Selection), ToolError> {
    let all = mesh.edges();
    let mut dir: DirMap = HashMap::new();
    let mut base = Vec::with_capacity(mesh.polys.len());
    let mut total = 0usize;
    for (pi, poly) in mesh.polys.iter().enumerate() {
        base.push(total);
        total += poly.len();
        for k in 0..poly.len() {
            let key = (poly[k], poly[(k + 1) % poly.len()]);
            if dir.insert(key, (pi as u32, k)).is_some() {
                return unsupported("edge used twice in one direction");
            }
        }
    }
    let n = o.levels;
    let mut strips: Vec<Strip> = Vec::new();
    for id in edges.iter() {
        let e = all.get(id as usize).ok_or(ToolError::BadEdge(id))?;
        let (Some(&(p0, k0)), Some(&(p1, k1))) = (dir.get(&(e.0, e.1)), dir.get(&(e.1, e.0)))
        else {
            return unsupported("open edge");
        };
        let (h0, h1) = (base[p0 as usize] + k0, base[p1 as usize] + k1);
        // a -> b is the direction of the lower half-edge
        let (a, b, f_b, f_a, half) = if h0 < h1 {
            (e.0, e.1, p0, p1, h0)
        } else {
            (e.1, e.0, p1, p0, h1)
        };
        strips.push(make_strip(mesh, a, b, f_a, f_b, half, o)?);
    }
    strips.sort_by_key(|s| s.half);
    let mut at: BTreeMap<u32, Vertex> = BTreeMap::new();
    for (si, s) in strips.iter().enumerate() {
        for v in [s.a, s.b] {
            at.entry(v)
                .or_insert_with(|| Vertex {
                    strips: Vec::new(),
                    ids: vec![0; n + 2],
                    n1: 0,
                    n2: 0,
                    cap: 0,
                    f0: 0,
                })
                .strips
                .push(si);
        }
    }
    for (v, info) in at.iter_mut() {
        classify(mesh, &dir, &strips, *v, info)?;
    }
    if o.solid {
        if n > 0 {
            return unsupported("Bevel Mode Solid with Subdivision (no reference data)");
        }
        if at.values().any(|i| i.strips.len() != 1) {
            return unsupported("Bevel Mode Solid at a vertex with two selected edges");
        }
    }

    // new points, in two passes (MODELING.md §3.4)
    let old = mesh.points.len() as u32;
    let mut made: Vec<DVec3> = Vec::new();
    let mut seen: BTreeSet<u32> = BTreeSet::new();
    for s in &strips {
        for v in [s.a, s.b] {
            if !seen.insert(v) {
                continue;
            }
            let info = &at[&v];
            let ends: Vec<usize> = if info.strips.len() == 1 {
                let first = if v == s.a { 0 } else { n + 1 };
                vec![first, n + 1 - first]
            } else {
                vec![0, n + 1]
            };
            let mut ids = Vec::new();
            for level in ends {
                let p = native_point(mesh, &strips, info, v, level)?;
                ids.push((level, old + made.len() as u32));
                made.push(p);
            }
            let info = at.get_mut(&v).expect("vertex");
            for (level, id) in ids {
                info.ids[level] = id;
            }
        }
    }
    seen.clear();
    for s in &strips {
        for v in [s.a, s.b] {
            if !seen.insert(v) {
                continue;
            }
            let order: Vec<usize> = if v == s.a {
                (1..=n).rev().collect()
            } else {
                (1..=n).collect()
            };
            let mut ids = Vec::new();
            for level in order {
                let info = &at[&v];
                let native = native_level(info, s, level, n);
                let p = native_point(mesh, &strips, info, v, native)?;
                ids.push((native, old + made.len() as u32));
                made.push(p);
            }
            let info = at.get_mut(&v).expect("vertex");
            for (level, id) in ids {
                info.ids[level] = id;
            }
        }
    }

    // polygons
    let mut polys: Vec<Vec<u32>> = Vec::with_capacity(mesh.polys.len());
    let mut appended: Vec<Vec<u32>> = Vec::new();
    for poly in &mesh.polys {
        let m = poly.len();
        let expand = |k: usize| -> Result<Vec<u32>, ToolError> {
            let v = poly[k];
            let (u, w) = (poly[(k + m - 1) % m], poly[(k + 1) % m]);
            match at.get(&v) {
                None => Ok(vec![v]),
                Some(info) => expand_corner(&strips, info, v, u, w, o),
            }
        };
        if !o.solid {
            let mut out = Vec::new();
            for k in 0..m {
                out.extend(expand(k)?);
            }
            polys.push(out);
            continue;
        }
        let sides: Vec<usize> = (0..m)
            .filter(|k| {
                strips.iter().any(|s| {
                    let (x, y) = (poly[*k], poly[(*k + 1) % m]);
                    (s.a, s.b) == (x, y) || (s.a, s.b) == (y, x)
                })
            })
            .collect();
        match sides.as_slice() {
            [] => {
                let mut out = Vec::new();
                for k in 0..m {
                    out.extend(expand(k)?);
                }
                polys.push(out);
            }
            [s] => {
                let s = *s;
                let (c0, c1) = (poly[s], poly[(s + 1) % m]);
                let x_id = end_on(&at, &strips, c0, poly[(s + m - 1) % m]);
                let y_id = end_on(&at, &strips, c1, poly[(s + 2) % m]);
                let (e1, e2) = ((s + m - 1) % m, (s + 1) % m);
                let ((i, ci), (j, cj)) = if e1 < e2 {
                    ((e1, x_id), (e2, y_id))
                } else {
                    ((e2, y_id), (e1, x_id))
                };
                let corner = |k: usize| -> Result<Vec<u32>, ToolError> {
                    if k == s || k == (s + 1) % m {
                        Ok(vec![poly[k]])
                    } else {
                        expand(k)
                    }
                };
                let mut part1 = Vec::new();
                for k in 0..=i {
                    part1.extend(corner(k)?);
                }
                part1.extend([ci, cj]);
                for k in j + 1..m {
                    part1.extend(corner(k)?);
                }
                let mut part2 = vec![ci];
                for k in i + 1..=j {
                    part2.extend(corner(k)?);
                }
                part2.push(cj);
                polys.push(part1);
                appended.push(part2);
            }
            _ => {
                return unsupported("Bevel Mode Solid with two selected edges in one polygon");
            }
        }
    }
    let mut selected = IndexSet::new();
    if o.solid {
        for part in appended {
            selected.insert(polys.len() as u32);
            polys.push(part);
        }
    } else {
        let first = polys.len() as u32;
        let mut rows: Vec<Vec<u32>> = Vec::new();
        let level = |s: &Strip, v: u32, j: usize| -> u32 {
            let info = &at[&v];
            info.ids[native_level(info, s, j, n)]
        };
        for s in &strips {
            let q = |v, j| level(s, v, j);
            rows.push(vec![q(s.a, 0), q(s.b, 0), q(s.b, 1), q(s.a, 1)]);
        }
        for s in &strips {
            let q = |v, j| level(s, v, j);
            for j in 1..=n {
                rows.push(vec![q(s.b, j), q(s.b, j + 1), q(s.a, j + 1), q(s.a, j)]);
            }
        }
        for (k, row) in rows.into_iter().enumerate() {
            selected.insert(first + k as u32);
            polys.push(row);
        }
    }

    // swap-remove the old points of every beveled vertex (chamfer only)
    let mut points = mesh.points.clone();
    points.extend(made);
    let mut slots: Vec<u32> = (0..points.len() as u32).collect();
    if !o.solid {
        for v in at.keys() {
            let last = slots.pop().expect("points");
            if (*v as usize) < slots.len() {
                slots[*v as usize] = last;
            }
        }
    }
    let mut final_of = vec![0u32; points.len()];
    for (slot, id) in slots.iter().enumerate() {
        final_of[*id as usize] = slot as u32;
    }
    let mut out = PolyMesh {
        points: slots.iter().map(|id| points[*id as usize]).collect(),
        polys: Vec::new(),
    };
    for poly in polys {
        let mut mapped: Vec<u32> = poly.iter().map(|v| final_of[*v as usize]).collect();
        mapped.dedup();
        while mapped.len() > 1 && mapped.first() == mapped.last() {
            mapped.pop();
        }
        out.polys.push(mapped);
    }
    let selection = Selection {
        polys: selected,
        ..Selection::default()
    };
    Ok((out, selection))
}

/// The creation id of the end point of a solid bevel at `v` on the edge `v – nbr`.
fn end_on(at: &BTreeMap<u32, Vertex>, strips: &[Strip], v: u32, nbr: u32) -> u32 {
    let info = &at[&v];
    let which = if nbr == info.n1 { 0 } else { 1 };
    info.ids[info.end_level_for(strips, v, which)]
}

/// The native level of strip `s`'s level `j` at a vertex.
fn native_level(info: &Vertex, s: &Strip, j: usize, n: usize) -> usize {
    if info.strips.len() == 1 || s.f_a == info.f0 {
        j
    } else {
        n + 1 - j
    }
}

fn make_strip(
    mesh: &PolyMesh,
    a: u32,
    b: u32,
    f_a: u32,
    f_b: u32,
    half: usize,
    o: &Options,
) -> Result<Strip, ToolError> {
    let (pa, pb) = (mesh.points[a as usize], mesh.points[b as usize]);
    let along = pb - pa;
    let len = along.length();
    if len < 1e-12 {
        return unsupported("zero-length edge");
    }
    let unit = |v: DVec3| if v.length() < 1e-12 { v } else { v.normalize() };
    let n_a = poly_normal(&mesh.points, &mesh.polys[f_a as usize]);
    let n_b = poly_normal(&mesh.points, &mesh.polys[f_b as usize]);
    let u_a = unit(n_a.cross(pa - pb));
    let u_b = unit(n_b.cross(pb - pa));
    if u_a == DVec3::ZERO || u_b == DVec3::ZERO {
        return unsupported("degenerate polygon next to a beveled edge");
    }
    let d = if o.proportional {
        o.offset * len
    } else {
        o.offset
    };
    let (pt_a, pt_b) = (pa + u_a * d, pa + u_b * d);
    let w = ((1.0 + -u_a.dot(u_b)) / 2.0).max(0.0).sqrt();
    let steps = (o.levels + 1) as f64;
    let profile = (0..=o.levels + 1)
        .map(|j| {
            let t = j as f64 / steps;
            let (c0, c1, c2) = ((1.0 - t) * (1.0 - t), 2.0 * w * t * (1.0 - t), t * t);
            (pt_a * c0 + pa * c1 + pt_b * c2) / (c0 + c1 + c2)
        })
        .collect();
    Ok(Strip {
        a,
        b,
        f_a,
        half,
        dir: along / len,
        profile,
    })
}

/// Checks the topology of `v` and fills the neighbour data.
fn classify(
    mesh: &PolyMesh,
    dir: &DirMap,
    strips: &[Strip],
    v: u32,
    info: &mut Vertex,
) -> Result<(), ToolError> {
    let mut nexts = BTreeSet::new();
    let mut prevs = BTreeSet::new();
    let mut count = 0;
    for poly in &mesh.polys {
        if let Some(k) = poly.iter().position(|p| *p == v) {
            count += 1;
            prevs.insert(poly[(k + poly.len() - 1) % poly.len()]);
            nexts.insert(poly[(k + 1) % poly.len()]);
        }
    }
    if count != 3 || nexts.len() != 3 || nexts != prevs {
        return unsupported("a beveled edge ends at a vertex that is not used by three polygons");
    }
    let other = |s: &Strip| if s.a == v { s.b } else { s.a };
    match info.strips.as_slice() {
        [s] => {
            let x = other(&strips[*s]);
            let (_, n1) =
                next_of(mesh, dir, x, v).ok_or(ToolError::Unsupported("open edge".into()))?;
            let (cap, n2) =
                next_of(mesh, dir, n1, v).ok_or(ToolError::Unsupported("open edge".into()))?;
            info.n1 = n1;
            info.n2 = n2;
            info.cap = cap;
            info.f0 = u32::MAX;
            Ok(())
        }
        [s, t] => {
            let (x, y) = (other(&strips[*s]), other(&strips[*t]));
            let (p, nxt) =
                next_of(mesh, dir, x, v).ok_or(ToolError::Unsupported("open edge".into()))?;
            info.f0 = if nxt == y {
                p
            } else {
                dir.get(&(y, v)).map(|d| d.0).unwrap_or(p)
            };
            Ok(())
        }
        _ => unsupported("three selected edges at one vertex (corner patch, no reference data)"),
    }
}

/// The position of native level `level` at vertex `v`.
fn native_point(
    mesh: &PolyMesh,
    strips: &[Strip],
    info: &Vertex,
    v: u32,
    level: usize,
) -> Result<DVec3, ToolError> {
    let line = |s: &Strip, j: usize| (s.profile[j], s.dir);
    if info.strips.len() == 1 {
        let s = &strips[info.strips[0]];
        let (q, t) = line(s, level);
        let cap = &mesh.polys[info.cap as usize];
        let nrm = poly_normal(&mesh.points, cap);
        let denom = t.dot(nrm);
        if denom.abs() < 1e-9 {
            return unsupported("a beveled edge is parallel to the polygon at its end");
        }
        let s_par = (mesh.points[v as usize] - q).dot(nrm) / denom;
        return Ok(q + t * s_par);
    }
    let last = strips[info.strips[0]].profile.len() - 1;
    let mut lines = Vec::new();
    for si in &info.strips {
        let s = &strips[*si];
        let j = if s.f_a == info.f0 {
            level
        } else {
            last - level
        };
        lines.push(line(s, j));
    }
    let ((p1, d1), (p2, d2)) = (lines[0], lines[1]);
    let cross = d1.cross(d2);
    let c2 = cross.length_squared();
    if c2 < 1e-18 {
        return unsupported("two beveled edges at a vertex are parallel");
    }
    let r = p2 - p1;
    let t1 = r.cross(d2).dot(cross) / c2;
    let t2 = r.cross(d1).dot(cross) / c2;
    Ok(((p1 + d1 * t1) + (p2 + d2 * t2)) * 0.5)
}

/// What the corner `u -> v -> w` of a polygon becomes in chamfer or solid mode.
fn expand_corner(
    strips: &[Strip],
    info: &Vertex,
    v: u32,
    u: u32,
    w: u32,
    o: &Options,
) -> Result<Vec<u32>, ToolError> {
    let last = o.levels + 1;
    if info.strips.len() == 2 {
        let other = |s: &Strip| if s.a == v { s.b } else { s.a };
        let sel: Vec<u32> = info.strips.iter().map(|s| other(&strips[*s])).collect();
        let both = sel.contains(&u) && sel.contains(&w);
        return Ok(vec![info.ids[if both { 0 } else { last }]]);
    }
    let x = {
        let s = &strips[info.strips[0]];
        if s.a == v { s.b } else { s.a }
    };
    let (l1, l2) = (
        info.end_level_for(strips, v, 0),
        info.end_level_for(strips, v, 1),
    );
    let (p1, p2) = (info.ids[l1], info.ids[l2]);
    if u == x {
        return Ok(vec![p1]);
    }
    if w == x {
        return Ok(vec![p2]);
    }
    if u != info.n1 || w != info.n2 {
        return unsupported("inconsistent polygon winding at a beveled vertex");
    }
    if o.solid {
        return Ok(vec![p1, v, p2]);
    }
    let mut out = vec![p1];
    if l1 == 0 {
        out.extend((1..=o.levels).map(|j| info.ids[j]));
    } else {
        out.extend((1..=o.levels).rev().map(|j| info.ids[j]));
    }
    out.push(p2);
    Ok(out)
}

impl Vertex {
    /// The native level of end point `which` (0 on `n1`, 1 on `n2`) for a one-edge vertex:
    /// `p1` is level 0 at the `a` end of the edge and the last level at the `b` end.
    fn end_level_for(&self, strips: &[Strip], v: u32, which: usize) -> usize {
        let last = self.ids.len() - 1;
        let first = if strips[self.strips[0]].a == v {
            0
        } else {
            last
        };
        if which == 0 { first } else { last - first }
    }
}
