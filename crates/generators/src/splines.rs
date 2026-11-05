//! Spline primitives: circle, rectangle, helix (docs/specs/GENERATORS2.md §6).
//! Splines do not export (docs/parity/splines.md); their geometry is derived from the
//! spline-input generators' golden cases.

use std::collections::BTreeMap;

use forge_core::{Category, NodeId, NodeType, ParamDesc, Scene, Unit, Value};
use glam::DVec3;

use crate::spline_sampling::{Interpolation, Sampling, add_params, sample};
use crate::{BuildError, Params};
use crate::{editable_spline, spline_prims};

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
    /// Further closed loops of the same spline (the inner circle of a Ring, the centre hole
    /// of a Cogwheel): the reference reports them as extra segments of one spline.
    pub extra: Vec<Curve>,
}

impl Curve {
    pub fn new(segments: Vec<[DVec3; 4]>, linear: bool, closed: bool, sampling: Sampling) -> Curve {
        Curve {
            segments,
            linear,
            closed,
            sampling,
            extra: Vec::new(),
        }
    }

    /// A polyline through `points` (closed ones join the last point to the first).
    pub fn polyline(points: &[DVec3], closed: bool, sampling: Sampling) -> Curve {
        let n = points.len();
        let spans = if closed { n } else { n - 1 };
        let segments = (0..spans)
            .map(|i| line(points[i], points[(i + 1) % n]))
            .collect();
        Curve::new(segments, true, closed, sampling)
    }

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

    /// The point lists of the main loop and of every extra loop.
    pub fn loops(&self) -> Vec<Vec<DVec3>> {
        std::iter::once(self.points())
            .chain(self.extra.iter().map(Curve::points))
            .collect()
    }

    pub fn transformed(&self, m: &glam::DMat4) -> Curve {
        Curve {
            segments: self
                .segments
                .iter()
                .map(|s| s.map(|p| m.transform_point3(p)))
                .collect(),
            extra: self.extra.iter().map(|c| c.transformed(m)).collect(),
            ..self.clone()
        }
    }
}

pub(crate) fn plane_param() -> ParamDesc {
    ParamDesc::enumeration(
        "plane",
        "Plane",
        "xy",
        &[("xy", "XY"), ("zy", "ZY"), ("xz", "XZ")],
    )
}

pub(crate) fn common(t: NodeType, default: Interpolation) -> NodeType {
    add_params(
        t.param(plane_param())
            .param(ParamDesc::bool("reverse", "Reverse", false)),
        default,
    )
}

pub(crate) fn distance(id: &str, label: &str, default: f64) -> ParamDesc {
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
    matches!(
        type_id,
        CIRCLE
            | RECTANGLE
            | HELIX
            | editable_spline::TYPE
            | crate::spline_mask::TYPE
            | crate::text_spline::TYPE
    ) || spline_prims::node_type(type_id).is_some()
}

/// Maps the spline's XY frame to its plane (§6.1).
pub(crate) fn on_plane(plane: &str, p: DVec3) -> DVec3 {
    match plane {
        "xz" => DVec3::new(p.x, -p.z, p.y),
        "zy" => DVec3::new(-p.z, p.y, p.x),
        _ => p,
    }
}

pub(crate) fn line(a: DVec3, b: DVec3) -> [DVec3; 4] {
    [a, a.lerp(b, 1.0 / 3.0), a.lerp(b, 2.0 / 3.0), b]
}

/// A straight piece of a curve that also has curved pieces: both handles sit on their end
/// points, as the reference's closing chord does (`arc-sector`: its Uniform samples follow
/// `3t^2 - 2t^3` along the chord, 7e-7). Pure polylines use [`line`] (they never get
/// interior points).
pub(crate) fn straight(a: DVec3, b: DVec3) -> [DVec3; 4] {
    [a, a, b, b]
}

