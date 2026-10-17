//! Polygon, vertex and corner normals (docs/specs/GEOMETRY.md §3).
//! Refs: Newell's method (Tampieri, Graphics Gems III, 1992); Thürmer & Wüthrich,
//! JGT 3(1) (1998); Gouraud, IEEE TC (1971).

use glam::DVec3;

use crate::PolyMesh;

/// How polygon normals are weighted when averaged at a vertex.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Weighting {
    /// Plain average (Gouraud).
    Uniform,
    /// Weighted by polygon area.
    Area,
    /// Weighted by the corner angle (Thürmer–Wüthrich). Parity-pending default.
    #[default]
    Angle,
}

/// Newell vector: twice the area times the unit normal. Computed relative to the
/// first vertex for precision far from the origin.
fn newell(points: &[DVec3], poly: &[u32]) -> DVec3 {
    let o = points[poly[0] as usize];
    let k = poly.len();
    (0..k)
        .map(|i| {
            let a = points[poly[i] as usize] - o;
            let b = points[poly[(i + 1) % k] as usize] - o;
            a.cross(b)
        })
        .sum()
}

/// Unit normal (right-hand rule), or zero for degenerate polygons.
pub fn poly_normal(points: &[DVec3], poly: &[u32]) -> DVec3 {
    let n = newell(points, poly);
    if n.length() < 1e-12 {
        DVec3::ZERO
    } else {
        n.normalize()
    }
}

pub fn poly_area(points: &[DVec3], poly: &[u32]) -> f64 {
    newell(points, poly).length() / 2.0
}

/// Interior angle at corner `i` of `poly`; 0 for zero-length edges.
fn corner_angle(points: &[DVec3], poly: &[u32], i: usize) -> f64 {
    let k = poly.len();
    let p = points[poly[i] as usize];
    let a = points[poly[(i + k - 1) % k] as usize] - p;
    let b = points[poly[(i + 1) % k] as usize] - p;
    if a.length() < 1e-300 || b.length() < 1e-300 {
        0.0
    } else {
        a.angle_between(b)
    }
}

fn weight(points: &[DVec3], poly: &[u32], corner: usize, w: Weighting) -> f64 {
    match w {
        Weighting::Uniform => 1.0,
        Weighting::Area => poly_area(points, poly),
        Weighting::Angle => corner_angle(points, poly, corner),
    }
}

/// One normal per point; zero for points no polygon uses.
pub fn vertex_normals(mesh: &PolyMesh, w: Weighting) -> Vec<DVec3> {
    let mut acc = vec![DVec3::ZERO; mesh.points.len()];
    for poly in &mesh.polys {
        let n = poly_normal(&mesh.points, poly);
        for (c, &v) in poly.iter().enumerate() {
            acc[v as usize] += n * weight(&mesh.points, poly, c, w);
        }
    }
    acc.into_iter().map(DVec3::normalize_or_zero).collect()
}

/// One normal per polygon corner. Polygons at a vertex whose normals differ from the
/// corner's polygon by more than `smoothing_angle` (radians) are left out.
pub fn corner_normals(mesh: &PolyMesh, smoothing_angle: f64, w: Weighting) -> Vec<Vec<DVec3>> {
    let normals: Vec<DVec3> = mesh
        .polys
        .iter()
        .map(|p| poly_normal(&mesh.points, p))
        .collect();
    // (poly, corner) incidences per vertex, in polygon order.
    let mut incident: Vec<Vec<(usize, usize)>> = vec![Vec::new(); mesh.points.len()];
    for (f, poly) in mesh.polys.iter().enumerate() {
        for (c, &v) in poly.iter().enumerate() {
            incident[v as usize].push((f, c));
        }
    }
    // Inclusive threshold with a little slack for exact right angles.
    let min_dot = smoothing_angle.cos() - 1e-9;
    mesh.polys
        .iter()
        .enumerate()
        .map(|(f, poly)| {
            poly.iter()
                .map(|&v| {
                    let included: Vec<(usize, usize)> = incident[v as usize]
                        .iter()
                        .copied()
                        .filter(|(g, _)| *g == f || normals[f].dot(normals[*g]) >= min_dot)
                        .collect();
                    let sum = |weighting: Weighting| {
                        included
                            .iter()
                            .map(|&(g, c)| {
                                normals[g] * weight(&mesh.points, &mesh.polys[g], c, weighting)
                            })
                            .sum::<DVec3>()
                    };
                    let weighted = sum(w);
                    // Degenerate corners (e.g. zero-length edges at a cone apex) carry no
                    // weight; fall back to the plain sum of the same polygons.
                    let n = if weighted.length() < 1e-12 {
                        sum(Weighting::Uniform)
                    } else {
                        weighted
                    };
                    n.normalize_or_zero()
                })
                .collect()
        })
        .collect()
}
