//! Spline-input generators: extrude, lathe, loft, sweep (docs/specs/GENERATORS2.md §7–10).

use std::collections::BTreeMap;

use forge_core::{Category, NodeId, NodeType, ParamDesc, Scene, Unit, Value};
use forge_geometry::PolyMesh;
use forge_geometry::triangulate::{triangulate_delaunay, triangulate_poly};
use glam::DVec3;

use crate::spline_sampling::{Interpolation, uniform_params};
use crate::splines::{Curve, is_spline, placed_curve};
use crate::{BuildError, Params};

pub const EXTRUDE: &str = "forge.extrude";
pub const LATHE: &str = "forge.lathe";
pub const LOFT: &str = "forge.loft";
pub const SWEEP: &str = "forge.sweep";

/// The spline children a generator consumes (§0): extrude/lathe 1, sweep 3 (profile, path,
/// rail), loft all.
pub(crate) fn spline_input_count(type_id: &str) -> Option<usize> {
    match type_id {
        EXTRUDE | LATHE => Some(1),
        SWEEP => Some(3),
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
            .param(ParamDesc::vec3("movement", "Movement", [0.0, 0.0, 100.0]).unit(Unit::Distance))
            .param(ParamDesc::int("subdivision", "Subdivision", 1).range(1, 4000)),
        "ngons",
    )
}

pub(crate) fn lathe_type() -> NodeType {
    caps(
        NodeType::new(LATHE, "Lathe", Category::Generator)
            .param(ParamDesc::float("angle", "Angle", std::f64::consts::TAU).unit(Unit::Angle))
            .param(ParamDesc::int("subdivision", "Subdivision", 32).range(1, 4000))
            .param(ParamDesc::float("movement", "Movement", 0.0).unit(Unit::Distance))
            .param(
                ParamDesc::float("scaling", "Scaling", 100.0)
                    .range(0.0, f64::MAX)
                    .unit(Unit::Percent),
            ),
        "ngons",
    )
}

pub(crate) fn loft_type() -> NodeType {
    caps(
        NodeType::new(LOFT, "Loft", Category::Generator)
            .param(ParamDesc::int("mesh_subdivision_u", "Mesh subdivision U", 16).range(2, 4000))
            .param(ParamDesc::int("mesh_subdivision_v", "Mesh subdivision V", 4).range(2, 4000))
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
            ))
            // UV-only: the mesh is the same either way (loft observation, Adapt UV row).
            .param(ParamDesc::bool("adapt_uv", "Adapt UV", true)),
        "ngons",
    )
}

