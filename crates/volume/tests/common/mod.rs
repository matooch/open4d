#![allow(dead_code)]
use forge_geometry::PolyMesh;
use glam::DVec3;

/// Axis aligned box, outward quads.
pub fn cube(half: DVec3, centre: DVec3) -> PolyMesh {
    let mut points = vec![];
    for i in 0..8 {
        points.push(
            centre
                + DVec3::new(
                    if i & 1 == 1 { half.x } else { -half.x },
                    if i & 2 == 2 { half.y } else { -half.y },
                    if i & 4 == 4 { half.z } else { -half.z },
                ),
        );
    }
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

/// UV sphere, outward.
pub fn sphere(r: f64, rings: usize, segs: usize, centre: DVec3) -> PolyMesh {
    let mut points = vec![centre + DVec3::new(0.0, r, 0.0)];
    for i in 1..rings {
        let th = std::f64::consts::PI * i as f64 / rings as f64;
        for j in 0..segs {
            let ph = std::f64::consts::TAU * j as f64 / segs as f64;
            points
                .push(centre + r * DVec3::new(th.sin() * ph.cos(), th.cos(), th.sin() * ph.sin()));
        }
    }
    points.push(centre + DVec3::new(0.0, -r, 0.0));
    let last = points.len() as u32 - 1;
    let ring = |i: usize, j: usize| 1 + ((i - 1) * segs + j % segs) as u32;
    let mut polys = Vec::new();
    for j in 0..segs {
        polys.push(vec![0, ring(1, j + 1), ring(1, j)]);
    }
    for i in 1..rings - 1 {
        for j in 0..segs {
            polys.push(vec![
                ring(i, j),
                ring(i, j + 1),
                ring(i + 1, j + 1),
                ring(i + 1, j),
            ]);
        }
    }
    for j in 0..segs {
        polys.push(vec![last, ring(rings - 1, j), ring(rings - 1, j + 1)]);
    }
    PolyMesh { points, polys }
}
