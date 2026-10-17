//! Hand-built test meshes, counter-clockwise seen from outside.
#![allow(dead_code)]

use forge_geometry::PolyMesh;
use glam::DVec3;

pub fn mesh(points: &[[f64; 3]], polys: &[&[u32]]) -> PolyMesh {
    PolyMesh {
        points: points.iter().map(|p| DVec3::from_array(*p)).collect(),
        polys: polys.iter().map(|p| p.to_vec()).collect(),
    }
}

/// Unit cube. Polys: 0 bottom(-z) 1 top(+z) 2 front(-y) 3 back(+y) 4 left(-x) 5 right(+x).
pub fn cube() -> PolyMesh {
    mesh(
        &[
            [0., 0., 0.],
            [1., 0., 0.],
            [1., 1., 0.],
            [0., 1., 0.],
            [0., 0., 1.],
            [1., 0., 1.],
            [1., 1., 1.],
            [0., 1., 1.],
        ],
        &[
            &[0, 3, 2, 1],
            &[4, 5, 6, 7],
            &[0, 1, 5, 4],
            &[2, 3, 7, 6],
            &[0, 4, 7, 3],
            &[1, 2, 6, 5],
        ],
    )
}

pub fn tetra() -> PolyMesh {
    mesh(
        &[[0., 0., 0.], [1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
        &[&[0, 2, 1], &[0, 1, 3], &[0, 3, 2], &[1, 2, 3]],
    )
}

/// 2×2 quads in z=0, points `y*3 + x`. Quads: 0 [0,1,4,3] 1 [1,2,5,4] 2 [3,4,7,6] 3 [4,5,8,7].
pub fn grid() -> PolyMesh {
    let mut points = Vec::new();
    for y in 0..3 {
        for x in 0..3 {
            points.push([x as f64, y as f64, 0.0]);
        }
    }
    let polys: Vec<Vec<u32>> = (0..2)
        .flat_map(|y| (0..2).map(move |x| (y * 3 + x) as u32))
        .map(|i| vec![i, i + 1, i + 4, i + 3])
        .collect();
    let refs: Vec<&[u32]> = polys.iter().map(Vec::as_slice).collect();
    mesh(&points, &refs)
}

pub fn assert_vec_eq(a: DVec3, b: DVec3) {
    assert!((a - b).abs().max_element() < 1e-9, "{a} != {b}");
}
