//! docs/specs/IO.md §4: STL and PLY against the exchange golden cases (O44: the six
//! `scene.stl` files) and round trips of both formats.

use std::path::Path;

use forge_geometry::PolyMesh;
use forge_io::obj::ObjObject;
use forge_io::{ply, stl};
use forge_parity_harness::{discover_cases, parse_obj};
use glam::{DMat4, DVec3};

fn object(name: &str, points: &[[f64; 3]], polys: &[&[u32]]) -> ObjObject {
    ObjObject {
        name: name.into(),
        mesh: PolyMesh {
            points: points.iter().map(|p| DVec3::from_array(*p)).collect(),
            polys: polys.iter().map(|p| p.to_vec()).collect(),
        },
        world: DMat4::IDENTITY,
        smoothing_angle: None,
        uvs: None,
    }
}

/// The golden binary STL: the 12 floats and the attribute of every triangle.
fn golden_triangles(bytes: &[u8]) -> Vec<([f32; 3], [[f32; 3]; 3])> {
    let n = u32::from_le_bytes(bytes[80..84].try_into().unwrap()) as usize;
    assert_eq!(bytes.len(), 84 + 50 * n);
    (0..n)
        .map(|i| {
            let rec = &bytes[84 + 50 * i..134 + 50 * i];
            let f = |k: usize| f32::from_le_bytes(rec[4 * k..4 * k + 4].try_into().unwrap());
            (
                [f(0), f(1), f(2)],
                [[f(3), f(4), f(5)], [f(6), f(7), f(8)], [f(9), f(10), f(11)]],
            )
        })
        .collect()
}

#[test]
fn the_binary_writer_equals_the_six_golden_stl_files() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden");
    let mut checked = Vec::new();
    for case in discover_cases(&root).unwrap() {
        if case.feature != "exchange" {
            continue;
        }
        let dir = root.join("exchange").join(&case.name);
        let golden = std::fs::read(dir.join("scene.stl")).unwrap();
        let obj = parse_obj(&std::fs::read_to_string(dir.join("mesh.obj")).unwrap()).unwrap();
        let o = ObjObject {
            name: case.name.clone(),
            mesh: PolyMesh {
                points: obj
                    .positions
                    .iter()
                    .map(|p| DVec3::from_array(*p))
                    .collect(),
                polys: obj.faces.clone(),
            },
            world: DMat4::IDENTITY,
            smoothing_angle: None,
            uvs: None,
        };
        let ours = stl::write(std::slice::from_ref(&o), &stl::WriteOptions::default());
        assert_eq!(ours.len(), golden.len(), "{}: file size", case.name);
        let (want, got) = (golden_triangles(&golden), golden_triangles(&ours));
        for (i, (w, g)) in want.iter().zip(&got).enumerate() {
            let close = |a: &[f32], b: &[f32]| a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1e-3);
            assert!(
                close(&w.0, &g.0),
                "{} tri {i}: normal {:?} vs {:?}",
                case.name,
                w.0,
                g.0
            );
            for k in 0..3 {
                assert!(close(&w.1[k], &g.1[k]), "{} tri {i} corner {k}", case.name);
            }
        }
        // reading the golden file back gives the OBJ's points and its polygons as triangles
        let read = stl::read(&golden, &stl::ReadOptions::default()).unwrap();
        let tris: usize = o.mesh.polys.iter().map(|p| p.len().saturating_sub(2)).sum();
        let read_polys: usize = read.iter().map(|r| r.mesh.polys.len()).sum();
        assert_eq!(read_polys, tris, "{}", case.name);
        for r in &read {
            assert!(r.mesh.polys.iter().all(|p| p.len() == 3));
            for p in &r.mesh.points {
                assert!(
                    o.mesh.points.iter().any(|q| (*q - *p).length() < 1e-2),
                    "{}: read point {p:?} not in the OBJ",
                    case.name
                );
            }
        }
        checked.push(case.name);
    }
    assert_eq!(checked.len(), 6, "{checked:?}");
}

/// The corner positions of every polygon, in corner order.
fn corners(o: &ObjObject) -> Vec<Vec<[f64; 3]>> {
    o.mesh
        .polys
        .iter()
        .map(|p| {
            p.iter()
                .map(|i| o.mesh.points[*i as usize].to_array())
                .collect()
        })
        .collect()
}

