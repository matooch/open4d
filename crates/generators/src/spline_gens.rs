//! Spline-input generators: extrude, lathe, loft, sweep (docs/specs/GENERATORS2.md §7–10).

use std::collections::BTreeMap;

use forge_core::{Category, NodeId, NodeType, ParamDesc, Scene, Unit, Value};
use forge_geometry::PolyMesh;
use forge_geometry::triangulate::triangulate_poly;
use glam::DVec3;

use crate::splines::{Curve, is_spline, placed_curve};
use crate::{BuildError, Params};

pub const EXTRUDE: &str = "forge.extrude";
pub const LATHE: &str = "forge.lathe";
pub const LOFT: &str = "forge.loft";
pub const SWEEP: &str = "forge.sweep";

/// The spline children a generator consumes (§0): extrude/lathe 1, sweep 2, loft all.
pub(crate) fn spline_input_count(type_id: &str) -> Option<usize> {
    match type_id {
        EXTRUDE | LATHE => Some(1),
        SWEEP => Some(2),
        LOFT => Some(usize::MAX),
        _ => None,
    }
}

pub fn spline_inputs_of(scene: &Scene, id: NodeId) -> Vec<NodeId> {
    let Some(node) = scene.node(id) else {
        return Vec::new();
    };
    let take = spline_input_count(&node.type_id).unwrap_or(0);
    node.children
        .iter()
        .copied()
        .filter(|c| scene.node(*c).is_some_and(|n| is_spline(&n.type_id)))
        .take(take)
        .collect()
}

fn caps(t: NodeType, cap_type: &str) -> NodeType {
    let cap = |id: &str, label: &str| {
        ParamDesc::enumeration(
            id,
            label,
            "cap",
            &[
                ("none", "None"),
                ("cap", "Cap"),
                ("fillet", "Fillet"),
                ("fillet_cap", "Fillet cap"),
            ],
        )
    };
    t.param(cap("cap_start", "Start"))
        .param(cap("cap_end", "End"))
        .param(ParamDesc::enumeration(
            "cap_type",
            "Cap type",
            cap_type,
            &[
                ("triangles", "Triangles"),
                ("quadrangles", "Quadrangles"),
                ("ngons", "N-gons"),
            ],
        ))
        .param(ParamDesc::bool("flip_normals", "Flip normals", false))
}

pub(crate) fn extrude_type() -> NodeType {
    caps(
        NodeType::new(EXTRUDE, "Extrude", Category::Generator)
            .param(ParamDesc::vec3("movement", "Movement", [0.0, 0.0, 20.0]).unit(Unit::Distance))
            .param(ParamDesc::int("subdivision", "Subdivision", 1).range(1, 1000)),
        "ngons",
    )
}

pub(crate) fn lathe_type() -> NodeType {
    caps(
        NodeType::new(LATHE, "Lathe", Category::Generator)
            .param(ParamDesc::float("angle", "Angle", std::f64::consts::TAU).unit(Unit::Angle))
            .param(ParamDesc::int("subdivision", "Subdivision", 24).range(3, 1000))
            .param(ParamDesc::float("movement", "Movement", 0.0).unit(Unit::Distance))
            .param(ParamDesc::float("scaling", "Scaling", 100.0).unit(Unit::Percent)),
        "triangles",
    )
}

pub(crate) fn loft_type() -> NodeType {
    caps(
        NodeType::new(LOFT, "Loft", Category::Generator)
            .param(ParamDesc::int("mesh_subdivision_u", "Mesh subdivision U", 30).range(3, 1000))
            .param(ParamDesc::int("mesh_subdivision_v", "Mesh subdivision V", 10).range(1, 1000))
            .param(ParamDesc::bool("organic_form", "Organic form", false))
            .param(ParamDesc::bool("loop", "Loop", false))
            .param(ParamDesc::bool(
                "subdivision_per_segment",
                "Subdivision per segment",
                true,
            ))
            .param(ParamDesc::bool(
                "linear_interpolation",
                "Linear interpolation",
                false,
            )),
        "triangles",
    )
}

