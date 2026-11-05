//! Spline tools on editable splines (docs/specs/SPLINES.md section 8): Reverse, Subdivide,
//! Chamfer, Create Outline and the spline booleans. Everything is a pure function on
//! [`SplineData`] (or on [`Operand`]s for the booleans); [`boolean_in_tx`] is the scene
//! level wrapper the command uses.

use std::f64::consts::{FRAC_PI_2, PI};

use forge_core::transform::PREFIX;
use forge_core::{EditError, NodeId, Scene, Tx, Value};
use forge_geometry::spline::{Spline, SplineKind};
use glam::{DMat4, DVec3};

use crate::editable_spline::{self, Segment, SplineData, SplineType, TYPE};
use crate::spline_sampling::{Interpolation, Sampling, sample_params};
use crate::splines::{CIRCLE_KAPPA, Curve};

/// A tool refused its input (`Failed`) or the document refused an edit.
#[derive(Debug, Clone, thiserror::Error)]
pub enum ToolError {
    #[error("{0}")]
    Failed(String),
    #[error("{0}")]
    Edit(#[from] EditError),
}

fn failed<T>(m: impl Into<String>) -> Result<T, ToolError> {
    Err(ToolError::Failed(m.into()))
}

// ---------------------------------------------------------------------------------------
// Reverse (section 8.4)
// ---------------------------------------------------------------------------------------

/// Reverse Sequence: every segment's points in reverse order, the handles swapped.
pub fn reverse(data: &SplineData) -> SplineData {
    let mut out = data.clone();
    for r in data.ranges() {
        for (k, i) in r.clone().enumerate() {
            let src = r.end - 1 - k;
            out.points[i] = data.points[src];
            out.left[i] = data.right[src];
            out.right[i] = data.left[src];
        }
    }
    out
}

// ---------------------------------------------------------------------------------------
// Subdivide (section 8.5)
// ---------------------------------------------------------------------------------------

/// Splits the cubic piece `p` at `t` (de Casteljau): the pieces before and after.
fn split_piece(p: &[DVec3; 4], t: f64) -> ([DVec3; 4], [DVec3; 4]) {
    let l = |a: DVec3, b: DVec3| a.lerp(b, t);
    let (a, b, c) = (l(p[0], p[1]), l(p[1], p[2]), l(p[2], p[3]));
    let (d, e) = (l(a, b), l(b, c));
    let f = l(d, e);
    ([p[0], a, d, f], [f, e, c, p[3]])
}

/// Subdivide: every span of every segment becomes `count` equal parts (count - 1 new points
/// at the parameters `k / count`).
pub fn subdivide(data: &SplineData, ty: SplineType, count: usize) -> Result<SplineData, ToolError> {
    if count == 0 {
        return failed("subdivisions must be at least 1");
    }
    if count == 1 {
        return Ok(data.clone());
    }
    let mut out = SplineData::default();
    for (range, seg) in data.ranges().into_iter().zip(&data.segments) {
        let n = range.len();
        let at = |k: usize| range.start + k % n;
        let spans = if seg.closed { n } else { n - 1 };
        let curve_pts: Vec<DVec3> = data.points[range.clone()].to_vec();
        let spline = match ty {
            SplineType::Cubic => Spline::new(SplineKind::CatmullRom, curve_pts, seg.closed).ok(),
            SplineType::Akima => Spline::new(SplineKind::Akima, curve_pts, seg.closed).ok(),
            _ => None,
        };
        // per original point: its (possibly changed) handles, then the points after it
        let mut left: Vec<DVec3> = range.clone().map(|i| data.left[i]).collect();
        let mut right: Vec<DVec3> = range.clone().map(|i| data.right[i]).collect();
        let mut between: Vec<Vec<(DVec3, DVec3, DVec3)>> = vec![Vec::new(); n];
        for i in 0..spans {
            let (a, b) = (at(i), at(i + 1));
            match ty {
                SplineType::Bezier => {
                    let mut cur = [
                        data.points[a],
                        data.points[a] + data.right[a],
                        data.points[b] + data.left[b],
                        data.points[b],
                    ];
                    for k in 1..count {
                        let (l, r) = split_piece(&cur, 1.0 / (count - k + 1) as f64);
                        // the right handle of the point before the cut is the cut piece's
                        match between[i].last_mut() {
                            Some(last) => last.2 = l[1] - l[0],
                            None => right[i] = l[1] - l[0],
                        }
                        between[i].push((l[3], l[2] - l[3], DVec3::ZERO));
                        cur = r;
                    }
                    if let Some(last) = between[i].last_mut() {
                        last.2 = cur[1] - cur[0];
                    }
                    left[(i + 1) % n] = cur[2] - cur[3];
                }
                _ => {
                    for k in 1..count {
                        let t = k as f64 / count as f64;
                        let p = match (&spline, ty) {
                            (Some(s), _) => s.eval(i, t),
                            _ => data.points[a].lerp(data.points[b], t),
                        };
                        between[i].push((p, DVec3::ZERO, DVec3::ZERO));
                    }
                }
            }
        }
        let start = out.points.len();
        for i in 0..n {
            out.points.push(data.points[range.start + i]);
            out.left.push(left[i]);
            out.right.push(right[i]);
            for (p, l, r) in &between[i] {
                out.points.push(*p);
                out.left.push(*l);
                out.right.push(*r);
            }
        }
        out.segments.push(Segment {
            len: out.points.len() - start,
            closed: seg.closed,
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------------------
// Chamfer (section 8.1)
// ---------------------------------------------------------------------------------------

/// The chamfered spline and whether it has curved pieces (the type must then be Bezier).
pub struct Chamfered {
    pub data: SplineData,
    pub curved: bool,
}

/// Chamfer: each selected corner point becomes two points `distance` away along the edges
/// (at most half an edge), joined by an arc (`flat` off) or a straight cut (`flat` on).
pub fn chamfer(
    data: &SplineData,
    distance: f64,
    flat: bool,
    indices: Option<&[usize]>,
) -> Result<Chamfered, ToolError> {
    if !(distance.is_finite() && distance > 0.0) {
        return failed("the chamfer radius must be positive");
    }
    if let Some(i) = indices.and_then(|l| l.iter().find(|i| **i >= data.points.len())) {
        return failed(format!("point {i} does not exist"));
    }
    let selected = |i: usize| indices.is_none_or(|l| l.contains(&i));
    let mut out = SplineData::default();
    let mut curved = false;
    for (range, seg) in data.ranges().into_iter().zip(&data.segments) {
        let n = range.len();
        let start = out.points.len();
        for k in 0..n {
            let i = range.start + k;
            let corner = seg.closed || (k > 0 && k + 1 < n);
            let keep = |out: &mut SplineData| {
                out.points.push(data.points[i]);
                out.left.push(data.left[i]);
                out.right.push(data.right[i]);
            };
            if !corner || !selected(i) {
                keep(&mut out);
                continue;
            }
            let prev = data.points[range.start + (k + n - 1) % n];
            let next = data.points[range.start + (k + 1) % n];
            let p = data.points[i];
            let (to_p, from_p) = (p - prev, next - p);
            let (len_in, len_out) = (to_p.length(), from_p.length());
            if len_in < 1e-9 || len_out < 1e-9 {
                keep(&mut out);
                continue;
            }
            let (dir_in, dir_out) = (to_p / len_in, from_p / len_out);
            let turn = dir_in.angle_between(dir_out);
            if !(1e-6..PI - 1e-6).contains(&turn) {
                keep(&mut out);
                continue;
            }
            let d = distance.min(len_in / 2.0).min(len_out / 2.0);
            let (a, b) = (p - dir_in * d, p + dir_out * d);
            let (ha, hb) = if flat {
                (DVec3::ZERO, DVec3::ZERO)
            } else {
                curved = true;
                let rho = d / (turn / 2.0).tan();
                let h = CIRCLE_KAPPA * rho * turn / FRAC_PI_2;
                (dir_in * h, -dir_out * h)
            };
            out.points.extend([a, b]);
            out.left.extend([DVec3::ZERO, hb]);
            out.right.extend([ha, DVec3::ZERO]);
        }
        out.segments.push(Segment {
            len: out.points.len() - start,
            closed: seg.closed,
        });
    }
    Ok(Chamfered { data: out, curved })
}

// ---------------------------------------------------------------------------------------
// Create Outline (section 8.2)
// ---------------------------------------------------------------------------------------

/// First non-zero vector of `candidates`, normalized (zero if all vanish).
fn direction(candidates: [DVec3; 3]) -> DVec3 {
    candidates
        .into_iter()
        .find(|v| v.length_squared() > 1e-18)
        .map_or(DVec3::ZERO, DVec3::normalize)
}

/// The vector 90 degrees to the right of `d` in the XY plane.
fn right_of(d: DVec3) -> DVec3 {
    DVec3::new(d.y, -d.x, 0.0)
}

/// Length of a cubic piece measured on 50 chords.
fn piece_length(p: &[DVec3; 4]) -> f64 {
    let mut prev = p[0];
    (1..=50)
        .map(|k| {
            let q = Curve::bezier(p, k as f64 / 50.0);
            let l = (q - prev).length();
            prev = q;
            l
        })
        .sum()
}

/// The offset curve of one closed segment (points, left, right).
fn offset_segment(
    data: &SplineData,
    range: &std::ops::Range<usize>,
    distance: f64,
) -> (Vec<DVec3>, Vec<DVec3>, Vec<DVec3>) {
    let n = range.len();
    let at = |k: usize| range.start + k % n;
    // the pieces and their start and end tangents
    let pieces: Vec<[DVec3; 4]> = (0..n)
        .map(|k| {
            let (a, b) = (at(k), at(k + 1));
            [
                data.points[a],
                data.points[a] + data.right[a],
                data.points[b] + data.left[b],
                data.points[b],
            ]
        })
        .collect();
    let t_start = |p: &[DVec3; 4]| direction([p[1] - p[0], p[2] - p[0], p[3] - p[0]]);
    let t_end = |p: &[DVec3; 4]| direction([p[3] - p[2], p[3] - p[1], p[3] - p[0]]);
    // new positions: along the normal, or the miter at a corner
    let moved: Vec<DVec3> = (0..n)
        .map(|k| {
            let dir_in = t_end(&pieces[(k + n - 1) % n]);
            let dir_out = t_start(&pieces[k]);
            let (n_in, n_out) = (right_of(dir_in), right_of(dir_out));
            let p = data.points[at(k)];
            if n_in.dot(n_out) > 1.0 - 1e-9 || (n_in + n_out).length() < 1e-9 {
                return p + n_out * distance;
            }
            let bisector = (n_in + n_out).normalize();
            let cos_half = bisector.dot(n_in).max(0.05);
            p + bisector * (distance / cos_half)
        })
        .collect();
    let mut left = vec![DVec3::ZERO; n];
    let mut right = vec![DVec3::ZERO; n];
    for (k, piece) in pieces.iter().enumerate() {
        let j = (k + 1) % n;
        let (h0, h1) = (piece[1] - piece[0], piece[2] - piece[3]);
        let chord = piece[3] - piece[0];
        let zero = h0.length_squared() < 1e-18 && h1.length_squared() < 1e-18;
        let thirds = (h0 - chord / 3.0).length() < 1e-9 && (h1 + chord / 3.0).length() < 1e-9;
        if zero {
            continue;
        }
        if thirds {
            let new_chord = moved[j] - moved[k];
            right[k] = new_chord / 3.0;
            left[j] = -new_chord / 3.0;
            continue;
        }
        let length = piece_length(piece);
        let (t0, t3) = (t_start(piece), t_end(piece));
        let phi = t0.cross(t3).z.atan2(t0.dot(t3));
        let scale = if length > 1e-12 {
            ((length + distance * phi) / length).max(0.0)
        } else {
            1.0
        };
        right[k] = h0 * scale;
        left[j] = h1 * scale;
    }
    (moved, left, right)
}

/// Create Outline: the offset of every (closed) segment by `distance`, to the right of the
/// direction of travel, added to the spline as reversed segments.
pub fn outline(data: &SplineData, distance: f64) -> Result<SplineData, ToolError> {
    if !(distance.is_finite() && distance != 0.0) {
        return failed("the outline distance must be a non-zero number");
    }
    if data.segments.iter().any(|s| !s.closed) {
        return failed("the outline of an open segment is not supported yet");
    }
    let mut offsets = SplineData::default();
    for (range, seg) in data.ranges().into_iter().zip(&data.segments) {
        let (p, l, r) = offset_segment(data, &range, distance);
        // the offset curve as a reversed list: last point first, handles swapped
        offsets.points.extend(p.iter().rev());
        offsets.left.extend(r.iter().rev());
        offsets.right.extend(l.iter().rev());
        offsets.segments.push(*seg);
    }
    let (first, second) = if distance > 0.0 {
        (&offsets, data)
    } else {
        (data, &offsets)
    };
    let mut out = first.clone();
    out.points.extend(&second.points);
    out.left.extend(&second.left);
    out.right.extend(&second.right);
    out.segments.extend(&second.segments);
    Ok(out)
}

// ---------------------------------------------------------------------------------------
// Spline booleans (section 8.3)
// ---------------------------------------------------------------------------------------

/// The boolean operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoolOp {
    Union,
    And,
    Or,
    Subtract,
    Intersect,
}

impl BoolOp {
    pub const ALL: [BoolOp; 5] = [
        BoolOp::Union,
        BoolOp::And,
        BoolOp::Or,
        BoolOp::Subtract,
        BoolOp::Intersect,
    ];

    pub fn id(self) -> &'static str {
        match self {
            BoolOp::Union => "union",
            BoolOp::And => "and",
            BoolOp::Or => "or",
            BoolOp::Subtract => "subtract",
            BoolOp::Intersect => "intersect",
        }
    }

    pub fn from_id(id: &str) -> Option<BoolOp> {
        Self::ALL.into_iter().find(|o| o.id() == id)
    }
}

/// One input of a boolean, already in the common space.
#[derive(Debug, Clone)]
pub struct Operand {
    pub data: SplineData,
    pub ty: SplineType,
    pub sampling: Sampling,
}

/// The result: loops as closed segments, with the type to give the spline.
#[derive(Debug, Clone)]
pub struct BoolResult {
    pub data: SplineData,
    pub ty: SplineType,
}

#[derive(Clone, Copy, Debug)]
struct Node {
    pos: DVec3,
    left: DVec3,
    right: DVec3,
    /// The intersection this point was made at.
    id: Option<usize>,
}

/// A chain of nodes between two intersections (or a whole loop without any).
#[derive(Clone, Debug)]
struct Piece {
    nodes: Vec<Node>,
    from: Option<usize>,
    to: Option<usize>,
    inside: bool,
}

#[derive(Clone, Copy, Debug)]
struct Hit {
    /// Global curve parameters on a and b.
    sa: f64,
    sb: f64,
    p: DVec3,
}

fn cross2(a: DVec3, b: DVec3) -> f64 {
    a.x * b.y - a.y * b.x
}

/// Crossings of two closed sampled outlines, sorted by the parameter on a.
fn intersections(pa: &[(f64, DVec3)], end_a: f64, pb: &[(f64, DVec3)], end_b: f64) -> Vec<Hit> {
    let edge = |pts: &[(f64, DVec3)], end: f64, i: usize| {
        let (s0, p0) = pts[i];
        let (s1, p1) = pts.get(i + 1).copied().unwrap_or((end, pts[0].1));
        (s0, p0, s1, p1)
    };
    let mut hits: Vec<Hit> = Vec::new();
    for i in 0..pa.len() {
        let (sa0, a0, sa1, a1) = edge(pa, end_a, i);
        let r = a1 - a0;
        for j in 0..pb.len() {
            let (sb0, b0, sb1, b1) = edge(pb, end_b, j);
            let s = b1 - b0;
            let denom = cross2(r, s);
            if denom.abs() < 1e-12 {
                continue;
            }
            let t = cross2(b0 - a0, s) / denom;
            let u = cross2(b0 - a0, r) / denom;
            if (0.0..1.0).contains(&t) && (0.0..1.0).contains(&u) {
                let p = a0 + r * t;
                if hits.iter().all(|h| (h.p - p).length() > 1e-9) {
                    hits.push(Hit {
                        sa: sa0 + t * (sa1 - sa0),
                        sb: sb0 + u * (sb1 - sb0),
                        p,
                    });
                }
            }
        }
    }
    hits.sort_by(|a, b| a.sa.total_cmp(&b.sa));
    hits
}

/// The nodes of a closed curve split at the given `(parameter, id, point)` hits. The new
/// points sit on the crossing but keep the handle offsets of the exact split.
fn split_nodes(curve: &Curve, hits: &[(f64, usize, DVec3)]) -> Vec<Node> {
    let mut verts: Vec<(DVec3, Option<usize>)> = Vec::new();
    let mut pieces: Vec<[DVec3; 4]> = Vec::new();
    for (i, seg) in curve.segments.iter().enumerate() {
        let (lo, hi) = (i as f64, (i + 1) as f64);
        let mut here: Vec<(f64, usize, DVec3)> = hits
            .iter()
            .filter(|h| h.0 >= lo - 1e-12 && h.0 < hi)
            .copied()
            .collect();
        here.sort_by(|a, b| a.0.total_cmp(&b.0));
        let mut start = (seg[0], None);
        if let Some(first) = here.first().filter(|h| h.0 - lo < 1e-9) {
            start = (first.2, Some(first.1));
            here.remove(0);
        }
        verts.push(start);
        let (mut cur, mut t_prev) = (*seg, 0.0);
        for (s, id, p) in here {
            let t = s - lo;
            let (l, r) = split_piece(&cur, (t - t_prev) / (1.0 - t_prev));
            pieces.push(l);
            verts.push((p, Some(id)));
            cur = r;
            t_prev = t;
        }
        pieces.push(cur);
    }
    let n = verts.len();
    (0..n)
        .map(|k| {
            let (after, before) = (&pieces[k], &pieces[(k + n - 1) % n]);
            Node {
                pos: verts[k].0,
                left: before[2] - before[3],
                right: after[1] - after[0],
                id: verts[k].1,
            }
        })
        .collect()
}

/// Even-odd test of `p` against the closed outline `poly` (XY plane).
fn inside_outline(poly: &[(f64, DVec3)], p: DVec3) -> bool {
    let mut inside = false;
    let n = poly.len();
    for i in 0..n {
        let (a, b) = (poly[i].1, poly[(i + 1) % n].1);
        if (a.y > p.y) != (b.y > p.y) && p.x < a.x + (p.y - a.y) / (b.y - a.y) * (b.x - a.x) {
            inside = !inside;
        }
    }
    inside
}

/// Cuts the cyclic `nodes` at their intersection points and classifies each piece against
/// the other outline.
fn cut_pieces(nodes: &[Node], other: &[(f64, DVec3)]) -> Vec<Piece> {
    let splits: Vec<usize> = (0..nodes.len())
        .filter(|i| nodes[*i].id.is_some())
        .collect();
    let probe = |chain: &[Node]| {
        let (a, b) = (chain[0], chain[1]);
        let mid = Curve::bezier(&[a.pos, a.pos + a.right, b.pos + b.left, b.pos], 0.5);
        inside_outline(other, mid)
    };
    if splits.is_empty() {
        return vec![Piece {
            inside: probe(nodes),
            nodes: nodes.to_vec(),
            from: None,
            to: None,
        }];
    }
    (0..splits.len())
        .map(|j| {
            let (a, b) = (splits[j], splits[(j + 1) % splits.len()]);
            let len = (b + nodes.len() - a) % nodes.len();
            let len = if len == 0 { nodes.len() } else { len };
            let chain: Vec<Node> = (0..=len).map(|k| nodes[(a + k) % nodes.len()]).collect();
            Piece {
                inside: probe(&chain),
                from: nodes[a].id,
                to: nodes[b].id,
                nodes: chain,
            }
        })
        .collect()
}

/// A piece walked forward or backward.
#[derive(Clone, Copy)]
struct Oriented<'a> {
    piece: &'a Piece,
    rev: bool,
}

impl Oriented<'_> {
    fn start(&self) -> Option<usize> {
        if self.rev {
            self.piece.to
        } else {
            self.piece.from
        }
    }

    fn end(&self) -> Option<usize> {
        if self.rev {
            self.piece.from
        } else {
            self.piece.to
        }
    }

    fn nodes(&self) -> Vec<Node> {
        if self.rev {
            self.piece
                .nodes
                .iter()
                .rev()
                .map(|n| Node {
                    left: n.right,
                    right: n.left,
                    ..*n
                })
                .collect()
        } else {
            self.piece.nodes.clone()
        }
    }
}

/// Chains oriented pieces end to start into loops (in the order of the first piece of each).
fn chain<'a>(items: &[Oriented<'a>]) -> Result<Vec<Vec<Oriented<'a>>>, ToolError> {
    let mut used = vec![false; items.len()];
    let mut loops = Vec::new();
    for i in 0..items.len() {
        if used[i] {
            continue;
        }
        used[i] = true;
        let mut lp = vec![items[i]];
        let first = items[i].start();
        let mut end = items[i].end();
        while first.is_some() && end != first {
            let Some(j) = (0..items.len()).find(|j| !used[*j] && items[*j].start() == end) else {
                return failed("the outlines do not form closed loops (tangent touch?)");
            };
            used[j] = true;
            lp.push(items[j]);
            end = items[j].end();
        }
        loops.push(lp);
    }
    Ok(loops)
}

