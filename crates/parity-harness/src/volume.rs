//! Geometric comparison of meshes extracted from volumes (docs/specs/VOLUME.md section 5).
//!
//! Voxel meshes of two implementations never share numbering and rarely share the exact
//! vertices, so the comparison is on the surface: two-sided Hausdorff distance, area, volume
//! and the Euler characteristic.

// Small fixed-size vector math over `[f64; 3]`: index loops read clearer than iterator chains.
#![allow(clippy::needless_range_loop)]

use std::collections::BTreeSet;

use crate::{Mesh, export_weld};

type V3 = [f64; 3];

fn sub(a: V3, b: V3) -> V3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn dot(a: V3, b: V3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn cross(a: V3, b: V3) -> V3 {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn madd(a: V3, b: V3, s: f64) -> V3 {
    [a[0] + b[0] * s, a[1] + b[1] * s, a[2] + b[2] * s]
}
fn len(a: V3) -> f64 {
    dot(a, a).sqrt()
}

type Tri = [V3; 3];

fn triangles(m: &Mesh) -> Vec<Tri> {
    let mut out = Vec::new();
    for f in &m.faces {
        for i in 1..f.len().saturating_sub(1) {
            out.push([
                m.positions[f[0] as usize],
                m.positions[f[i] as usize],
                m.positions[f[i + 1] as usize],
            ]);
        }
    }
    out
}

/// Closest point on a triangle (Ericson 2004, 5.1.5).
fn closest(t: &Tri, p: V3) -> V3 {
    let (a, b, c) = (t[0], t[1], t[2]);
    let ab = sub(b, a);
    let ac = sub(c, a);
    let ap = sub(p, a);
    let (d1, d2) = (dot(ab, ap), dot(ac, ap));
    if d1 <= 0.0 && d2 <= 0.0 {
        return a;
    }
    let bp = sub(p, b);
    let (d3, d4) = (dot(ab, bp), dot(ac, bp));
    if d3 >= 0.0 && d4 <= d3 {
        return b;
    }
    let vc = d1 * d4 - d3 * d2;
    if vc <= 0.0 && d1 >= 0.0 && d3 <= 0.0 {
        return madd(a, ab, d1 / (d1 - d3));
    }
    let cp = sub(p, c);
    let (d5, d6) = (dot(ab, cp), dot(ac, cp));
    if d6 >= 0.0 && d5 <= d6 {
        return c;
    }
    let vb = d5 * d2 - d1 * d6;
    if vb <= 0.0 && d2 >= 0.0 && d6 <= 0.0 {
        return madd(a, ac, d2 / (d2 - d6));
    }
    let va = d3 * d6 - d5 * d4;
    if va <= 0.0 && d4 - d3 >= 0.0 && d5 - d6 >= 0.0 {
        let w = (d4 - d3) / ((d4 - d3) + (d5 - d6));
        return madd(b, sub(c, b), w);
    }
    let denom = 1.0 / (va + vb + vc);
    madd(madd(a, ab, vb * denom), ac, vc * denom)
}

struct Node {
    lo: V3,
    hi: V3,
    start: usize,
    count: usize,
    left: usize,
    right: usize,
}

/// A bounding volume hierarchy over triangles for point distance queries.
struct Tree {
    tris: Vec<Tri>,
    nodes: Vec<Node>,
}

impl Tree {
    fn new(mut tris: Vec<Tri>) -> Tree {
        let mut nodes = Vec::new();
        if !tris.is_empty() {
            let n = tris.len();
            build(&mut tris, &mut nodes, 0, n);
        }
        Tree { tris, nodes }
    }

    fn distance(&self, p: V3) -> f64 {
        let mut best = f64::MAX;
        let mut stack = vec![0usize];
        while let Some(n) = stack.pop() {
            let Some(node) = self.nodes.get(n) else { break };
            let mut d2 = 0.0;
            for k in 0..3 {
                let e = (node.lo[k] - p[k]).max(p[k] - node.hi[k]).max(0.0);
                d2 += e * e;
            }
            if d2.sqrt() > best {
                continue;
            }
            if node.count > 0 {
                for t in &self.tris[node.start..node.start + node.count] {
                    best = best.min(len(sub(p, closest(t, p))));
                }
            } else {
                stack.push(node.left);
                stack.push(node.right);
            }
        }
        best
    }
}

fn build(tris: &mut [Tri], nodes: &mut Vec<Node>, start: usize, end: usize) -> usize {
    let (mut lo, mut hi) = ([f64::MAX; 3], [f64::MIN; 3]);
    let (mut clo, mut chi) = ([f64::MAX; 3], [f64::MIN; 3]);
    for t in &tris[start..end] {
        let mut c = [0.0; 3];
        for p in t {
            for k in 0..3 {
                lo[k] = lo[k].min(p[k]);
                hi[k] = hi[k].max(p[k]);
                c[k] += p[k] / 3.0;
            }
        }
        for k in 0..3 {
            clo[k] = clo[k].min(c[k]);
            chi[k] = chi[k].max(c[k]);
        }
    }
    let id = nodes.len();
    nodes.push(Node {
        lo,
        hi,
        start,
        count: end - start,
        left: 0,
        right: 0,
    });
    if end - start > 4 {
        let ext = sub(chi, clo);
        let axis = if ext[0] >= ext[1] && ext[0] >= ext[2] {
            0
        } else if ext[1] >= ext[2] {
            1
        } else {
            2
        };
        tris[start..end].sort_by(|a, b| {
            let ka = a[0][axis] + a[1][axis] + a[2][axis];
            let kb = b[0][axis] + b[1][axis] + b[2][axis];
            ka.total_cmp(&kb)
        });
        let mid = start + (end - start) / 2;
        let l = build(tris, nodes, start, mid);
        let r = build(tris, nodes, mid, end);
        nodes[id].left = l;
        nodes[id].right = r;
        nodes[id].count = 0;
    }
    id
}

fn samples(m: &Mesh) -> Vec<V3> {
    let mut out = m.positions.clone();
    for f in &m.faces {
        let mut c = [0.0; 3];
        for &i in f {
            let p = m.positions[i as usize];
            for k in 0..3 {
                c[k] += p[k] / f.len() as f64;
            }
        }
        out.push(c);
    }
    out
}

fn area(tris: &[Tri]) -> f64 {
    tris.iter()
        .map(|t| 0.5 * len(cross(sub(t[1], t[0]), sub(t[2], t[0]))))
        .sum()
}

/// Edges, and whether every edge has exactly two faces running in opposite directions.
fn topology(m: &Mesh) -> (usize, bool) {
    let mut directed: std::collections::BTreeMap<(u32, u32), usize> = Default::default();
    for f in &m.faces {
        for i in 0..f.len() {
            *directed.entry((f[i], f[(i + 1) % f.len()])).or_default() += 1;
        }
    }
    let edges: BTreeSet<(u32, u32)> = directed
        .keys()
        .map(|&(a, b)| if a < b { (a, b) } else { (b, a) })
        .collect();
    let closed = !directed.is_empty()
        && directed
            .iter()
            .all(|(&(a, b), &n)| n == 1 && directed.get(&(b, a)) == Some(&1));
    (edges.len(), closed)
}

fn signed_volume(tris: &[Tri]) -> f64 {
    tris.iter()
        .map(|t| dot(t[0], cross(t[1], t[2])) / 6.0)
        .sum()
}

/// What [`compare_volume_mesh`] measured.
#[derive(Debug, Clone, PartialEq)]
pub struct VolumeMeshReport {
    /// Largest distance from a sample of the expected mesh to the actual surface.
    pub hausdorff_expected_to_actual: f64,
    /// Largest distance from a sample of the actual mesh to the expected surface.
    pub hausdorff_actual_to_expected: f64,
    pub area: (f64, f64),
    /// `None` unless both meshes are closed.
    pub volume: Option<(f64, f64)>,
    /// `V - E + F` of the welded meshes.
    pub euler: (i64, i64),
    pub closed: (bool, bool),
    pub failures: Vec<String>,
}

impl VolumeMeshReport {
    pub fn passed(&self) -> bool {
        self.failures.is_empty()
    }
}

fn euler(m: &Mesh) -> (i64, bool) {
    let used: BTreeSet<u32> = m.faces.iter().flatten().copied().collect();
    let (e, closed) = topology(m);
    (used.len() as i64 - e as i64 + m.faces.len() as i64, closed)
}

/// Same iso-surface up to discretisation: Hausdorff distance at most `voxel / 2` in both
/// directions, area and volume within 2 %, equal Euler characteristic (VOLUME.md section 5).
pub fn compare_volume_mesh(expected: &Mesh, actual: &Mesh, voxel: f64) -> VolumeMeshReport {
    let (e, a) = (export_weld(expected), export_weld(actual));
    let mut failures = Vec::new();
    if e.faces.is_empty() || a.faces.is_empty() {
        let both = e.faces.is_empty() && a.faces.is_empty();
        if !both {
            failures.push("exactly one mesh is empty".to_string());
        }
        return VolumeMeshReport {
            hausdorff_expected_to_actual: 0.0,
            hausdorff_actual_to_expected: 0.0,
            area: (0.0, 0.0),
            volume: None,
            euler: (0, 0),
            closed: (false, false),
            failures,
        };
    }
    let (et, at) = (triangles(&e), triangles(&a));
    let (etree, atree) = (Tree::new(et.clone()), Tree::new(at.clone()));
    let h_ea = samples(&e)
        .iter()
        .map(|p| atree.distance(*p))
        .fold(0.0, f64::max);
    let h_ae = samples(&a)
        .iter()
        .map(|p| etree.distance(*p))
        .fold(0.0, f64::max);
    let limit = voxel / 2.0;
    if h_ea > limit {
        failures.push(format!(
            "hausdorff: expected to actual {h_ea} exceeds voxel/2 = {limit}"
        ));
    }
    if h_ae > limit {
        failures.push(format!(
            "hausdorff: actual to expected {h_ae} exceeds voxel/2 = {limit}"
        ));
    }
    let areas = (area(&et), area(&at));
    if (areas.0 - areas.1).abs() > 0.02 * areas.0 {
        failures.push(format!(
            "area: expected {}, got {} (more than 2 % apart)",
            areas.0, areas.1
        ));
    }
    let ((ee, ec), (ae, ac)) = (euler(&e), euler(&a));
    let volume = (ec && ac).then(|| (signed_volume(&et).abs(), signed_volume(&at).abs()));
    if let Some((ev, av)) = volume
        && (ev - av).abs() > 0.02 * ev
    {
        failures.push(format!(
            "volume: expected {ev}, got {av} (more than 2 % apart)"
        ));
    }
    if ee != ae {
        failures.push(format!("euler characteristic: expected {ee}, got {ae}"));
    }
    VolumeMeshReport {
        hausdorff_expected_to_actual: h_ea,
        hausdorff_actual_to_expected: h_ae,
        area: areas,
        volume,
        euler: (ee, ae),
        closed: (ec, ac),
        failures,
    }
}