pub(crate) fn sweep_type() -> NodeType {
    caps(
        NodeType::new(SWEEP, "Sweep", Category::Generator)
            .param(ParamDesc::float("end_scale", "End scale", 100.0).unit(Unit::Percent))
            .param(ParamDesc::float("end_rotation", "End rotation", 0.0).unit(Unit::Angle))
            .param(ParamDesc::bool("banking", "Banking", true))
            .param(ParamDesc::bool(
                "constant_cross_section",
                "Constant cross section",
                true,
            )),
        "triangles",
    )
}

/// A cap polygon `q` as observed (§7): n-gons as is; Triangles splits a 4-point cap
/// `[q3, q2, q0], [q2, q1, q0]` at the start (reversed outline) and
/// `[q0, q1, q3], [q1, q2, q3]` at the end; larger outlines are ear-clipped.
fn cap_faces(
    points: &[DVec3],
    q: &[u32],
    start: bool,
    cap_type: &str,
) -> Result<Vec<Vec<u32>>, BuildError> {
    // The n-gon starts at the vertex its triangulation fans around: q0 at the start;
    // at the end q3 for 4-point outlines (`extrude/rectangle`), q0 for the circle.
    let outline: Vec<u32> = if start {
        std::iter::once(q[0])
            .chain(q[1..].iter().rev().copied())
            .collect()
    } else if q.len() == 4 {
        vec![q[3], q[0], q[1], q[2]]
    } else {
        q.to_vec()
    };
    match cap_type {
        "ngons" => Ok(vec![outline]),
        "quadrangles" if q.len() <= 4 => Ok(vec![outline]),
        "quadrangles" => Err(BuildError::Unsupported("quadrangle caps over 4 points")),
        _ if q.len() == 4 => Ok(if start {
            vec![vec![q[3], q[2], q[0]], vec![q[2], q[1], q[0]]]
        } else {
            vec![vec![q[0], q[1], q[3]], vec![q[1], q[2], q[3]]]
        }),
        _ => Ok(triangulate_poly(points, &outline)
            .into_iter()
            .map(|t| t.to_vec())
            .collect()),
    }
}

fn no_fillets(p: &Params) -> Result<(), BuildError> {
    for id in ["cap_start", "cap_end"] {
        if p.enumeration(id).starts_with("fillet") {
            return Err(BuildError::Unsupported("fillet caps"));
        }
    }
    if p.bool("flip_normals") {
        return Err(BuildError::Unsupported("flipped normals"));
    }
    Ok(())
}

/// Extrude (§7).
pub fn extrude(curve: &Curve, values: &BTreeMap<String, Value>) -> Result<PolyMesh, BuildError> {
    let ty = extrude_type();
    let p = Params { ty: &ty, values };
    no_fillets(&p)?;
    let ring = curve.points(5f64.to_radians());
    let n = ring.len();
    let sub = p.int("subdivision");
    let mv = match p.get("movement") {
        Value::Vec3(v) => DVec3::from_array(v),
        _ => unreachable!(),
    };
    let mut points = Vec::with_capacity(n * (sub + 1));
    for j in 0..=sub {
        let off = mv * (j as f64 / sub as f64);
        points.extend(ring.iter().map(|&q| q + off));
    }
    let r = |j: usize, i: usize| (j * n + i % n) as u32;
    let spans = if curve.closed { n } else { n - 1 };
    let mut polys = Vec::new();
    for j in 0..sub {
        for i in 0..spans {
            polys.push(vec![r(j, i), r(j, i + 1), r(j + 1, i + 1), r(j + 1, i)]);
        }
    }
    if curve.closed {
        let cap_type = p.enumeration("cap_type");
        if p.enumeration("cap_start") == "cap" {
            let q: Vec<u32> = (0..n).map(|i| r(0, i)).collect();
            polys.extend(cap_faces(&points, &q, true, &cap_type)?);
        }
        if p.enumeration("cap_end") == "cap" {
            let q: Vec<u32> = (0..n).map(|i| r(sub, i)).collect();
            polys.extend(cap_faces(&points, &q, false, &cap_type)?);
        }
    }
    Ok(PolyMesh { points, polys })
}

