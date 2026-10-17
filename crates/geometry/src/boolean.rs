//! Boolean operations on polygon solids (docs/specs/BOOLE.md §2).
//! Refs: Thibault & Naylor, SIGGRAPH 1987 (BSP splitting); Jacobson et al., SIGGRAPH
//! 2013 (generalized winding number); van Oosterom & Strackee 1983 (triangle solid
//! angle); Hertel & Mehlhorn 1983 (convex decomposition).

use std::collections::{BTreeMap, HashMap};

use glam::{DVec2, DVec3};

use crate::PolyMesh;
use crate::normals::poly_normal;
use crate::triangulate::triangulate_poly;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoolOp {
    Union,
    /// A minus B.
    Subtract,
    Intersect,
    /// A's surface outside B, left open.
    Without,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BoolOptions {
    pub op: BoolOp,
    /// One polygon per cut region (§4) instead of convex pieces.
    pub hide_new_edges: bool,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct BoolResult {
    pub mesh: PolyMesh,
    /// Per output polygon: true if it comes from B's surface.
    pub from_b: Vec<bool>,
}

#[derive(Debug, Clone, Copy)]
struct Plane {
    n: DVec3,
    w: f64,
}

impl Plane {
    fn dist(&self, p: DVec3) -> f64 {
        self.n.dot(p) - self.w
    }
}

/// A convex polygon of an operand, tagged with its source polygon.
#[derive(Debug, Clone)]
struct Piece {
    pts: Vec<DVec3>,
    src: usize,
    plane: Plane,
}

fn centroid(pts: &[DVec3]) -> DVec3 {
    pts.iter().copied().sum::<DVec3>() / pts.len() as f64
}

/// Right-handed 2D frame of a plane with normal `n`.
fn frame(n: DVec3) -> (DVec3, DVec3) {
    let u = n.any_orthogonal_vector().normalize();
    (u, n.cross(u))
}

fn is_convex(pts: &[DVec3], n: DVec3, eps: f64) -> bool {
    let k = pts.len();
    (0..k).all(|i| {
        let (a, b, c) = (pts[i], pts[(i + 1) % k], pts[(i + 2) % k]);
        let (e1, e2) = (b - a, c - b);
        e1.cross(e2).dot(n) >= -eps * (e1.length() + e2.length())
    })
}

/// Step 1: convex pieces of every non-degenerate polygon.
fn pieces(mesh: &PolyMesh, eps: f64) -> Vec<Piece> {
    let mut out = Vec::new();
    for (src, poly) in mesh.polys.iter().enumerate() {
        let n = poly_normal(&mesh.points, poly);
        if n == DVec3::ZERO {
            continue;
        }
        let pts: Vec<DVec3> = poly.iter().map(|&i| mesh.points[i as usize]).collect();
        let plane = Plane {
            n,
            w: n.dot(centroid(&pts)),
        };
        if is_convex(&pts, n, eps) {
            out.push(Piece { pts, src, plane });
        } else {
            for t in triangulate_poly(&mesh.points, poly) {
                let pts = t.iter().map(|&i| mesh.points[i as usize]).collect();
                out.push(Piece { pts, src, plane });
            }
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Step 2: BSP splitting
// ---------------------------------------------------------------------------

struct Node {
    plane: Plane,
    front: Option<Box<Node>>,
    back: Option<Box<Node>>,
}

enum Side {
    Coplanar,
    Front,
    Back,
    Spanning(Vec<DVec3>, Vec<DVec3>),
}

/// The edge point where `plane` cuts `a`–`b`, computed in a canonical endpoint order
/// so neighbouring polygons get the same point.
fn cut(plane: &Plane, a: DVec3, b: DVec3) -> DVec3 {
    let (p, q) = if a.to_array() < b.to_array() {
        (a, b)
    } else {
        (b, a)
    };
    let (dp, dq) = (plane.dist(p), plane.dist(q));
    p + (q - p) * (dp / (dp - dq))
}

fn split(plane: &Plane, pts: &[DVec3], eps: f64) -> Side {
    // -1 back, 0 on, 1 front
    let sides: Vec<i8> = pts
        .iter()
        .map(|p| {
            let d = plane.dist(*p);
            if d > eps {
                1
            } else if d < -eps {
                -1
            } else {
                0
            }
        })
        .collect();
    let front = sides.contains(&1);
    let back = sides.contains(&-1);
    match (front, back) {
        (false, false) => Side::Coplanar,
        (true, false) => Side::Front,
        (false, true) => Side::Back,
        (true, true) => {
            let (mut f, mut b) = (Vec::new(), Vec::new());
            let k = pts.len();
            for i in 0..k {
                let j = (i + 1) % k;
                let (si, sj) = (sides[i], sides[j]);
                if si >= 0 {
                    f.push(pts[i]);
                }
                if si <= 0 {
                    b.push(pts[i]);
                }
                if si * sj < 0 {
                    let p = cut(plane, pts[i], pts[j]);
                    f.push(p);
                    b.push(p);
                }
            }
            Side::Spanning(f, b)
        }
    }
}

fn build_bsp(mut polys: Vec<Vec<DVec3>>, planes: Vec<Plane>, eps: f64) -> Option<Box<Node>> {
    if polys.is_empty() {
        return None;
    }
    let mut planes = planes;
    let plane = planes[0];
    let (mut fp, mut fpl, mut bp, mut bpl) = (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    for (pts, pl) in polys.drain(..).zip(planes.drain(..)).skip(1) {
        match split(&plane, &pts, eps) {
            Side::Coplanar => {}
            Side::Front => {
                fp.push(pts);
                fpl.push(pl);
            }
            Side::Back => {
                bp.push(pts);
                bpl.push(pl);
            }
            Side::Spanning(f, b) => {
                fp.push(f);
                fpl.push(pl);
                bp.push(b);
                bpl.push(pl);
            }
        }
    }
    Some(Box::new(Node {
        plane,
        front: build_bsp(fp, fpl, eps),
        back: build_bsp(bp, bpl, eps),
    }))
}

/// Splits `p` at every node plane it crosses; coplanar pieces visit both subtrees.
fn split_down(node: Option<&Node>, p: Piece, eps: f64, out: &mut Vec<Piece>) {
    let Some(node) = node else {
        out.push(p);
        return;
    };
    match split(&node.plane, &p.pts, eps) {
        Side::Front => split_down(node.front.as_deref(), p, eps, out),
        Side::Back => split_down(node.back.as_deref(), p, eps, out),
        Side::Coplanar => {
            let mut mid = Vec::new();
            split_down(node.front.as_deref(), p, eps, &mut mid);
            for q in mid {
                split_down(node.back.as_deref(), q, eps, out);
            }
        }
        Side::Spanning(f, b) => {
            let (src, plane) = (p.src, p.plane);
            split_down(
                node.front.as_deref(),
                Piece { pts: f, src, plane },
                eps,
                out,
            );
            split_down(node.back.as_deref(), Piece { pts: b, src, plane }, eps, out);
        }
    }
}

fn fragments(pieces: &[Piece], other: &[Piece], eps: f64) -> Vec<Piece> {
    let tree = build_bsp(
        other.iter().map(|p| p.pts.clone()).collect(),
        other.iter().map(|p| p.plane).collect(),
        eps,
    );
    let mut out = Vec::new();
    for p in pieces {
        split_down(tree.as_deref(), p.clone(), eps, &mut out);
    }
    out.retain(|p| p.pts.len() >= 3);
    out
}

// ---------------------------------------------------------------------------
// Step 3: classification
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Class {
    Inside,
    Outside,
    Same,
    Opposite,
}

/// Signed solid angle of triangle `abc` seen from the origin (van Oosterom–Strackee).
fn solid_angle(a: DVec3, b: DVec3, c: DVec3) -> f64 {
    let (la, lb, lc) = (a.length(), b.length(), c.length());
    let num = a.dot(b.cross(c));
    let den = la * lb * lc + a.dot(b) * lc + b.dot(c) * la + c.dot(a) * lb;
    2.0 * num.atan2(den)
}

fn winding_number(solid: &[Piece], p: DVec3) -> f64 {
    let mut sum = 0.0;
    for piece in solid {
        let q: Vec<DVec3> = piece.pts.iter().map(|x| *x - p).collect();
        for i in 1..q.len() - 1 {
            sum += solid_angle(q[0], q[i], q[i + 1]);
        }
    }
    sum / (4.0 * std::f64::consts::PI)
}

fn inside_convex(pts: &[DVec3], n: DVec3, p: DVec3, eps: f64) -> bool {
    let k = pts.len();
    (0..k).all(|i| {
        let (a, b) = (pts[i], pts[(i + 1) % k]);
        let e = b - a;
        e.cross(p - a).dot(n) >= -eps * e.length()
    })
}

fn classify(f: &Piece, other: &[Piece], eps: f64) -> Class {
    let c = centroid(&f.pts);
    for o in other {
        let along = f.plane.n.dot(o.plane.n);
        if along.abs() > 1.0 - 1e-9
            && o.plane.dist(c).abs() <= eps
            && inside_convex(&o.pts, o.plane.n, c, eps)
        {
            return if along > 0.0 {
                Class::Same
            } else {
                Class::Opposite
            };
        }
    }
    if winding_number(other, c) > 0.5 {
        Class::Inside
    } else {
        Class::Outside
    }
}

// ---------------------------------------------------------------------------
// Steps 5–7: welding, merging, cleaning, polygons
// ---------------------------------------------------------------------------

struct Welder {
    eps: f64,
    cell: f64,
    grid: HashMap<[i64; 3], Vec<u32>>,
    pts: Vec<DVec3>,
    /// (operand, input index) for input points.
    origin: Vec<Option<(u8, u32)>>,
}

impl Welder {
    fn key(&self, p: DVec3) -> [i64; 3] {
        (p / self.cell).floor().as_i64vec3().to_array()
    }

    fn id(&mut self, p: DVec3, origin: Option<(u8, u32)>) -> u32 {
        let k = self.key(p);
        let near = |i: &&u32| (self.pts[**i as usize] - p).abs().max_element() <= self.eps;
        let found = (-1..=1)
            .flat_map(|dx| (-1..=1).flat_map(move |dy| (-1..=1).map(move |dz| [dx, dy, dz])))
            .filter_map(|d| self.grid.get(&[k[0] + d[0], k[1] + d[1], k[2] + d[2]]))
            .find_map(|ids| ids.iter().find(near).copied());
        if let Some(i) = found {
            if self.origin[i as usize].is_none() {
                self.origin[i as usize] = origin;
            }
            return i;
        }
        let i = self.pts.len() as u32;
        self.pts.push(p);
        self.origin.push(origin);
        self.grid.entry(k).or_default().push(i);
        i
    }
}

/// Points of `pts` (ids) lying strictly inside segment `a`–`b`, ordered from `a`.
fn on_segment(pts: &[DVec3], cands: &[u32], a: u32, b: u32, eps: f64) -> Vec<u32> {
    let (pa, pb) = (pts[a as usize], pts[b as usize]);
    let d = pb - pa;
    let len2 = d.length_squared();
    if len2 == 0.0 {
        return Vec::new();
    }
    let len = len2.sqrt();
    let mut hits: Vec<(f64, u32)> = cands
        .iter()
        .filter(|&&c| c != a && c != b)
        .filter_map(|&c| {
            let p = pts[c as usize];
            let t = (p - pa).dot(d) / len2;
            let off = (p - pa - d * t).length();
            (t * len > eps && (1.0 - t) * len > eps && off <= eps).then_some((t, c))
        })
        .collect();
    hits.sort_by(|x, y| x.0.total_cmp(&y.0));
    hits.dedup_by_key(|h| h.1);
    hits.into_iter().map(|h| h.1).collect()
}

/// Spatial index of point ids sorted by x, for [`on_segment`] candidates.
struct ByX {
    ids: Vec<u32>,
    xs: Vec<f64>,
}

impl ByX {
    fn new(pts: &[DVec3], mut ids: Vec<u32>) -> ByX {
        ids.sort_by(|a, b| pts[*a as usize].x.total_cmp(&pts[*b as usize].x));
        ids.dedup();
        let xs = ids.iter().map(|&i| pts[i as usize].x).collect();
        ByX { ids, xs }
    }

    fn near(&self, pts: &[DVec3], a: u32, b: u32, eps: f64) -> Vec<u32> {
        let (pa, pb) = (pts[a as usize], pts[b as usize]);
        let lo = pa.min(pb) - DVec3::splat(eps);
        let hi = pa.max(pb) + DVec3::splat(eps);
        let start = self.xs.partition_point(|x| *x < lo.x);
        self.ids[start..]
            .iter()
            .take_while(|&&i| pts[i as usize].x <= hi.x)
            .copied()
            .filter(|&i| {
                let p = pts[i as usize];
                p.cmpge(lo).all() && p.cmple(hi).all()
            })
            .collect()
    }
}

fn subdivide_loop(pts: &[DVec3], index: &ByX, lp: &[u32], eps: f64) -> Vec<u32> {
    let mut out = Vec::with_capacity(lp.len());
    for i in 0..lp.len() {
        let (a, b) = (lp[i], lp[(i + 1) % lp.len()]);
        out.push(a);
        out.extend(on_segment(pts, &index.near(pts, a, b, eps), a, b, eps));
    }
    out
}

fn signed_area(pts: &[DVec3], lp: &[u32], n: DVec3) -> f64 {
    let k = lp.len();
    let o = pts[lp[0] as usize];
    (0..k)
        .map(|i| {
            (pts[lp[i] as usize] - o)
                .cross(pts[lp[(i + 1) % k] as usize] - o)
                .dot(n)
        })
        .sum::<f64>()
        / 2.0
}

fn to2d(pts: &[DVec3], lp: &[u32], n: DVec3) -> Vec<DVec2> {
    let (u, v) = frame(n);
    lp.iter()
        .map(|&i| {
            let p = pts[i as usize];
            DVec2::new(p.dot(u), p.dot(v))
        })
        .collect()
}

fn point_in_polygon(poly: &[DVec2], p: DVec2) -> bool {
    let mut inside = false;
    let k = poly.len();
    for i in 0..k {
        let (a, b) = (poly[i], poly[(i + 1) % k]);
        if (a.y > p.y) != (b.y > p.y) && p.x < a.x + (p.y - a.y) / (b.y - a.y) * (b.x - a.x) {
            inside = !inside;
        }
    }
    inside
}

/// Kept fragments of one source polygon: (normal, flipped, welded loops).
type Group = (DVec3, bool, Vec<Vec<u32>>);

/// One merged region of a source polygon: an outer loop and its holes (welded ids).
struct Region {
    operand: u8,
    src: usize,
    n: DVec3,
    outer: Vec<u32>,
    holes: Vec<Vec<u32>>,
}

/// Step 5: merges the kept fragments of one source polygon into regions.
fn merge_group(
    pts: &[DVec3],
    frags: &[Vec<u32>],
    n: DVec3,
    eps: f64,
) -> Vec<(Vec<u32>, Vec<Vec<u32>>)> {
    let all: Vec<u32> = frags.iter().flatten().copied().collect();
    let index = ByX::new(pts, all);
    let mut count: BTreeMap<(u32, u32), i64> = BTreeMap::new();
    for f in frags {
        let f = subdivide_loop(pts, &index, f, eps);
        for i in 0..f.len() {
            *count.entry((f[i], f[(i + 1) % f.len()])).or_insert(0) += 1;
        }
    }
    // cancel opposite pairs
    let mut out: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
    for (&(a, b), &c) in &count {
        let back = count.get(&(b, a)).copied().unwrap_or(0);
        for _ in 0..(c - back).max(0) {
            out.entry(a).or_default().push(b);
        }
    }
    let (u, v) = frame(n);
    let dir = |a: u32, b: u32| {
        let d = pts[b as usize] - pts[a as usize];
        DVec2::new(d.dot(u), d.dot(v))
    };
    let mut loops = Vec::new();
    while let Some((&start, _)) = out.iter().find(|(_, vs)| !vs.is_empty()) {
        let mut lp = vec![start];
        let mut cur = start;
        let mut prev: Option<u32> = None;
        loop {
            let outs = out.get_mut(&cur).expect("traced edge exists");
            let pick = match (prev, outs.len()) {
                (_, 0) => break,
                (None, _) | (_, 1) => 0,
                (Some(p), _) => {
                    let din = dir(p, cur);
                    let turn = |w: u32| {
                        let d = dir(cur, w);
                        din.perp_dot(d).atan2(din.dot(d))
                    };
                    (0..outs.len())
                        .min_by(|&i, &j| turn(outs[i]).total_cmp(&turn(outs[j])))
                        .expect("non-empty")
                }
            };
            let next = outs.swap_remove(pick);
            if next == start {
                break;
            }
            prev = Some(cur);
            cur = next;
            lp.push(cur);
            if lp.len() > count.len() + 1 {
                break; // malformed input; give up on this loop
            }
        }
        if lp.len() >= 3 {
            loops.push(lp);
        }
    }
    let (mut outers, mut holes): (Vec<_>, Vec<_>) = loops
        .into_iter()
        .map(|l| {
            let a = signed_area(pts, &l, n);
            (l, a)
        })
        .filter(|(_, a)| a.abs() > eps * eps)
        .partition(|(_, a)| *a > 0.0);
    outers.sort_by(|x, y| x.1.total_cmp(&y.1));
    let mut regions: Vec<(Vec<u32>, Vec<Vec<u32>>)> = outers
        .iter()
        .map(|(l, _)| (l.clone(), Vec::new()))
        .collect();
    let outer2d: Vec<Vec<DVec2>> = outers.iter().map(|(l, _)| to2d(pts, l, n)).collect();
    for (h, _) in holes.drain(..) {
        // a point inside the hole: the centroid of its first ear
        let rev: Vec<u32> = h.iter().rev().copied().collect();
        let ear = &ear_clip(pts, &rev, n, eps)[0];
        let probe = to2d(pts, ear, n).into_iter().sum::<DVec2>() / ear.len() as f64;
        if let Some(r) = outer2d.iter().position(|o| point_in_polygon(o, probe)) {
            regions[r].1.push(h);
        }
    }
    regions
}

fn collinear(pts: &[DVec3], a: u32, v: u32, b: u32, eps: f64) -> bool {
    let (pa, pv, pb) = (pts[a as usize], pts[v as usize], pts[b as usize]);
    let d = pb - pa;
    let len2 = d.length_squared();
    if len2 == 0.0 || (pv - pa).dot(pb - pv) <= 0.0 {
        return false;
    }
    let t = (pv - pa).dot(d) / len2;
    (pv - pa - d * t).length() <= eps
}

// --- convex decomposition (§4) ---

fn cross2(a: DVec2, b: DVec2) -> f64 {
    a.perp_dot(b)
}

fn segments_cross(a: DVec2, b: DVec2, c: DVec2, d: DVec2) -> bool {
    let o = |p: DVec2, q: DVec2, r: DVec2| cross2(q - p, r - p);
    let (d1, d2, d3, d4) = (o(c, d, a), o(c, d, b), o(a, b, c), o(a, b, d));
    ((d1 > 0.0) != (d2 > 0.0)) && ((d3 > 0.0) != (d4 > 0.0)) && d1 != 0.0 && d2 != 0.0
}

/// Bridges holes into the outer loop (shortest visible vertex pairs); returns the
/// weakly simple loop and the bridge edges.
fn bridge(
    pts: &[DVec3],
    outer: &[u32],
    holes: &[Vec<u32>],
    n: DVec3,
) -> (Vec<u32>, Vec<(u32, u32)>) {
    let (u, v) = frame(n);
    let p2 = |i: u32| {
        let p = pts[i as usize];
        DVec2::new(p.dot(u), p.dot(v))
    };
    let mut lp = outer.to_vec();
    let mut rest: Vec<Vec<u32>> = holes.to_vec();
    let mut bridges = Vec::new();
    while !rest.is_empty() {
        let edges = |l: &[u32]| -> Vec<(u32, u32)> {
            (0..l.len()).map(|i| (l[i], l[(i + 1) % l.len()])).collect()
        };
        let mut all_edges = edges(&lp);
        for h in &rest {
            all_edges.extend(edges(h));
        }
        let mut best: Option<(f64, usize, usize, usize)> = None;
        for (hi, h) in rest.iter().enumerate() {
            for (hk, &hv) in h.iter().enumerate() {
                for (ok, &ov) in lp.iter().enumerate() {
                    if ov == hv {
                        continue;
                    }
                    let (a, b) = (p2(ov), p2(hv));
                    let len = (b - a).length();
                    if best.is_some_and(|bst| bst.0 <= len) {
                        continue;
                    }
                    let blocked = all_edges.iter().any(|&(c, d)| {
                        c != ov
                            && d != ov
                            && c != hv
                            && d != hv
                            && segments_cross(a, b, p2(c), p2(d))
                    });
                    // the bridge must leave the outer vertex into the region
                    let k = lp.len();
                    let (pv, nv) = (p2(lp[(ok + k - 1) % k]), p2(lp[(ok + 1) % k]));
                    let inward = {
                        let (e1, e2, w) = (a - pv, nv - a, b - a);
                        if cross2(e1, e2) >= 0.0 {
                            cross2(e1, w) > 0.0 && cross2(e2, w) > 0.0
                        } else {
                            cross2(e1, w) > 0.0 || cross2(e2, w) > 0.0
                        }
                    };
                    if !blocked && inward {
                        best = Some((len, hi, hk, ok));
                    }
                }
            }
        }
        let Some((_, hi, hk, ok)) = best else {
            break; // no visible bridge: drop remaining holes
        };
        let h = rest.swap_remove(hi);
        let (ov, hv) = (lp[ok], h[hk]);
        let mut spliced = lp[..=ok].to_vec();
        spliced.extend((0..=h.len()).map(|i| h[(hk + i) % h.len()]));
        spliced.push(ov);
        spliced.extend_from_slice(&lp[ok + 1..]);
        lp = spliced;
        bridges.push((ov, hv));
    }
    (lp, bridges)
}

/// Ear clipping of a weakly simple loop (repeated ids allowed).
fn ear_clip(pts: &[DVec3], lp: &[u32], n: DVec3, eps: f64) -> Vec<Vec<u32>> {
    let p2 = to2d(pts, lp, n);
    let mut rem: Vec<usize> = (0..lp.len()).collect();
    let mut out = Vec::new();
    while rem.len() > 3 {
        let m = rem.len();
        let ear = (0..m).find(|&i| {
            let (a, b, c) = (rem[(i + m - 1) % m], rem[i], rem[(i + 1) % m]);
            let (pa, pb, pc) = (p2[a], p2[b], p2[c]);
            if cross2(pb - pa, pc - pb) <= eps * eps {
                return false;
            }
            !rem.iter().any(|&o| {
                let id = lp[o];
                id != lp[a]
                    && id != lp[b]
                    && id != lp[c]
                    && cross2(pb - pa, p2[o] - pa) >= -eps * eps
                    && cross2(pc - pb, p2[o] - pb) >= -eps * eps
                    && cross2(pa - pc, p2[o] - pc) >= -eps * eps
            })
        });
        let Some(i) = ear else {
            // degenerate remainder: fan it
            for k in 1..m - 1 {
                out.push(vec![lp[rem[0]], lp[rem[k]], lp[rem[k + 1]]]);
            }
            return out;
        };
        let (a, b, c) = (rem[(i + m - 1) % m], rem[i], rem[(i + 1) % m]);
        out.push(vec![lp[a], lp[b], lp[c]]);
        rem.remove(i);
    }
    out.push(rem.iter().map(|&k| lp[k]).collect());
    out
}

fn poly_is_convex(pts: &[DVec3], lp: &[u32], n: DVec3, eps: f64) -> bool {
    let p: Vec<DVec3> = lp.iter().map(|&i| pts[i as usize]).collect();
    is_convex(&p, n, eps)
}

/// Hertel–Mehlhorn: drop every diagonal whose removal keeps both sides convex.
fn hertel_mehlhorn(
    pts: &[DVec3],
    mut polys: Vec<Vec<u32>>,
    keep: &[(u32, u32)],
    n: DVec3,
    eps: f64,
) -> Vec<Vec<u32>> {
    let fixed = |a: u32, b: u32| {
        keep.iter()
            .any(|&(x, y)| (x, y) == (a, b) || (x, y) == (b, a))
    };
    'outer: loop {
        for i in 0..polys.len() {
            for j in i + 1..polys.len() {
                let (p, q) = (&polys[i], &polys[j]);
                for k in 0..p.len() {
                    let (a, b) = (p[k], p[(k + 1) % p.len()]);
                    if fixed(a, b) {
                        continue;
                    }
                    let Some(l) = (0..q.len()).find(|&l| q[l] == b && q[(l + 1) % q.len()] == a)
                    else {
                        continue;
                    };
                    // p: ... a b ...; q: ... b a ...  → p up to a, then q after a to b
                    let mut merged: Vec<u32> =
                        (0..p.len()).map(|t| p[(k + 1 + t) % p.len()]).collect(); // b .. a
                    merged.extend((2..q.len()).map(|t| q[(l + t) % q.len()])); // after a .. before b
                    let unique = {
                        let mut s = merged.clone();
                        s.sort_unstable();
                        s.dedup();
                        s.len() == merged.len()
                    };
                    if unique && poly_is_convex(pts, &merged, n, eps) {
                        polys[i] = merged;
                        polys.swap_remove(j);
                        continue 'outer;
                    }
                }
            }
        }
        return polys;
    }
}

fn convex_pieces(
    pts: &[DVec3],
    outer: &[u32],
    holes: &[Vec<u32>],
    n: DVec3,
    eps: f64,
) -> Vec<Vec<u32>> {
    let (lp, bridges) = bridge(pts, outer, holes, n);
    let tris = ear_clip(pts, &lp, n, eps);
    hertel_mehlhorn(pts, tris, &bridges, n, eps)
}

fn same_cycle(a: &[u32], b: &[u32]) -> bool {
    a.len() == b.len()
        && (a.is_empty()
            || b.iter()
                .position(|&x| x == a[0])
                .is_some_and(|s| (0..a.len()).all(|i| a[i] == b[(s + i) % b.len()])))
}

/// Boolean of two polygon solids given in the same space (docs/specs/BOOLE.md §2).
pub fn boolean(a: &PolyMesh, b: &PolyMesh, opts: &BoolOptions) -> BoolResult {
    let all: Vec<DVec3> = a.points.iter().chain(&b.points).copied().collect();
    let diag = match all.first() {
        None => 0.0,
        Some(&f) => {
            let (lo, hi) = all
                .iter()
                .fold((f, f), |(lo, hi), p| (lo.min(*p), hi.max(*p)));
            (hi - lo).length()
        }
    };
    let eps = (1e-9 * diag).max(1e-12);

    let (pa, pb) = (pieces(a, eps), pieces(b, eps));
    let fa = fragments(&pa, &pb, eps);
    let fb = fragments(&pb, &pa, eps);

    use BoolOp::*;
    use Class::*;
    let keep_a = |c: Class| match opts.op {
        Union => matches!(c, Outside | Same),
        Subtract | Without => matches!(c, Outside | Opposite),
        Intersect => matches!(c, Inside | Same),
    };
    // Some(flip) if kept
    let keep_b = |c: Class| match (opts.op, c) {
        (Union, Outside) => Some(false),
        (Subtract, Inside) => Some(true),
        (Intersect, Inside) => Some(false),
        _ => None,
    };

    let mut w = Welder {
        eps,
        cell: eps * 4.0,
        grid: HashMap::new(),
        pts: Vec::new(),
        origin: Vec::new(),
    };
    let input_ids: Vec<Vec<u32>> = [(0u8, a), (1u8, b)]
        .iter()
        .map(|&(op, m)| {
            (m.points.iter().enumerate())
                .map(|(i, p)| w.id(*p, Some((op, i as u32))))
                .collect()
        })
        .collect();
    let mut groups: BTreeMap<(u8, usize), Group> = BTreeMap::new();
    let mut add = |w: &mut Welder, op: u8, f: &Piece, flip: bool| {
        let mut lp: Vec<u32> = f.pts.iter().map(|p| w.id(*p, None)).collect();
        lp.dedup();
        while lp.len() > 1 && lp.first() == lp.last() {
            lp.pop();
        }
        if lp.len() < 3 {
            return;
        }
        if flip {
            lp.reverse();
        }
        let n = if flip { -f.plane.n } else { f.plane.n };
        groups
            .entry((op, f.src))
            .or_insert_with(|| (n, flip, Vec::new()))
            .2
            .push(lp);
    };
    for f in &fa {
        if keep_a(classify(f, &pb, eps)) {
            add(&mut w, 0, f, false);
        }
    }
    if opts.op != Without {
        for f in &fb {
            if let Some(flip) = keep_b(classify(f, &pa, eps)) {
                add(&mut w, 1, f, flip);
            }
        }
    }

    let pts = w.pts.clone();
    let mut regions: Vec<Region> = Vec::new();
    for (&(operand, src), (n, _, frags)) in &groups {
        for (outer, holes) in merge_group(&pts, frags, *n, eps) {
            regions.push(Region {
                operand,
                src,
                n: *n,
                outer,
                holes,
            });
        }
    }

    // Step 6: T-junctions across regions, then collinear new points.
    let used: Vec<u32> = regions
        .iter()
        .flat_map(|r| r.outer.iter().chain(r.holes.iter().flatten()))
        .copied()
        .collect();
    let index = ByX::new(&pts, used);
    for r in &mut regions {
        r.outer = subdivide_loop(&pts, &index, &r.outer, eps);
        for h in &mut r.holes {
            *h = subdivide_loop(&pts, &index, h, eps);
        }
    }
    let mut removable = vec![true; pts.len()];
    for (i, o) in w.origin.iter().enumerate() {
        if o.is_some() {
            removable[i] = false;
        }
    }
    for r in &regions {
        for lp in std::iter::once(&r.outer).chain(&r.holes) {
            let k = lp.len();
            for i in 0..k {
                let (p, v, q) = (lp[(i + k - 1) % k], lp[i], lp[(i + 1) % k]);
                if !collinear(&pts, p, v, q, eps) {
                    removable[v as usize] = false;
                }
            }
        }
    }
    for r in &mut regions {
        r.outer.retain(|&v| !removable[v as usize]);
        for h in &mut r.holes {
            h.retain(|&v| !removable[v as usize]);
        }
        r.holes.retain(|h| h.len() >= 3);
    }
    regions.retain(|r| r.outer.len() >= 3);

    // Step 7: polygons.
    let mut polys: Vec<(Vec<u32>, bool)> = Vec::new();
    for r in &regions {
        let source = || -> Vec<u32> {
            let m = if r.operand == 0 { a } else { b };
            let flip = groups[&(r.operand, r.src)].1;
            let mut s: Vec<u32> = m.polys[r.src]
                .iter()
                .map(|&i| input_ids[r.operand as usize][i as usize])
                .collect();
            if flip {
                s.reverse();
            }
            s
        };
        let whole = r.holes.is_empty()
            && (opts.hide_new_edges
                || poly_is_convex(&pts, &r.outer, r.n, eps)
                || same_cycle(&r.outer, &source()));
        if whole {
            polys.push((r.outer.clone(), r.operand == 1));
        } else {
            for p in convex_pieces(&pts, &r.outer, &r.holes, r.n, eps) {
                polys.push((p, r.operand == 1));
            }
        }
    }

    // Output order: A's input points, B's input points, then new points by first use.
    let mut order: Vec<u32> = Vec::new();
    let mut seen = vec![false; pts.len()];
    for v in polys.iter().flat_map(|(p, _)| p) {
        seen[*v as usize] = true;
    }
    let mut inputs: Vec<((u8, u32), u32)> = w
        .origin
        .iter()
        .enumerate()
        .filter_map(|(i, o)| o.filter(|_| seen[i]).map(|o| (o, i as u32)))
        .collect();
    inputs.sort();
    let mut remap = vec![u32::MAX; pts.len()];
    for (_, i) in inputs {
        remap[i as usize] = order.len() as u32;
        order.push(i);
    }
    for v in polys.iter().flat_map(|(p, _)| p) {
        if remap[*v as usize] == u32::MAX {
            remap[*v as usize] = order.len() as u32;
            order.push(*v);
        }
    }
    BoolResult {
        mesh: PolyMesh {
            points: order.iter().map(|&i| pts[i as usize]).collect(),
            polys: polys
                .iter()
                .map(|(p, _)| p.iter().map(|&v| remap[v as usize]).collect())
                .collect(),
        },
        from_b: polys.iter().map(|(_, fb)| *fb).collect(),
    }
}
