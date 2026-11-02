//! Spline primitives: circle, rectangle, helix (docs/specs/GENERATORS2.md §6).
//! Splines do not export (docs/parity/splines.md); their geometry is derived from the
//! spline-input generators' golden cases.

use std::collections::BTreeMap;

use forge_core::{Category, NodeId, NodeType, ParamDesc, Scene, Unit, Value};
use glam::DVec3;

use crate::spline_sampling::{Interpolation, Sampling, add_params, sample};
use crate::{BuildError, Params};

pub const CIRCLE: &str = "forge.circle_spline";
pub const RECTANGLE: &str = "forge.rectangle_spline";
pub const HELIX: &str = "forge.helix_spline";

/// Circle quarter handle length (observed: 0.5533333 at every sample of
/// `extrude/defaults`, not the textbook 0.5523).
pub const CIRCLE_KAPPA: f64 = 83.0 / 150.0;

/// A spline as cubic Bezier segments `[p0, c0, c1, p1]` (lines have `linear` set) with its
/// Intermediate Points settings. Brief 037's primitives build one of these and nothing else.
#[derive(Debug, Clone, PartialEq)]
pub struct Curve {
    pub segments: Vec<[DVec3; 4]>,
    pub linear: bool,
    pub closed: bool,
    pub sampling: Sampling,
}

impl Curve {
    pub(crate) fn bezier(seg: &[DVec3; 4], t: f64) -> DVec3 {
        let u = 1.0 - t;
        seg[0] * (u * u * u)
            + seg[1] * (3.0 * u * u * t)
            + seg[2] * (3.0 * u * t * t)
            + seg[3] * (t * t * t)
    }

    pub(crate) fn tangent(seg: &[DVec3; 4], t: f64) -> DVec3 {
        let u = 1.0 - t;
        (seg[1] - seg[0]) * (3.0 * u * u)
            + (seg[2] - seg[1]) * (6.0 * u * t)
            + (seg[3] - seg[2]) * (3.0 * t * t)
    }

    /// Point at global parameter `s` in `[0, segments]`.
    pub fn at(&self, s: f64) -> DVec3 {
        let n = self.segments.len();
        let i = (s.floor() as usize).min(n - 1);
        Self::bezier(&self.segments[i], s - i as f64)
    }

    /// The spline's points with its own Intermediate Points settings applied
    /// ([`spline_sampling::sample`]).
    pub fn points(&self) -> Vec<DVec3> {
        sample(self, &self.sampling)
    }

    pub fn transformed(&self, m: &glam::DMat4) -> Curve {
        Curve {
            segments: self
                .segments
                .iter()
                .map(|s| s.map(|p| m.transform_point3(p)))
                .collect(),
            ..self.clone()
        }
    }
}

fn plane_param() -> ParamDesc {
    ParamDesc::enumeration(
        "plane",
        "Plane",
        "xy",
        &[("xy", "XY"), ("zy", "ZY"), ("xz", "XZ")],
    )
}

fn common(t: NodeType, default: Interpolation) -> NodeType {
    add_params(
        t.param(plane_param())
            .param(ParamDesc::bool("reverse", "Reverse", false)),
        default,
    )
}

fn distance(id: &str, label: &str, default: f64) -> ParamDesc {
    ParamDesc::float(id, label, default)
        .range(0.0, f64::MAX)
        .unit(Unit::Distance)
}

pub(crate) fn circle_type() -> NodeType {
    common(
        NodeType::new(CIRCLE, "Circle", Category::Generator)
            .param(ParamDesc::bool("ellipse", "Ellipse", false))
            .param(ParamDesc::bool("ring", "Ring", false))
            .param(distance("radius", "Radius", 200.0))
            .param(distance("radius_y", "Radius Y", 200.0))
            .param(distance("inner_radius", "Inner radius", 100.0)),
        Interpolation::Uniform,
    )
}

pub(crate) fn rectangle_type() -> NodeType {
    common(
        NodeType::new(RECTANGLE, "Rectangle", Category::Generator)
            .param(distance("width", "Width", 400.0))
            .param(distance("height", "Height", 400.0))
            .param(ParamDesc::bool("rounding", "Rounding", false))
            .param(distance("rounding_radius", "Radius", 50.0)),
        Interpolation::Adaptive,
    )
}

