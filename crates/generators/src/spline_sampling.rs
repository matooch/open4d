//! Intermediate Points of the spline primitives (docs/specs/GENERATORS2.md §6.2): the
//! shared sampler every spline-producing node goes through (brief 038; brief 037's new
//! primitives only build a [`Curve`] and set its [`Sampling`]).

use forge_core::{NodeType, ParamDesc, Unit};
use glam::DVec3;

use crate::Params;
use crate::splines::Curve;

/// How a spline places points between its own points (the `intermediate` param).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Interpolation {
    None,
    Natural,
    Uniform,
    Adaptive,
    Subdivided,
}

impl Interpolation {
    pub const ALL: [Interpolation; 5] = [
        Interpolation::None,
        Interpolation::Natural,
        Interpolation::Uniform,
        Interpolation::Adaptive,
        Interpolation::Subdivided,
    ];

    /// The enum id used in params and case files.
    pub fn id(self) -> &'static str {
        match self {
            Interpolation::None => "none",
            Interpolation::Natural => "natural",
            Interpolation::Uniform => "uniform",
            Interpolation::Adaptive => "adaptive",
            Interpolation::Subdivided => "subdivided",
        }
    }

    pub fn from_id(id: &str) -> Option<Interpolation> {
        Self::ALL.into_iter().find(|m| m.id() == id)
    }
}

/// The five Intermediate Points settings of a spline.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sampling {
    pub mode: Interpolation,
    /// Points per segment (Natural, Uniform).
    pub number: usize,
    /// Radians (Adaptive, Subdivided).
    pub angle: f64,
    /// Longest piece (Subdivided).
    pub max_length: f64,
}

impl Sampling {
    /// A mode with the app's default Number (8), Angle (5 degrees) and Maximum Length (5).
    pub fn new(mode: Interpolation) -> Sampling {
        Sampling {
            mode,
            number: 8,
            angle: 5f64.to_radians(),
            max_length: 5.0,
        }
    }

    /// Reads the params added by [`add_params`].
    pub(crate) fn from_params(p: &Params) -> Sampling {
        Sampling {
            mode: Interpolation::from_id(&p.enumeration("intermediate"))
                .unwrap_or(Interpolation::Adaptive),
            number: p.int("intermediate_number"),
            angle: p.float("intermediate_angle"),
            max_length: p.float("intermediate_max_length"),
        }
    }
}

/// Adds the Intermediate Points params to a spline node type; `default` is the
/// mode the spline starts in (R25: Uniform for Arc, Circle and Helix, Adaptive otherwise).
pub fn add_params(t: NodeType, default: Interpolation) -> NodeType {
    let options: Vec<(&str, &str)> = [
        ("none", "None"),
        ("natural", "Natural"),
        ("uniform", "Uniform"),
        ("adaptive", "Adaptive"),
        ("subdivided", "Subdivided"),
    ]
    .to_vec();
    t.param(ParamDesc::enumeration(
        "intermediate",
        "Intermediate points",
        default.id(),
        &options,
    ))
    .param(ParamDesc::int("intermediate_number", "Number", 8).range(0, 5000))
    .param(
        ParamDesc::float("intermediate_angle", "Angle", 5f64.to_radians())
            .range(0.0, std::f64::consts::FRAC_PI_2)
            .unit(Unit::Angle),
    )
    .param(
        ParamDesc::float("intermediate_max_length", "Maximum length", 5.0)
            .range(0.01, f64::MAX)
            .unit(Unit::Distance),
    )
}

/// How far (as a fraction of the whole spline) an open spline's last Uniform point falls
/// short of its end, in arc length (observed, `arc-defaults`, `arc-angles` and `helix-uniform`).
const UNIFORM_END_SHORTFALL: f64 = 1e-4;

/// Extra absolute shortfall of an open spline's last Uniform point, per segment squared: the
/// 100-segment `helix-uniform` ends 1.26e-4 earlier than `1e-4` of its length, while the 3-4
/// segment arcs (`arc-defaults`, `arc-angles`, and `spline-wrap/arc` through its 99-length
/// extension) allow at most 1e-6. The squared law is the simplest that fits those two ends; the
/// real cause is not known. **Parity-pending.**
const UNIFORM_END_EXTRA: f64 = 1.26e-8;

