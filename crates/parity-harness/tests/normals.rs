//! docs/specs/INFRA.md §4: exported corner normals.

use forge_parity_harness::{max_normal_error, parse_obj_corner_normals};

const SRC: &str = "v 0 0 0\nv 1 0 0\nv 0 1 0\nv 1 1 0\nvn 0 0 1\nvn 0 0 -1\nvt 0 0\n\
f 1/1/1 2/1/1 3/1/2\nf 2//2 4//-1 3//1\nf 1 2 4\n";

#[test]
fn reads_normals_per_corner() {
    let n = parse_obj_corner_normals(SRC).unwrap();
    assert_eq!(n.len(), 3);
    assert_eq!(
        n[0],
        vec![Some([0., 0., 1.]), Some([0., 0., 1.]), Some([0., 0., -1.])]
    );
    assert_eq!(
        n[1],
        vec![Some([0., 0., -1.]), Some([0., 0., -1.]), Some([0., 0., 1.])]
    );
    assert_eq!(n[2], vec![None, None, None]);
}

#[test]
fn bad_normal_indices_are_errors_with_lines() {
    let e = parse_obj_corner_normals("v 0 0 0\nv 1 0 0\nv 0 1 0\nvn 0 0 1\nf 1//1 2//2 3//1\n")
        .unwrap_err();
    assert_eq!(e.line, 5);
    let e = parse_obj_corner_normals("v 0 0 0\nv 1 0 0\nv 0 1 0\nvn 0 0 1\nf 1//0 2//1 3//1\n")
        .unwrap_err();
    assert_eq!(e.line, 5);
}

#[test]
fn normal_error_compares_shapes_and_values() {
    let a = vec![vec![[0., 0., 1.], [0., 0., 1.], [0., 1., 0.]]];
    let mut b = a.clone();
    assert_eq!(max_normal_error(&a, &b), Some(0.0));
    b[0][2] = [0., 0.75, 0.];
    assert_eq!(max_normal_error(&a, &b), Some(0.25));
    b[0].pop();
    assert_eq!(max_normal_error(&a, &b), None);
    assert_eq!(max_normal_error(&a, &[]), None);
}