pub(crate) fn helix_type() -> NodeType {
    common(
        NodeType::new(HELIX, "Helix", Category::Generator)
            .param(distance("start_radius", "Start radius", 200.0))
            .param(ParamDesc::float("start_angle", "Start angle", 0.0).unit(Unit::Angle))
            .param(distance("end_radius", "End radius", 200.0))
            .param(
                ParamDesc::float("end_angle", "End angle", 720f64.to_radians()).unit(Unit::Angle),
            )
            .param(
                ParamDesc::float("radial_bias", "Radial bias", 50.0)
                    .range(0.0, 100.0)
                    .unit(Unit::Percent),
            )
            .param(
                ParamDesc::float("height", "Height", 200.0)
                    .range(0.0, f64::MAX)
                    .unit(Unit::Distance),
            )
            .param(
                ParamDesc::float("height_bias", "Height bias", 50.0)
                    .range(0.0, 100.0)
                    .unit(Unit::Percent),
            )
            .param(ParamDesc::int("subdivision", "Subdivision", 100).range(3, 100_000)),
        Interpolation::Uniform,
    )
}

pub(crate) fn is_spline(type_id: &str) -> bool {
    matches!(type_id, CIRCLE | RECTANGLE | HELIX)
}

/// Maps the spline's XY frame to its plane (§6.1).
fn on_plane(plane: &str, p: DVec3) -> DVec3 {
    match plane {
        "xz" => DVec3::new(p.x, -p.z, p.y),
        "zy" => DVec3::new(-p.z, p.y, p.x),
        _ => p,
    }
}

fn line(a: DVec3, b: DVec3) -> [DVec3; 4] {
    [a, a.lerp(b, 1.0 / 3.0), a.lerp(b, 2.0 / 3.0), b]
}

/// The spline of a spline-primitive node, in its local space.
pub fn build_curve(type_id: &str, values: &BTreeMap<String, Value>) -> Result<Curve, BuildError> {
    let ty = match type_id {
        CIRCLE => circle_type(),
        RECTANGLE => rectangle_type(),
        HELIX => helix_type(),
        _ => return Err(BuildError::UnknownType(type_id.to_string())),
    };
    let p = Params { ty: &ty, values };
    let sampling = Sampling::from_params(&p);
    if p.bool("reverse") {
        return Err(BuildError::Unsupported("reversed spline"));
    }
    let mut curve = match type_id {
        CIRCLE => {
            if p.bool("ellipse") || p.bool("ring") {
                return Err(BuildError::Unsupported("circle ellipse/ring"));
            }
            let r = p.float("radius");
            let k = CIRCLE_KAPPA * r;
            // quarters counter-clockwise from (r, 0)
            let q = |a: DVec3, b: DVec3, ta: DVec3, tb: DVec3| [a, a + ta * k, b - tb * k, b];
            let (e, n, w, s) = (
                DVec3::new(r, 0.0, 0.0),
                DVec3::new(0.0, r, 0.0),
                DVec3::new(-r, 0.0, 0.0),
                DVec3::new(0.0, -r, 0.0),
            );
            Curve {
                segments: vec![
                    q(e, n, DVec3::Y, -DVec3::X),
                    q(n, w, -DVec3::X, -DVec3::Y),
                    q(w, s, -DVec3::Y, DVec3::X),
                    q(s, e, DVec3::X, DVec3::Y),
                ],
                linear: false,
                closed: true,
                sampling,
            }
        }
        RECTANGLE => {
            if p.bool("rounding") {
                return Err(BuildError::Unsupported("rounded rectangle"));
            }
            let (w, h) = (p.float("width") / 2.0, p.float("height") / 2.0);
            let c = [
                DVec3::new(w, h, 0.0),
                DVec3::new(-w, h, 0.0),
                DVec3::new(-w, -h, 0.0),
                DVec3::new(w, -h, 0.0),
            ];
            Curve {
                segments: (0..4).map(|i| line(c[i], c[(i + 1) % 4])).collect(),
                linear: true,
                closed: true,
                sampling,
            }
        }
        _ => {
            if (p.float("radial_bias") - 50.0).abs() > 1e-9
                || (p.float("height_bias") - 50.0).abs() > 1e-9
            {
                return Err(BuildError::Unsupported("helix bias"));
            }
            let shape = HelixShape {
                start_angle: p.float("start_angle"),
                end_angle: p.float("end_angle"),
                start_radius: p.float("start_radius"),
                end_radius: p.float("end_radius"),
                height: p.float("height"),
            };
            if sampling.mode == Interpolation::Adaptive {
                shape.adaptive_polyline(p.int("subdivision") as f64, sampling)
            } else {
                shape.bezier(p.int("subdivision"), sampling)
            }
        }
    };
    let plane = p.enumeration("plane");
    for seg in &mut curve.segments {
        *seg = seg.map(|q| on_plane(&plane, q));
    }
    Ok(curve)
}

