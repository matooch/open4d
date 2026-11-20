//! Mesh, spline and points to SDF; CSG (docs/specs/VOLUME.md sections 2-4).

mod common;

use common::{cube, sphere};
use forge_volume::{
    CsgOp, PointUnion, SdfParams, SparseGrid, VolumeError, combine, mesh_to_sdf, negate,
    points_to_sdf, smooth_min, spline_to_sdf,
};
use glam::{DVec3, IVec3};

fn params(voxel: f64) -> SdfParams {
    SdfParams {
        voxel_size: voxel,
        ..SdfParams::default()
    }
}

#[test]
fn sphere_sdf_error_is_below_a_tenth_of_a_voxel() {
    for voxel in [10.0, 5.0] {
        let p = params(voxel);
        let g = mesh_to_sdf(&sphere(100.0, 48, 64, DVec3::new(3.0, -2.0, 1.0)), &p);
        assert!(g.active_count() > 500);
        let mut worst: f64 = 0.0;
        for (i, v) in g.active_voxels() {
            let w = g.index_to_world(i);
            let exact = (w - DVec3::new(3.0, -2.0, 1.0)).length() - 100.0;
            worst = worst.max((f64::from(v) - exact).abs());
        }
        assert!(worst < 0.1 * voxel, "voxel {voxel}: error {worst}");
    }
}

#[test]
fn band_is_two_voxels_each_side_and_the_interior_is_filled() {
    let p = params(10.0);
    let g = mesh_to_sdf(&sphere(100.0, 24, 32, DVec3::ZERO), &p);
    assert_eq!(g.background(), 20.0);
    for (i, v) in g.active_voxels() {
        assert!((-20.0..=20.0).contains(&v), "{i} {v}");
    }
    // deep inside: stored inactive at -interior band; far outside: background
    assert_eq!(g.get(IVec3::ZERO), -20.0);
    assert!(!g.is_active(IVec3::ZERO));
    assert_eq!(g.get(IVec3::new(30, 0, 0)), 20.0);
    // a voxel on the surface is exactly zero and not negative
    let c = mesh_to_sdf(&cube(DVec3::splat(100.0), DVec3::ZERO), &p);
    let v = c.get(IVec3::new(-10, 0, 0));
    assert_eq!(v, 0.0);
    assert!(v.is_sign_positive());
    assert_eq!(c.get(IVec3::new(-9, 0, 0)), -10.0);
    assert_eq!(c.get(IVec3::new(-11, 0, 0)), 10.0);
}

#[test]
fn an_open_mesh_still_has_unsigned_distance() {
    // one quad: no inside, every voxel in the band is positive
    let mesh = forge_geometry::PolyMesh {
        points: vec![
            DVec3::new(-50.0, 5.0, -50.0),
            DVec3::new(50.0, 5.0, -50.0),
            DVec3::new(50.0, 5.0, 50.0),
            DVec3::new(-50.0, 5.0, 50.0),
        ],
        polys: vec![vec![0, 1, 2, 3]],
    };
    let g = mesh_to_sdf(&mesh, &params(10.0));
    let v = g.get(IVec3::new(0, 0, 0));
    assert!((v - 5.0).abs() < 1e-5);
    assert!(g.active_voxels().iter().all(|(_, v)| *v >= 0.0));
}

#[test]
fn cube_csg() {
    let p = params(10.0);
    let a = mesh_to_sdf(&cube(DVec3::splat(50.0), DVec3::ZERO), &p);
    let b = mesh_to_sdf(&cube(DVec3::splat(50.0), DVec3::new(50.0, 0.0, 0.0)), &p);
    let at = |g: &SparseGrid<f32>, x: i32, y: i32, z: i32| g.get(IVec3::new(x, y, z));

    let u = combine(&a, &b, CsgOp::Union).unwrap();
    let i = combine(&a, &b, CsgOp::Intersect).unwrap();
    let s = combine(&a, &b, CsgOp::Subtract).unwrap();
    // x = -3 in a only, x = 8 in b only, x = 2 in both (a spans -5..5, b spans 0..10)
    assert!(at(&u, -3, 0, 0) < 0.0 && at(&u, 8, 0, 0) < 0.0 && at(&u, 2, 0, 0) < 0.0);
    assert!(at(&u, -8, 0, 0) > 0.0 && at(&u, 13, 0, 0) > 0.0);
    assert!(at(&i, 2, 0, 0) < 0.0);
    assert!(at(&i, -3, 0, 0) > 0.0 && at(&i, 8, 0, 0) > 0.0);
    assert!(at(&s, -3, 0, 0) < 0.0);
    assert!(at(&s, 2, 0, 0) > 0.0 && at(&s, 8, 0, 0) > 0.0);
    // values are the min / max of the inputs
    assert_eq!(at(&u, 2, 0, 0), at(&a, 2, 0, 0).min(at(&b, 2, 0, 0)));
    assert_eq!(at(&i, 2, 0, 0), at(&a, 2, 0, 0).max(at(&b, 2, 0, 0)));
    // deep inside a (unstored interior) is combined correctly: far inside a, outside b
    let big_a = mesh_to_sdf(&cube(DVec3::splat(200.0), DVec3::ZERO), &p);
    let small_b = mesh_to_sdf(&cube(DVec3::splat(30.0), DVec3::ZERO), &p);
    let hole = combine(&big_a, &small_b, CsgOp::Subtract).unwrap();
    assert!(at(&hole, 0, 0, 0) > 0.0, "inside the hole");
    assert!(
        at(&hole, 12, 0, 0) < 0.0,
        "in the wall, in the unstored interior of a"
    );
    let common = combine(&big_a, &small_b, CsgOp::Intersect).unwrap();
    assert!(at(&common, 0, 0, 0) < 0.0);
    assert!(at(&common, 12, 0, 0) > 0.0);
}