/// A smooth curve through `points` with the reference's automatic tangents: at a point the
/// tangent runs along `next - previous` and both handles are a third of the shorter
/// adjacent chord long; the two ends of an open curve use their single chord (fitted to
/// the flower and cycloid exports, 7e-7). Coincident neighbours give a zero handle.
pub(crate) fn auto_tangent_curve(points: &[DVec3], closed: bool, sampling: Sampling) -> Curve {
    let n = points.len();
    let handle = |i: usize| -> DVec3 {
        let (prev, next) = if closed {
            (points[(i + n - 1) % n], points[(i + 1) % n])
        } else {
            (points[i.saturating_sub(1)], points[(i + 1).min(n - 1)])
        };
        let dir = (next - prev).normalize_or_zero();
        let len = if closed || (i > 0 && i + 1 < n) {
            (points[i] - prev).length().min((next - points[i]).length())
        } else {
            (next - prev).length()
        };
        dir * (len / 3.0)
    };
    let spans = if closed { n } else { n - 1 };
    let segments = (0..spans)
        .map(|i| {
            let j = (i + 1) % n;
            [
                points[i],
                points[i] + handle(i),
                points[j] - handle(j),
                points[j],
            ]
        })
        .collect();
    Curve::new(segments, false, closed, sampling)
}

/// A quarter-turn style Bezier arc from `a` to `b` around `center`: the handles are tangent
/// to the circle and `CIRCLE_KAPPA * radius * (turn / quarter turn)` long (circle quarters,
/// the Arc, the fillets of rounded n-sides and rectangles all follow it).
pub(crate) fn arc_segment(center: DVec3, radius: f64, from: f64, turn: f64) -> [DVec3; 4] {
    let at = |a: f64| center + DVec3::new(a.cos(), a.sin(), 0.0) * radius;
    let tan = |a: f64| DVec3::new(-a.sin(), a.cos(), 0.0);
    let h = CIRCLE_KAPPA * radius * turn / std::f64::consts::FRAC_PI_2;
    let to = from + turn;
    [
        at(from),
        at(from) + tan(from) * h,
        at(to) - tan(to) * h,
        at(to),
    ]
}

fn node_type_of(type_id: &str) -> Option<NodeType> {
    match type_id {
        CIRCLE => Some(circle_type()),
        RECTANGLE => Some(rectangle_type()),
        HELIX => Some(helix_type()),
        editable_spline::TYPE => Some(editable_spline::node_type()),
        crate::text_spline::TYPE => Some(crate::text_spline::node_type()),
        other => spline_prims::node_type(other),
    }
}

/// The spline of a spline-primitive node, in its local space.
pub fn build_curve(type_id: &str, values: &BTreeMap<String, Value>) -> Result<Curve, BuildError> {
    if type_id == editable_spline::TYPE {
        return editable_spline::build_curve(values);
    }
    if type_id == crate::text_spline::TYPE {
        return crate::text_spline::build_curve(values);
    }
    if type_id == crate::spline_mask::TYPE {
        return Err(BuildError::Failed(
            "a spline mask is built from its children".into(),
        ));
    }
    let ty = node_type_of(type_id).ok_or_else(|| BuildError::UnknownType(type_id.to_string()))?;
    let p = Params { ty: &ty, values };
    let sampling = Sampling::from_params(&p);
    if p.bool("reverse") {
        return Err(BuildError::Unsupported("reversed spline"));
    }
    let mut curve = match type_id {
        CIRCLE => circle(&p, sampling)?,
        RECTANGLE => rectangle(&p, sampling)?,
        HELIX => helix(&p, sampling)?,
        other => spline_prims::build(other, &p, sampling)?,
    };
    let plane = p.enumeration("plane");
    if plane != "xy" {
        let m = |c: &mut Curve| {
            for seg in &mut c.segments {
                *seg = seg.map(|q| on_plane(&plane, q));
            }
        };
        m(&mut curve);
        curve.extra.iter_mut().for_each(m);
    }
    Ok(curve)
}

