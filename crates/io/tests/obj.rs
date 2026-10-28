//! docs/specs/IO.md §1: the OBJ writer and reader.

use forge_geometry::PolyMesh;
use forge_io::obj::{ObjObject, ReadOptions, WriteOptions, read, write};
use glam::{DMat4, DVec3};

fn object(name: &str, points: &[[f64; 3]], polys: &[&[u32]]) -> ObjObject {
    ObjObject {
        name: name.to_string(),
        mesh: PolyMesh {
            points: points.iter().map(|p| DVec3::from_array(*p)).collect(),
            polys: polys.iter().map(|p| p.to_vec()).collect(),
        },
        world: DMat4::IDENTITY,
        smoothing_angle: Some(40f64.to_radians()),
    }
}

fn quad() -> ObjObject {
    object(
        "Quad",
        &[
            [0.1, 0.0, 0.0],
            [1.0 / 3.0, 0.0, 0.0],
            [1.0, 2.5e-9, -3.0],
            [0.0, 1.0, 1e12],
            [9.0, 9.0, 9.0], // unused
        ],
        &[&[1, 2, 3, 0], &[0, 1, 2]],
    )
}

#[test]
fn round_trip_is_exact_with_weld_off() {
    let objects = vec![
        quad(),
        object(
            "Tri",
            &[
                [0.0, 0.0, 0.0],
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
            ],
            &[&[1, 2, 3]],
        ),
    ];
    for normals in [true, false] {
        let text = write(
            &objects,
            &WriteOptions {
                normals,
                weld: false,
            },
        );
        let back = read(&text, &ReadOptions::default()).unwrap();
        assert_eq!(back.len(), 2);
        for (a, b) in objects.iter().zip(&back) {
            assert_eq!(a.name, b.name);
            assert_eq!(a.mesh, b.mesh, "{}", a.name);
            assert_eq!(b.world, DMat4::IDENTITY);
        }
    }
}

