//! docs/specs/BOOLE.md §2–4: boolean operations on simple boxes.

use std::collections::BTreeMap;

use forge_geometry::PolyMesh;
use forge_geometry::boolean::{BoolOp, BoolOptions, BoolResult, boolean};
use forge_parity_harness::{Mesh, Tolerance, compare_geometric};
use glam::DVec3;

/// Axis-aligned box, outward counter-clockwise quads.
fn boxm(lo: [f64; 3], hi: [f64; 3]) -> PolyMesh {
    let points = (0..8)
        .map(|i| {
            DVec3::new(
                if i & 1 == 0 { lo[0] } else { hi[0] },
                if i & 2 == 0 { lo[1] } else { hi[1] },
                if i & 4 == 0 { lo[2] } else { hi[2] },
            )
        })
        .collect();
    let polys = vec![
        vec![0, 2, 3, 1],
        vec![4, 5, 7, 6],
        vec![0, 1, 5, 4],
        vec![2, 6, 7, 3],
        vec![0, 4, 6, 2],
        vec![1, 3, 7, 5],
    ];
    PolyMesh { points, polys }
}

fn cube(c: [f64; 3], half: f64) -> PolyMesh {
    boxm(
        [c[0] - half, c[1] - half, c[2] - half],
        [c[0] + half, c[1] + half, c[2] + half],
    )
}

fn run(a: &PolyMesh, b: &PolyMesh, op: BoolOp, hide: bool) -> BoolResult {
    let r = boolean(
        a,
        b,
        &BoolOptions {
            op,
            hide_new_edges: hide,
        },
    );
    r.mesh.validate().expect("valid polygons");
    assert_eq!(r.from_b.len(), r.mesh.polys.len());
    r
}

fn volume(m: &PolyMesh) -> f64 {
    m.polys
        .iter()
        .map(|f| {
            let p = |i: usize| m.points[f[i] as usize];
            (1..f.len() - 1)
                .map(|i| p(0).dot(p(i).cross(p(i + 1))) / 6.0)
                .sum::<f64>()
        })
        .sum()
}

fn closed(m: &PolyMesh) -> bool {
    let mut e: BTreeMap<(u32, u32), usize> = BTreeMap::new();
    for f in &m.polys {
        for i in 0..f.len() {
            *e.entry((f[i], f[(i + 1) % f.len()])).or_default() += 1;
        }
    }
    e.iter()
        .all(|(&(a, b), &n)| n == 1 && e.get(&(b, a)) == Some(&1))
}

fn harness(m: &PolyMesh) -> Mesh {
    Mesh {
        positions: m.points.iter().map(|p| p.to_array()).collect(),
        faces: m.polys.clone(),
    }
}

fn same_solid(a: &PolyMesh, b: &PolyMesh) {
    let r = compare_geometric(&harness(a), &harness(b), &Tolerance::default());
    assert!(r.passed(), "{:?}", r.failures);
}

fn close(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-6 * b.abs().max(1.0), "{a} vs {b}");
}

const A: [f64; 3] = [0.0; 3];
const B: [f64; 3] = [50.0; 3];

#[test]
fn overlapping_cubes_all_modes() {
    let (a, b) = (cube(A, 100.0), cube(B, 100.0));
    let big = 200f64.powi(3);
    let lens = 150f64.powi(3);

    let u = run(&a, &b, BoolOp::Union, false);
    assert_eq!(u.mesh.points.len(), 20);
    assert_eq!(u.mesh.polys.len(), 18, "9 per cube: each L in two quads");
    assert!(closed(&u.mesh));
    close(volume(&u.mesh), 2.0 * big - lens);

    let s = run(&a, &b, BoolOp::Subtract, false);
    assert_eq!(s.mesh.points.len(), 14);
    assert!(closed(&s.mesh));
    close(volume(&s.mesh), big - lens);
    assert!(
        s.mesh.points.contains(&DVec3::splat(-50.0)),
        "new corner of B inside A"
    );
    assert_eq!(s.from_b.iter().filter(|b| **b).count(), 3);

    let i = run(&a, &b, BoolOp::Intersect, false);
    same_solid(&i.mesh, &boxm([-50.0; 3], [100.0; 3]));
    assert_eq!(i.mesh.polys.len(), 6);

    let w = run(&a, &b, BoolOp::Without, false);
    assert_eq!(w.mesh.points.len(), 13);
    assert!(!closed(&w.mesh), "left open");
    assert!(w.from_b.iter().all(|b| !b));
}

#[test]
fn hide_new_edges_keeps_each_cut_face_whole() {
    let (a, b) = (cube(A, 100.0), cube(B, 100.0));
    let hide = run(&a, &b, BoolOp::Subtract, true);
    let sizes: Vec<usize> = hide.mesh.polys.iter().map(Vec::len).collect();
    assert_eq!(hide.mesh.polys.len(), 9);
    assert_eq!(sizes.iter().filter(|&&n| n == 6).count(), 3, "{sizes:?}");
    let split = run(&a, &b, BoolOp::Subtract, false);
    assert!(split.mesh.polys.iter().all(|p| p.len() <= 4));
    assert_eq!(split.mesh.polys.len(), 12, "each L in two quads");
    same_solid(&hide.mesh, &split.mesh);
}

