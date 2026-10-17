//! Mesh / matrix / image diff metrics (docs/specs/INFRA.md §4).

use forge_parity_harness::{Mesh, Tolerance, compare_meshes, max_matrix_error, psnr};

fn tri() -> Mesh {
    Mesh {
        positions: vec![[0., 0., 0.], [1., 0., 0.], [0., 1., 0.]],
        faces: vec![vec![0, 1, 2]],
    }
}

fn has(failures: &[String], needle: &str) -> bool {
    failures.iter().any(|f| f.contains(needle))
}

#[test]
fn identical_meshes_pass() {
    let r = compare_meshes(&tri(), &tri(), &Tolerance::default());
    assert!(r.passed(), "{:?}", r.failures);
    assert_eq!(r.max_position_error, Some(0.0));
}

#[test]
fn empty_meshes_compare_equal() {
    let r = compare_meshes(&Mesh::default(), &Mesh::default(), &Tolerance::default());
    assert!(r.passed(), "{:?}", r.failures);
}

#[test]
fn position_error_is_judged_against_tolerance() {
    let mut a = tri();
    a.positions[2][1] += 0.5e-4;
    assert!(compare_meshes(&tri(), &a, &Tolerance::default()).passed());

    a.positions[2][1] += 1e-4;
    let r = compare_meshes(&tri(), &a, &Tolerance::default());
    assert!(!r.passed());
    assert!((r.max_position_error.unwrap() - 1.5e-4).abs() < 1e-12);
    assert!(has(&r.failures, "position"), "{:?}", r.failures);
    assert!(has(&r.failures, "bbox"), "{:?}", r.failures);
}

#[test]
fn count_mismatch_skips_per_vertex_check() {
    let mut a = tri();
    a.positions.push([5., 5., 5.]);
    let r = compare_meshes(&tri(), &a, &Tolerance::default());
    assert!(!r.passed());
    assert_eq!(r.max_position_error, None);
    assert!(has(&r.failures, "vertex count"), "{:?}", r.failures);
}

#[test]
fn topology_is_strict_about_order() {
    let mut a = tri();
    a.faces[0] = vec![1, 2, 0]; // same cycle, different start vertex
    let r = compare_meshes(&tri(), &a, &Tolerance::default());
    assert!(!r.passed());
    assert!(has(&r.failures, "topology"), "{:?}", r.failures);

    let mut b = tri();
    b.faces.push(vec![0, 2, 1]);
    let r = compare_meshes(&tri(), &b, &Tolerance::default());
    assert!(has(&r.failures, "face count"), "{:?}", r.failures);
}

#[test]
fn matrix_error_is_max_abs_element_difference() {
    let id: [f64; 16] = std::array::from_fn(|i| if i % 5 == 0 { 1.0 } else { 0.0 });
    let mut m = id;
    m[3] = -0.25;
    m[12] = 0.125;
    assert_eq!(max_matrix_error(&id, &id), 0.0);
    assert_eq!(max_matrix_error(&id, &m), 0.25);
}

#[test]
fn psnr_matches_definition() {
    assert_eq!(psnr(&[0.2, 0.4], &[0.2, 0.4]), f64::INFINITY);
    let p = psnr(&[0.0; 8], &[0.1; 8]); // MSE = 0.01 -> 20 dB
    assert!((p - 20.0).abs() < 1e-5, "{p}");
}

#[test]
#[should_panic]
fn psnr_rejects_length_mismatch() {
    psnr(&[0.0; 3], &[0.0; 4]);
}

#[test]
#[should_panic]
fn psnr_rejects_empty_input() {
    psnr(&[], &[]);
}