fn quad_and_triangle() -> ObjObject {
    object(
        "Thing",
        &[
            [0.0, 0.0, 0.0],
            [10.0, 0.0, 0.0],
            [10.0, 20.0, 0.0],
            [0.0, 20.0, 0.0],
            [5.0, 5.0, 30.0],
        ],
        &[&[0, 1, 2, 3], &[0, 1, 4]],
    )
}

#[test]
fn the_writer_swaps_y_and_z_reverses_the_winding_and_fans_quads() {
    let bytes = stl::write(&[quad_and_triangle()], &stl::WriteOptions::default());
    assert_eq!(&bytes[..18], b"Forge3D STL export");
    let t = golden_triangles(&bytes);
    assert_eq!(t.len(), 3);
    // quad [a, b, c, d] -> [a, c, b], [a, d, c]; scene (x, y, z) -> (x, z, y)
    assert_eq!(
        t[0].1,
        [[0.0, 0.0, 0.0], [10.0, 0.0, 20.0], [10.0, 0.0, 0.0]]
    );
    assert_eq!(
        t[1].1,
        [[0.0, 0.0, 0.0], [0.0, 0.0, 20.0], [10.0, 0.0, 20.0]]
    );
    // the triangle [a, b, e] -> [a, e, b]
    assert_eq!(
        t[2].1,
        [[0.0, 0.0, 0.0], [5.0, 30.0, 5.0], [10.0, 0.0, 0.0]]
    );
    // the quad faces +Z in the scene: scene +Z is the file's +Y, and the reversed winding keeps the normal outward
    assert_eq!(t[0].0, [0.0, 1.0, 0.0]);
}

#[test]
fn stl_round_trips_in_binary_and_ascii() {
    let original = object(
        "Thing",
        &[
            [0.0, 0.0, 0.0],
            [10.0, 0.0, 0.0],
            [0.0, 20.0, 0.0],
            [5.0, 5.0, 30.0],
        ],
        &[&[0, 1, 2], &[0, 1, 3], &[1, 2, 3], &[2, 0, 3]],
    );
    for binary in [true, false] {
        let opts = stl::WriteOptions { binary, scale: 1.0 };
        let bytes = stl::write(std::slice::from_ref(&original), &opts);
        if !binary {
            assert!(bytes.starts_with(b"solid Thing\n"));
        }
        let back = stl::read(&bytes, &stl::ReadOptions::default()).unwrap();
        assert_eq!(back.len(), 1, "binary {binary}");
        // points come back in first-occurrence order of the written triangles, so compare the
        // corner positions of every polygon
        assert_eq!(corners(&back[0]), corners(&original), "binary {binary}");
        if !binary {
            assert_eq!(back[0].name, "Thing");
        }
    }
    // scale multiplies on both sides
    let big = stl::write(
        std::slice::from_ref(&original),
        &stl::WriteOptions {
            binary: true,
            scale: 2.0,
        },
    );
    let read = stl::read(&big, &stl::ReadOptions { scale: 0.5 }).unwrap();
    assert_eq!(corners(&read[0]), corners(&original));
}

#[test]
fn stl_world_matrix_is_applied_and_objects_follow_each_other() {
    let mut a = quad_and_triangle();
    a.world = DMat4::from_translation(DVec3::new(100.0, 0.0, 0.0));
    let b = quad_and_triangle();
    let bytes = stl::write(&[a, b], &stl::WriteOptions::default());
    let t = golden_triangles(&bytes);
    assert_eq!(t.len(), 6);
    assert_eq!(t[0].1[0], [100.0, 0.0, 0.0]);
    assert_eq!(t[3].1[0], [0.0, 0.0, 0.0]);
}

#[test]
fn stl_ngons_are_ear_clipped() {
    let pent = object(
        "P",
        &[
            [0.0, 0.0, 0.0],
            [10.0, 0.0, 0.0],
            [13.0, 8.0, 0.0],
            [5.0, 14.0, 0.0],
            [-3.0, 8.0, 0.0],
        ],
        &[&[0, 1, 2, 3, 4]],
    );
    let bytes = stl::write(&[pent], &stl::WriteOptions::default());
    assert_eq!(golden_triangles(&bytes).len(), 3);
}

