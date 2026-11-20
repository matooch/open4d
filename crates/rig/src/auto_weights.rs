//! Auto weights by bone heat diffusion (docs/specs/RIG.md §4).
//!
//! Baran and Popovic, "Automatic rigging and animation of 3D characters" (2007): for every
//! bone `i` solve `(L + D H_i) w_i = D H_i p_i` on the mesh, with `L` the cotangent
//! stiffness matrix, `D` the vertex areas, `H_i` the heat of the vertices whose nearest visible
//! bone is `i` (`1 / d²`), `p_i` the indicator of those vertices.

use std::collections::BTreeMap;

use forge_core::{EditError, NodeId, Scene, Tx};
use forge_geometry::PolyMesh;
use forge_geometry::triangulate::triangulate;
use glam::{DMat4, DVec3};

use crate::joint::{first_child_joint, is_joint};
use crate::weights::{WeightMap, bind, find_weight_tag, tag_joints};

/// One bone, a segment (a point when `a == b`), in the mesh's space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bone {
    pub a: DVec3,
    pub b: DVec3,
}

impl Bone {
    /// The closest point of the segment to `p`.
    pub fn closest(&self, p: DVec3) -> DVec3 {
        let ab = self.b - self.a;
        let len2 = ab.length_squared();
        if len2 == 0.0 {
            return self.a;
        }
        self.a + ab * ((p - self.a).dot(ab) / len2).clamp(0.0, 1.0)
    }
}

/// Settings of the solve.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AutoWeightOptions {
    /// The heat constant `c` in `h = c / d²` (1 in the paper).
    pub heat: f64,
    /// Only a bone that sees the vertex through the mesh interior may heat it.
    pub visibility: bool,
    /// Weights below this are dropped before normalising (0 keeps all).
    pub prune: f64,
}

impl Default for AutoWeightOptions {
    fn default() -> Self {
        AutoWeightOptions {
            heat: 1.0,
            visibility: true,
            prune: 0.0,
        }
    }
}

type Row = Vec<(usize, f64)>;

/// The cotangent stiffness matrix as sparse rows, and the barycentric vertex areas.
fn laplacian(mesh: &PolyMesh) -> (Vec<Row>, Vec<f64>) {
    let n = mesh.points.len();
    let mut edges: Vec<BTreeMap<usize, f64>> = vec![BTreeMap::new(); n];
    let mut area = vec![0.0; n];
    for (_, t) in triangulate(mesh) {
        let p = t.map(|i| mesh.points[i as usize]);
        let a2 = (p[1] - p[0]).cross(p[2] - p[0]).length();
        if a2 < 1e-18 {
            continue;
        }
        for k in 0..3 {
            let (i, j, o) = (
                t[k] as usize,
                t[(k + 1) % 3] as usize,
                t[(k + 2) % 3] as usize,
            );
            area[i] += a2 / 6.0;
            // the angle at `o` is opposite the edge i-j
            let u = mesh.points[i] - mesh.points[o];
            let v = mesh.points[j] - mesh.points[o];
            let cot = u.dot(v) / u.cross(v).length();
            // clamp: a negative weight would make the matrix indefinite
            let w = (0.5 * cot).max(0.0);
            *edges[i].entry(j).or_insert(0.0) += w;
            *edges[j].entry(i).or_insert(0.0) += w;
        }
    }
    let rows = edges
        .into_iter()
        .enumerate()
        .map(|(i, e)| {
            let diag: f64 = e.values().sum();
            let mut row: Row = vec![(i, diag)];
            row.extend(e.into_iter().map(|(j, w)| (j, -w)));
            row
        })
        .collect();
    (rows, area)
}

