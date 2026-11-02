//! The remaining spline primitives (docs/specs/GENERATORS2.md §6.3): Arc, n-Side, Star,
//! Flower, 4-Side, Cogwheel, Cycloid, Profile, Cissoid and Formula. Each builds a
//! [`Curve`]; the Intermediate Points settings go through the shared sampler
//! (`spline_sampling`, §6.2).

use std::f64::consts::{FRAC_PI_2, PI, TAU};

use forge_core::{Category, NodeType, ParamDesc, Unit};
use glam::DVec3;

use crate::spline_sampling::{Interpolation, Sampling};
use crate::splines::{Curve, arc_segment, auto_tangent_curve, common, distance, straight};
use crate::{BuildError, Params};

pub const ARC: &str = "forge.arc_spline";
pub const NSIDE: &str = "forge.nside_spline";
pub const STAR: &str = "forge.star_spline";
pub const FLOWER: &str = "forge.flower_spline";
pub const FOURSIDE: &str = "forge.fourside_spline";
pub const COGWHEEL: &str = "forge.cogwheel_spline";
pub const CYCLOID: &str = "forge.cycloid_spline";
pub const PROFILE: &str = "forge.profile_spline";
pub const CISSOID: &str = "forge.cissoid_spline";
pub const FORMULA: &str = "forge.formula_spline";

/// Every type this module builds.
pub(crate) const TYPES: [&str; 10] = [
    ARC, NSIDE, STAR, FLOWER, FOURSIDE, COGWHEEL, CYCLOID, PROFILE, CISSOID, FORMULA,
];

fn radians(id: &str, label: &str, degrees: f64) -> ParamDesc {
    ParamDesc::float(id, label, degrees.to_radians()).unit(Unit::Angle)
}

fn percent(id: &str, label: &str, default: f64, min: f64, max: f64) -> ParamDesc {
    ParamDesc::float(id, label, default)
        .range(min, max)
        .unit(Unit::Percent)
}

fn count(id: &str, label: &str, default: i64, min: i64) -> ParamDesc {
    ParamDesc::int(id, label, default).range(min, 999_999_999)
}

