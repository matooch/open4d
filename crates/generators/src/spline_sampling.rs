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
/// short of its end (observed, `arc-defaults` and `helix-uniform`).
const UNIFORM_END_SHORTFALL: f64 = 1e-4;

/// The spline's points as the generators consume them: each segment's start point and its
/// interior samples, then the end point of an open spline (a closed one doesn't repeat
/// its first point). Straight (`linear`) splines get no interior samples.
pub fn sample(curve: &Curve, s: &Sampling) -> Vec<DVec3> {
    let mut out = Vec::new();
    for seg in &curve.segments {
        out.push(seg[0]);
        if !curve.linear {
            out.extend(interior(seg, s).into_iter().map(|t| Curve::bezier(seg, t)));
        }
    }
    if !curve.closed {
        let last = curve.segments.last().expect("a spline has a segment");
        let shifted = s.mode == Interpolation::Uniform && s.number > 0 && !curve.linear;
        if shifted {
            let t = (1.0 - UNIFORM_END_SHORTFALL * curve.segments.len() as f64).max(0.0);
            out.push(Curve::bezier(last, t));
        } else {
            out.push(last[3]);
        }
    }
    out
}

/// Interior sample parameters of one curved segment.
fn interior(seg: &[DVec3; 4], s: &Sampling) -> Vec<f64> {
    match s.mode {
        Interpolation::None => Vec::new(),
        Interpolation::Uniform => uniform_params(seg, s.number),
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

/// Adaptive (§6.2): walk t in steps of 1/50, accumulating the tangent's turn, and emit a
/// sample once it reaches `angle`; the last interior sample is placed one step before the
/// end.
fn adaptive_params(seg: &[DVec3; 4], angle: f64) -> Vec<f64> {
    const FINE: usize = 50;
    let mut out = Vec::new();
    let (mut last, mut turned, mut prev) = (0.0, 0.0, Curve::tangent(seg, 0.0));
    for k in 1..FINE {
        let t = k as f64 / FINE as f64;
        let d = Curve::tangent(seg, t);
        turned += turn_between(prev, d);
        prev = d;
        if turned + 1e-12 >= angle {
            let step = t - last;
            if t + step > 1.0 + 1e-9 {
                out.push(1.0 - step);
                return out;
            }
            out.push(t);
            last = t;
            turned = 0.0;
        }
    }
    out
}

/// Uniform (§6.2): `number` samples at equal arc length inside the segment, so the
/// segment's two ends stay put and its `number + 1` pieces are equally long. The length is
/// that of the polyline through the segment at `t = k/50` and `t` is interpolated linearly
/// between those vertices (observed: `circle-uniform` matches to 5e-7 with 50, not with
/// 40 or 64 pieces or an exact length).
fn uniform_params(seg: &[DVec3; 4], number: usize) -> Vec<f64> {
    const PIECES: usize = 50;
    if number == 0 {
        return Vec::new();
    }
    let mut table = vec![0.0];
    let mut prev = seg[0];
    for k in 1..=PIECES {
        let q = Curve::bezier(seg, k as f64 / PIECES as f64);
        table.push(table[k - 1] + (q - prev).length());
        prev = q;
    }
    let total = table[PIECES];
    if total <= 0.0 {
        return natural_params(number);
    }
    (1..=number)
        .map(|k| {
            let target = total * k as f64 / (number + 1) as f64;
            let hi = table.partition_point(|l| *l < target).clamp(1, PIECES);
            let (l0, l1) = (table[hi - 1], table[hi]);
            let f = if l1 > l0 {
                (target - l0) / (l1 - l0)
            } else {
                0.0
            };
            ((hi - 1) as f64 + f) / PIECES as f64
        })
        .collect()
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
