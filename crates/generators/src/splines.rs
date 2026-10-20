//! Spline primitives: circle, rectangle, helix (docs/specs/GENERATORS2.md §6).
//! Splines do not export (docs/parity/splines.md); their geometry is derived from the
//! spline-input generators' golden cases.

use std::collections::BTreeMap;

use forge_core::{Category, NodeId, NodeType, ParamDesc, Scene, Unit, Value};
use glam::DVec3;

use crate::{BuildError, Params};

pub const CIRCLE: &str = "forge.circle_spline";
pub const RECTANGLE: &str = "forge.rectangle_spline";
pub const HELIX: &str = "forge.helix_spline";

/// Circle quarter handle length (observed: 0.5533333 at every sample of
/// `extrude/defaults`, not the textbook 0.5523).
pub const CIRCLE_KAPPA: f64 = 83.0 / 150.0;

/// A spline as cubic Bezier segments `[p0, c0, c1, p1]` (lines have `linear` set).
#[derive(Debug, Clone, PartialEq)]
pub struct Curve {
    pub segments: Vec<[DVec3; 4]>,
    pub linear: bool,
    pub closed: bool,
}

impl Curve {
    fn bezier(seg: &[DVec3; 4], t: f64) -> DVec3 {
        let u = 1.0 - t;
        seg[0] * (u * u * u)
            + seg[1] * (3.0 * u * u * t)
            + seg[2] * (3.0 * u * t * t)
            + seg[3] * (t * t * t)
    }

    fn tangent(seg: &[DVec3; 4], t: f64) -> DVec3 {
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

    /// The spline's own points (Intermediate Points = Adaptive, 5°): the segment start
    /// points plus, on curved segments, the observed adaptive samples (§6.2). Closed
    /// splines don't repeat the first point.
    pub fn points(&self, angle: f64) -> Vec<DVec3> {
        let mut out = Vec::new();
        for seg in &self.segments {
            out.push(seg[0]);
            if !self.linear {
                out.extend(
                    adaptive_params(seg, angle)
                        .into_iter()
                        .map(|t| Self::bezier(seg, t)),
                );
            }
        }
        if !self.closed {
            out.push(self.segments.last().expect("non-empty")[3]);
        }
        out
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

/// Interior sample parameters of one curved segment (§6.2): walk t in steps of 1/50,
/// accumulating the tangent's turn, and emit a sample once it reaches `angle`; the last
/// interior sample is placed one step before the end.
fn adaptive_params(seg: &[DVec3; 4], angle: f64) -> Vec<f64> {
    const FINE: usize = 50;
    let mut out = Vec::new();
    let (mut last, mut turned, mut prev) = (0.0, 0.0, Curve::tangent(seg, 0.0));
    for k in 1..FINE {
        let t = k as f64 / FINE as f64;
        let d = Curve::tangent(seg, t);
        turned += prev.angle_between(d);
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

fn plane_param() -> ParamDesc {
    ParamDesc::enumeration(
        "plane",
        "Plane",
        "xy",
        &[("xy", "XY"), ("zy", "ZY"), ("xz", "XZ")],
    )
}

fn common(t: NodeType) -> NodeType {
    t.param(plane_param())
        .param(ParamDesc::bool("reverse", "Reverse", false))
        .param(ParamDesc::enumeration(
            "intermediate",
            "Intermediate points",
            "adaptive",
            &[
                ("none", "None"),
                ("natural", "Natural"),
                ("uniform", "Uniform"),
                ("adaptive", "Adaptive"),
                ("subdivided", "Subdivided"),
            ],
        ))
        .param(ParamDesc::int("intermediate_number", "Number", 8).range(0, 1000))
        .param(
            ParamDesc::float("intermediate_angle", "Angle", 5f64.to_radians())
                .range(0.0, std::f64::consts::PI)
                .unit(Unit::Angle),
        )
        .param(
            ParamDesc::float("intermediate_max_length", "Maximum length", 5.0)
                .range(0.0, f64::MAX)
                .unit(Unit::Distance),
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
    )
}

pub(crate) fn rectangle_type() -> NodeType {
    common(
        NodeType::new(RECTANGLE, "Rectangle", Category::Generator)
            .param(distance("width", "Width", 400.0))
            .param(distance("height", "Height", 400.0))
            .param(ParamDesc::bool("rounding", "Rounding", false))
            .param(distance("rounding_radius", "Radius", 50.0)),
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
            .param(ParamDesc::float("radial_bias", "Radial bias", 50.0).unit(Unit::Percent))
            .param(ParamDesc::float("height", "Height", 200.0).unit(Unit::Distance))
            .param(ParamDesc::float("height_bias", "Height bias", 50.0).unit(Unit::Percent))
            .param(ParamDesc::int("subdivision", "Subdivision", 100).range(3, 100_000)),
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
    if p.enumeration("intermediate") != "adaptive"
        || (p.float("intermediate_angle") - 5f64.to_radians()).abs() > 1e-12
    {
        return Err(BuildError::Unsupported(
            "spline intermediate points other than Adaptive 5°",
        ));
    }
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
            }
        }
        _ => {
            if (p.float("radial_bias") - 50.0).abs() > 1e-9
                || (p.float("height_bias") - 50.0).abs() > 1e-9
            {
                return Err(BuildError::Unsupported("helix bias"));
            }
            let (a0, a1) = (p.float("start_angle"), p.float("end_angle"));
            let (r0, r1) = (p.float("start_radius"), p.float("end_radius"));
            let turns = (a1 - a0).abs() / std::f64::consts::TAU;
            let n = ((p.int("subdivision") as f64 * turns).round() as usize).max(1);
            let pt = |i: usize| {
                let s = i as f64 / n as f64;
                let (a, r) = (a0 + (a1 - a0) * s, r0 + (r1 - r0) * s);
                DVec3::new(r * a.cos(), r * a.sin(), -p.float("height") * s)
            };
            Curve {
                segments: (0..n).map(|i| line(pt(i), pt(i + 1))).collect(),
                linear: true,
                closed: false,
            }
        }
    };
    let plane = p.enumeration("plane");
    for seg in &mut curve.segments {
        *seg = seg.map(|q| on_plane(&plane, q));
    }
    Ok(curve)
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