fn circle_loop(rx: f64, ry: f64, sampling: Sampling) -> Curve {
    let (kx, ky) = (CIRCLE_KAPPA * rx, CIRCLE_KAPPA * ry);
    // quarters counter-clockwise from (rx, 0)
    let ka = |t: DVec3| if t.x != 0.0 { kx } else { ky };
    let q = |a: DVec3, b: DVec3, ta: DVec3, tb: DVec3| [a, a + ta * ka(ta), b - tb * ka(tb), b];
    let (e, n, w, s) = (
        DVec3::new(rx, 0.0, 0.0),
        DVec3::new(0.0, ry, 0.0),
        DVec3::new(-rx, 0.0, 0.0),
        DVec3::new(0.0, -ry, 0.0),
    );
    Curve::new(
        vec![
            q(e, n, DVec3::Y, -DVec3::X),
            q(n, w, -DVec3::X, -DVec3::Y),
            q(w, s, -DVec3::Y, DVec3::X),
            q(s, e, DVec3::X, DVec3::Y),
        ],
        false,
        true,
        sampling,
    )
}

fn circle(p: &Params, sampling: Sampling) -> Result<Curve, BuildError> {
    let ellipse = p.bool("ellipse");
    if ellipse && p.bool("ring") {
        return Err(BuildError::Unsupported("circle ellipse together with ring"));
    }
    let r = p.float("radius");
    let mut curve = if ellipse {
        circle_loop(r, p.float("radius_y"), sampling)
    } else {
        circle_loop(r, r, sampling)
    };
    if p.bool("ring") {
        curve.extra.push(circle_loop(
            p.float("inner_radius"),
            p.float("inner_radius"),
            sampling,
        ));
    }
    Ok(curve)
}

fn rectangle(p: &Params, sampling: Sampling) -> Result<Curve, BuildError> {
    let (w, h) = (p.float("width") / 2.0, p.float("height") / 2.0);
    let c = [
        DVec3::new(w, h, 0.0),
        DVec3::new(-w, h, 0.0),
        DVec3::new(-w, -h, 0.0),
        DVec3::new(w, -h, 0.0),
    ];
    if !p.bool("rounding") {
        return Ok(Curve::new(
            (0..4).map(|i| line(c[i], c[(i + 1) % 4])).collect(),
            true,
            true,
            sampling,
        ));
    }
    // Quarter arcs at the four corners, each followed by the straight piece to the next
    // corner, starting with the arc of the top right corner (spec §6.1).
    let r = p.float("rounding_radius").min(w).min(h);
    let mut segments = Vec::new();
    let centers = [
        DVec3::new(w - r, h - r, 0.0),
        DVec3::new(-w + r, h - r, 0.0),
        DVec3::new(-w + r, -h + r, 0.0),
        DVec3::new(w - r, -h + r, 0.0),
    ];
    for (k, center) in centers.iter().enumerate() {
        let arc = arc_segment(
            *center,
            r,
            k as f64 * std::f64::consts::FRAC_PI_2,
            std::f64::consts::FRAC_PI_2,
        );
        segments.push(arc);
        let next = centers[(k + 1) % 4];
        let next_start = arc_segment(next, r, (k + 1) as f64 * std::f64::consts::FRAC_PI_2, 0.0)[0];
        segments.push(straight(arc[3], next_start));
    }
    Ok(Curve::new(segments, false, true, sampling))
}