/// The points of a chained loop; a joint takes its left handle from the piece before and its
/// right handle from the piece after.
fn loop_nodes(lp: &[Oriented]) -> Vec<Node> {
    if lp.len() == 1 && lp[0].start().is_none() {
        return lp[0].nodes();
    }
    let parts: Vec<Vec<Node>> = lp.iter().map(Oriented::nodes).collect();
    let mut out = Vec::new();
    for (k, part) in parts.iter().enumerate() {
        let before = &parts[(k + parts.len() - 1) % parts.len()];
        for (m, node) in part[..part.len() - 1].iter().enumerate() {
            let mut node = *node;
            if m == 0 {
                node.left = before[before.len() - 1].left;
            }
            out.push(node);
        }
    }
    out
}

fn curve_of(op: &Operand, what: &str) -> Result<Curve, ToolError> {
    if op.data.segments.len() != 1 {
        return failed(format!("{what} must have exactly one segment"));
    }
    if !op.data.segments[0].closed {
        return failed(format!("{what} must be a closed spline"));
    }
    let curve = op.data.curve(op.ty, op.sampling);
    if curve.segments.len() < 2 {
        return failed(format!("{what} needs at least two points"));
    }
    Ok(curve)
}

fn pick(pieces: &[Piece], inside: bool, rev: bool) -> Vec<Oriented<'_>> {
    pieces
        .iter()
        .filter(|p| p.inside == inside)
        .map(|piece| Oriented { piece, rev })
        .collect()
}

