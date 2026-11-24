//! U7 native geometric acceptance: conformal cylinder and UV island packing.

use std::collections::BTreeSet;
use std::f64::consts::TAU;

use forge_geometry::mesh::Edge;
use forge_geometry::uv_edit::{pack, relax};
use forge_geometry::{CornerUvs, PolyMesh};
use glam::{DVec2, DVec3};

#[test]
fn cut_cylinder_relaxes_near_isometrically() {
    let n = 24u32;
    let mut mesh = PolyMesh::default();
    for h in 0..=4 {
        for i in 0..n {
            let (s, c) = (TAU * f64::from(i) / f64::from(n)).sin_cos();
            mesh.points.push(DVec3::new(c, f64::from(h) * 0.5, s));
        }
    }
    for h in 0..4 {
        for i in 0..n {
            let a = h * n + i;
            let b = h * n + (i + 1) % n;
            mesh.polys.push(vec![a, b, b + n, a + n]);
        }
    }
    let edges = mesh.edge_ids();
    let seams = (0..4)
        .map(|h| edges[&Edge::new(h * n, (h + 1) * n)])
        .collect();
    let uv = relax(&mesh, None, &seams).unwrap();
    assert!(uv.matches(&mesh));
    assert!(
        DVec2::from(uv.corners[0][0]).distance(DVec2::from(uv.corners[n as usize - 1][1])) > 1.0
    );
    let mut lo = f64::INFINITY;
    let mut hi: f64 = 0.0;
    for (face, corners) in mesh.polys.iter().zip(&uv.corners) {
        for i in 0..4 {
            let j = (i + 1) % 4;
            let ratio = DVec2::from(corners[i]).distance(DVec2::from(corners[j]))
                / mesh.points[face[i] as usize].distance(mesh.points[face[j] as usize]);
            lo = lo.min(ratio);
            hi = hi.max(ratio);
        }
    }
    assert!(lo > 0.0 && hi / lo < 1.00001, "relative stretch {lo}–{hi}");
}

#[test]
fn skyline_packs_rectangles_with_padding_and_preserves_relative_scale() {
    let mut mesh = PolyMesh::default();
    let mut uv = CornerUvs::default();
    for (w, h) in [(3.0, 1.0), (1.0, 3.0), (2.5, 0.75), (1.3, 2.4), (4.0, 1.0)] {
        let start = mesh.points.len() as u32;
        let shape = [[0.0, 0.0], [w, 0.0], [w, h], [0.0, h]];
        mesh.points
            .extend(shape.map(|[u, v]| DVec3::new(u, v, 0.0)));
        mesh.polys.push((start..start + 4).collect());
        uv.corners.push(shape.to_vec());
    }
    let result = pack(&mesh, &uv, &BTreeSet::new(), 0.025).unwrap();
    let bounds: Vec<_> = result
        .corners
        .iter()
        .zip(&uv.corners)
        .map(|(p, old)| {
            let (lo, hi) = (DVec2::from(p[0]), DVec2::from(p[2]));
            assert!(lo.cmpge(DVec2::splat(0.025 - 1e-9)).all());
            assert!(hi.cmple(DVec2::splat(0.975 + 1e-9)).all());
            (lo, hi, (hi - lo) / DVec2::from(old[2]))
        })
        .collect();
    for (i, &(lo, hi, scale)) in bounds.iter().enumerate() {
        assert!(scale.abs_diff_eq(bounds[0].2, 1e-9));
        assert!((scale.x - scale.y).abs() < 1e-9);
        for &(b, t, _) in &bounds[..i] {
            let gap = (lo - t).max(b - hi);
            assert!(gap.max_element() >= 0.05 - 1e-9, "packed padding: {gap:?}");
        }
    }
    assert_eq!(result, pack(&mesh, &uv, &BTreeSet::new(), 0.025).unwrap());
}