fn helix(p: &Params, sampling: Sampling) -> Result<Curve, BuildError> {
    if (p.float("radial_bias") - 50.0).abs() > 1e-9 || (p.float("height_bias") - 50.0).abs() > 1e-9
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
    Ok(if sampling.mode == Interpolation::Adaptive {
        shape.adaptive_polyline(p.int("subdivision") as f64, sampling)
    } else {
        shape.bezier(p.int("subdivision"), sampling)
    })
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

    /// R20 form (`sweep/helix`, Adaptive): lines through `subdivision` points per turn, the
    /// points next to both ends skipped.
    fn adaptive_polyline(&self, subdivision: f64, sampling: Sampling) -> Curve {
        let turns = (self.end_angle - self.start_angle).abs() / std::f64::consts::TAU;
        let n = ((subdivision * turns).round() as usize).max(1);
        let keep: Vec<usize> = (0..=n)
            .filter(|&i| n < 4 || (i != 1 && i != n - 1))
            .collect();
        Curve::new(
            keep.windows(2)
                .map(|w| {
                    line(
                        self.at(w[0] as f64 / n as f64),
                        self.at(w[1] as f64 / n as f64),
                    )
                })
                .collect(),
            true,
            false,
            sampling,
        )
    }

    /// R25 form (`splines-2/helix-uniform`): `n + 1` points at `s = i/n`. The handles at a
    /// point lie along `f(s + 1/n) - f(s - 1/n)` and are a third of the chord of the piece
    /// they belong to long (GENERATORS2.md §6.1).
    fn bezier(&self, n: usize, sampling: Sampling) -> Curve {
        let d = 1.0 / n as f64;
        let handle = |s: f64, chord: f64| {
            (self.at(s + d) - self.at(s - d)).normalize_or_zero() * (chord / 3.0)
        };
        let segments = (0..n)
            .map(|i| {
                let (s0, s1) = (i as f64 * d, (i + 1) as f64 * d);
                let (p0, p1) = (self.at(s0), self.at(s1));
                let chord = (p1 - p0).length();
                [p0, p0 + handle(s0, chord), p1 - handle(s1, chord), p1]
            })
            .collect();
        Curve::new(segments, false, false, sampling)
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
    let curve = if node.type_id == crate::spline_mask::TYPE {
        match crate::spline_mask::curve_of(scene, id)? {
            Some(c) => c,
            None => return Ok(None),
        }
    } else {
        build_curve(&node.type_id, &node.params)?
    };
    let m = scene.local_transform(id).expect("node exists").matrix();
    Ok(Some(curve.transformed(&m)))
}

/// The sampled loops of a spline node in its own space, each with whether it is closed: what the
/// viewport draws (VIEWPORT.md section 11). `None` if `id` is no spline or has no curve; loops of
/// fewer than two points are left out.
pub fn sampled_loops(scene: &Scene, id: NodeId) -> Option<Vec<(Vec<DVec3>, bool)>> {
    let node = scene.node(id)?;
    if !is_spline(&node.type_id) {
        return None;
    }
    let curve = if node.type_id == crate::spline_mask::TYPE {
        crate::spline_mask::curve_of(scene, id).ok()??
    } else {
        build_curve(&node.type_id, &node.params).ok()?
    };
    let loops: Vec<(Vec<DVec3>, bool)> = std::iter::once(&curve)
        .chain(curve.extra.iter())
        .map(|c| (c.points(), c.closed))
        .filter(|(p, _)| p.len() >= 2)
        .collect();
    (!loops.is_empty()).then_some(loops)
}

/// A spline node as a polyline helper mesh for the spline-driven deformers (MODIFIERS.md
/// §20.0): its sampled points in its own space, one 2-point edge per chord (plus the closing
/// edge of a closed spline). `None` if `id` is no spline or has no curve.
pub fn polyline_mesh(scene: &Scene, id: NodeId) -> Option<forge_geometry::PolyMesh> {
    let node = scene.node(id)?;
    if !is_spline(&node.type_id) {
        return None;
    }
    let curve = if node.type_id == crate::spline_mask::TYPE {
        crate::spline_mask::curve_of(scene, id).ok()??
    } else {
        build_curve(&node.type_id, &node.params).ok()?
    };
    let points = curve.points();
    let n = points.len();
    if n < 2 {
        return None;
    }
    let spans = if curve.closed && n > 2 { n } else { n - 1 };
    let polys = (0..spans)
        .map(|k| vec![k as u32, ((k + 1) % n) as u32])
        .collect();
    Some(forge_geometry::PolyMesh { points, polys })
}