#[test]
fn negate_and_mismatched_lattices() {
    let a = mesh_to_sdf(&cube(DVec3::splat(50.0), DVec3::ZERO), &params(10.0));
    let n = negate(&a);
    assert_eq!(n.get(IVec3::ZERO), -a.get(IVec3::ZERO));
    assert_eq!(n.background(), -a.background());
    assert!(n.is_active(IVec3::new(-5, 0, 0)));
    let b = mesh_to_sdf(&cube(DVec3::splat(50.0), DVec3::ZERO), &params(5.0));
    assert_eq!(
        combine(&a, &b, CsgOp::Union).unwrap_err(),
        VolumeError::GridMismatch
    );
}

#[test]
fn smooth_union_is_below_min_near_the_seam_and_equal_far_from_it() {
    assert_eq!(smooth_min(1.0, 5.0, 2.0), 1.0);
    let s = smooth_min(1.0, 1.0, 2.0);
    assert!((s - 0.5).abs() < 1e-12, "{s}");
    assert!(smooth_min(1.0, 1.5, 2.0) < 1.0);
    assert_eq!(smooth_min(3.0, 1.0, 0.0), 1.0);
    let p = params(10.0);
    let a = mesh_to_sdf(&sphere(40.0, 24, 32, DVec3::new(-30.0, 0.0, 0.0)), &p);
    let b = mesh_to_sdf(&sphere(40.0, 24, 32, DVec3::new(30.0, 0.0, 0.0)), &p);
    let hard = combine(&a, &b, CsgOp::Union).unwrap();
    let soft = combine(&a, &b, CsgOp::SmoothUnion(20.0)).unwrap();
    let mid = IVec3::new(0, 4, 0);
    assert!(soft.get(mid) < hard.get(mid));
    let far = IVec3::new(-7, 0, 0);
    assert!((soft.get(far) - hard.get(far)).abs() < 1.0);
}

#[test]
fn spline_is_a_capsule_chain() {
    let p = params(10.0);
    let line = [DVec3::new(-50.0, 0.0, 0.0), DVec3::new(50.0, 0.0, 0.0)];
    let g = spline_to_sdf(&line, false, 15.0, &p);
    let at = |i: IVec3| f64::from(g.get(i));
    assert!((at(IVec3::new(0, 1, 0)) - (10.0 - 15.0)).abs() < 1e-5);
    assert!((at(IVec3::new(0, 3, 0)) - (30.0 - 15.0)).abs() < 1e-5);
    // round end cap
    assert!((at(IVec3::new(7, 0, 0)) - (20.0 - 15.0)).abs() < 1e-5);
    assert!((at(IVec3::new(0, 0, 0)) - -15.0).abs() < 1e-5);
    // a closed square also covers its last edge
    let square = [
        DVec3::new(-50.0, 0.0, -50.0),
        DVec3::new(50.0, 0.0, -50.0),
        DVec3::new(50.0, 0.0, 50.0),
        DVec3::new(-50.0, 0.0, 50.0),
    ];
    let open = spline_to_sdf(&square, false, 5.0, &p);
    let closed = spline_to_sdf(&square, true, 5.0, &p);
    assert!(open.get(IVec3::new(-5, 0, 0)) > 0.0);
    assert!(closed.get(IVec3::new(-5, 0, 0)) < 0.0);
}

#[test]
fn points_are_a_union_of_spheres() {
    let p = params(10.0);
    let spheres = [
        (DVec3::new(-30.0, 0.0, 0.0), 25.0),
        (DVec3::new(30.0, 0.0, 0.0), 15.0),
    ];
    let g = points_to_sdf(&spheres, PointUnion::Min, &p);
    let at = |i: IVec3| f64::from(g.get(i));
    assert!((at(IVec3::new(-5, 0, 0)) - -5.0).abs() < 1e-5);
    assert!((at(IVec3::new(4, 0, 0)) - -5.0).abs() < 1e-5);
    // deeper than the interior band: the filled value
    assert_eq!(at(IVec3::new(-3, 0, 0)), -20.0);
    assert!((at(IVec3::new(0, 0, 0)) - 5.0).abs() < 1e-5); // 30 - 25, 30 - 15 = 15
    let soft = points_to_sdf(&spheres, PointUnion::Smooth(30.0), &p);
    assert!(soft.get(IVec3::new(0, 0, 0)) < g.get(IVec3::new(0, 0, 0)));
}
