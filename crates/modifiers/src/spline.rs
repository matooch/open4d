//! The spline-driven deformers: Spline Wrap, Spline and Spline Rail
//! (docs/specs/MODIFIERS.md §20). Pure maths on a sampled polyline; the scene glue is in
//! `lib.rs`.

use forge_geometry::PolyMesh;
use glam::{DQuat, DVec3};

/// A linked spline as its sampled polyline (§20.0).
#[derive(Debug, Clone)]
pub struct Polyline {
    pts: Vec<DVec3>,
    /// Cumulative chord length at each vertex (`cum[0] = 0`).
    cum: Vec<f64>,
    closed: bool,
    /// Unit direction of each chord (`pts[k] -> pts[k + 1]`, the closing chord last when closed).
    chords: Vec<DVec3>,
    /// Parallel-transport frame `(N, B, T)` at each vertex.
    frames: Vec<[DVec3; 3]>,
}

impl Polyline {
    /// The polyline of a resolved spline helper mesh: `points` in order, one 2-point edge per
    /// chord, closed iff there is an edge per point. `None` for fewer than two distinct points.
    pub fn from_mesh(mesh: &PolyMesh, to_space: impl Fn(DVec3) -> DVec3) -> Option<Polyline> {
        let mut pts: Vec<DVec3> = mesh.points.iter().map(|p| to_space(*p)).collect();
        let closed = pts.len() > 2 && mesh.polys.len() == pts.len();
        if pts.len() < 2 {
            return None;
        }
        pts.shrink_to_fit();
        Polyline::new(pts, closed)
    }

    pub fn new(pts: Vec<DVec3>, closed: bool) -> Option<Polyline> {
        let n = pts.len();
        let spans = if closed { n } else { n - 1 };
        let mut cum = vec![0.0];
        let mut chords = Vec::with_capacity(spans);
        for k in 0..spans {
            let d = pts[(k + 1) % n] - pts[k];
            cum.push(cum[k] + d.length());
            chords.push(d.normalize_or_zero());
        }
        if cum[spans] <= 0.0 || !cum[spans].is_finite() {
            return None;
        }
        let frames = transport_frames(&pts, &chords, closed);
        Some(Polyline {
            pts,
            cum,
            closed,
            chords,
            frames,
        })
    }

    /// Total length `L`.
    pub fn length(&self) -> f64 {
        *self.cum.last().expect("a polyline has a length")
    }

    fn spans(&self) -> usize {
        self.chords.len()
    }

    /// The span containing `s` (`0 <= s <= L`) and the length along it.
    fn locate(&self, s: f64) -> (usize, f64) {
        let spans = self.spans();
        let k = self
            .cum
            .partition_point(|l| *l <= s)
            .saturating_sub(1)
            .min(spans - 1);
        (k, s - self.cum[k])
    }

    /// The point at arc length `s`. Open splines continue along the end chords (`extend`)
    /// or hold the end (`!extend`); closed splines wrap.
    pub fn point_at(&self, s: f64, extend: bool) -> DVec3 {
        let l = self.length();
        let s = if self.closed { s.rem_euclid(l) } else { s };
        let n = self.pts.len();
        if s < 0.0 || s > l {
            let (end, dir, over) = if s < 0.0 {
                (self.pts[0], self.chords[0], s)
            } else {
                (self.pts[n - 1], self.chords[self.spans() - 1], s - l)
            };
            return if extend { end + dir * over } else { end };
        }
        let (k, along) = self.locate(s);
        self.pts[k] + self.chords[k] * along
    }

    /// The frame `(N, B, T)` at arc length `s` (§20.0): the chord's tangent with the normal
    /// interpolated between the span's vertices; constant outside an open spline.
    pub fn frame_at(&self, s: f64) -> [DVec3; 3] {
        let l = self.length();
        let s = if self.closed {
            s.rem_euclid(l)
        } else {
            s.clamp(0.0, l)
        };
        let (k, along) = self.locate(s);
        let n = self.pts.len();
        let k1 = (k + 1) % n;
        let f = along / (self.cum[k + 1] - self.cum[k]).max(1e-300);
        let t = self.chords[k];
        if t == DVec3::ZERO {
            return self.frames[k];
        }
        let n0 = self.frames[k][0];
        let n1 = self.frames[k1.min(self.frames.len() - 1)][0];
        let nn = n0.lerp(n1, f);
        let nn = (nn - t * nn.dot(t)).normalize_or_zero();
        let nn = if nn == DVec3::ZERO { n0 } else { nn };
        [nn, t.cross(nn), t]
    }

    /// The vertex frame `(N, B, T)` of the first vertex.
    pub fn start_frame(&self) -> [DVec3; 3] {
        self.frames[0]
    }