pub(crate) fn sweep_type() -> NodeType {
    caps(
        NodeType::new(SWEEP, "Sweep", Category::Generator)
            .param(
                ParamDesc::float("end_scale", "End scale", 100.0)
                    .range(0.0, f64::MAX)
                    .unit(Unit::Percent),
            )
            .param(ParamDesc::float("end_rotation", "End rotation", 0.0).unit(Unit::Angle))
            .param(ParamDesc::bool("banking", "Banking", true))
            .param(ParamDesc::bool(
                "constant_cross_section",
                "Constant cross section",
                true,
            ))
            .param(
                ParamDesc::float("start_growth", "Start growth", 0.0)
                    .range(0.0, 100.0)
                    .unit(Unit::Percent),
            )
            .param(
                ParamDesc::float("end_growth", "End growth", 100.0)
                    .range(0.0, 100.0)
                    .unit(Unit::Percent),
            )
            .param(ParamDesc::bool(
                "parallel_movement",
                "Parallel movement",
                false,
            ))
            .param(ParamDesc::bool("keep_segments", "Keep segments", false))
            // Rail options: only meaningful with a third spline child (Unsupported).
            .param(ParamDesc::bool(
                "use_rail_direction",
                "Use rail direction",
                true,
            ))
            .param(ParamDesc::bool("two_rail", "2-Rail", true))
            .param(ParamDesc::bool("use_rail_scale", "Use rail scale", true))
            // UV-only.
            .param(ParamDesc::bool("stick_uvs", "Stick UVs", false)),
        "ngons",
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
    let loops = curve.loops();
    if loops.len() > 1 && (p.enumeration("cap_start") == "cap" || p.enumeration("cap_end") == "cap")
    {
        return Err(BuildError::Unsupported(
            "caps of a spline with several loops",
        ));
    }
    let ring: Vec<DVec3> = loops.iter().flatten().copied().collect();
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
    let r = |j: usize, i: usize| (j * n + i) as u32;
    let mut polys = Vec::new();
    for j in 0..sub {
        let mut start = 0;
        for (l, lp) in loops.iter().enumerate() {
            let m = lp.len();
            let spans = if curve.closed { m } else { m - 1 };
            for i in 0..spans {
                let (a, b) = (start + i, start + (i + 1) % m);
                // the further loops are holes: their faces wind the other way (`circle-ring`)
                polys.push(if l == 0 {
                    vec![r(j, a), r(j, b), r(j + 1, b), r(j + 1, a)]
                } else {
                    vec![r(j, a), r(j + 1, a), r(j + 1, b), r(j, b)]
                });
            }
            start += m;
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
    let (movement, scaling) = (p.float("movement"), p.float("scaling") / 100.0);
    let profile = curve.points();
    let m = profile.len();
    let (angle, seg) = (p.float("angle"), p.int("subdivision"));
    // A full turn closes on itself only when nothing moves or scales along it (§8).
    let full = (angle.abs() - std::f64::consts::TAU).abs() < 1e-9
        && movement == 0.0
        && (scaling - 1.0).abs() < 1e-12;
    let columns = if full { seg } else { seg + 1 };
    let mut points = Vec::with_capacity(columns * m);
    for k in 0..columns {
        let f = k as f64 / seg as f64;
        let (s, c) = (angle * f).sin_cos();
        let scale = 1.0 + (scaling - 1.0) * f;
        points.extend(profile.iter().map(|q| {
            let (x, z) = (q.x * scale, q.z * scale);
            DVec3::new(x * c - z * s, q.y + movement * f, x * s + z * c)
        }));
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
        let ngon = cap_type == "ngons";
        if p.enumeration("cap_start") == "cap" {
            let q: Vec<u32> = (0..m).map(|j| at(0, j)).collect();
            if ngon {
                // R25 lathe n-gon: the reversed outline from the last point (`lathe/partial`)
                polys.push(q.iter().rev().copied().collect());
            } else {
                polys.extend(cap_faces(&points, &q, true, &cap_type)?);
            }
        }
        if p.enumeration("cap_end") == "cap" {
            let q: Vec<u32> = (0..m).map(|j| at(seg, j)).collect();
            if ngon {
                polys.push(q);
            } else {
                polys.extend(cap_faces(&points, &q, false, &cap_type)?);
            }
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
    if type_id != EXTRUDE && curves.iter().any(|c| !c.extra.is_empty()) {
        return Err(BuildError::Unsupported("a spline with several loops"));
    }
    match type_id {
        EXTRUDE => extrude(first, values).map(Some),
        LATHE => lathe(first, values).map(Some),
        LOFT => loft(&curves, values).map(Some),
        _ if curves.len() < 2 => Ok(None),
        _ => sweep(&curves[0], &curves[1], curves.get(2), values).map(Some),
    }
}

/// Start and end caps of a ring `q` (closed outline) as Delaunay triangles in the
/// reference's listing: the start wound like `q`, the end reversed (§9, §10;
/// GEOMETRY.md §6.1).
fn triangle_caps(
    points: &[DVec3],
    start: &[u32],
    end: &[u32],
    p: &Params,
) -> Result<Vec<Vec<u32>>, BuildError> {
    let mut out = Vec::new();
    match p.enumeration("cap_type").as_str() {
        "ngons" => {
            // one n-gon per cap, wound like the triangles (`rechecks/loft-defaults`)
            if p.enumeration("cap_start") == "cap" {
                out.push(start.to_vec());
            }
            if p.enumeration("cap_end") == "cap" {
                out.push(end.iter().rev().copied().collect());
            }
            return Ok(out);
        }
        "quadrangles" => return Err(BuildError::Unsupported("quadrangle caps of rings")),
        _ => {}
    }
    if p.enumeration("cap_start") == "cap" {
        out.extend(
            triangulate_delaunay(points, start)
                .into_iter()
                .map(|t| t.to_vec()),
        );
    }
    if p.enumeration("cap_end") == "cap" {
        let rev: Vec<u32> = end.iter().rev().copied().collect();
        out.extend(
            triangulate_delaunay(points, &rev)
                .into_iter()
                .map(|t| t.to_vec()),
        );
    }
    Ok(out)
}

/// Loft (§9, R25 per D18): `U` points per ring, closed sections sampled uniformly in spline parameter, joined by cubic
/// Hermite curves per sample.
pub fn loft(curves: &[Curve], values: &BTreeMap<String, Value>) -> Result<PolyMesh, BuildError> {
    let ty = loft_type();
    let p = Params { ty: &ty, values };
    no_fillets(&p)?;
    if p.bool("organic_form") || p.bool("loop") || !p.bool("subdivision_per_segment") {
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
    let u = p.int("mesh_subdivision_u");
    let v = p.int("mesh_subdivision_v");
    let linear = p.bool("linear_interpolation");
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
                if linear {
                    points.push(p0 + (p1 - p0) * t);
                    continue;
                }
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
            polys.push(vec![r(j, i), r(j + 1, i), r(j + 1, i + 1), r(j, i + 1)]);
        }
    }
    let start: Vec<u32> = (0..u).map(|i| r(0, i)).collect();
    let end: Vec<u32> = (0..u).map(|i| r(rows - 1, i)).collect();
    polys.extend(triangle_caps(&points, &start, &end, &p)?);
    Ok(PolyMesh { points, polys })
}

/// Sweep (§10): the profile placed at every path point in a mitred, parallel-transported
/// frame, scaled and twisted linearly towards the path end.
pub fn sweep(
    profile: &Curve,
    path: &Curve,
    rail: Option<&Curve>,
    values: &BTreeMap<String, Value>,
) -> Result<PolyMesh, BuildError> {
    let ty = sweep_type();
    let p = Params { ty: &ty, values };
    no_fillets(&p)?;
    if !p.bool("constant_cross_section")
        || p.float("start_growth") != 0.0
        || p.float("end_growth") != 100.0
        || p.bool("parallel_movement")
        || p.bool("keep_segments")
    {
        return Err(BuildError::Unsupported(
            "sweep options other than the defaults",
        ));
    }
    let shape = profile.points();
    let route = path.points();
    let (n, m) = (route.len(), shape.len());
    if n < 2 {
        return Err(BuildError::Unsupported("sweep along a single point"));
    }
    let closed = path.closed;
    let banking = p.bool("banking");
    if let Some(rail) = rail {
        let points = rail_rings(&shape, path, rail, &p)?;
        return sweep_faces(points, n, m, profile.closed, closed, &p);
    }
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
        if !banking {
            // Banking off: the profile's x axis stays horizontal, `Y × tangent` (`sweep/banking`);
            // where the tangent is vertical it keeps the previous normal.
            let side = DVec3::Y.cross(t);
            if side.length() > 1e-6 {
                normal = side.normalize();
            }
        } else if k > 0 {
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
    sweep_faces(points, n, m, profile.closed, closed, &p)
}

/// 2-Rail sweep (§10a): ring `k` is the profile's x extent mapped onto the segment from path
/// point `k` to the rail point at the same spline parameter; the profile's y is dropped
/// (`sweep/rail`, parity-pending).
fn rail_rings(
    shape: &[DVec3],
    path: &Curve,
    rail: &Curve,
    p: &Params,
) -> Result<Vec<DVec3>, BuildError> {
    if !(p.bool("two_rail") && p.bool("use_rail_direction") && p.bool("use_rail_scale")) {
        return Err(BuildError::Unsupported(
            "sweep rail options other than the defaults",
        ));
    }
    if p.float("end_scale") != 100.0 || p.float("end_rotation") != 0.0 {
        return Err(BuildError::Unsupported(
            "sweep rail with end scale or rotation",
        ));
    }
    // Rail point k: the rail at the path point's spline parameter (`sweep/rail`: the rail rings'
    // z fractions are exactly k/900); polylines pair point by point.
    let route = path.points();
    let rail_points: Vec<DVec3> = if path.linear || rail.linear {
        let r = rail.points();
        if r.len() != route.len() {
            return Err(BuildError::Unsupported(
                "sweep rail with another point count",
            ));
        }
        r
    } else if path.sampling.mode == Interpolation::Uniform
        && path.sampling.number > 0
        && !path.closed
        && rail.segments.len() == path.segments.len()
    {
        uniform_params(path, path.sampling.number)
            .into_iter()
            .map(|s| rail.at(s))
            .collect()
    } else {
        return Err(BuildError::Unsupported(
            "sweep rail other than a curve with the path's segments and uniform points",
        ));
    };
    let lo = shape.iter().map(|q| q.x).fold(f64::INFINITY, f64::min);
    let hi = shape.iter().map(|q| q.x).fold(f64::NEG_INFINITY, f64::max);
    if hi - lo < 1e-9 {
        return Err(BuildError::Unsupported(
            "sweep rail with a profile of no width",
        ));
    }
    let mut points = Vec::with_capacity(route.len() * shape.len());
    for (a, b) in route.iter().zip(&rail_points) {
        points.extend(
            shape
                .iter()
                .map(|q| *a + (*b - *a) * ((q.x - lo) / (hi - lo))),
        );
    }
    Ok(points)
}

/// Quads per ring pair and profile span, plus the caps of open paths.
fn sweep_faces(
    points: Vec<DVec3>,
    n: usize,
    m: usize,
    profile_closed: bool,
    closed: bool,
    p: &Params,
) -> Result<PolyMesh, BuildError> {
    let ring = |k: usize, i: usize| ((k % n) * m + i % m) as u32;
    let spans = if profile_closed { m } else { m - 1 };
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
    if !closed && profile_closed {
        let start: Vec<u32> = (0..m).rev().map(|i| ring(0, i)).collect();
        let end: Vec<u32> = (0..m).rev().map(|i| ring(n - 1, i)).collect();
        polys.extend(triangle_caps(&points, &start, &end, p)?);
    }
    Ok(PolyMesh { points, polys })
}