#[test]
fn writer_applies_world_and_follows_the_observed_layout() {
    let mut o = object(
        "Tri",
        &[[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
        &[&[0, 1, 2]],
    );
    o.world = DMat4::from_translation(DVec3::new(10.0, 20.0, 30.0));
    let text = write(&[o], &WriteOptions::default());
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines[0], "# Forge3D OBJ export");
    assert!(lines.contains(&"o Tri"));
    assert!(lines.contains(&"v 10 20 30"));
    assert!(lines.contains(&"v 11 20 30"));
    assert!(lines.contains(&"vn 0 0 1"));
    assert!(lines.contains(&"f 1//1 2//1 3//1"));
    assert_eq!(lines.iter().filter(|l| l.starts_with("vn ")).count(), 1);
    let plain = write(
        &[object(
            "T",
            &[[0.0; 3], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            &[&[0, 1, 2]],
        )],
        &WriteOptions {
            normals: false,
            weld: true,
        },
    );
    assert!(plain.lines().any(|l| l == "f 1 2 3"));
    assert!(!plain.contains("vn"));
}

#[test]
fn weld_merges_equal_points_across_objects_and_keeps_corners() {
    let a = object(
        "A",
        &[[0.0; 3], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 0.0]],
        &[&[0, 1, 2], &[3, 1, 2]],
    );
    let b = object(
        "B",
        &[[1.0, 0.0, 0.0], [5.0, 5.0, 5.0], [-0.0, 1.0, 0.0]],
        &[&[0, 1, 2]],
    );
    let text = write(
        &[a, b],
        &WriteOptions {
            normals: false,
            weld: true,
        },
    );
    assert_eq!(text.lines().filter(|l| l.starts_with("v ")).count(), 4);
    assert_eq!(text.lines().filter(|l| *l == "f 1 2 3").count(), 2);
    assert!(text.lines().any(|l| l == "f 2 4 3"));
    // the reader shares the file's points; B refers back to A's
    let back = read(&text, &ReadOptions::default()).unwrap();
    assert_eq!(back.len(), 2);
    // B's points: the shared (1,0,0) and (0,1,0) of A, then its own (5,5,5)
    assert_eq!(back[1].mesh.points.len(), 3);
    assert_eq!(back[1].mesh.polys, vec![vec![0, 2, 1]]);
    assert_eq!(back[1].mesh.points[0], DVec3::new(1.0, 0.0, 0.0));
    assert_eq!(back[1].mesh.points[1], DVec3::new(0.0, 1.0, 0.0));
    assert_eq!(back[1].mesh.points[2], DVec3::new(5.0, 5.0, 5.0));
}

#[test]
fn normals_are_one_per_corner_and_deduplicated() {
    // a unit cube (faceted): 6 distinct normals
    let p = |x: f64, y: f64, z: f64| [x, y, z];
    let cube = object(
        "Cube",
        &[
            p(0., 0., 0.),
            p(0., 1., 0.),
            p(1., 0., 0.),
            p(1., 1., 0.),
            p(1., 0., 1.),
            p(1., 1., 1.),
            p(0., 0., 1.),
            p(0., 1., 1.),
        ],
        &[
            &[0, 1, 3, 2],
            &[2, 3, 5, 4],
            &[4, 5, 7, 6],
            &[6, 7, 1, 0],
            &[1, 7, 5, 3],
            &[6, 0, 2, 4],
        ],
    );
    let mut faceted = cube;
    faceted.smoothing_angle = None;
    let text = write(&[faceted], &WriteOptions::default());
    assert_eq!(text.lines().filter(|l| l.starts_with("vn ")).count(), 6);
    assert_eq!(text.lines().filter(|l| l.starts_with("f ")).count(), 6);
}

#[test]
fn negative_indices_are_relative_to_the_points_read_so_far() {
    let text = "v 0 0 0\nv 1 0 0\nv 0 1 0\nf -3 -2 -1\nv 1 1 0\nf -1 -2 -3\n";
    let o = read(text, &ReadOptions::default()).unwrap();
    assert_eq!(o.len(), 1);
    assert_eq!(o[0].mesh.polys, vec![vec![0, 1, 2], vec![3, 2, 1]]);
    assert_eq!(o[0].name, "Object");
}

#[test]
fn face_corner_forms_and_ignored_statements() {
    let text = "# comment\r\nmtllib x.mtl\r\nv 0 0 0 1\r\nv 1 0 0\r\nv 1 1 0\r\nv 0 1 0\r\n\
vt 0 0\r\nvt 1 0\r\nvt 1 1\r\nvn 0 0 1\r\nusemtl m\r\ns off\r\n\
f 1 2 3\r\nf 1/1 2/2 3/3\r\nf 1//1 2//1 3//1 4//1\r\nf 1/1/1 2/2/1 3/3/1\r\nl 1 2\r\nf 1 2 \\\r\n 3\r\n";
    let o = read(text, &ReadOptions::default()).unwrap();
    assert_eq!(o.len(), 1);
    assert_eq!(
        o[0].mesh.polys,
        vec![
            vec![0, 1, 2],
            vec![0, 1, 2],
            vec![0, 1, 2, 3],
            vec![0, 1, 2],
            vec![0, 1, 2]
        ]
    );
    assert_eq!(o[0].mesh.points.len(), 4);
}

#[test]
fn o_and_g_start_new_objects() {
    let text = "g Lonely\no First\nv 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n\
g Second part\nv 5 0 0\nv 6 0 0\nv 5 1 0\nf 4 5 6\n\
o Third\nf 1 4 5\n";
    let o = read(text, &ReadOptions::default()).unwrap();
    let names: Vec<&str> = o.iter().map(|x| x.name.as_str()).collect();
    assert_eq!(names, ["First", "Second part", "Third"]);
    assert_eq!(o[0].mesh.points.len(), 3);
    assert_eq!(o[1].mesh.points[0], DVec3::new(5.0, 0.0, 0.0));
    assert_eq!(o[1].mesh.polys, vec![vec![0, 1, 2]]);
    // Third declares no points: it uses global points 1, 4, 5 in ascending order
    assert_eq!(o[2].mesh.points.len(), 3);
    assert_eq!(o[2].mesh.points[0], DVec3::new(0.0, 0.0, 0.0));
    assert_eq!(o[2].mesh.polys, vec![vec![0, 1, 2]]);
}

#[test]
fn all_points_first_then_objects_use_only_their_points() {
    let text = "v 0 0 0\nv 1 0 0\nv 0 1 0\nv 7 0 0\nv 8 0 0\nv 7 1 0\n\
o A\nf 1 2 3\no B\nf 6 5 4\n";
    let o = read(text, &ReadOptions::default()).unwrap();
    // points before the first `o` belong to a face-less object that is dropped; A and B
    // get the points their faces use
    assert_eq!(o[0].mesh.points.len(), 3);
    assert_eq!(o[1].mesh.points.len(), 3);
    assert_eq!(o[1].mesh.points[0], DVec3::new(7.0, 0.0, 0.0));
    assert_eq!(o[1].mesh.polys, vec![vec![2, 1, 0]]);
}

#[test]
fn flip_z_negates_z_and_reverses_winding() {
    let text = "v 1 2 3\nv 4 5 -6\nv 7 8 9\nv 0 0 0\nf 1 2 3 4\n";
    let o = read(text, &ReadOptions { flip_z: true }).unwrap();
    assert_eq!(o[0].mesh.points[0], DVec3::new(1.0, 2.0, -3.0));
    assert_eq!(o[0].mesh.points[1], DVec3::new(4.0, 5.0, 6.0));
    assert_eq!(o[0].mesh.points[3].z, 0.0);
    assert_eq!(o[0].mesh.polys, vec![vec![3, 2, 1, 0]]);
    let plain = read(text, &ReadOptions::default()).unwrap();
    assert_eq!(plain[0].mesh.polys, vec![vec![0, 1, 2, 3]]);
    assert_eq!(plain[0].mesh.points[0].z, 3.0);
}

#[test]
fn errors_name_the_line() {
    for (text, line) in [
        ("v 0 0 0\nv 1 0 0\nf 1 2\n", 3),
        ("v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 4\n", 4),
        ("v 0 0 0\nv 1 0 0\nv 0 1 0\nf 0 1 2\n", 4),
        ("v 0 0 0\nv 1 0 x\n", 2),
        ("v 0 0 NaN\n", 1),
        ("v 0 0\n", 1),
        ("v 0 0 0\nv 1 0 0\nv 0 1 0\nf -4 1 2\n", 4),
    ] {
        let e = read(text, &ReadOptions::default()).unwrap_err();
        assert_eq!(e.line, line, "{text:?}: {e}");
    }
}

#[test]
fn empty_input_gives_no_objects() {
    assert!(read("", &ReadOptions::default()).unwrap().is_empty());
    assert!(
        read("# only a comment\nvn 0 0 1\n", &ReadOptions::default())
            .unwrap()
            .is_empty()
    );
}