pub(crate) fn node_type(type_id: &str) -> Option<NodeType> {
    let node = |id: &str, label: &str| NodeType::new(id, label, Category::Generator);
    let adaptive = Interpolation::Adaptive;
    Some(match type_id {
        ARC => common(
            node(ARC, "Arc")
                .param(ParamDesc::enumeration(
                    "arc_type",
                    "Type",
                    "arc",
                    &[
                        ("arc", "Arc"),
                        ("sector", "Sector"),
                        ("segment", "Segment"),
                        ("ring", "Ring"),
                    ],
                ))
                .param(distance("radius", "Radius", 200.0))
                .param(distance("inner_radius", "Inner radius", 100.0))
                .param(radians("start_angle", "Start angle", 0.0).range(-TAU, TAU))
                .param(radians("end_angle", "End angle", 90.0).range(-TAU, TAU)),
            Interpolation::Uniform,
        ),
        NSIDE => common(
            node(NSIDE, "n-Side")
                .param(distance("radius", "Radius", 200.0))
                .param(count("sides", "Sides", 6, 2))
                .param(ParamDesc::bool("rounding", "Rounding", false))
                .param(distance("rounding_radius", "Radius", 50.0)),
            adaptive,
        ),
        STAR => common(
            node(STAR, "Star")
                .param(distance("inner_radius", "Inner radius", 100.0))
                .param(distance("outer_radius", "Outer radius", 200.0))
                .param(percent("twist", "Twist", 0.0, -100.0, 100.0))
                .param(ParamDesc::int("points", "Points", 8).range(3, 1_000_000)),
            adaptive,
        ),
        FLOWER => common(
            node(FLOWER, "Flower")
                .param(distance("inner_radius", "Inner radius", 100.0))
                .param(distance("outer_radius", "Outer radius", 200.0))
                .param(count("petals", "Petals", 8, 3)),
            adaptive,
        ),
        FOURSIDE => common(
            node(FOURSIDE, "4-Side")
                .param(ParamDesc::enumeration(
                    "fourside_type",
                    "Type",
                    "diamond",
                    &[
                        ("diamond", "Diamond"),
                        ("kite", "Kite"),
                        ("parallelogram", "Parallelogram"),
                        ("trapezium", "Trapezium"),
                    ],
                ))
                .param(distance("a", "a", 200.0))
                .param(distance("b", "b", 100.0))
                .param(radians("angle", "Angle", 30.0).range(-FRAC_PI_2, FRAC_PI_2)),
            adaptive,
        ),
        COGWHEEL => common(cogwheel_params(node(COGWHEEL, "Cogwheel")), adaptive),
        CYCLOID => common(
            node(CYCLOID, "Cycloid")
                .param(ParamDesc::enumeration(
                    "cycloid_type",
                    "Type",
                    "cycloid",
                    &[
                        ("cycloid", "Cycloid"),
                        ("epicycloid", "Epicycloid"),
                        ("hypocycloid", "Hypocycloid"),
                    ],
                ))
                .param(distance("radius", "Radius", 25.0))
                .param(distance("r", "r", 50.0))
                .param(distance("a", "a", 75.0))
                .param(radians("start_angle", "Start angle", 0.0))
                .param(radians("end_angle", "End angle", 360.0)),
            adaptive,
        ),
        PROFILE => common(
            node(PROFILE, "Profile")
                .param(ParamDesc::enumeration(
                    "profile_type",
                    "Type",
                    "h_shape",
                    &[
                        ("h_shape", "H Shape"),
                        ("l_shape", "L Shape"),
                        ("t_shape", "T Shape"),
                        ("u_shape", "U Shape"),
                        ("z_shape", "Z Shape"),
                    ],
                ))
                .param(distance("height", "Height", 200.0))
                .param(distance("b", "b", 100.0))
                .param(distance("s", "s", 20.0))
                .param(distance("t", "t", 20.0)),
            adaptive,
        ),
        CISSOID => common(
            node(CISSOID, "Cissoid")
                .param(ParamDesc::enumeration(
                    "cissoid_type",
                    "Type",
                    "cissoid",
                    &[
                        ("cissoid", "Cissoid"),
                        ("lemniscate", "Lemniscate"),
                        ("strophoid", "Strophoid"),
                    ],
                ))
                .param(distance("width", "Width", 400.0))
                .param(ParamDesc::float("tension", "Tension", 2.0).range(1.5, 100.0)),
            adaptive,
        ),
        FORMULA => common(
            node(FORMULA, "Formula")
                .param(ParamDesc::string("x_t", "X(t)", "100.0*t"))
                .param(ParamDesc::string("y_t", "Y(t)", "100.0*Sin(t*PI)"))
                .param(ParamDesc::string("z_t", "Z(t)", "0.0"))
                .param(ParamDesc::float("t_min", "Tmin", -1.0))
                .param(ParamDesc::float("t_max", "Tmax", 1.0))
                .param(count("samples", "Samples", 20, 2))
                .param(ParamDesc::bool(
                    "cubic_interpolation",
                    "Cubic interpolation",
                    false,
                )),
            adaptive,
        ),
        _ => return None,
    })
}