    /// The point at the fraction `f` of the spline (§20.3): inside an open spline the
    /// fraction runs over the vertex index, `f = k / (n - 1)` is vertex `k` (the sampled spline
    /// is not equally spaced to the last digits; `spline-rail/defaults` says index, not length);
    /// outside it continues along the end chord by `f` (or `f - 1`) times the length.
    pub fn at_fraction(&self, f: f64, extend: bool) -> DVec3 {
        let n = self.pts.len();
        if self.closed {
            let x = (f.rem_euclid(1.0)) * n as f64;
            let k = (x.floor() as usize).min(n - 1);
            let a = self.pts[k];
            return a + (self.pts[(k + 1) % n] - a) * (x - k as f64);
        }
        if (0.0..=1.0).contains(&f) {
            let x = f * (n - 1) as f64;
            let k = (x.floor() as usize).min(n - 2);
            let a = self.pts[k];
            return a + (self.pts[k + 1] - a) * (x - k as f64);
        }
        if !extend {
            return if f < 0.0 {
                self.pts[0]
            } else {
                self.pts[n - 1]
            };
        }
        if f < 0.0 {
            self.pts[0] + self.chords[0] * (f * self.length())
        } else {
            self.pts[n - 1] + self.chords[self.spans() - 1] * ((f - 1.0) * self.length())
        }
    }

    /// The closest point of the polyline to `p`: `(distance, arc length)`.
    pub fn nearest(&self, p: DVec3) -> (f64, f64) {
        let n = self.pts.len();
        let mut best = (f64::INFINITY, 0.0);
        for k in 0..self.spans() {
            let a = self.pts[k];
            let b = self.pts[(k + 1) % n];
            let ab = b - a;
            let len2 = ab.length_squared();
            let u = if len2 > 0.0 {
                ((p - a).dot(ab) / len2).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let d = (p - (a + ab * u)).length();
            if d < best.0 {
                best = (d, self.cum[k] + u * len2.sqrt());
            }
        }
        best
    }
}

/// Parallel-transport frames at the vertices (§20.0).
fn transport_frames(pts: &[DVec3], chords: &[DVec3], closed: bool) -> Vec<[DVec3; 3]> {
    let n = pts.len();
    let tangent = |k: usize| -> DVec3 {
        let prev = if k > 0 {
            Some(chords[k - 1])
        } else if closed {
            chords.last().copied()
        } else {
            None
        };
        let next = chords.get(k).copied().filter(|_| k < chords.len());
        match (prev, next) {
            (Some(a), Some(b)) => {
                let s = a + b;
                if s.length() > 1e-12 { s.normalize() } else { b }
            }
            (Some(a), None) => a,
            (None, Some(b)) => b,
            (None, None) => DVec3::Y,
        }
    };
    let t0 = tangent(0);
    let mut normal = (DVec3::Z.cross(t0)).normalize_or_zero();
    if normal == DVec3::ZERO {
        normal = t0.any_orthonormal_vector();
    }
    let mut out = Vec::with_capacity(n);
    let mut prev_t = t0;
    for k in 0..n {
        let t = tangent(k);
        if k > 0 && t != DVec3::ZERO && prev_t != DVec3::ZERO {
            normal = (DQuat::from_rotation_arc(prev_t, t) * normal).normalize_or_zero();
        }
        if normal == DVec3::ZERO {
            normal = t.any_orthonormal_vector();
        }
        out.push([normal, t.cross(normal), t]);
        prev_t = t;
    }
    out
}

/// Which object axis runs along the spline (§20.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WrapAxis {
    /// 0 = X, 1 = Y, 2 = Z.
    pub index: usize,
    pub negative: bool,
}

/// Parameters of Spline Wrap (§20.1); fractions, not percent.
#[derive(Debug, Clone)]
pub struct WrapParams {
    pub axis: WrapAxis,
    pub strength: f64,
    pub offset: f64,
    pub from: f64,
    pub to: f64,
    pub keep_length: bool,
    pub extend: bool,
    pub rotate: bool,
    /// `(center, size)` of the fixed box.
    pub fixed_box: Option<(DVec3, DVec3)>,
}

