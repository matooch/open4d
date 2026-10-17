#![allow(dead_code)]

use forge_geometry::PolyMesh;
use glam::DVec3;

pub const W: f64 = 800.0;
pub const H: f64 = 600.0;

pub fn close(a: DVec3, b: DVec3, eps: f64) {
    assert!((a - b).abs().max_element() < eps, "{a} != {b}");
}

/// Axis-aligned box from `lo` to `hi`, outward CCW quads (bottom, top, -z, +z, -x, +x).
pub fn boxed(lo: DVec3, hi: DVec3) -> PolyMesh {
    let p = |x: bool, y: bool, z: bool| {
        DVec3::new(
            if x { hi.x } else { lo.x },
            if y { hi.y } else { lo.y },
            if z { hi.z } else { lo.z },
        )
    };
    // index = x + 2y + 4z
    let points = (0..8)
        .map(|i| p(i & 1 == 1, i & 2 == 2, i & 4 == 4))
        .collect();
    PolyMesh {
        points,
        polys: vec![
            vec![0, 1, 5, 4], // y = lo
            vec![2, 6, 7, 3], // y = hi
            vec![0, 2, 3, 1], // z = lo
            vec![4, 5, 7, 6], // z = hi
            vec![0, 4, 6, 2], // x = lo
            vec![1, 3, 7, 5], // x = hi
        ],
    }
}

pub fn unit_box() -> PolyMesh {
    boxed(DVec3::splat(-1.0), DVec3::splat(1.0))
}
