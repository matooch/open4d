//! docs/specs/GEOMETRY.md §3.

mod common;

use std::f64::consts::PI;

use common::{assert_vec_eq, cube, mesh};
use forge_geometry::normals::{Weighting, corner_normals, poly_area, poly_normal, vertex_normals};
use glam::DVec3;

#[test]
fn cube_poly_normals_point_outward() {
    let m = cube();
    let want = [
        DVec3::NEG_Z,
        DVec3::Z,
        DVec3::NEG_Y,
        DVec3::Y,
        DVec3::NEG_X,
        DVec3::X,
    ];
    for (poly, n) in m.polys.iter().zip(want) {
        assert_vec_eq(poly_normal(&m.points, poly), n);
        assert!((poly_area(&m.points, poly) - 1.0).abs() < 1e-12);
    }
}

#[test]
fn newell_handles_concave_ngons_and_degenerates() {
    let l = mesh(
        &[
            [0., 0., 0.],
            [2., 0., 0.],
            [2., 1., 0.],
            [1., 1., 0.],
            [1., 2., 0.],
            [0., 2., 0.],
        ],
        &[&[0, 1, 2, 3, 4, 5], &[0, 1, 1]],
    );
    assert_vec_eq(poly_normal(&l.points, &l.polys[0]), DVec3::Z);
    assert!((poly_area(&l.points, &l.polys[0]) - 3.0).abs() < 1e-12);
    let line = [DVec3::ZERO, DVec3::X, DVec3::X * 2.0];
    assert_eq!(poly_normal(&line, &[0, 1, 2]), DVec3::ZERO);
}

/// Vertex 0 shared by a big right-angle triangle (+z) and a small 45° one (-x).
fn two_tris() -> forge_geometry::PolyMesh {
    mesh(
        &[
            [0., 0., 0.],
            [2., 0., 0.],
            [0., 2., 0.],
            [0., 0., 1.],
            [0., 1., 1.],
            [9., 9., 9.],
        ],
        &[&[0, 1, 2], &[0, 3, 4]],
    )
}

#[test]
fn weightings_follow_their_definitions() {
    let m = two_tris();
    let at0 = |w| vertex_normals(&m, w)[0];
    assert_vec_eq(at0(Weighting::Uniform), DVec3::new(-1., 0., 1.).normalize());
    assert_vec_eq(at0(Weighting::Area), DVec3::new(-1., 0., 4.).normalize());
    assert_vec_eq(at0(Weighting::Angle), DVec3::new(-1., 0., 2.).normalize());
    assert_eq!(Weighting::default(), Weighting::Angle);
    assert_eq!(
        vertex_normals(&m, Weighting::Angle)[5],
        DVec3::ZERO,
        "unused vertex"
    );
}

#[test]
fn cube_corner_vertex_normal_is_symmetric() {
    let n = vertex_normals(&cube(), Weighting::Angle);
    assert_vec_eq(n[0], DVec3::splat(-1.0).normalize());
    assert_vec_eq(n[6], DVec3::splat(1.0).normalize());
}

#[test]
fn smoothing_angle_splits_or_merges() {
    let m = cube();
    let flat = corner_normals(&m, 89f64.to_radians(), Weighting::Angle);
    for (poly, corners) in m.polys.iter().zip(&flat) {
        assert_eq!(corners.len(), poly.len());
        for c in corners {
            assert_vec_eq(*c, poly_normal(&m.points, poly));
        }
    }
    let smooth = vertex_normals(&m, Weighting::Angle);
    for angle in [90f64.to_radians(), PI] {
        let cn = corner_normals(&m, angle, Weighting::Angle);
        for (poly, corners) in m.polys.iter().zip(&cn) {
            for (v, c) in poly.iter().zip(corners) {
                assert_vec_eq(*c, smooth[*v as usize]);
            }
        }
    }
}