/// Lathe (§8): the profile swept around Y.
pub fn lathe(curve: &Curve, values: &BTreeMap<String, Value>) -> Result<PolyMesh, BuildError> {
    let ty = lathe_type();
    let p = Params { ty: &ty, values };
    no_fillets(&p)?;
    if p.float("movement") != 0.0 || (p.float("scaling") - 100.0).abs() > 1e-9 {
        return Err(BuildError::Unsupported("lathe movement/scaling"));
    }
    let profile = curve.points(5f64.to_radians());
    let m = profile.len();
    let (angle, seg) = (p.float("angle"), p.int("subdivision"));
    let full = (angle.abs() - std::f64::consts::TAU).abs() < 1e-9;
    let columns = if full { seg } else { seg + 1 };
    let mut points = Vec::with_capacity(columns * m);
    for k in 0..columns {
        let (s, c) = (angle * k as f64 / seg as f64).sin_cos();
        points.extend(
            profile
                .iter()
                .map(|q| DVec3::new(q.x * c - q.z * s, q.y, q.x * s + q.z * c)),
        );
    }
    let at = |k: usize, j: usize| ((k % columns) * m + j % m) as u32;
    let rows = if curve.closed { m } else { m - 1 };
    let mut polys = Vec::new();
    for k in 0..seg {
        for j in 0..rows {
            polys.push(vec![at(k, j), at(k, j + 1), at(k + 1, j + 1), at(k + 1, j)]);
        }
    }
    if !full && curve.closed {
        let cap_type = p.enumeration("cap_type");
        if p.enumeration("cap_start") == "cap" {
            let q: Vec<u32> = (0..m).map(|j| at(0, j)).collect();
            polys.extend(cap_faces(&points, &q, true, &cap_type)?);
        }
        if p.enumeration("cap_end") == "cap" {
            let q: Vec<u32> = (0..m).map(|j| at(seg, j)).collect();
            polys.extend(cap_faces(&points, &q, false, &cap_type)?);
        }
    }
    Ok(PolyMesh { points, polys })
}

pub(crate) fn evaluate_spline_gen(
    scene: &Scene,
    id: NodeId,
    type_id: &str,
    values: &BTreeMap<String, Value>,
) -> Result<Option<PolyMesh>, BuildError> {
    let mut curves = Vec::new();
    for c in spline_inputs_of(scene, id) {
        if let Some(curve) = placed_curve(scene, c)? {
            curves.push(curve);
        }
    }
    let Some(first) = curves.first() else {
        return Ok(None);
    };
    match type_id {
        EXTRUDE => extrude(first, values).map(Some),
        LATHE => lathe(first, values).map(Some),
        LOFT => loft(&curves, values).map(Some),
        _ if curves.len() < 2 => Ok(None),
        _ => sweep(&curves[0], &curves[1], values).map(Some),
    }
}

/// Start and end caps of a ring `q` (closed outline) as ear-clipped triangles: the
/// start wound like `q`, the end reversed (§9, §10; the reference's own triangulation
/// is not derived).
fn triangle_caps(points: &[DVec3], start: &[u32], end: &[u32], p: &Params) -> Vec<Vec<u32>> {
    let mut out = Vec::new();
    if p.enumeration("cap_start") == "cap" {
        out.extend(
            triangulate_poly(points, start)
                .into_iter()
                .map(|t| t.to_vec()),
        );
    }
    if p.enumeration("cap_end") == "cap" {
        let rev: Vec<u32> = end.iter().rev().copied().collect();
        out.extend(
            triangulate_poly(points, &rev)
                .into_iter()
                .map(|t| t.to_vec()),
        );
    }
    out
}