#[test]
fn disjoint_cubes() {
    let (a, b) = (cube(A, 100.0), cube([500.0, 0.0, 0.0], 100.0));
    same_solid(&run(&a, &b, BoolOp::Subtract, true).mesh, &a);
    same_solid(&run(&a, &b, BoolOp::Without, true).mesh, &a);
    let u = run(&a, &b, BoolOp::Union, true);
    assert_eq!((u.mesh.points.len(), u.mesh.polys.len()), (16, 12));
    assert!(run(&a, &b, BoolOp::Intersect, true).mesh.polys.is_empty());
}

#[test]
fn touching_cubes_keep_a_whole_when_subtracting() {
    // B's -X face lies exactly on A's +X face (observed: A stays a whole cube).
    let (a, b) = (cube(A, 100.0), cube([200.0, 0.0, 0.0], 100.0));
    let s = run(&a, &b, BoolOp::Subtract, true);
    same_solid(&s.mesh, &a);
    let u = run(&a, &b, BoolOp::Union, true);
    assert_eq!(u.mesh.polys.len(), 10, "the touching faces vanish");
    assert!(closed(&u.mesh));
    close(volume(&u.mesh), 2.0 * 200f64.powi(3));
    assert!(run(&a, &b, BoolOp::Intersect, true).mesh.polys.is_empty());
}

#[test]
fn coplanar_same_side_faces() {
    // A = [0,2]³, B = [1,3]×[0,2]×[0,2]: four faces of each are coplanar.
    let (a, b) = (
        boxm([0.0; 3], [2.0; 3]),
        boxm([1.0, 0.0, 0.0], [3.0, 2.0, 2.0]),
    );
    let u = run(&a, &b, BoolOp::Union, true);
    assert!(closed(&u.mesh));
    close(volume(&u.mesh), 12.0);
    same_solid(
        &run(&a, &b, BoolOp::Intersect, true).mesh,
        &boxm([1.0, 0.0, 0.0], [2.0; 3]),
    );
    same_solid(
        &run(&a, &b, BoolOp::Subtract, true).mesh,
        &boxm([0.0; 3], [1.0, 2.0, 2.0]),
    );
}

#[test]
fn nested_cube_leaves_a_cavity() {
    let (a, b) = (cube(A, 100.0), cube([10.0, 0.0, 0.0], 20.0));
    let s = run(&a, &b, BoolOp::Subtract, true);
    assert_eq!((s.mesh.points.len(), s.mesh.polys.len()), (16, 12));
    assert!(closed(&s.mesh));
    close(volume(&s.mesh), 200f64.powi(3) - 40f64.powi(3));
    same_solid(&run(&a, &b, BoolOp::Union, true).mesh, &a);
    same_solid(&run(&a, &b, BoolOp::Intersect, true).mesh, &b);
}

#[test]
fn tunnel_gives_faces_with_holes() {
    let a = cube(A, 100.0);
    let b = boxm([-50.0, -50.0, -200.0], [50.0, 50.0, 200.0]);
    for hide in [false, true] {
        let s = run(&a, &b, BoolOp::Subtract, hide);
        assert_eq!(s.mesh.points.len(), 16, "hide {hide}");
        assert!(closed(&s.mesh), "hide {hide}");
        close(volume(&s.mesh), 200f64.powi(3) - 100.0 * 100.0 * 200.0);
        let n = DVec3::Z;
        let top_area: f64 = s
            .mesh
            .polys
            .iter()
            .filter(|p| p.iter().all(|&v| s.mesh.points[v as usize].z == 100.0))
            .map(|p| {
                let q = |i: usize| s.mesh.points[p[i] as usize];
                (1..p.len() - 1)
                    .map(|i| (q(i) - q(0)).cross(q(i + 1) - q(0)).dot(n) / 2.0)
                    .sum::<f64>()
            })
            .sum();
        close(top_area, 200.0 * 200.0 - 100.0 * 100.0);
    }
}

#[test]
fn non_convex_input_polygons() {
    // L-shaped prism (two 6-gon caps) minus a box through its corner.
    let l = [
        [0.0, 0.0],
        [2.0, 0.0],
        [2.0, 1.0],
        [1.0, 1.0],
        [1.0, 2.0],
        [0.0, 2.0],
    ];
    let mut points: Vec<DVec3> = l.iter().map(|p| DVec3::new(p[0], p[1], 0.0)).collect();
    points.extend(l.iter().map(|p| DVec3::new(p[0], p[1], 1.0)));
    let mut polys = vec![(0..6).rev().collect::<Vec<u32>>(), (6..12).collect()];
    for i in 0..6u32 {
        let j = (i + 1) % 6;
        polys.push(vec![i, j, j + 6, i + 6]);
    }
    let prism = PolyMesh { points, polys };
    close(volume(&prism), 3.0);
    let b = boxm([-1.0, -1.0, -1.0], [0.5, 0.5, 2.0]);
    let s = run(&prism, &b, BoolOp::Subtract, true);
    assert!(closed(&s.mesh));
    close(volume(&s.mesh), 3.0 - 0.25);
    let u = run(&prism, &b, BoolOp::Union, false);
    assert!(closed(&u.mesh));
    close(volume(&u.mesh), 3.0 + 1.5 * 1.5 * 3.0 - 0.25);
}

#[test]
fn empty_operands() {
    let a = cube(A, 100.0);
    let none = PolyMesh::default();
    same_solid(&run(&a, &none, BoolOp::Subtract, false).mesh, &a);
    same_solid(&run(&a, &none, BoolOp::Union, false).mesh, &a);
    assert!(
        run(&a, &none, BoolOp::Intersect, false)
            .mesh
            .polys
            .is_empty()
    );
    same_solid(&run(&none, &a, BoolOp::Union, false).mesh, &a);
}