fn cogwheel_params(t: NodeType) -> NodeType {
    let ratio =
        |id: &str, label: &str, default: f64, max: f64| percent(id, label, default, 0.0, max);
    t.param(ParamDesc::bool("legacy_mode", "Legacy mode", false))
        .param(ParamDesc::bool("show_guides", "Show guides", false))
        .param(ParamDesc::color(
            "guide_color",
            "Guide color",
            [1.0, 0.913725, 0.498039],
        ))
        .param(ParamDesc::enumeration(
            "teeth_type",
            "Type",
            "involute",
            &[
                ("none", "None"),
                ("involute", "Involute"),
                ("ratchet", "Ratchet"),
                ("flat", "Flat"),
            ],
        ))
        .param(ParamDesc::int("teeth", "Teeth", 20).range(5, 2000))
        .param(ParamDesc::bool("lock_radius", "Lock radius", false))
        .param(radians("teeth_orientation", "Orientation", 0.0))
        .param(ParamDesc::bool("undercut", "Undercut", false))
        .param(distance("root_radius", "Root radius", 180.0))
        .param(distance("addendum_radius", "Addendum radius", 220.0))
        .param(distance("pitch_radius", "Pitch radius", 200.0))
        .param(distance("module", "Module", 20.0))
        .param(distance("diametral_pitch", "Diametral pitch", 0.05))
        .param(distance("dedendum", "Dedendum", 20.0))
        .param(radians("pressure_angle", "Pressure angle", 20.0).range(0.0, 2.0 * PI))
        .param(ParamDesc::float("radial_ratio", "Radial ratio", 1.5).range(0.0, 20.0))
        .param(ratio("witness_ratio", "Witness ratio", 10.0, 100.0))
        .param(ratio("undercut_ratio", "Undercut ratio", 10.0, 100.0))
        .param(ParamDesc::enumeration(
            "inner_type",
            "Type",
            "none",
            &[
                ("none", "None"),
                ("spokes", "Spokes"),
                ("holes", "Holes"),
                ("arches", "Arches"),
                ("waves", "Waves"),
            ],
        ))
        .param(radians("inner_orientation", "Orientation", 0.0))
        .param(ParamDesc::bool("invert", "Invert", false))
        .param(ParamDesc::bool("center_hole", "Center hole", true))
        .param(distance("hole_radius", "Radius", 15.0))
        .param(ParamDesc::bool("cutout", "Cutout", false))
        .param(ParamDesc::float("cutout_depth", "Depth", 10.0).unit(Unit::Distance))
        .param(distance("cutout_width", "Width", 10.0))
        .param(radians("cutout_orientation", "Cutout orientation", 0.0))
}

/// Builds the curve of one of [`TYPES`] in its XY frame.
pub(crate) fn build(type_id: &str, p: &Params, sampling: Sampling) -> Result<Curve, BuildError> {
    match type_id {
        ARC => arc(p, sampling),
        NSIDE => nside(p, sampling),
        STAR => Ok(star(p, sampling)),
        FLOWER => Ok(flower(p, sampling)),
        FOURSIDE => fourside(p, sampling),
        CYCLOID => cycloid(p, sampling),
        PROFILE => profile(p, sampling),
        CISSOID => cissoid(p, sampling),
        COGWHEEL => Err(BuildError::Unsupported("cogwheel")),
        FORMULA => Err(BuildError::Unsupported(
            "formula spline: the expression library (brief 052) does not exist yet",
        )),
        other => Err(BuildError::UnknownType(other.to_string())),
    }
}

fn polar(radius: f64, angle: f64) -> DVec3 {
    DVec3::new(radius * angle.cos(), radius * angle.sin(), 0.0)
}

/// Arc (§6.3): four equal Bezier pieces from the start to the end angle; `sector` closes
/// them with the chord.
fn arc(p: &Params, sampling: Sampling) -> Result<Curve, BuildError> {
    let closed = match p.enumeration("arc_type").as_str() {
        "arc" => false,
        "sector" => true,
        _ => return Err(BuildError::Unsupported("arc type segment/ring")),
    };
    let (r, a0, a1) = (
        p.float("radius"),
        p.float("start_angle"),
        p.float("end_angle"),
    );
    let turn = (a1 - a0) / 4.0;
    let center = DVec3::ZERO;
    let mut segments: Vec<[DVec3; 4]> = (0..4)
        .map(|k| arc_segment(center, r, a0 + turn * k as f64, turn))
        .collect();
    if closed {
        segments.push(straight(polar(r, a1), polar(r, a0)));
    }
    Ok(Curve::new(segments, false, closed, sampling))
}