fn join<'a>(x: Vec<Oriented<'a>>, y: Vec<Oriented<'a>>) -> Vec<Oriented<'a>> {
    x.into_iter().chain(y).collect()
}

/// Spline Union, And, Or, Subtract and Intersect of `a` and `b` (section 8.3).
pub fn boolean(a: &Operand, b: &Operand, op: BoolOp) -> Result<BoolResult, ToolError> {
    let (ca, cb) = (
        curve_of(a, "the first spline")?,
        curve_of(b, "the second spline")?,
    );
    let (pa, pb) = (
        sample_params(&ca, &a.sampling),
        sample_params(&cb, &b.sampling),
    );
    let hits = intersections(&pa, ca.segments.len() as f64, &pb, cb.segments.len() as f64);
    if hits.len() % 2 == 1 {
        return failed("the outlines touch each other without crossing");
    }
    let hits_a: Vec<(f64, usize, DVec3)> = hits
        .iter()
        .enumerate()
        .map(|(i, h)| (h.sa, i, h.p))
        .collect();
    let hits_b: Vec<(f64, usize, DVec3)> = hits
        .iter()
        .enumerate()
        .map(|(i, h)| (h.sb, i, h.p))
        .collect();
    let (na, nb) = (split_nodes(&ca, &hits_a), split_nodes(&cb, &hits_b));
    let (pieces_a, pieces_b) = (cut_pieces(&na, &pb), cut_pieces(&nb, &pa));
    let a_out = pick(&pieces_a, false, false);
    let a_in = pick(&pieces_a, true, false);
    let b_out = pick(&pieces_b, false, false);
    let b_in = pick(&pieces_b, true, false);
    let b_out_rev = pick(&pieces_b, false, true);
    let b_in_rev = pick(&pieces_b, true, true);
    // the loop sets, in the order of the export (section 8.3, rule 4)
    let b_minus_a = join(a_in.clone(), b_out_rev);
    let both = join(a_in, b_in);
    let a_minus_b = join(a_out.clone(), b_in_rev);
    let sets: Vec<Vec<Oriented<'_>>> = match op {
        BoolOp::Union => vec![join(a_out, b_out)],
        BoolOp::And => vec![both],
        BoolOp::Subtract => vec![a_minus_b],
        BoolOp::Or => vec![b_minus_a, a_minus_b],
        BoolOp::Intersect => vec![b_minus_a, both, a_minus_b],
    };
    let linear = ca.linear && cb.linear;
    let mut data = SplineData::default();
    for set in &sets {
        for mut lp in chain(set)? {
            // a loop with a reversed piece starts with its first reversed piece
            if let Some(k) = lp.iter().position(|o| o.rev) {
                lp.rotate_left(k);
            }
            let nodes = loop_nodes(&lp);
            if nodes.len() < 2 {
                continue;
            }
            data.segments.push(Segment {
                len: nodes.len(),
                closed: true,
            });
            for n in nodes {
                data.points.push(n.pos);
                data.left.push(if linear { DVec3::ZERO } else { n.left });
                data.right.push(if linear { DVec3::ZERO } else { n.right });
            }
        }
    }
    if data.points.is_empty() {
        return failed("the result is empty");
    }
    Ok(BoolResult {
        data,
        ty: if linear {
            SplineType::Linear
        } else {
            SplineType::Bezier
        },
    })
}

