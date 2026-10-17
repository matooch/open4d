//! Spline curves (docs/specs/GEOMETRY.md §5).
//! Refs: Farin, *Curves and Surfaces for CAGD* (2002); Bartels, Beatty & Barsky (1987);
//! Catmull & Rom (1974); Akima, JACM 17(4) (1970).

use glam::DVec3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplineKind {
    Linear,
    Bezier,
    CatmullRom,
    BSpline,
    Akima,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum SplineError {
    #[error("a spline needs at least 2 points")]
    TooFewPoints,
    #[error("a Bézier spline needs one [in, out] handle pair per point")]
    HandleCount,
    #[error("use Spline::bezier for Bézier splines")]
    IsBezier,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Spline {
    pub kind: SplineKind,
    pub points: Vec<DVec3>,
    /// Bézier only: `[in, out]` tangent handles as offsets from each point.
    pub handles: Vec<[DVec3; 2]>,
    pub closed: bool,
}

/// Hermite basis at `t`: (value weights, derivative weights) for (p0, m0, p1, m1).
fn hermite(t: f64) -> ([f64; 4], [f64; 4]) {
    let (t2, t3) = (t * t, t * t * t);
    (
        [
            2.0 * t3 - 3.0 * t2 + 1.0,
            t3 - 2.0 * t2 + t,
            -2.0 * t3 + 3.0 * t2,
            t3 - t2,
        ],
        [
            6.0 * t2 - 6.0 * t,
            3.0 * t2 - 4.0 * t + 1.0,
            -6.0 * t2 + 6.0 * t,
            3.0 * t2 - 2.0 * t,
        ],
    )
}

fn combine(w: [f64; 4], v: [DVec3; 4]) -> DVec3 {
    w.iter().zip(v).map(|(w, v)| v * *w).sum()
}

const GL5: [(f64, f64); 5] = [
    (0.0, 0.568_888_888_888_888_9),
    (-0.538_469_310_105_683_1, 0.478_628_670_499_366_5),
    (0.538_469_310_105_683_1, 0.478_628_670_499_366_5),
    (-0.906_179_845_938_664, 0.236_926_885_056_189_1),
    (0.906_179_845_938_664, 0.236_926_885_056_189_1),
];

impl Spline {
    pub fn new(kind: SplineKind, points: Vec<DVec3>, closed: bool) -> Result<Spline, SplineError> {
        if kind == SplineKind::Bezier {
            return Err(SplineError::IsBezier);
        }
        if points.len() < 2 {
            return Err(SplineError::TooFewPoints);
        }
        Ok(Spline {
            kind,
            points,
            handles: Vec::new(),
            closed,
        })
    }

    pub fn bezier(
        points: Vec<DVec3>,
        handles: Vec<[DVec3; 2]>,
        closed: bool,
    ) -> Result<Spline, SplineError> {
        if points.len() < 2 {
            return Err(SplineError::TooFewPoints);
        }
        if handles.len() != points.len() {
            return Err(SplineError::HandleCount);
        }
        Ok(Spline {
            kind: SplineKind::Bezier,
            points,
            handles,
            closed,
        })
    }

    pub fn segment_count(&self) -> usize {
        let n = self.points.len();
        if self.closed { n } else { n - 1 }
    }

    /// Point `k`, wrapping when closed; open ends get reflected phantom points.
    fn p(&self, k: isize) -> DVec3 {
        let n = self.points.len() as isize;
        if self.closed {
            return self.points[k.rem_euclid(n) as usize];
        }
        let at = |i: isize| self.points[i as usize];
        match k {
            k if k < 0 => 2.0 * at(0) - at(1),
            k if k >= n => 2.0 * at(n - 1) - at(n - 2),
            k => at(k),
        }
    }

    /// Slope between points `k` and `k + 1`, extrapolated past open ends (Akima).
    fn slope(&self, k: isize) -> DVec3 {
        let n = self.points.len() as isize;
        if self.closed {
            return self.p(k + 1) - self.p(k);
        }
        let last = n - 2; // index of the last real slope
        match k {
            k if k < 0 => 2.0 * self.slope(k + 1) - self.slope(k + 2),
            k if k > last => 2.0 * self.slope(k - 1) - self.slope(k - 2),
            k => self.points[k as usize + 1] - self.points[k as usize],
        }
    }

    fn akima_tangent(&self, i: isize) -> DVec3 {
        let m = [
            self.slope(i - 2),
            self.slope(i - 1),
            self.slope(i),
            self.slope(i + 1),
        ];
        let mut out = DVec3::ZERO;
        for c in 0..3 {
            let w1 = (m[3][c] - m[2][c]).abs();
            let w2 = (m[1][c] - m[0][c]).abs();
            out[c] = if w1 + w2 == 0.0 {
                (m[1][c] + m[2][c]) / 2.0
            } else {
                (w1 * m[1][c] + w2 * m[2][c]) / (w1 + w2)
            };
        }
        out
    }

    /// Value and derivative of segment `seg` at `t ∈ [0, 1]`.
    ///
    /// # Panics
    /// If `seg >= segment_count()`.
    fn eval_both(&self, seg: usize, t: f64) -> (DVec3, DVec3) {
        assert!(seg < self.segment_count(), "segment {seg} out of range");
        let i = seg as isize;
        let (p0, p1) = (self.p(i), self.p(i + 1));
        let hermite_seg = |m0: DVec3, m1: DVec3| {
            let (w, dw) = hermite(t);
            let v = [p0, m0, p1, m1];
            (combine(w, v), combine(dw, v))
        };
        match self.kind {
            SplineKind::Linear => (p0.lerp(p1, t), p1 - p0),
            SplineKind::Bezier => {
                let j = (seg + 1) % self.points.len();
                let b = [p0, p0 + self.handles[seg][1], p1 + self.handles[j][0], p1];
                let s = 1.0 - t;
                let w = [s * s * s, 3.0 * s * s * t, 3.0 * s * t * t, t * t * t];
                let d = 3.0
                    * (s * s * (b[1] - b[0]) + 2.0 * s * t * (b[2] - b[1]) + t * t * (b[3] - b[2]));
                (combine(w, b), d)
            }
            SplineKind::CatmullRom => {
                hermite_seg((p1 - self.p(i - 1)) / 2.0, (self.p(i + 2) - p0) / 2.0)
            }
            SplineKind::Akima if self.points.len() == 2 && !self.closed => {
                (p0.lerp(p1, t), p1 - p0)
            }
            SplineKind::Akima => hermite_seg(self.akima_tangent(i), self.akima_tangent(i + 1)),
            SplineKind::BSpline => {
                let c = [self.p(i - 1), p0, p1, self.p(i + 2)];
                let (s, t2, t3) = (1.0 - t, t * t, t * t * t);
                let w = [
                    s * s * s,
                    3.0 * t3 - 6.0 * t2 + 4.0,
                    -3.0 * t3 + 3.0 * t2 + 3.0 * t + 1.0,
                    t3,
                ];
                let dw = [
                    -3.0 * s * s,
                    9.0 * t2 - 12.0 * t,
                    -9.0 * t2 + 6.0 * t + 3.0,
                    3.0 * t2,
                ];
                (combine(w, c) / 6.0, combine(dw, c) / 6.0)
            }
        }
    }

    pub fn eval(&self, seg: usize, t: f64) -> DVec3 {
        self.eval_both(seg, t).0
    }

    pub fn derivative(&self, seg: usize, t: f64) -> DVec3 {
        self.eval_both(seg, t).1
    }

    /// `steps` samples per segment, plus the end point for open splines.
    pub fn tessellate(&self, steps: usize) -> Vec<DVec3> {
        let mut out = Vec::with_capacity(self.segment_count() * steps + 1);
        for seg in 0..self.segment_count() {
            for k in 0..steps {
                out.push(self.eval(seg, k as f64 / steps as f64));
            }
        }
        if !self.closed {
            out.push(self.eval(self.segment_count() - 1, 1.0));
        }
        out
    }

    /// Arc length of `seg` between `a` and `b` (5-point Gauss–Legendre).
    fn length_between(&self, seg: usize, a: f64, b: f64) -> f64 {
        let (mid, half) = ((a + b) / 2.0, (b - a) / 2.0);
        GL5.iter()
            .map(|(x, w)| w * self.derivative(seg, mid + half * x).length())
            .sum::<f64>()
            * half
    }

    fn segment_length(&self, seg: usize, intervals: usize) -> f64 {
        (0..intervals)
            .map(|k| {
                let (a, b) = (
                    k as f64 / intervals as f64,
                    (k + 1) as f64 / intervals as f64,
                );
                self.length_between(seg, a, b)
            })
            .sum()
    }

    pub fn length(&self) -> f64 {
        (0..self.segment_count())
            .map(|s| self.segment_length(s, 16))
            .sum()
    }

    /// `count` points equally spaced by arc length (closed: around the loop, start not repeated).
    ///
    /// # Panics
    /// If `count < 2`.
    pub fn sample_by_length(&self, count: usize) -> Vec<DVec3> {
        assert!(count >= 2, "sample_by_length needs at least 2 samples");
        const PER_SEG: usize = 64;
        // (global parameter, cumulative length)
        let mut table = vec![(0.0, 0.0)];
        let mut total = 0.0;
        for seg in 0..self.segment_count() {
            for k in 0..PER_SEG {
                let (a, b) = (k as f64 / PER_SEG as f64, (k + 1) as f64 / PER_SEG as f64);
                total += self.length_between(seg, a, b);
                table.push((seg as f64 + b, total));
            }
        }
        let divisions = if self.closed { count } else { count - 1 };
        (0..count)
            .map(|k| {
                let target = total * k as f64 / divisions as f64;
                let hi = table
                    .partition_point(|(_, len)| *len < target)
                    .clamp(1, table.len() - 1);
                let ((u0, l0), (u1, l1)) = (table[hi - 1], table[hi]);
                let u = if l1 > l0 {
                    u0 + (u1 - u0) * (target - l0) / (l1 - l0)
                } else {
                    u0
                };
                let seg = (u.floor() as usize).min(self.segment_count() - 1);
                self.eval(seg, u - seg as f64)
            })
            .collect()
    }
}