/// Wraps `points` (in the deformer's space) along `line` (§20.1).
pub fn spline_wrap(points: &mut [DVec3], line: &Polyline, p: &WrapParams) {
    if points.is_empty() {
        return;
    }
    let along = (p.axis.index + 2) % 3;
    let (ni, bi) = ((along + 1) % 3, (along + 2) % 3);
    let (min, size) = match p.fixed_box {
        Some((c, s)) => (c[along] - s[along] / 2.0, s[along]),
        None => {
            let (lo, hi) = points
                .iter()
                .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), q| {
                    (lo.min(q[along]), hi.max(q[along]))
                });
            (lo, hi - lo)
        }
    };
    let l = line.length();
    let start = line.start_frame();
    for q in points.iter_mut() {
        let a = q[along];
        let dist = if p.axis.negative {
            min + size - a
        } else {
            a - min
        };
        let s = if p.keep_length {
            (p.from + p.offset) * l + dist
        } else {
            let u = if size > 0.0 { dist / size } else { 0.0 };
            (p.from + p.offset + u * (p.to - p.from)) * l
        };
        let s = if p.extend { s } else { s.clamp(0.0, l) };
        let [n, b, _] = if p.rotate { line.frame_at(s) } else { start };
        let wrapped = line.point_at(s, p.extend) + n * q[ni] + b * q[bi];
        *q += (wrapped - *q) * p.strength;
    }
}

/// How Spline Rail treats points outside its box (§20.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RailMode {
    Limited,
    WithinBox,
    Unlimited,
}

/// Parameters of Spline Rail (§20.3).
#[derive(Debug, Clone)]
pub struct RailParams {
    pub mode: RailMode,
    pub size: DVec3,
    /// Width scale before the start / after the end (fractions).
    pub scale_before: f64,
    pub scale_after: f64,
}

/// Spline Rail with a left and a right Z spline (or one of them) (§20.3).
pub fn spline_rail(
    points: &mut [DVec3],
    left: Option<&Polyline>,
    right: Option<&Polyline>,
    p: &RailParams,
) {
    if left.is_none() && right.is_none() {
        return;
    }
    let half = p.size / 2.0;
    for q in points.iter_mut() {
        let inside = q.abs().cmple(half).all();
        if p.mode == RailMode::WithinBox && !inside {
            continue;
        }
        let u = if p.size.x > 0.0 {
            q.x / p.size.x + 0.5
        } else {
            0.5
        };
        let t_raw = if p.size.z > 0.0 {
            q.z / p.size.z + 0.5
        } else {
            0.0
        };
        let (t, carry) = match p.mode {
            RailMode::Limited => (t_raw.clamp(0.0, 1.0), t_raw - t_raw.clamp(0.0, 1.0)),
            _ => (t_raw, 0.0),
        };
        let extend = p.mode != RailMode::WithinBox;
        let at = |l: &Polyline| l.at_fraction(t, extend);
        let width_scale = if t < 0.0 {
            p.scale_before
        } else if t > 1.0 {
            p.scale_after
        } else {
            1.0
        };
        let mut base = match (left, right) {
            (Some(l), Some(r)) => {
                let (a, b) = (at(l), at(r));
                let mid = (a + b) * 0.5;
                mid + (b - a) * ((u - 0.5) * width_scale)
            }
            (Some(s), None) | (None, Some(s)) => at(s) + DVec3::new(q.x, 0.0, 0.0),
            (None, None) => unreachable!("checked above"),
        };
        if carry != 0.0 {
            // rigid carry along the mean end chord (Limited)
            let side = if carry < 0.0 { 0.0 } else { 1.0 };
            let end_dir = |l: &Polyline| l.frame_at(side * l.length())[2];
            let dir = match (left, right) {
                (Some(l), Some(r)) => (end_dir(l) + end_dir(r)).normalize_or_zero(),
                (Some(s), None) | (None, Some(s)) => end_dir(s),
                (None, None) => DVec3::ZERO,
            };
            base += dir * (carry * p.size.z);
        }
        *q = base + DVec3::new(0.0, q.y, 0.0);
    }
}

/// Parameters of the Spline deformer (§20.2).
#[derive(Debug, Clone, Copy)]
pub struct SplineDeformParams {
    pub radius: f64,
}