/// A `forge.spline` node as an operand in the space of `base` (z must vanish there).
pub fn operand_in(scene: &Scene, base: NodeId, id: NodeId) -> Result<Operand, ToolError> {
    let (mut data, ty, sampling) = editable_spline::settings_of(scene, id).map_err(|_| {
        ToolError::Failed("select editable splines (make them editable first)".into())
    })?;
    let world = |n: NodeId| {
        scene
            .world_matrix(n)
            .ok_or_else(|| ToolError::Failed(format!("no node {n}")))
    };
    let m: DMat4 = world(base)?.inverse() * world(id)?;
    for v in data.points.iter_mut() {
        *v = m.transform_point3(*v);
    }
    for v in data.left.iter_mut().chain(data.right.iter_mut()) {
        *v = m.transform_vector3(*v);
    }
    let flat = data.points.iter().all(|p| p.z.abs() < 1e-6);
    if !flat {
        return failed("the splines must lie in one plane (the first spline's XY plane)");
    }
    Ok(Operand { data, ty, sampling })
}

/// The boolean of two `forge.spline` nodes, computed in the space of `a`; nothing is edited.
pub fn boolean_of_nodes(
    scene: &Scene,
    a: NodeId,
    b: NodeId,
    op: BoolOp,
) -> Result<BoolResult, ToolError> {
    if a == b {
        return failed("a spline cannot be combined with itself");
    }
    boolean(&operand_in(scene, a, a)?, &operand_in(scene, a, b)?, op)
}