#[test]
fn stl_reader_errors() {
    let r = |b: &[u8]| {
        stl::read(b, &stl::ReadOptions::default())
            .unwrap_err()
            .to_string()
    };
    assert!(r(b"").contains("empty"));
    let mut short = vec![0u8; 84];
    short[80] = 2; // two triangles declared, none present, and not an ASCII file
    assert!(r(&short).contains("size"), "{}", r(&short));
    let ascii = b"solid x\nfacet normal 0 0 1\nouter loop\nvertex 0 0 0\nvertex 1 0 0\nendloop\nendfacet\nendsolid x\n";
    assert!(r(ascii).contains("three vertices"));
    let nan = b"solid x\nfacet normal 0 0 1\nouter loop\nvertex 0 0 0\nvertex 1 0 nan\nvertex 0 1 0\nendloop\nendfacet\nendsolid x\n";
    assert!(r(nan).contains("number"));
}

#[test]
fn ply_round_trips_in_ascii_and_binary_with_ngons() {
    let original = vec![
        object(
            "A",
            &[
                [0.0, 0.0, 0.0],
                [10.0, 0.0, 0.0],
                [10.0, 20.0, 0.0],
                [0.0, 20.0, 0.0],
                [5.0, 5.0, 30.0],
            ],
            &[&[0, 1, 2, 3], &[0, 1, 4], &[1, 2, 4, 3, 0]],
        ),
        object(
            "B",
            &[[1.5, 2.5, -3.5], [4.0, 0.0, 0.0], [0.0, 4.0, 0.0]],
            &[&[0, 1, 2]],
        ),
    ];
    for binary in [true, false] {
        let bytes = ply::write(&original, &ply::WriteOptions { binary, scale: 1.0 });
        assert!(bytes.starts_with(b"ply\n"));
        let header = String::from_utf8_lossy(&bytes[..120.min(bytes.len())]).to_string();
        assert!(header.contains(if binary {
            "binary_little_endian 1.0"
        } else {
            "ascii 1.0"
        }));
        let back = ply::read(&bytes, &ply::ReadOptions::default()).unwrap();
        assert_eq!(back.len(), 1);
        let all: Vec<DVec3> = original
            .iter()
            .flat_map(|o| o.mesh.points.clone())
            .collect();
        assert_eq!(back[0].mesh.points, all);
        // the second object's indices are global
        assert_eq!(back[0].mesh.polys[0], vec![0, 1, 2, 3]);
        assert_eq!(back[0].mesh.polys[3], vec![5, 6, 7]);
        assert_eq!(back[0].mesh.polys[2], vec![1, 2, 4, 3, 0]);
    }
}

#[test]
fn ply_reader_skips_other_properties_and_reads_big_endian_and_errors() {
    let text = "ply\nformat ascii 1.0\ncomment made elsewhere\nelement vertex 3\nproperty double x\nproperty double y\nproperty double z\nproperty uchar red\nelement face 1\nproperty list uchar int vertex_index\nproperty uchar flags\nelement edge 1\nproperty int a\nproperty int b\nend_header\n0 0 0 255\n1 0 0 255\n0 1 0 255\n3 0 1 2 7\n0 1\n";
    let back = ply::read(text.as_bytes(), &ply::ReadOptions::default()).unwrap();
    assert_eq!(back[0].mesh.points.len(), 3);
    assert_eq!(back[0].mesh.polys, vec![vec![0, 1, 2]]);
    // big endian: one triangle of f32
    let mut be = b"ply\nformat binary_big_endian 1.0\nelement vertex 3\nproperty float x\nproperty float y\nproperty float z\nelement face 1\nproperty list uchar uint vertex_indices\nend_header\n".to_vec();
    for v in [0.0f32, 0.0, 0.0, 2.0, 0.0, 0.0, 0.0, 3.0, 0.0] {
        be.extend(v.to_be_bytes());
    }
    be.push(3);
    for i in [0u32, 1, 2] {
        be.extend(i.to_be_bytes());
    }
    let back = ply::read(&be, &ply::ReadOptions { scale: 10.0 }).unwrap();
    assert_eq!(back[0].mesh.points[1], DVec3::new(20.0, 0.0, 0.0));
    let r = |b: &[u8]| {
        ply::read(b, &ply::ReadOptions::default())
            .unwrap_err()
            .to_string()
    };
    assert!(r(b"nope").contains("magic"));
    assert!(r(b"ply\nformat ascii 2.0\nend_header\n").contains("format"));
    let bad_index = "ply\nformat ascii 1.0\nelement vertex 1\nproperty float x\nproperty float y\nproperty float z\nelement face 1\nproperty list uchar uint vertex_indices\nend_header\n0 0 0\n3 0 1 2\n";
    assert!(r(bad_index.as_bytes()).contains("index"));
    let short = &be[..be.len() - 6];
    assert!(r(short).contains("shorter"), "{}", r(short));
}