/// The helix curve `f(s) = (r cos a, r sin a, -height * s)`, `s` in `[0, 1]`, with the angle
/// and the radius linear in `s` (docs/specs/GENERATORS2.md §6.1).
struct HelixShape {
    start_angle: f64,
    end_angle: f64,
    start_radius: f64,
    end_radius: f64,
    height: f64,
}

impl HelixShape {
    fn at(&self, s: f64) -> DVec3 {
        let a = self.start_angle + (self.end_angle - self.start_angle) * s;
        let r = self.start_radius + (self.end_radius - self.start_radius) * s;
        DVec3::new(r * a.cos(), r * a.sin(), -self.height * s)
    }

    /// Speed `|f'(s)|`.
    fn speed(&self, s: f64) -> f64 {
        let da = self.end_angle - self.start_angle;
        let r = self.start_radius + (self.end_radius - self.start_radius) * s;
        let dr = self.end_radius - self.start_radius;
        (dr * dr + r * r * da * da + self.height * self.height).sqrt()
    }

    /// Length of the piece `[s0, s1]` (Simpson, exact for a constant radius).
    fn length(&self, s0: f64, s1: f64) -> f64 {
        const N: usize = 16;
        let h = (s1 - s0) / N as f64;
        let mut sum = self.speed(s0) + self.speed(s1);
        for k in 1..N {
            let w = if k % 2 == 1 { 4.0 } else { 2.0 };
            sum += w * self.speed(s0 + h * k as f64);
        }
        sum * h / 3.0
    }

    /// R20 form (`sweep/helix`, Adaptive): lines through `subdivision` points per turn, the
    /// points next to both ends skipped.
    fn adaptive_polyline(&self, subdivision: f64, sampling: Sampling) -> Curve {
        let turns = (self.end_angle - self.start_angle).abs() / std::f64::consts::TAU;
        let n = ((subdivision * turns).round() as usize).max(1);
        let keep: Vec<usize> = (0..=n)
            .filter(|&i| n < 4 || (i != 1 && i != n - 1))
            .collect();
        Curve {
            segments: keep
                .windows(2)
                .map(|w| {
                    line(
                        self.at(w[0] as f64 / n as f64),
                        self.at(w[1] as f64 / n as f64),
                    )
                })
                .collect(),
            linear: true,
            closed: false,
            sampling,
        }
    }

    /// R25 form (`splines-2/helix-uniform`): `n + 1` points at `s = i/n` with tangent handles
    /// along `f(s + 1/n) - f(s - 1/n)`, each a third of its piece's length long.
    fn bezier(&self, n: usize, sampling: Sampling) -> Curve {
        let d = 1.0 / n as f64;
        let handle = |s: f64, piece: f64| {
            let dir = (self.at(s + d) - self.at(s - d)).normalize_or_zero();
            dir * (piece / 3.0)
        };
        let segments = (0..n)
            .map(|i| {
                let (s0, s1) = (i as f64 * d, (i + 1) as f64 * d);
                let len = self.length(s0, s1);
                let (p0, p1) = (self.at(s0), self.at(s1));
                [p0, p0 + handle(s0, len), p1 - handle(s1, len), p1]
            })
            .collect();
        Curve {
            segments,
            linear: false,
            closed: false,
            sampling,
        }
    }
}

/// A spline node's curve in its parent's space (placed by its local transform).
pub(crate) fn placed_curve(scene: &Scene, id: NodeId) -> Result<Option<Curve>, BuildError> {
    let Some(node) = scene.node(id) else {
        return Ok(None);
    };
    if !is_spline(&node.type_id) {
        return Ok(None);
    }
    let curve = build_curve(&node.type_id, &node.params)?;
    let m = scene.local_transform(id).expect("node exists").matrix();
    Ok(Some(curve.transformed(&m)))
}