/// Conjugate gradients with a Jacobi preconditioner on `rows · x = rhs` (symmetric positive
/// definite). Returns `x`.
fn solve(rows: &[Row], diag_extra: &[f64], rhs: &[f64]) -> Vec<f64> {
    let n = rhs.len();
    let mul = |x: &[f64], out: &mut [f64]| {
        for (i, row) in rows.iter().enumerate() {
            let mut s = diag_extra[i] * x[i];
            for (j, a) in row {
                s += a * x[*j];
            }
            out[i] = s;
        }
    };
    let inv_diag: Vec<f64> = rows
        .iter()
        .enumerate()
        .map(|(i, r)| {
            let d = r[0].1 + diag_extra[i];
            if d > 0.0 { 1.0 / d } else { 1.0 }
        })
        .collect();
    let mut x = vec![0.0; n];
    let mut r = rhs.to_vec();
    let mut z: Vec<f64> = r.iter().zip(&inv_diag).map(|(a, b)| a * b).collect();
    let mut p = z.clone();
    let mut rz: f64 = r.iter().zip(&z).map(|(a, b)| a * b).sum();
    let norm0 = rhs.iter().map(|v| v * v).sum::<f64>().sqrt();
    if norm0 == 0.0 {
        return x;
    }
    let mut ap = vec![0.0; n];
    for _ in 0..(10 * n).clamp(100, 5000) {
        mul(&p, &mut ap);
        let pap: f64 = p.iter().zip(&ap).map(|(a, b)| a * b).sum();
        if pap <= 0.0 {
            break;
        }
        let alpha = rz / pap;
        for i in 0..n {
            x[i] += alpha * p[i];
            r[i] -= alpha * ap[i];
        }
        if r.iter().map(|v| v * v).sum::<f64>().sqrt() <= 1e-12 * norm0 {
            break;
        }
        for i in 0..n {
            z[i] = r[i] * inv_diag[i];
        }
        let rz_new: f64 = r.iter().zip(&z).map(|(a, b)| a * b).sum();
        let beta = rz_new / rz;
        rz = rz_new;
        for i in 0..n {
            p[i] = z[i] + beta * p[i];
        }
    }
    x
}

/// True when the segment `from`..`to` crosses a triangle that does not use vertex `skip`.
fn blocked(mesh: &PolyMesh, tris: &[[u32; 3]], skip: u32, from: DVec3, to: DVec3) -> bool {
    let d = to - from;
    for t in tris {
        if t.contains(&skip) {
            continue;
        }
        let [a, b, c] = t.map(|i| mesh.points[i as usize]);
        let e1 = b - a;
        let e2 = c - a;
        let h = d.cross(e2);
        let det = e1.dot(h);
        if det.abs() < 1e-14 {
            continue;
        }
        let s = from - a;
        let u = s.dot(h) / det;
        if !(0.0..=1.0).contains(&u) {
            continue;
        }
        let q = s.cross(e1);
        let v = d.dot(q) / det;
        if v < 0.0 || u + v > 1.0 {
            continue;
        }
        let t_hit = e2.dot(q) / det;
        if t_hit > 1e-6 && t_hit < 1.0 - 1e-6 {
            return true;
        }
    }
    false
}

/// Bone heat weights of `mesh` for `bones`: slot `i` is bone `i`. Rows sum to 1; a vertex
/// that no heat reaches (a separate component without a visible bone) belongs to its nearest
/// bone. Deterministic.
pub fn auto_weights(mesh: &PolyMesh, bones: &[Bone], opts: &AutoWeightOptions) -> WeightMap {
    let n = mesh.points.len();
    if n == 0 || bones.is_empty() {
        return WeightMap::new();
    }
    let (rows, area) = laplacian(mesh);
    let tris: Vec<[u32; 3]> = triangulate(mesh).into_iter().map(|(_, t)| t).collect();
    // nearest visible bone per vertex, and the nearest bone ignoring visibility
    let mut heat_bone: Vec<Option<(usize, f64)>> = vec![None; n];
    let mut nearest: Vec<usize> = vec![0; n];
    let floor = {
        let (lo, hi) = mesh.points.iter().fold(
            (DVec3::splat(f64::MAX), DVec3::splat(f64::MIN)),
            |(l, h), p| (l.min(*p), h.max(*p)),
        );
        1e-4 * (hi - lo).length().max(1e-9)
    };
    for (v, p) in mesh.points.iter().enumerate() {
        let mut order: Vec<(f64, usize)> = bones
            .iter()
            .enumerate()
            .map(|(i, b)| ((b.closest(*p) - *p).length(), i))
            .collect();
        order.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
        nearest[v] = order[0].1;
        for (d, i) in &order {
            let seen =
                !opts.visibility || !blocked(mesh, &tris, v as u32, *p, bones[*i].closest(*p));
            if seen {
                heat_bone[v] = Some((*i, d.max(floor)));
                break;
            }
        }
    }
    // `H` is shared by every bone: each vertex is heated by its nearest visible bone only; the
    // indicator `p_b` marks which bone that is.
    let dh: Vec<f64> = heat_bone
        .iter()
        .zip(&area)
        .map(|(hb, a)| hb.map_or(0.0, |(_, d)| opts.heat / (d * d) * a))
        .collect();
    let mut dense = vec![vec![0.0; bones.len()]; n];
    for b in 0..bones.len() {
        let rhs: Vec<f64> = heat_bone
            .iter()
            .zip(&dh)
            .map(|(hb, h)| match hb {
                Some((i, _)) if *i == b => *h,
                _ => 0.0,
            })
            .collect();
        if rhs.iter().all(|x| *x == 0.0) {
            continue;
        }
        let w = solve(&rows, &dh, &rhs);
        for (row, w) in dense.iter_mut().zip(w) {
            row[b] = w.max(0.0);
        }
    }
    for (v, row) in dense.iter_mut().enumerate() {
        if opts.prune > 0.0 {
            let sum: f64 = row.iter().sum();
            if sum > 0.0 {
                for w in row.iter_mut() {
                    if *w / sum < opts.prune {
                        *w = 0.0;
                    }
                }
            }
        }
        let sum: f64 = row.iter().sum();
        if sum > 1e-12 {
            for w in row.iter_mut() {
                *w /= sum;
            }
        } else {
            row.iter_mut().for_each(|w| *w = 0.0);
            row[nearest[v]] = 1.0;
        }
    }
    WeightMap::from_dense(&dense)
}

