//! Polygon Reduction (docs/specs/MODIFIERS.md §22.3): quadric edge collapse.

use std::collections::{BTreeMap, BTreeSet};

use forge_geometry::PolyMesh;
use glam::DVec3;

pub struct ReductionParams {
    /// Fraction (the percent param is stored as 0..1).
    pub strength: f64,
    pub quality: f64,
    pub co_planar: bool,
    pub boundary: bool,
    pub polygon_quality: bool,
}

impl Default for ReductionParams {
    fn default() -> Self {
        Self {
            strength: 0.9,
            quality: 1000.0,
            co_planar: true,
            boundary: true,
            polygon_quality: false,
        }
    }
}

/// Ten coefficients of a symmetric 4x4 quadric for
/// `ax² + by² + cz² + 2dxy + 2exz + 2fyz + 2gx + 2hy + 2iz + j`.
#[derive(Clone, Copy, Default)]
struct Quadric([f64; 10]);

impl Quadric {
    fn plane(n: DVec3, p: DVec3, weight: f64) -> Quadric {
        let d = -n.dot(p);
        let (a, b, c) = (n.x, n.y, n.z);
        Quadric([
            a * a * weight,
            b * b * weight,
            c * c * weight,
            a * b * weight,
            a * c * weight,
            b * c * weight,
            a * d * weight,
            b * d * weight,
            c * d * weight,
            d * d * weight,
        ])
    }

    fn add(mut self, o: Quadric) -> Quadric {
        for (a, b) in self.0.iter_mut().zip(o.0) {
            *a += b;
        }
        self
    }

    fn eval(self, p: DVec3) -> f64 {
        let q = self.0;
        q[0] * p.x * p.x
            + q[1] * p.y * p.y
            + q[2] * p.z * p.z
            + 2.0 * q[3] * p.x * p.y
            + 2.0 * q[4] * p.x * p.z
            + 2.0 * q[5] * p.y * p.z
            + 2.0 * q[6] * p.x
            + 2.0 * q[7] * p.y
            + 2.0 * q[8] * p.z
            + q[9]
    }
}

fn triangulate(mesh: &PolyMesh) -> Vec<[u32; 3]> {
    let mut out = Vec::new();
    for f in &mesh.polys {
        if f.len() < 3 {
            continue;
        }
        for k in 1..f.len() - 1 {
            out.push([f[0], f[k], f[k + 1]]);
        }
    }
    out
}

fn plane_quadrics(points: &[DVec3], tris: &[[u32; 3]]) -> Vec<Quadric> {
    let mut quad = vec![Quadric::default(); points.len()];
    for t in tris {
        let n = (points[t[1] as usize] - points[t[0] as usize])
            .cross(points[t[2] as usize] - points[t[0] as usize])
            .normalize_or_zero();
        if n.length_squared() == 0.0 {
            continue;
        }
        let area = (points[t[1] as usize] - points[t[0] as usize])
            .cross(points[t[2] as usize] - points[t[0] as usize])
            .length()
            * 0.5;
        let q = Quadric::plane(n, points[t[0] as usize], area.max(1e-9));
        for v in t {
            quad[*v as usize] = quad[*v as usize].add(q);
        }
    }
    quad
}

/// The cheapest collapsible edge: exactly two triangles share it and its endpoints share
/// exactly two neighbours.
fn best_edge(points: &[DVec3], tris: &[[u32; 3]], quad: &[Quadric]) -> Option<(f64, u32, u32)> {
    let mut neighbours: Vec<BTreeSet<u32>> = vec![BTreeSet::new(); points.len()];
    let mut shared: BTreeMap<(u32, u32), usize> = BTreeMap::new();
    for t in tris {
        for (a, b) in [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])] {
            neighbours[a as usize].insert(b);
            neighbours[b as usize].insert(a);
            *shared.entry((a.min(b), a.max(b))).or_insert(0) += 1;
        }
    }
    let mut best: Option<(f64, u32, u32)> = None;
    for ((a, b), count) in shared {
        if count != 2 {
            continue;
        }
        let common = neighbours[a as usize]
            .intersection(&neighbours[b as usize])
            .count();
        if common != 2 {
            continue;
        }
        let mid = (points[a as usize] + points[b as usize]) * 0.5;
        let cost = quad[a as usize].add(quad[b as usize]).eval(mid);
        if best.is_none_or(|(c, _, _)| cost < c) {
            best = Some((cost, a, b));
        }
    }
    best
}