/// Moves the points within `radius` of `original` towards the modifying spline (§20.2).
pub fn spline_deform(
    points: &mut [DVec3],
    original: &Polyline,
    modifying: &Polyline,
    p: &SplineDeformParams,
) {
    if p.radius <= 0.0 {
        return;
    }
    for q in points.iter_mut() {
        let (d, s) = original.nearest(*q);
        if d >= p.radius {
            continue;
        }
        let f = s / original.length();
        let w = 1.0 - d / p.radius;
        let delta = modifying.at_fraction(f, false) - original.at_fraction(f, false);
        *q += delta * w;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn quarter_circle() -> Polyline {
        let pts: Vec<DVec3> = (0..=18)
            .map(|k| {
                let a = (k as f64) * 5f64.to_radians();
                DVec3::new(200.0 * a.cos(), 200.0 * a.sin(), 0.0)
            })
            .collect();
        Polyline::new(pts, false).unwrap()
    }

    #[test]
    fn it_extends_open_splines_along_their_end_chords_and_wraps_closed_ones() {
        let line = quarter_circle();
        let l = line.length();
        let before = line.point_at(-10.0, true);
        let first = line.point_at(0.0, true);
        assert!(((first - before).length() - 10.0).abs() < 1e-9);
        assert_eq!(line.point_at(-10.0, false), first);
        let ring = Polyline::new(
            vec![
                DVec3::new(1.0, 0.0, 0.0),
                DVec3::new(0.0, 1.0, 0.0),
                DVec3::new(-1.0, 0.0, 0.0),
                DVec3::new(0.0, -1.0, 0.0),
            ],
            true,
        )
        .unwrap();
        let rl = ring.length();
        assert!((ring.point_at(0.3, true) - ring.point_at(0.3 + rl, true)).length() < 1e-9);
        assert!(l > 0.0);
    }

    #[test]
    fn its_frame_is_orthonormal_and_planar_splines_keep_the_binormal() {
        let line = quarter_circle();
        for s in [0.0, 50.0, 150.0, line.length(), line.length() + 20.0] {
            let [n, b, t] = line.frame_at(s);
            assert!((n.length() - 1.0).abs() < 1e-9 && (b.length() - 1.0).abs() < 1e-9);
            assert!(n.dot(b).abs() < 1e-9 && n.dot(t).abs() < 1e-9 && b.dot(t).abs() < 1e-9);
            assert!((n.cross(b) - t).length() < 1e-9);
            assert!((b - DVec3::Z).length() < 1e-9);
        }
    }

    #[test]
    fn spline_wrap_lays_a_straight_bar_along_the_curve() {
        let line = quarter_circle();
        let l = line.length();
        // a bar along z from 0 to 1 unit of length fraction, one unit wide in x
        let mut pts = vec![DVec3::new(0.0, 0.0, 0.0), DVec3::new(10.0, 0.0, 1.0)];
        let p = WrapParams {
            axis: WrapAxis {
                index: 0,
                negative: false,
            },
            strength: 1.0,
            offset: 0.0,
            from: 0.0,
            to: 1.0,
            keep_length: false,
            extend: true,
            rotate: true,
            fixed_box: Some((DVec3::new(0.0, 0.0, 0.5), DVec3::new(1.0, 1.0, 1.0))),
        };
        spline_wrap(&mut pts, &line, &p);
        assert!((pts[0] - line.point_at(0.0, true)).length() < 1e-9);
        let [n, _, _] = line.frame_at(l);
        assert!((pts[1] - (line.point_at(l, true) + n * 10.0)).length() < 1e-9);
        // strength 0 keeps the points
        let mut keep = vec![DVec3::new(3.0, 4.0, 5.0)];
        spline_wrap(&mut keep, &line, &WrapParams { strength: 0.0, ..p });
        assert_eq!(keep[0], DVec3::new(3.0, 4.0, 5.0));
    }

    #[test]
    fn spline_rail_interpolates_between_two_rails() {
        let rail = |x: f64| {
            Polyline::new(
                vec![DVec3::new(x, 0.0, 0.0), DVec3::new(x, 0.0, 100.0)],
                false,
            )
            .unwrap()
        };
        let (l, r) = (rail(-150.0), rail(150.0));
        let p = RailParams {
            mode: RailMode::Unlimited,
            size: DVec3::splat(200.0),
            scale_before: 1.0,
            scale_after: 1.0,
        };
        let mut pts = vec![
            DVec3::new(-100.0, 7.0, -100.0),
            DVec3::new(100.0, 0.0, 100.0),
            DVec3::new(0.0, 0.0, 0.0),
        ];
        spline_rail(&mut pts, Some(&l), Some(&r), &p);
        assert!((pts[0] - DVec3::new(-150.0, 7.0, 0.0)).length() < 1e-9);
        assert!((pts[1] - DVec3::new(150.0, 0.0, 100.0)).length() < 1e-9);
        assert!((pts[2] - DVec3::new(0.0, 0.0, 50.0)).length() < 1e-9);
    }

    #[test]
    fn spline_deform_moves_only_points_inside_the_radius() {
        let a = Polyline::new(vec![DVec3::ZERO, DVec3::new(100.0, 0.0, 0.0)], false).unwrap();
        let b = Polyline::new(
            vec![DVec3::new(0.0, 50.0, 0.0), DVec3::new(100.0, 50.0, 0.0)],
            false,
        )
        .unwrap();
        let mut pts = vec![
            DVec3::new(50.0, 0.0, 0.0),
            DVec3::new(50.0, 10.0, 0.0),
            DVec3::new(50.0, 40.0, 0.0),
        ];
        spline_deform(&mut pts, &a, &b, &SplineDeformParams { radius: 30.0 });
        assert!((pts[0].y - 50.0).abs() < 1e-9);
        assert!(pts[1].y > 10.0 && pts[1].y < 60.0);
        assert_eq!(pts[2], DVec3::new(50.0, 40.0, 0.0));
    }
}
