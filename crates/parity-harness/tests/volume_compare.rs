//! `compare_volume_mesh` on spheres (docs/specs/VOLUME.md section 5).

use forge_parity_harness::{Mesh, compare_volume_mesh};

/// UV sphere with poles; `rings` latitude rings, `segs` longitudes, outward quads.
fn sphere(r: f64, rings: usize, segs: usize, centre: [f64; 3]) -> Mesh {
    let mut positions = vec![[centre[0], centre[1] + r, centre[2]]];
    for i in 1..rings {
        let th = std::f64::consts::PI * i as f64 / rings as f64;
        for j in 0..segs {
            let ph = std::f64::consts::TAU * j as f64 / segs as f64;
            positions.push([
                centre[0] + r * th.sin() * ph.cos(),
                centre[1] + r * th.cos(),
                centre[2] + r * th.sin() * ph.sin(),
            ]);
        }
    }
    positions.push([centre[0], centre[1] - r, centre[2]]);
    let last = positions.len() as u32 - 1;
    let ring = |i: usize, j: usize| 1 + ((i - 1) * segs + j % segs) as u32;
    let mut faces = Vec::new();
    for j in 0..segs {
        faces.push(vec![0, ring(1, j + 1), ring(1, j)]);
    }
    for i in 1..rings - 1 {
        for j in 0..segs {
            faces.push(vec![
                ring(i, j),
                ring(i, j + 1),
                ring(i + 1, j + 1),
                ring(i + 1, j),
            ]);
        }
    }
    for j in 0..segs {
        faces.push(vec![last, ring(rings - 1, j), ring(rings - 1, j + 1)]);
    }
    Mesh { positions, faces }
}

/// A torus: `major` and `minor` radius, `u` by `v` quads.
fn torus(major: f64, minor: f64, u: usize, v: usize) -> Mesh {
    let mut positions = Vec::new();
    for i in 0..u {
        for j in 0..v {
            let a = std::f64::consts::TAU * i as f64 / u as f64;
            let b = std::f64::consts::TAU * j as f64 / v as f64;
            positions.push([
                (major + minor * b.cos()) * a.cos(),
                minor * b.sin(),
                (major + minor * b.cos()) * a.sin(),
            ]);
        }
    }
    let id = |i: usize, j: usize| ((i % u) * v + j % v) as u32;
    let mut faces = Vec::new();
    for i in 0..u {
        for j in 0..v {
            faces.push(vec![id(i, j), id(i + 1, j), id(i + 1, j + 1), id(i, j + 1)]);
        }
    }
    Mesh { positions, faces }
}

#[test]
fn a_mesh_matches_itself_and_a_renumbered_copy() {
    let a = sphere(100.0, 24, 32, [0.0; 3]);
    let r = compare_volume_mesh(&a, &a, 10.0);
    assert!(r.passed(), "{:?}", r.failures);
    assert_eq!(r.euler, (2, 2));
    assert_eq!(r.closed, (true, true));
    let mut b = a.clone();
    b.positions.reverse();
    let n = b.positions.len() as u32 - 1;
    for f in &mut b.faces {
        for i in f.iter_mut() {
            *i = n - *i;
        }
        f.rotate_left(1);
    }
    let r = compare_volume_mesh(&a, &b, 10.0);
    assert!(r.passed(), "{:?}", r.failures);
    assert!(r.hausdorff_expected_to_actual < 1e-9);
}

#[test]
fn different_tessellations_of_one_sphere_pass_at_voxel_scale() {
    let a = sphere(100.0, 24, 32, [0.0; 3]);
    let b = sphere(100.0, 40, 48, [0.0; 3]);
    let r = compare_volume_mesh(&a, &b, 10.0);
    assert!(r.passed(), "{:?}", r.failures);
    assert!(r.hausdorff_expected_to_actual > 0.0);
}

#[test]
fn a_shifted_sphere_fails_the_hausdorff_limit() {
    let a = sphere(100.0, 24, 32, [0.0; 3]);
    let b = sphere(100.0, 24, 32, [6.0, 0.0, 0.0]);
    let r = compare_volume_mesh(&a, &b, 10.0);
    assert!(!r.passed());
    assert!(
        r.failures.iter().any(|f| f.contains("hausdorff")),
        "{:?}",
        r.failures
    );
    // below voxel / 2 the same shift passes
    let c = sphere(100.0, 24, 32, [4.0, 0.0, 0.0]);
    assert!(compare_volume_mesh(&a, &c, 10.0).passed());
}

#[test]
fn a_larger_sphere_fails_area_and_volume() {
    let a = sphere(100.0, 24, 32, [0.0; 3]);
    let b = sphere(103.0, 24, 32, [0.0; 3]);
    // 3 % radius: inside voxel / 2 but 6 % area and 9 % volume
    let r = compare_volume_mesh(&a, &b, 10.0);
    assert!(!r.passed());
    assert!(
        r.failures.iter().any(|f| f.starts_with("area")),
        "{:?}",
        r.failures
    );
    assert!(
        r.failures.iter().any(|f| f.starts_with("volume")),
        "{:?}",
        r.failures
    );
    assert!(!r.failures.iter().any(|f| f.starts_with("hausdorff")));
}

#[test]
fn a_torus_is_not_a_sphere() {
    let a = sphere(60.0, 24, 32, [0.0; 3]);
    let t = torus(60.0, 25.0, 40, 24);
    let r = compare_volume_mesh(&a, &t, 1000.0);
    assert_eq!(r.euler, (2, 0));
    assert!(
        r.failures.iter().any(|f| f.starts_with("euler")),
        "{:?}",
        r.failures
    );
}

#[test]
fn empty_meshes() {
    let a = sphere(10.0, 8, 8, [0.0; 3]);
    let e = Mesh::default();
    assert!(compare_volume_mesh(&e, &e, 1.0).passed());
    assert!(!compare_volume_mesh(&a, &e, 1.0).passed());
}
