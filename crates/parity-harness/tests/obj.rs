//! OBJ reading and import transforms (docs/specs/INFRA.md §4).

use forge_parity_harness::{Import, Mesh, apply_import, parse_obj};

#[test]
fn reads_vertices_and_all_face_index_forms() {
    let src = "# c\nv 1 2 3\nv 4 5 6 1.0\nv 7 8 9\nv 0 0 0\nvt 0 0\nvn 0 0 1\ng grp\ns off\n\
               usemtl m\nf 1 2 3\nf 1/1 2/1 3/1\nf 1//1 2//1 3//1\nf 1/1/1 2/1/1 3/1/1 4/1/1\nf -4 -3 -2\n";
    let m = parse_obj(src).unwrap();
    assert_eq!(
        m.positions,
        vec![[1., 2., 3.], [4., 5., 6.], [7., 8., 9.], [0., 0., 0.]]
    );
    assert_eq!(
        m.faces,
        vec![
            vec![0, 1, 2],
            vec![0, 1, 2],
            vec![0, 1, 2],
            vec![0, 1, 2, 3],
            vec![0, 1, 2]
        ]
    );
}

#[test]
fn negative_indices_are_relative_to_vertices_seen_so_far() {
    let m = parse_obj("v 0 0 0\nv 1 0 0\nv 0 1 0\nf -3 -2 -1\nv 1 1 0\nf -3 -2 -1\n").unwrap();
    assert_eq!(m.faces, vec![vec![0, 1, 2], vec![1, 2, 3]]);
}

#[test]
fn errors_carry_line_numbers() {
    let cases = [
        ("v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 4\n", 4), // out of range
        ("v 0 0 0\nv 1 0 0\nv 0 1 0\nf 0 1 2\n", 4), // zero index
        ("v 0 0 0\nv 1 0 0\nf 1 2\n", 3),            // too few vertices
        ("v 0 0\n", 1),                              // too few coords
        ("v 0 x 0\n", 1),                            // bad number
    ];
    for (src, line) in cases {
        let err = parse_obj(src).unwrap_err();
        assert_eq!(err.line, line, "{src:?} -> {err}");
    }
}

#[test]
fn import_scales_then_flips_z_and_reverses_winding() {
    let mesh = Mesh {
        positions: vec![[100., 0., 50.], [0., 200., -50.], [0., 0., 0.]],
        faces: vec![vec![0, 1, 2]],
    };
    let out = apply_import(
        mesh.clone(),
        &Import {
            scale: 0.01,
            flip_z: true,
        },
    );
    assert_eq!(
        out.positions,
        vec![[1., 0., -0.5], [0., 2., 0.5], [0., 0., 0.]]
    );
    assert_eq!(out.faces, vec![vec![2, 1, 0]]);
    assert_eq!(apply_import(mesh.clone(), &Import::default()), mesh);
}