/// n-Side (§6.3): a regular polygon, optionally with filleted corners.
fn nside(p: &Params, sampling: Sampling) -> Result<Curve, BuildError> {
    let (r, n) = (p.float("radius"), p.int("sides"));
    let step = TAU / n as f64;
    let corner = |k: usize| polar(r, step * k as f64);
    if !p.bool("rounding") {
        let pts: Vec<DVec3> = (0..n).map(corner).collect();
        return Ok(Curve::polyline(&pts, true, sampling));
    }
    if n < 3 {
        return Err(BuildError::Unsupported("rounded 2-sided n-side"));
    }
    // Fillet circle of radius `rf` tangent to both edges of a corner: the arc starts
    // `rf * tan(pi / n)` before the corner (at most half an edge).
    let edge = (corner(1) - corner(0)).length();
    let rf = p
        .float("rounding_radius")
        .min(edge / 2.0 / (PI / n as f64).tan());
    let reach = rf * (PI / n as f64).tan();
    let mut segments = Vec::new();
    let mut ends = Vec::new();
    for k in 0..n {
        let v = corner(k);
        let from_prev = (corner((k + n - 1) % n) - v).normalize();
        let to_next = (corner((k + 1) % n) - v).normalize();
        let a = v + from_prev * reach;
        // the fillet circle's centre lies on the bisector
        let bisector = (from_prev + to_next).normalize();
        let center = v + bisector * (rf / (PI / n as f64).cos());
        let a0 = (a - center).y.atan2((a - center).x);
        let seg = arc_segment(center, rf, a0, step);
        ends.push((seg[0], seg[3]));
        segments.push(seg);
    }
    let mut out = Vec::new();
    for k in 0..n {
        out.push(segments[k]);
        out.push(straight(ends[k].1, ends[(k + 1) % n].0));
    }
    Ok(Curve::new(out, false, true, sampling))
}

/// Star (§6.3): alternating outer and inner points; `twist` turns the inner points by that
/// fraction of the point step.
fn star(p: &Params, sampling: Sampling) -> Curve {
    let n = p.int("points");
    let step = TAU / n as f64;
    let twist = p.float("twist") / 100.0;
    let mut pts = Vec::with_capacity(2 * n);
    for k in 0..n {
        pts.push(polar(p.float("outer_radius"), step * k as f64));
        pts.push(polar(
            p.float("inner_radius"),
            step * (k as f64 + 0.5 + twist),
        ));
    }
    Curve::polyline(&pts, true, sampling)
}

/// Flower (§6.3, parity-pending beyond the defaults): per petal an inner point and two lobe
/// points, joined by the automatic tangents of [`auto_tangent_curve`].
fn flower(p: &Params, sampling: Sampling) -> Curve {
    let n = p.int("petals");
    let (ri, ro) = (p.float("inner_radius"), p.float("outer_radius"));
    let step = TAU / n as f64;
    let mut pts = Vec::with_capacity(3 * n);
    for k in 0..n {
        let a = step * k as f64;
        let inner = polar(ri, a);
        let lobe = inner + polar(ro - ri, a + step / 2.0);
        // the second lobe point is the first one reflected across the petal axis
        let (c, s) = ((2.0 * a + step).cos(), (2.0 * a + step).sin());
        let mirror = DVec3::new(c * lobe.x + s * lobe.y, s * lobe.x - c * lobe.y, 0.0);
        pts.extend([inner, lobe, mirror]);
    }
    auto_tangent_curve(&pts, true, sampling)
}