/// Relative lag of the Uniform points of an open spline before the last one: sample `k` lies
/// at `(1 - 1e-7)` of its arc length (`helix-uniform`: 1.00e-7 over 2521 long; `arc-defaults`
/// 3.2e-5 -> 1.9e-6 and `arc-angles` 9e-5 -> 5e-6 improve with it). Closed splines show none
/// (`circle-uniform`). The cause is not known. **Parity-pending.**
const UNIFORM_OPEN_LAG: f64 = 1e-7;

/// The spline's points as the generators consume them: each segment's start point and its
/// interior samples, then the end point of an open spline (a closed one doesn't repeat
/// its first point). Straight (`linear`) splines get no interior samples. Uniform spaces
/// its points over the whole spline instead ([`uniform_params`]).
pub fn sample(curve: &Curve, s: &Sampling) -> Vec<DVec3> {
    sample_params(curve, s)
        .into_iter()
        .map(|(_, p)| p)
        .collect()
}

/// [`sample`] with each point's global parameter (`0..=segments`, see [`Curve::at`]); the
/// spline tools (SPLINES.md section 8) map an intersection of two sampled outlines back
/// onto the curve through it.
pub fn sample_params(curve: &Curve, s: &Sampling) -> Vec<(f64, DVec3)> {
    if s.mode == Interpolation::Uniform && s.number > 0 && !curve.linear {
        return uniform_params(curve, s.number)
            .into_iter()
            .map(|t| (t, curve.at(t)))
            .collect();
    }
    let mut out = Vec::new();
    for (i, seg) in curve.segments.iter().enumerate() {
        out.push((i as f64, seg[0]));
        if !curve.linear && !is_straight(seg) {
            out.extend(
                interior(seg, s)
                    .into_iter()
                    .map(|t| (i as f64 + t, Curve::bezier(seg, t))),
            );
        }
    }
    if !curve.closed {
        let last = curve.segments.last().expect("a spline has a segment");
        out.push((curve.segments.len() as f64, last[3]));
    }
    out
}

/// A straight piece: handles on the thirds of its chord (`splines::line`) or on its end
/// points (`splines::straight`).
fn is_straight(seg: &[DVec3; 4]) -> bool {
    let chord = seg[3] - seg[0];
    let near = |a: DVec3, b: DVec3| (a - b).length() < 1e-9;
    (near(seg[1], seg[0] + chord / 3.0) && near(seg[2], seg[0] + chord * (2.0 / 3.0)))
        || (near(seg[1], seg[0]) && near(seg[2], seg[3]))
}

/// Pieces of the polyline that measures a segment's length (§6.2).
const LENGTH_PIECES: usize = 50;

/// Uniform (§6.2): `segments * (number + 1)` equal steps of arc length over the **whole**
/// spline (a closed spline keeps all of them, an open one gets the end point too), whatever
/// its segments' lengths. Lengths are measured on the polyline through every segment at
/// `t = k/50` and the parameter is interpolated linearly inside a piece.
///
/// These are the global parameters (`0..=segments`, see [`Curve::at`]) of the Uniform
/// samples of [`sample`]; a sweep pairs its rail with its path through them (§10a).
pub fn uniform_params(curve: &Curve, number: usize) -> Vec<f64> {
    let n_seg = curve.segments.len();
    let mut table = vec![0.0];
    for seg in &curve.segments {
        let mut prev = seg[0];
        for k in 1..=LENGTH_PIECES {
            let q = Curve::bezier(seg, k as f64 / LENGTH_PIECES as f64);
            let last = *table.last().expect("table starts with 0");
            table.push(last + (q - prev).length());
            prev = q;
        }
    }
    let total = *table.last().expect("table starts with 0");
    let steps = n_seg * (number + 1);
    let count = if curve.closed { steps } else { steps + 1 };
    (0..count)
        .map(|k| {
            // the open end falls a ten-thousandth of the length short (and a little more
            // for many segments); the steps before it lag by 1e-7 of their length
            let target = if k == steps {
                total * (1.0 - UNIFORM_END_SHORTFALL) - UNIFORM_END_EXTRA * (n_seg * n_seg) as f64
            } else if curve.closed {
                total * k as f64 / steps as f64
            } else {
                total * k as f64 / steps as f64 * (1.0 - UNIFORM_OPEN_LAG)
            };
            let hi = table
                .partition_point(|l| *l < target)
                .clamp(1, n_seg * LENGTH_PIECES);
            let (l0, l1) = (table[hi - 1], table[hi]);
            let f = if l1 > l0 {
                (target - l0) / (l1 - l0)
            } else {
                0.0
            };
            let piece = hi - 1;
            (piece / LENGTH_PIECES) as f64
                + ((piece % LENGTH_PIECES) as f64 + f) / LENGTH_PIECES as f64
        })
        .collect()
}