/// The bones of `joints` in `object`'s space: each joint's world position to its first child
/// joint's, a point for a joint without a child joint. Missing joints become a point at the
/// origin.
pub fn bones_for_joints(scene: &Scene, object: NodeId, joints: &[NodeId]) -> Vec<Bone> {
    let inv = scene
        .world_matrix(object)
        .map_or(DMat4::IDENTITY, |m| m.inverse());
    joints
        .iter()
        .map(|j| {
            let at = |id: NodeId| {
                scene
                    .world_matrix(id)
                    .map_or(DVec3::ZERO, |m| inv.transform_point3(m.w_axis.truncate()))
            };
            let a = at(*j);
            let b = if is_joint(scene, *j) {
                first_child_joint(scene, *j).map_or(a, at)
            } else {
                a
            };
            Bone { a, b }
        })
        .collect()
}

/// Computes bone heat weights for the weight tag's joints against `mesh` (the host object's
/// unskinned mesh), stores them on the tag and binds the current pose.
///
/// # Errors
/// Fails for a node that is not a weight tag with a host.
pub fn auto_bind(
    tx: &mut Tx<'_>,
    tag: NodeId,
    mesh: &PolyMesh,
    opts: &AutoWeightOptions,
) -> Result<(), EditError> {
    let scene = tx.scene();
    let host = scene
        .node(tag)
        .and_then(|n| n.host)
        .ok_or(EditError::UnknownNode(tag))?;
    debug_assert!(find_weight_tag(scene, host).is_some());
    let joints = tag_joints(scene, tag);
    let bones = bones_for_joints(scene, host, &joints);
    let map = auto_weights(mesh, &bones, opts);
    tx.set_param(tag, "weights", map.to_value())?;
    bind(tx, tag)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cube() -> PolyMesh {
        let p = |x: f64, y: f64, z: f64| DVec3::new(x, y, z);
        PolyMesh {
            points: vec![
                p(0.0, 0.0, 0.0),
                p(10.0, 0.0, 0.0),
                p(10.0, 10.0, 0.0),
                p(0.0, 10.0, 0.0),
                p(0.0, 0.0, 10.0),
                p(10.0, 0.0, 10.0),
                p(10.0, 10.0, 10.0),
                p(0.0, 10.0, 10.0),
            ],
            polys: vec![
                vec![0, 3, 2, 1],
                vec![4, 5, 6, 7],
                vec![0, 1, 5, 4],
                vec![2, 3, 7, 6],
                vec![1, 2, 6, 5],
                vec![3, 0, 4, 7],
            ],
        }
    }

    #[test]
    fn visibility_is_blocked_by_the_far_side_of_the_mesh() {
        let m = cube();
        let tris: Vec<[u32; 3]> = triangulate(&m).into_iter().map(|(_, t)| t).collect();
        let outside = DVec3::new(20.0, 5.0, 5.0);
        // vertex 0 (x = 0) looks through the cube at a point beyond x = 10
        assert!(blocked(&m, &tris, 0, m.points[0], outside));
        // vertex 1 (x = 10) sees it directly
        assert!(!blocked(&m, &tris, 1, m.points[1], outside));
        // a bone inside is visible from every vertex
        assert!(!blocked(&m, &tris, 0, m.points[0], DVec3::splat(5.0)));
    }

    #[test]
    fn the_cotangent_laplacian_has_zero_row_sums() {
        let (rows, area) = laplacian(&cube());
        for r in &rows {
            let s: f64 = r.iter().map(|(_, a)| a).sum();
            assert!(s.abs() < 1e-12);
        }
        assert!((area.iter().sum::<f64>() - 600.0).abs() < 1e-9);
    }

    #[test]
    fn a_single_bone_takes_every_weight() {
        let w = auto_weights(
            &cube(),
            &[Bone {
                a: DVec3::splat(5.0),
                b: DVec3::splat(5.0),
            }],
            &AutoWeightOptions::default(),
        );
        for p in 0..8 {
            assert_eq!(w.get(p), &[(0, 1.0)]);
        }
    }
}