/// Loft (§9): closed sections sampled uniformly in spline parameter, joined by cubic
/// Hermite curves per sample.
pub fn loft(curves: &[Curve], values: &BTreeMap<String, Value>) -> Result<PolyMesh, BuildError> {
    let ty = loft_type();
    let p = Params { ty: &ty, values };
    no_fillets(&p)?;
    if p.bool("organic_form")
        || p.bool("loop")
        || p.bool("linear_interpolation")
        || !p.bool("subdivision_per_segment")
    {
        return Err(BuildError::Unsupported(
            "loft options other than the defaults",
        ));
    }
    if curves.len() < 2 {
        return Err(BuildError::Unsupported("loft of one section"));
    }
    if curves.iter().any(|c| !c.closed) {
        return Err(BuildError::Unsupported("loft of open sections"));
    }
    let u = p.int("mesh_subdivision_u") - 1;
    let v = p.int("mesh_subdivision_v");
    let sections: Vec<Vec<DVec3>> = curves
        .iter()
        .map(|c| {
            let segs = c.segments.len() as f64;
            (0..u).map(|i| c.at(segs * i as f64 / u as f64)).collect()
        })
        .collect();
    let s = sections.len();
    // per sample index, its point in every section
    let columns: Vec<Vec<DVec3>> = (0..u)
        .map(|i| sections.iter().map(|sec| sec[i]).collect())
        .collect();
    let mut points = Vec::with_capacity(u * ((s - 1) * v + 1));
    for k in 0..s - 1 {
        let last = if k == s - 2 { v } else { v - 1 };
        for j in 0..=last {
            let t = j as f64 / v as f64;
            let (h00, h10, h01, h11) = (
                1.0 - 3.0 * t * t + 2.0 * t * t * t,
                t - 2.0 * t * t + t * t * t,
                3.0 * t * t - 2.0 * t * t * t,
                t * t * t - t * t,
            );
            for column in &columns {
                let pt = |m: usize| column[m];
                let (p0, p1) = (pt(k), pt(k + 1));
                // Tangent at section m, scaled to the chord of the segment it serves.
                let tangent = |m: usize, chord: f64| {
                    let dir = if m == 0 {
                        pt(1) - pt(0)
                    } else if m == s - 1 {
                        pt(m) - pt(m - 1)
                    } else {
                        pt(m + 1) - pt(m - 1)
                    };
                    dir.normalize_or_zero() * chord
                };
                let chord = (p1 - p0).length();
                let m0 = tangent(k, chord);
                let m1 = tangent(k + 1, chord);
                points.push(p0 * h00 + m0 * h10 + p1 * h01 + m1 * h11);
            }
        }
    }
    let rows = points.len() / u;
    let r = |j: usize, i: usize| (j * u + i % u) as u32;
    let mut polys = Vec::new();
    for j in 0..rows - 1 {
        for i in 0..u {
            polys.push(vec![r(j, i + 1), r(j + 1, i + 1), r(j + 1, i), r(j, i)]);
        }
    }
    let start: Vec<u32> = (0..u).map(|i| r(0, i)).collect();
    let end: Vec<u32> = (0..u).map(|i| r(rows - 1, i)).collect();
    polys.extend(triangle_caps(&points, &start, &end, &p));
    Ok(PolyMesh { points, polys })
}

