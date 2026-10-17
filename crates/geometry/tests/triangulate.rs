//! docs/specs/GEOMETRY.md §6.

mod common;

use forge_geometry::normals::{poly_area, poly_normal};
use forge_geometry::triangulate::{triangulate, triangulate_poly};
use glam::DVec3;

fn check(points: &[DVec3], poly: &[u32]) -> Vec<[u32; 3]> {
    let tris = triangulate_poly(points, poly);
    assert_eq!(tris.len(), poly.len() - 2);
    let n = poly_normal(points, poly);
    let mut area = 0.0;
    for t in &tris {
        assert!(
            t.iter().all(|v| poly.contains(v)),
            "{t:?} uses foreign vertices"
        );
        let tn = poly_normal(points, t);
        assert!(tn.dot(n) > 0.0, "{t:?} is flipped");
        area += poly_area(points, t);
    }
    assert!(
        (area - poly_area(points, poly)).abs() < 1e-9,
        "areas differ"
    );
    tris
}

fn flat(pts: &[[f64; 2]]) -> Vec<DVec3> {
    pts.iter().map(|p| DVec3::new(p[0], p[1], 0.0)).collect()
}

#[test]
fn triangles_pass_through() {
    let p = flat(&[[0., 0.], [1., 0.], [0., 1.]]);
    assert_eq!(triangulate_poly(&p, &[2, 0, 1]), vec![[2, 0, 1]]);
}

#[test]
fn convex_and_concave_ngons() {
    let quad = flat(&[[0., 0.], [1., 0.], [1., 1.], [0., 1.]]);
    check(&quad, &[0, 1, 2, 3]);
    // L shape (one reflex corner at vertex 3)
    let l = flat(&[[0., 0.], [2., 0.], [2., 1.], [1., 1.], [1., 2.], [0., 2.]]);
    let tris = check(&l, &[0, 1, 2, 3, 4, 5]);
    // no triangle may cover the notch at (1.5, 1.5)
    let notch = DVec3::new(1.5, 1.5, 0.0);
    for t in tris {
        let [a, b, c] = t.map(|i| l[i as usize]);
        let s = |p: DVec3, q: DVec3| (q - p).cross(notch - p).z;
        assert!(
            !(s(a, b) > 0.0 && s(b, c) > 0.0 && s(c, a) > 0.0),
            "notch covered by {t:?}"
        );
    }
    // star with 5 reflex corners
    let star: Vec<[f64; 2]> = (0..10)
        .map(|i| {
            let r = if i % 2 == 0 { 2.0 } else { 0.8 };
            let a = i as f64 * std::f64::consts::PI / 5.0;
            [r * a.cos(), r * a.sin()]
        })
        .collect();
    check(&flat(&star), &(0..10).collect::<Vec<u32>>());
}

#[test]
fn works_in_any_plane_and_winding() {
    // quad in the YZ plane, wound clockwise when seen from +X
    let p: Vec<DVec3> = [
        [0., 0., 0.],
        [0., 0., 1.],
        [0., 1., 1.],
        [0., 1., 0.],
        [0., 0.5, 0.5],
    ]
    .iter()
    .map(|a| DVec3::from_array(*a))
    .collect();
    check(&p, &[0, 1, 2, 3]);
    check(&p, &[3, 2, 1, 0]);
}

#[test]
fn degenerate_input_falls_back_to_a_fan() {
    let line = flat(&[[0., 0.], [1., 0.], [2., 0.], [3., 0.]]);
    assert_eq!(
        triangulate_poly(&line, &[0, 1, 2, 3]),
        vec![[0, 1, 2], [0, 2, 3]]
    );
}

#[test]
fn whole_mesh_keeps_poly_ids() {
    let tris = triangulate(&common::cube());
    assert_eq!(tris.len(), 12);
    assert!(tris.iter().filter(|(p, _)| *p == 5).count() == 2);
}