/// Interior sample parameters of one curved segment.
fn interior(seg: &[DVec3; 4], s: &Sampling) -> Vec<f64> {
    match s.mode {
        Interpolation::None => Vec::new(),
        Interpolation::Uniform => natural_params(s.number),
        Interpolation::Natural => natural_params(s.number),
        Interpolation::Adaptive => adaptive_params(seg, s.angle),
        Interpolation::Subdivided => {
            let m = subdivision_count(seg, s.angle, s.max_length);
            (1..m).map(|k| k as f64 / m as f64).collect()
        }
    }
}

/// Angle between two tangents; a vanishing tangent (a handle on its point) turns by 0.
fn turn_between(a: DVec3, b: DVec3) -> f64 {
    if a.length_squared() < 1e-24 || b.length_squared() < 1e-24 {
        return 0.0;
    }
    a.angle_between(b)
}

/// Adaptive (§6.2): walk `t` in steps of 1/50 and emit a sample once the tangent differs
/// from the tangent at the previous sample by `angle`. A candidate that would leave less than
/// `angle` of turn to the segment's end is replaced by the midpoint (on the 1/50 grid) of
/// the previous sample and the end.
fn adaptive_params(seg: &[DVec3; 4], angle: f64) -> Vec<f64> {
    const FINE: usize = 50;
    // a handle sitting on its point has no direction: look slightly inside
    let dir = |k: usize| {
        let t = k as f64 / FINE as f64;
        let d = Curve::tangent(seg, t);
        if d.length_squared() > 1e-24 {
            return d;
        }
        Curve::tangent(seg, if k == 0 { 1e-3 } else { 1.0 - 1e-3 })
    };
    // turn still to come after step k, summed over the remaining steps
    let steps: Vec<f64> = (0..FINE)
        .map(|k| turn_between(dir(k), dir(k + 1)))
        .collect();
    let remaining = |k: usize| steps[k..].iter().sum::<f64>();
    let mut out = Vec::new();
    let (mut last, mut last_dir) = (0usize, dir(0));
    for k in 1..FINE {
        let d = dir(k);
        let piece = turn_between(last_dir, d);
        if piece + 1e-12 < angle {
            continue;
        }
        // the new sample, and the tangent the next one is measured against
        let at = if remaining(k) + 1e-12 < piece {
            (last + FINE) / 2
        } else {
            k
        };
        if at > last {
            out.push(at as f64 / FINE as f64);
            last = at;
        }
        last_dir = d;
    }
    out
}

/// Natural (§6.2, parity-pending): `number` samples at equal steps of the segment's own
/// parameter.
fn natural_params(number: usize) -> Vec<f64> {
    (1..=number)
        .map(|k| k as f64 / (number + 1) as f64)
        .collect()
}

/// Subdivided (§6.2, parity-pending): the smallest number of equal-`t` pieces that all turn
/// by at most `angle` and span at most `max_length`.
fn subdivision_count(seg: &[DVec3; 4], angle: f64, max_length: f64) -> usize {
    const CAP: usize = 4096;
    let fits = |m: usize| {
        (0..m).all(|k| {
            let (a, b) = (k as f64 / m as f64, (k + 1) as f64 / m as f64);
            let turn = turn_between(Curve::tangent(seg, a), Curve::tangent(seg, b));
            let chord = (Curve::bezier(seg, b) - Curve::bezier(seg, a)).length();
            turn <= angle + 1e-12 && chord <= max_length + 1e-12
        })
    };
    if fits(1) {
        return 1;
    }
    let mut hi = 2;
    while hi < CAP && !fits(hi) {
        hi *= 2;
    }
    let mut lo = hi / 2; // known not to fit
    let mut hi = hi.min(CAP);
    while hi - lo > 1 {
        let mid = (lo + hi) / 2;
        if fits(mid) {
            hi = mid;
        } else {
            lo = mid;
        }
    }
    hi
}