/// 4-Side (§6.3): Diamond and Kite (observed); the other two types are not.
fn fourside(p: &Params, sampling: Sampling) -> Result<Curve, BuildError> {
    let (a, b) = (p.float("a"), p.float("b"));
    let pts = match p.enumeration("fourside_type").as_str() {
        "diamond" => [
            DVec3::new(a / 2.0, 0.0, 0.0),
            DVec3::new(0.0, b / 2.0, 0.0),
            DVec3::new(-a / 2.0, 0.0, 0.0),
            DVec3::new(0.0, -b / 2.0, 0.0),
        ],
        "kite" => {
            let d = a.hypot(b);
            let (x, top, bottom) = if d > 0.0 {
                (a * b / d, b * b / d, a * a / d)
            } else {
                (0.0, 0.0, 0.0)
            };
            [
                DVec3::new(x, 0.0, 0.0),
                DVec3::new(0.0, top, 0.0),
                DVec3::new(-x, 0.0, 0.0),
                DVec3::new(0.0, -bottom, 0.0),
            ]
        }
        _ => return Err(BuildError::Unsupported("4-side parallelogram/trapezium")),
    };
    Ok(Curve::polyline(&pts, true, sampling))
}

/// Profile (§6.3): the H shape (observed), in the +x/+y quadrant.
fn profile(p: &Params, sampling: Sampling) -> Result<Curve, BuildError> {
    if p.enumeration("profile_type") != "h_shape" {
        return Err(BuildError::Unsupported("profile shapes other than H"));
    }
    let (h, b, s, t) = (p.float("height"), p.float("b"), p.float("s"), p.float("t"));
    let (w0, w1) = ((b - s) / 2.0, (b + s) / 2.0);
    let v = |x: f64, y: f64| DVec3::new(x, y, 0.0);
    let pts = [
        v(0.0, 0.0),
        v(b, 0.0),
        v(b, t),
        v(w1, t),
        v(w1, h - t),
        v(b, h - t),
        v(b, h),
        v(0.0, h),
        v(0.0, h - t),
        v(w0, h - t),
        v(w0, t),
        v(0.0, t),
    ];
    Ok(Curve::polyline(&pts, true, sampling))
}

/// Cissoid (§6.3): an open polyline of 101 points, `x = t y`.
fn cissoid(p: &Params, sampling: Sampling) -> Result<Curve, BuildError> {
    if p.enumeration("cissoid_type") != "cissoid" {
        return Err(BuildError::Unsupported("lemniscate/strophoid"));
    }
    let (w, tension) = (p.float("width"), p.float("tension"));
    let scale = w / 2.0 * (1.0 + tension * tension) / (tension * tension);
    let pts: Vec<DVec3> = (0..=100)
        .map(|k| {
            let t = -1.0 + 0.02 * k as f64;
            let u2 = tension * tension * t * t;
            let y = scale * u2 / (1.0 + u2);
            DVec3::new(t * y, y, 0.0)
        })
        .collect();
    Ok(Curve::polyline(&pts, false, sampling))
}

/// Cycloid (§6.3): `x = R t - a sin t`, `y = R - a cos t`; Epicycloid `x = (R + r) cos t -
/// a cos((R + r) t / r)`, `y = (R + r) sin t - a sin((R + r) t / r)`. 62 pieces per full
/// turn, joined by the automatic tangents of [`auto_tangent_curve`].
fn cycloid(p: &Params, sampling: Sampling) -> Result<Curve, BuildError> {
    let (big, r, a) = (p.float("radius"), p.float("r"), p.float("a"));
    let (t0, t1) = (p.float("start_angle"), p.float("end_angle"));
    let point: Box<dyn Fn(f64) -> DVec3> = match p.enumeration("cycloid_type").as_str() {
        "cycloid" => Box::new(move |t| DVec3::new(big * t - a * t.sin(), big - a * t.cos(), 0.0)),
        "epicycloid" => {
            let k = (big + r) / r;
            Box::new(move |t| {
                DVec3::new(
                    (big + r) * t.cos() - a * (k * t).cos(),
                    (big + r) * t.sin() - a * (k * t).sin(),
                    0.0,
                )
            })
        }
        _ => return Err(BuildError::Unsupported("hypocycloid")),
    };
    let pieces = ((((t1 - t0) / TAU) * 62.0).round().abs() as usize).max(1);
    let pts: Vec<DVec3> = (0..=pieces)
        .map(|i| point(t0 + (t1 - t0) * i as f64 / pieces as f64))
        .collect();
    Ok(auto_tangent_curve(&pts, false, sampling))
}
