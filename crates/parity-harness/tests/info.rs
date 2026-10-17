//! docs/specs/INFRA.md §4 mesh_info / obj_extras, §4.1 golden CLI.

use std::process::Command;

use forge_parity_harness::{Mesh, mesh_info, obj_extras, parse_obj};

const QUAD_AND_TRI: &str = "v -1 0 -2\nv 1 0 -2\nv 1 0.5 2\nv -1 0 2\nv 9 9 9\nv 0 3 0\n\
vn 0 1 0\nvt 0 0\nvt 1 0\nf 1 2 3 4\nf 1 4 6\n";

#[test]
fn info_counts_faces_by_size_and_unused_points() {
    let m = parse_obj(QUAD_AND_TRI).unwrap();
    let i = mesh_info(&m);
    assert_eq!((i.vertices, i.unused_vertices, i.faces), (6, 1, 2));
    assert_eq!(i.face_sizes.get(&3), Some(&1));
    assert_eq!(i.face_sizes.get(&4), Some(&1));
    assert_eq!(i.bbox, Some(([-1.0, 0.0, -2.0], [9.0, 9.0, 9.0])));
    let e = obj_extras(QUAD_AND_TRI);
    assert_eq!((e.normals, e.uvs), (1, 2));
}

#[test]
fn info_display_format() {
    let m = parse_obj(QUAD_AND_TRI).unwrap();
    assert_eq!(
        mesh_info(&m).to_string(),
        "vertices: 6 (unused 1)\n\
         faces: 2 (3-gons 1, 4-gons 1)\n\
         bbox min: (-1, 0, -2)\n\
         bbox max: (9, 9, 9)\n\
         size: (10, 9, 11)\n\
         centre: (4, 4.5, 3.5)\n"
    );
    assert_eq!(
        mesh_info(&Mesh::default()).to_string(),
        "vertices: 0 (unused 0)\nfaces: 0\nbbox: empty\n"
    );
}

fn golden(args: &[&str]) -> (i32, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_golden"))
        .args(args)
        .output()
        .unwrap();
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

#[test]
fn cli_info_and_errors() {
    let dir = tempfile::tempdir().unwrap();
    let good = dir.path().join("m.obj");
    std::fs::write(&good, QUAD_AND_TRI).unwrap();
    let (code, out, _) = golden(&["info", good.to_str().unwrap()]);
    assert_eq!(code, 0);
    assert!(out.starts_with("vertices: 6 (unused 1)\n"), "{out}");
    assert!(out.ends_with("normals: 1\nuvs: 2\n"), "{out}");

    let bad = dir.path().join("bad.obj");
    std::fs::write(&bad, "v 0 0 0\nf 1 2 3\n").unwrap();
    let (code, _, err) = golden(&["info", bad.to_str().unwrap()]);
    assert_eq!(code, 2);
    assert!(err.contains("line 2"), "{err}");

    let (code, _, _) = golden(&["info", "does/not/exist.obj"]);
    assert_eq!(code, 2);
    let (code, _, err) = golden(&["frobnicate"]);
    assert_eq!(code, 2);
    assert!(err.contains("usage"), "{err}");
}

#[test]
fn cli_check() {
    let fixture = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/repo");
    let (code, out, _) = golden(&["check", fixture]);
    assert_eq!((code, out.as_str()), (0, "ok: 1 cases\n"));

    let dir = tempfile::tempdir().unwrap();
    let case = dir.path().join("tests/golden/cube/default");
    std::fs::create_dir_all(&case).unwrap();
    std::fs::write(
        case.join("case.toml"),
        "feature = \"cube\"\nobservation = \"docs/parity/cube.md\"\n",
    )
    .unwrap();
    let (code, out, _) = golden(&["check", dir.path().to_str().unwrap()]);
    assert_eq!(code, 1);
    assert!(out.starts_with("cube/default: observation"), "{out}");
}