/// Puts the `result` of a boolean of `a` and `b` next to `a` (same parent and transform) and
/// deletes the inputs unless `keep_inputs`.
pub fn place_boolean(
    tx: &mut Tx,
    a: NodeId,
    b: NodeId,
    result: &BoolResult,
    keep_inputs: bool,
) -> Result<NodeId, ToolError> {
    let scene = tx.scene();
    let node = scene
        .node(a)
        .ok_or_else(|| ToolError::Failed(format!("no node {a}")))?;
    let parent = node.parent;
    let list = match parent {
        Some(p) => scene
            .node(p)
            .map(|n| n.children.clone())
            .unwrap_or_default(),
        None => scene.roots().to_vec(),
    };
    let index = list.iter().position(|c| *c == a).unwrap_or(list.len());
    let transform: Vec<(String, Value)> = node
        .params
        .iter()
        .filter(|(k, _)| k.starts_with(PREFIX))
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    let name = node.name.clone();
    let id = tx.create(TYPE, parent, Some(index))?;
    tx.rename(id, &name)?;
    for (k, v) in transform {
        tx.set_param(id, &k, v)?;
    }
    for (k, v) in result.data.to_values() {
        tx.set_param(id, &k, v)?;
    }
    let s = Sampling::new(Interpolation::Adaptive);
    tx.set_param(id, "type", Value::Enum(result.ty.id().into()))?;
    tx.set_param(id, "intermediate", Value::Enum(s.mode.id().into()))?;
    tx.set_param(id, "intermediate_number", Value::Int(s.number as i64))?;
    tx.set_param(id, "intermediate_angle", Value::Float(s.angle))?;
    tx.set_param(id, "intermediate_max_length", Value::Float(s.max_length))?;
    if !keep_inputs {
        tx.delete(a)?;
        tx.delete(b)?;
    }
    Ok(id)
}