/// Collapses edges until `round((1 - strength) * triangles)` triangles remain (the rule
/// observed in O19). The input is triangulated first and stays all triangles.
pub fn reduce(mesh: &mut PolyMesh, p: &ReductionParams) {
    let mut points = mesh.points.clone();
    let mut tris = triangulate(mesh);
    if tris.is_empty() {
        return;
    }
    let target = (((1.0 - p.strength.clamp(0.0, 1.0)) * tris.len() as f64).round() as usize).max(4);
    let mut quad = plane_quadrics(&points, &tris);
    let mut guard = tris.len() * 4 + points.len();
    while tris.len() > target && guard > 0 {
        guard -= 1;
        let Some((_, a, b)) = best_edge(&points, &tris, &quad) else {
            break;
        };
        points[a as usize] = (points[a as usize] + points[b as usize]) * 0.5;
        for t in &mut tris {
            for v in t.iter_mut() {
                if *v == b {
                    *v = a;
                }
            }
        }
        tris.retain(|t| t[0] != t[1] && t[1] != t[2] && t[0] != t[2]);
        // A collapse can produce the same triangle twice; keep one of each.
        let mut seen = BTreeSet::new();
        tris.retain(|t| {
            let mut key = *t;
            key.sort_unstable();
            seen.insert(key)
        });
        quad[a as usize] = quad[a as usize].add(quad[b as usize]);
    }
    let mut used = BTreeSet::new();
    for t in &tris {
        used.extend(t.iter().copied());
    }
    let mut remap = vec![u32::MAX; points.len()];
    let mut out = Vec::with_capacity(used.len());
    for old in used {
        remap[old as usize] = out.len() as u32;
        out.push(points[old as usize]);
    }
    for t in &mut tris {
        for v in t.iter_mut() {
            *v = remap[*v as usize];
        }
    }
    mesh.points = out;
    mesh.polys = tris.into_iter().map(|t| t.to_vec()).collect();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_closed_triangulated_sphere_reaches_the_face_target() {
        // A small icosahedron stands in for the sphere (20 triangles, 12 points).
        let phi = (1.0 + 5.0_f64.sqrt()) * 0.5;
        let raw = [
            [-1.0, phi, 0.0],
            [1.0, phi, 0.0],
            [-1.0, -phi, 0.0],
            [1.0, -phi, 0.0],
            [0.0, -1.0, phi],
            [0.0, 1.0, phi],
            [0.0, -1.0, -phi],
            [0.0, 1.0, -phi],
            [phi, 0.0, -1.0],
            [phi, 0.0, 1.0],
            [-phi, 0.0, -1.0],
            [-phi, 0.0, 1.0],
        ];
        let points: Vec<DVec3> = raw
            .iter()
            .map(|p| DVec3::from_array(*p).normalize() * 100.0)
            .collect();
        let faces = [
            [0, 11, 5],
            [0, 5, 1],
            [0, 1, 7],
            [0, 7, 10],
            [0, 10, 11],
            [1, 5, 9],
            [5, 11, 4],
            [11, 10, 2],
            [10, 7, 6],
            [7, 1, 8],
            [3, 9, 4],
            [3, 4, 2],
            [3, 2, 6],
            [3, 6, 8],
            [3, 8, 9],
            [4, 9, 5],
            [2, 4, 11],
            [6, 2, 10],
            [8, 6, 7],
            [9, 8, 1],
        ];
        let mut mesh = PolyMesh {
            points,
            polys: faces.into_iter().map(|f| f.to_vec()).collect(),
        };
        reduce(
            &mut mesh,
            &ReductionParams {
                strength: 0.5,
                ..Default::default()
            },
        );
        assert_eq!(mesh.polys.len(), 10);
        assert_eq!(
            mesh.points.len(),
            7,
            "closed all-triangle mesh: V = F/2 + 2"
        );
    }

    #[test]
    fn zero_strength_keeps_the_triangles() {
        let mut mesh = PolyMesh {
            points: vec![DVec3::ZERO, DVec3::X, DVec3::Y, DVec3::Z],
            polys: vec![vec![0, 1, 2], vec![0, 1, 3], vec![1, 2, 3], vec![0, 2, 3]],
        };
        reduce(
            &mut mesh,
            &ReductionParams {
                strength: 0.0,
                ..Default::default()
            },
        );
        assert_eq!(mesh.polys.len(), 4);
    }
}
