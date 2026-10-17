//! docs/specs/SUBDIVISION.md §2: the Catmull–Clark step.

mod common;

use common::{assert_vec_eq, cube, grid, mesh};
use forge_geometry::subdivide::{catmull_clark, linear};
use glam::DVec3;

#[test]
fn counts_follow_the_rule() {
    let c = cube();
    let one = catmull_clark(&c);
    assert_eq!((one.points.len(), one.polys.len()), (8 + 12 + 6, 24));
    let two = catmull_clark(&one);
    assert_eq!((two.points.len(), two.polys.len()), (98, 96));
    // every n-gon becomes n quads
    let tri = mesh(&[[0., 0., 0.], [1., 0., 0.], [0., 1., 0.]], &[&[0, 1, 2]]);
    let t = catmull_clark(&tri);
    assert_eq!((t.points.len(), t.polys.len()), (3 + 3 + 1, 3));
    assert!(t.polys.iter().all(|p| p.len() == 4));
}

#[test]
fn originals_come_first_then_face_and_new_edge_points() {
    let c = cube();
    let s = catmull_clark(&c);
    // face 0 of the test cube is [0, 3, 2, 1] (z = 0 face): its face point is vertex 8
    assert_vec_eq(s.points[8], DVec3::new(0.5, 0.5, 0.0));
    // its first quad starts at corner 1: [3, E(3,2), F, E(0,3)]
    let p = &s.polys[0];
    assert_eq!((p[0], p[2]), (3, 8));
    s.validate().unwrap();
}

#[test]
fn boundary_rules_apply_to_corners_and_edges() {
    let g = grid(); // 2×2 quads, corner (0,0) with boundary neighbours (1,0) and (0,1)
    let s = catmull_clark(&g);
    assert_vec_eq(s.points[0], DVec3::new(1.0 / 8.0, 1.0 / 8.0, 0.0));
    // a fully creased cube keeps its corners
    let c = cube();
    let all: std::collections::BTreeSet<_> = c.edges().into_iter().collect();
    let sharp = forge_geometry::subdivide::subdivide_levels_with_creases(&c, 1, true, &all);
    for v in 0..8 {
        assert_vec_eq(sharp.points[v], c.points[v]);
    }
    // a bent open strip: boundary vertices move by (6v + a + b)/8 along the boundary
    let strip = mesh(
        &[
            [0., 0., 0.],
            [1., 0., 0.],
            [2., 0., 1.],
            [0., 1., 0.],
            [1., 1., 0.],
            [2., 1., 1.],
        ],
        &[&[0, 1, 4, 3], &[1, 2, 5, 4]],
    );
    let s = catmull_clark(&strip);
    let want =
        (6.0 * DVec3::new(1., 0., 0.) + DVec3::new(0., 0., 0.) + DVec3::new(2., 0., 1.)) / 8.0;
    assert_vec_eq(s.points[1], want);
}

#[test]
fn linear_keeps_the_shape() {
    let s = linear(&cube());
    assert_eq!((s.points.len(), s.polys.len()), (26, 24));
    let (lo, hi) = s
        .points
        .iter()
        .fold((DVec3::splat(9.0), DVec3::splat(-9.0)), |(a, b), p| {
            (a.min(*p), b.max(*p))
        });
    assert_vec_eq(lo, DVec3::ZERO);
    assert_vec_eq(hi, DVec3::ONE);
}

#[test]
fn levels_zero_and_one_and_counts() {
    use forge_geometry::subdivide::subdivide_levels;
    let c = cube();
    assert_eq!(subdivide_levels(&c, 0, true), c);
    assert_eq!(subdivide_levels(&c, 1, true), catmull_clark(&c));
    let two = subdivide_levels(&c, 2, true);
    assert_eq!((two.points.len(), two.polys.len()), (98, 96));
    two.validate().unwrap();
    let three = subdivide_levels(&c, 3, true);
    assert_eq!((three.points.len(), three.polys.len()), (386, 384));
    // positions are those of repeated steps, only reordered
    let mut a: Vec<[i64; 3]> = catmull_clark(&catmull_clark(&c))
        .points
        .iter()
        .map(|p| (*p * 1e6).round().as_i64vec3().to_array())
        .collect();
    let mut b: Vec<[i64; 3]> = two
        .points
        .iter()
        .map(|p| (*p * 1e6).round().as_i64vec3().to_array())
        .collect();
    a.sort();
    b.sort();
    assert_eq!(a, b);
}