/// [`boolean_of_nodes`] and [`place_boolean`] in one go.
pub fn boolean_in_tx(
    tx: &mut Tx,
    a: NodeId,
    b: NodeId,
    op: BoolOp,
    keep_inputs: bool,
) -> Result<NodeId, ToolError> {
    let result = boolean_of_nodes(tx.scene(), a, b, op)?;
    place_boolean(tx, a, b, &result, keep_inputs)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square(size: f64, at: DVec3) -> SplineData {
        let h = size / 2.0;
        SplineData::from_points(
            vec![
                at + DVec3::new(h, h, 0.0),
                at + DVec3::new(-h, h, 0.0),
                at + DVec3::new(-h, -h, 0.0),
                at + DVec3::new(h, -h, 0.0),
            ],
            true,
        )
    }

    fn operand(data: SplineData) -> Operand {
        Operand {
            data,
            ty: SplineType::Linear,
            sampling: Sampling::new(Interpolation::Adaptive),
        }
    }

    fn area(data: &SplineData) -> f64 {
        data.ranges()
            .iter()
            .map(|r| {
                let p = &data.points[r.clone()];
                (0..p.len())
                    .map(|i| cross2(p[i], p[(i + 1) % p.len()]))
                    .sum::<f64>()
                    / 2.0
            })
            .sum()
    }

    #[test]
    fn reverse_twice_is_the_identity() {
        let mut d = square(10.0, DVec3::ZERO);
        d.right[1] = DVec3::X;
        d.left[2] = DVec3::Y;
        let r = reverse(&d);
        assert_eq!(r.points[0], d.points[3]);
        assert_eq!(r.left[2], d.right[1]);
        assert_eq!(reverse(&r), d);
    }

    #[test]
    fn subdivide_keeps_a_bezier_curve() {
        let d = SplineData {
            points: vec![DVec3::ZERO, DVec3::new(30.0, 0.0, 0.0)],
            left: vec![DVec3::ZERO, DVec3::new(0.0, -10.0, 0.0)],
            right: vec![DVec3::new(0.0, 10.0, 0.0), DVec3::ZERO],
            segments: vec![Segment {
                len: 2,
                closed: false,
            }],
        };
        let s = subdivide(&d, SplineType::Bezier, 3).unwrap();
        assert_eq!(s.points.len(), 4);
        let whole = [
            d.points[0],
            d.points[0] + d.right[0],
            d.points[1] + d.left[1],
            d.points[1],
        ];
        for (k, i) in [(1.0 / 3.0, 1), (2.0 / 3.0, 2)] {
            assert!((s.points[i] - Curve::bezier(&whole, k)).length() < 1e-9);
        }
        // the middle piece is the original curve between t = 1/3 and 2/3
        let mid = [
            s.points[1],
            s.points[1] + s.right[1],
            s.points[2] + s.left[2],
            s.points[2],
        ];
        assert!((Curve::bezier(&mid, 0.5) - Curve::bezier(&whole, 0.5)).length() < 1e-9);
    }

    #[test]
    fn subdivide_open_spline_has_no_closing_span() {
        let d = SplineData::from_points(vec![DVec3::ZERO, DVec3::X * 10.0], false);
        let s = subdivide(&d, SplineType::Linear, 2).unwrap();
        assert_eq!(s.points.len(), 3);
        assert!(subdivide(&d, SplineType::Linear, 0).is_err());
    }

    #[test]
    fn chamfer_flat_cuts_the_corner() {
        let d = square(100.0, DVec3::ZERO);
        let c = chamfer(&d, 10.0, true, None).unwrap();
        assert_eq!(c.data.points.len(), 8);
        assert!(!c.curved);
        assert_eq!(c.data.points[0], DVec3::new(50.0, 40.0, 0.0));
        assert_eq!(c.data.points[1], DVec3::new(40.0, 50.0, 0.0));
        // one corner only
        let one = chamfer(&d, 10.0, true, Some(&[2])).unwrap();
        assert_eq!(one.data.points.len(), 5);
        assert!(chamfer(&d, 10.0, true, Some(&[9])).is_err());
        assert!(chamfer(&d, 0.0, true, None).is_err());
    }

    #[test]
    fn chamfer_leaves_the_ends_of_an_open_spline() {
        let d = SplineData::from_points(
            vec![
                DVec3::ZERO,
                DVec3::new(10.0, 0.0, 0.0),
                DVec3::new(10.0, 10.0, 0.0),
            ],
            false,
        );
        let c = chamfer(&d, 2.0, true, None).unwrap();
        assert_eq!(c.data.points.len(), 4);
        assert_eq!(c.data.points[0], DVec3::ZERO);
    }

    #[test]
    fn outline_of_a_square_is_a_bigger_square() {
        let d = square(100.0, DVec3::ZERO);
        let o = outline(&d, 10.0).unwrap();
        assert_eq!(o.segments.len(), 2);
        // offset first and reversed (clockwise), then the original
        assert!(
            (area(&SplineData {
                points: o.points[..4].to_vec(),
                left: o.left[..4].to_vec(),
                right: o.right[..4].to_vec(),
                segments: vec![o.segments[0]],
            }) + 120.0 * 120.0)
                .abs()
                < 1e-9
        );
        let inner = outline(&d, -10.0).unwrap();
        assert_eq!(inner.points[0], d.points[0]);
        assert!(outline(&d, 0.0).is_err());
        let mut open = d.clone();
        open.segments[0].closed = false;
        assert!(outline(&open, 5.0).is_err());
    }

    #[test]
    fn square_booleans() {
        let a = operand(square(100.0, DVec3::ZERO));
        let b = operand(square(100.0, DVec3::new(50.0, 0.0, 0.0)));
        let total = |op| {
            let r = boolean(&a, &b, op).unwrap();
            (r.data.segments.len(), area(&r.data).abs())
        };
        assert_eq!(total(BoolOp::Union), (1, 15000.0));
        assert_eq!(total(BoolOp::And), (1, 5000.0));
        assert_eq!(total(BoolOp::Subtract), (1, 5000.0));
        assert_eq!(total(BoolOp::Or).0, 2);
        assert_eq!(total(BoolOp::Intersect).0, 3);
        // the lens is left out of Or
        assert!((boolean(&a, &b, BoolOp::Or).unwrap().data.points.len() as f64) > 0.0);
    }

    #[test]
    fn disjoint_and_nested_squares() {
        let a = operand(square(100.0, DVec3::ZERO));
        let far = operand(square(10.0, DVec3::new(500.0, 0.0, 0.0)));
        let inner = operand(square(10.0, DVec3::ZERO));
        assert_eq!(
            boolean(&a, &far, BoolOp::Union)
                .unwrap()
                .data
                .segments
                .len(),
            2
        );
        assert!(boolean(&a, &far, BoolOp::And).is_err());
        assert_eq!(
            boolean(&a, &far, BoolOp::Subtract)
                .unwrap()
                .data
                .segments
                .len(),
            1
        );
        // a hole: the inner square comes back reversed
        let hole = boolean(&a, &inner, BoolOp::Subtract).unwrap();
        assert_eq!(hole.data.segments.len(), 2);
        assert!((area(&hole.data).abs() - (10000.0 - 100.0)).abs() < 1e-9);
        assert_eq!(
            boolean(&a, &inner, BoolOp::Union)
                .unwrap()
                .data
                .segments
                .len(),
            1
        );
        assert!(boolean(&inner, &a, BoolOp::Subtract).is_err());
    }

    #[test]
    fn open_or_multi_segment_inputs_fail() {
        let mut open = square(10.0, DVec3::ZERO);
        open.segments[0].closed = false;
        let a = operand(square(10.0, DVec3::ZERO));
        assert!(boolean(&a, &operand(open), BoolOp::Union).is_err());
    }
}