/// Sweep (§10): the profile placed at every path point in a mitred, parallel-transported
/// frame, scaled and twisted linearly towards the path end.
pub fn sweep(
    profile: &Curve,
    path: &Curve,
    values: &BTreeMap<String, Value>,
) -> Result<PolyMesh, BuildError> {
    let ty = sweep_type();
    let p = Params { ty: &ty, values };
    no_fillets(&p)?;
    if !p.bool("banking") || !p.bool("constant_cross_section") {
        return Err(BuildError::Unsupported(
            "sweep options other than the defaults",
        ));
    }
    let shape = profile.points(5f64.to_radians());
    let route = path.points(5f64.to_radians());
    let (n, m) = (route.len(), shape.len());
    if n < 2 {
        return Err(BuildError::Unsupported("sweep along a single point"));
    }
    let closed = path.closed;
    let unit = |a: DVec3, b: DVec3| (b - a).normalize_or_zero();
    let (incoming, outgoing): (Vec<Option<DVec3>>, Vec<Option<DVec3>>) = (0..n)
        .map(|k| {
            let prev = if k > 0 {
                Some(route[k - 1])
            } else if closed {
                Some(route[n - 1])
            } else {
                None
            };
            let next = if k + 1 < n {
                Some(route[k + 1])
            } else if closed {
                Some(route[0])
            } else {
                None
            };
            (
                prev.map(|q| unit(q, route[k])),
                next.map(|q| unit(route[k], q)),
            )
        })
        .unzip();
    let tangents: Vec<DVec3> = (0..n)
        .map(|k| match (incoming[k], outgoing[k]) {
            (Some(a), Some(b)) => (a + b).normalize_or_zero(),
            (Some(a), None) => a,
            (None, Some(b)) => b,
            (None, None) => DVec3::Y,
        })
        .collect();
    // Initial normal: away from the bend at the first point, else any perpendicular.
    let bend = match (incoming[0], outgoing[0]) {
        (Some(a), Some(b)) => a - b,
        _ => outgoing[1.min(n - 1)]
            .zip(incoming[1.min(n - 1)])
            .map_or(DVec3::ZERO, |(b, a)| a - b),
    };
    let t0 = tangents[0];
    let mut normal = (bend - t0 * bend.dot(t0)).normalize_or_zero();
    if normal == DVec3::ZERO {
        normal = t0.any_orthonormal_vector();
    }
    let (end_scale, end_rotation) = (p.float("end_scale") / 100.0, p.float("end_rotation"));
    // Scale and twist spread by length along the path (closed: the closing chord counts).
    let mut along = vec![0.0; n];
    for k in 1..n {
        along[k] = along[k - 1] + (route[k] - route[k - 1]).length();
    }
    let total = along[n - 1]
        + if closed {
            (route[0] - route[n - 1]).length()
        } else {
            0.0
        };
    let mut points = Vec::with_capacity(n * m);
    for k in 0..n {
        let t = tangents[k];
        if k > 0 {
            // parallel transport of the normal from the previous tangent to this one
            let q = glam::DQuat::from_rotation_arc(tangents[k - 1], t);
            normal = (q * normal).normalize_or_zero();
        }
        let binormal = t.cross(normal);
        // Constant cross section: stretch along the bend by 1/cos(half turn), but only
        // where the path turns by 5° or more (observed: rings 15 and 16 of
        // `sweep/defaults`, turning 4.6°, are not stretched).
        let mitre = match (incoming[k], outgoing[k]) {
            (Some(a), Some(b)) if a.angle_between(b) >= 5f64.to_radians() - 1e-9 => {
                1.0 / t.dot(b).max(1e-6)
            }
            _ => 1.0,
        };
        let f = if total > 0.0 { along[k] / total } else { 0.0 };
        let scale = 1.0 + (end_scale - 1.0) * f;
        let (s, c) = (-end_rotation * f).sin_cos();
        points.extend(shape.iter().map(|q| {
            let (x, y) = (q.x * scale, q.y * scale);
            let (x, y) = (x * c - y * s, x * s + y * c);
            route[k] + normal * (x * mitre) + binormal * y
        }));
    }
    let ring = |k: usize, i: usize| ((k % n) * m + i % m) as u32;
    let spans = if profile.closed { m } else { m - 1 };
    let rings = if closed { n } else { n - 1 };
    let mut polys = Vec::new();
    for k in 0..rings {
        for i in 0..spans {
            polys.push(vec![
                ring(k, i),
                ring(k, i + 1),
                ring(k + 1, i + 1),
                ring(k + 1, i),
            ]);
        }
    }
    if !closed && profile.closed {
        let start: Vec<u32> = (0..m).rev().map(|i| ring(0, i)).collect();
        let end: Vec<u32> = (0..m).rev().map(|i| ring(n - 1, i)).collect();
        polys.extend(triangle_caps(&points, &start, &end, &p));
    }
    Ok(PolyMesh { points, polys })
}
