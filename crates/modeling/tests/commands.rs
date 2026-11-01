//! Behaviour of the mesh commands the golden cases cannot see (docs/specs/MODELING.md §4).

use std::collections::HashMap;

use forge_geometry::PolyMesh;
use forge_geometry::mesh::poly_edges;
use forge_geometry::selection::Selection;
use forge_modeling::{Settings, ToolError, tool_desc, tool_fn};
use glam::DVec3;
use serde_json::json;

fn settings(id: &str, object: serde_json::Value) -> Settings {
    let desc = tool_desc(id).unwrap();
    Settings::from_json(&desc, object.as_object().unwrap()).unwrap()
}

fn run(
    id: &str,
    mesh: &PolyMesh,
    sel: &Selection,
    s: serde_json::Value,
) -> Result<PolyMesh, ToolError> {
    tool_fn(id).unwrap()(mesh, sel, &settings(id, s)).map(|r| r.0)
}

fn polys(items: &[u32]) -> Selection {
    let mut s = Selection::default();
    for i in items {
        s.polys.insert(*i);
    }
    s
}

fn points(items: &[u32]) -> Selection {
    let mut s = Selection::default();
    for i in items {
        s.points.insert(*i);
    }
    s
}

/// `n x n` quads in the plane z = 0, counter-clockwise seen from +z, points row by row.
fn grid(n: usize) -> PolyMesh {
    let w = n + 1;
    let points = (0..w * w)
        .map(|k| DVec3::new((k % w) as f64, (k / w) as f64, 0.0))
        .collect();
    let mut polys = Vec::new();
    for y in 0..n {
        for x in 0..n {
            let a = (y * w + x) as u32;
            polys.push(vec![a, a + 1, a + w as u32 + 1, a + w as u32]);
        }
    }
    PolyMesh { points, polys }
}

fn cube() -> PolyMesh {
    let p = |x: f64, y: f64, z: f64| DVec3::new(x, y, z);
    PolyMesh {
        points: vec![
            p(-1.0, -1.0, -1.0),
            p(-1.0, 1.0, -1.0),
            p(1.0, -1.0, -1.0),
            p(1.0, 1.0, -1.0),
            p(1.0, -1.0, 1.0),
            p(1.0, 1.0, 1.0),
            p(-1.0, -1.0, 1.0),
            p(-1.0, 1.0, 1.0),
        ],
        polys: vec![
            vec![0, 1, 3, 2],
            vec![2, 3, 5, 4],
            vec![4, 5, 7, 6],
            vec![6, 7, 1, 0],
            vec![1, 7, 5, 3],
            vec![6, 0, 2, 4],
        ],
    }
}

/// Every undirected edge is used by exactly two polygons, in opposite directions.
fn assert_closed(mesh: &PolyMesh) {
    mesh.validate().unwrap();
    let mut dir: HashMap<(u32, u32), u32> = HashMap::new();
    for poly in &mesh.polys {
        for k in 0..poly.len() {
            *dir.entry((poly[k], poly[(k + 1) % poly.len()]))
                .or_default() += 1;
        }
    }
    for ((a, b), n) in &dir {
        assert_eq!(*n, 1, "edge {a}->{b} used twice in one direction");
        assert!(dir.contains_key(&(*b, *a)), "edge {a}->{b} has no twin");
    }
}

fn volume(mesh: &PolyMesh) -> f64 {
    let mut v = 0.0;
    for poly in &mesh.polys {
        for k in 1..poly.len() - 1 {
            let [a, b, c] = [poly[0], poly[k], poly[k + 1]].map(|i| mesh.points[i as usize]);
            v += a.dot(b.cross(c)) / 6.0;
        }
    }
    v
}

#[test]
fn subdivide_counts_and_closed() {
    for levels in 1..=3 {
        let out = run(
            "subdivide",
            &cube(),
            &Selection::default(),
            json!({"subdivisions": levels}),
        )
        .unwrap();
        let n = 1usize << levels;
        assert_eq!(out.polys.len(), 6 * n * n, "level {levels}");
        assert_eq!(
            out.points.len(),
            6 * n * n + 2,
            "Euler: V = F + 2 for a quad cube"
        );
        assert_closed(&out);
    }
}

#[test]
fn subdivide_hyper_shrinks_the_volume_and_plain_keeps_it() {
    let plain = run(
        "subdivide",
        &cube(),
        &Selection::default(),
        json!({"subdivisions": 2}),
    )
    .unwrap();
    assert!((volume(&plain) - 8.0).abs() < 1e-9);
    let hyper = run(
        "subdivide",
        &cube(),
        &Selection::default(),
        json!({"subdivisions": 2, "hyper": true}),
    )
    .unwrap();
    assert!(volume(&hyper) < 8.0 && volume(&hyper) > 2.0);
    // the same topology and numbering as the plain result
    assert_eq!(hyper.polys, plain.polys);
}

#[test]
fn subdivide_triangles_and_ngons_stay_valid() {
    let tri = PolyMesh {
        points: vec![DVec3::ZERO, DVec3::X, DVec3::Y],
        polys: vec![vec![0, 1, 2]],
    };
    let out = run("subdivide", &tri, &Selection::default(), json!({})).unwrap();
    assert_eq!((out.points.len(), out.polys.len()), (7, 3));
    out.validate().unwrap();
    let out = run(
        "subdivide",
        &tri,
        &Selection::default(),
        json!({"subdivisions": 2}),
    )
    .unwrap();
    assert_eq!(out.polys.len(), 12);
    out.validate().unwrap();
}

#[test]
fn subdivide_partial_selection_is_unsupported() {
    let r = run("subdivide", &cube(), &polys(&[0]), json!({}));
    assert!(matches!(r, Err(ToolError::Unsupported(_))), "{r:?}");
    let r = run("subdivide", &cube(), &polys(&[9]), json!({}));
    assert_eq!(r, Err(ToolError::BadPolygon(9)));
}

#[test]
fn triangulate_fan_ngons_and_concave() {
    let pent = PolyMesh {
        points: (0..5)
            .map(|k| {
                let a = k as f64 * std::f64::consts::TAU / 5.0;
                DVec3::new(a.cos(), a.sin(), 0.0)
            })
            .collect(),
        polys: vec![vec![0, 1, 2, 3, 4]],
    };
    let out = run("triangulate", &pent, &Selection::default(), json!({})).unwrap();
    assert_eq!(out.polys, vec![vec![0, 1, 2], vec![0, 2, 3], vec![0, 3, 4]]);
    // an arrow-head quad: the fan from corner 0 would turn a triangle around
    let dart = PolyMesh {
        points: vec![
            DVec3::new(0.0, 0.0, 0.0),
            DVec3::new(2.0, -2.0, 0.0),
            DVec3::new(1.0, 0.0, 0.0),
            DVec3::new(2.0, 2.0, 0.0),
        ],
        polys: vec![vec![0, 1, 2, 3]],
    };
    let out = run("triangulate", &dart, &Selection::default(), json!({})).unwrap();
    assert_eq!(out.polys.len(), 2);
    for t in &out.polys {
        let [a, b, c] = [t[0], t[1], t[2]].map(|i| dart.points[i as usize]);
        assert!((b - a).cross(c - a).z > 0.0, "{t:?} keeps the winding");
    }
    // only the selected polygons, the rest keeps its index
    let out = run("triangulate", &grid(2), &polys(&[1]), json!({})).unwrap();
    assert_eq!(out.polys.len(), 5);
    assert_eq!(out.polys[0].len(), 4);
    assert_eq!(out.polys[1].len(), 3);
    assert_eq!(out.polys[4].len(), 3);
}

#[test]
fn untriangulate_round_trip_and_options() {
    let g = grid(3);
    let tri = run("triangulate", &g, &Selection::default(), json!({})).unwrap();
    assert_eq!(tri.polys.len(), 18);
    let back = run("untriangulate", &tri, &Selection::default(), json!({})).unwrap();
    assert_eq!(back.polys, g.polys);
    // N-gons: the whole coplanar plane becomes one polygon
    let one = run(
        "untriangulate",
        &tri,
        &Selection::default(),
        json!({"ngons": true}),
    )
    .unwrap();
    assert_eq!(one.polys.len(), 1);
    assert_eq!(one.polys[0].len(), 12, "the boundary of a 3 x 3 grid");
    // the cube is not coplanar across its edges: quads stay quads, nothing else merges
    let c = run("triangulate", &cube(), &Selection::default(), json!({})).unwrap();
    let back = run(
        "untriangulate",
        &c,
        &Selection::default(),
        json!({"ngons": true}),
    )
    .unwrap();
    assert_eq!(back.polys.len(), 6);
    // a hinge (two triangles 90 degrees apart) is not merged
    let hinge = PolyMesh {
        points: vec![DVec3::ZERO, DVec3::X, DVec3::Y, DVec3::Z],
        polys: vec![vec![0, 1, 2], vec![0, 3, 1]],
    };
    let out = run(
        "untriangulate",
        &hinge,
        &Selection::default(),
        json!({"angle_deg": 45.0}),
    )
    .unwrap();
    assert_eq!(out.polys.len(), 2);
    // a pair that would give a concave quad is not merged
    let dart = PolyMesh {
        points: vec![
            DVec3::new(0.0, 0.0, 0.0),
            DVec3::new(2.0, -2.0, 0.0),
            DVec3::new(1.0, 0.0, 0.0),
            DVec3::new(2.0, 2.0, 0.0),
        ],
        polys: vec![vec![0, 1, 2], vec![0, 2, 3]],
    };
    let out = run("untriangulate", &dart, &Selection::default(), json!({})).unwrap();
    assert_eq!(out.polys.len(), 2);
}

#[test]
fn reverse_normals_flips_the_winding_and_keeps_the_first_corner() {
    let out = run("reverse_normals", &cube(), &polys(&[4]), json!({})).unwrap();
    assert_eq!(out.polys[4], vec![1, 3, 5, 7]);
    assert_eq!(out.polys[0], cube().polys[0]);
    let all = run("reverse_normals", &cube(), &Selection::default(), json!({})).unwrap();
    assert!((volume(&all) + 8.0).abs() < 1e-9, "every polygon reversed");
    let twice = run("reverse_normals", &all, &Selection::default(), json!({})).unwrap();
    assert_eq!(twice, cube());
}

#[test]
fn optimize_options() {
    let mut m = cube();
    // a duplicate of point 1, a duplicate polygon, a degenerate polygon and an unused point
    m.points.push(m.points[1]);
    m.points.push(DVec3::new(9.0, 9.0, 9.0));
    m.polys.push(vec![8, 7, 5, 3]);
    m.polys.push(vec![0, 1, 3, 3]);
    let all = run("optimize", &m, &Selection::default(), json!({})).unwrap();
    assert_eq!(
        all,
        cube(),
        "points merged, copies and degenerate polygons gone"
    );
    // tolerance: points 0.5 apart merge only when the tolerance reaches that
    let mut near = cube();
    near.points.push(near.points[1] + DVec3::new(0.0, 0.5, 0.0));
    near.polys.push(vec![8, 7, 5, 3]);
    let tight = run(
        "optimize",
        &near,
        &Selection::default(),
        json!({"tolerance": 0.4}),
    )
    .unwrap();
    assert_eq!(tight.points.len(), 9);
    let loose = run(
        "optimize",
        &near,
        &Selection::default(),
        json!({"tolerance": 0.5}),
    )
    .unwrap();
    assert_eq!(loose.points.len(), 8);
    // switches
    let no_points = run(
        "optimize",
        &m,
        &Selection::default(),
        json!({"optimize_points": false}),
    )
    .unwrap();
    assert_eq!(
        no_points.points.len(),
        9,
        "the unused point goes, the copy is still used"
    );
    let keep = run(
        "optimize",
        &m,
        &Selection::default(),
        json!({"delete_unused_points": false}),
    )
    .unwrap();
    assert_eq!(keep.points.len(), 10);
    let polys_on = run(
        "optimize",
        &m,
        &Selection::default(),
        json!({"optimize_polygons": false}),
    )
    .unwrap();
    assert_eq!(
        polys_on.polys.len(),
        8,
        "the duplicate stays, the degenerate one stays"
    );
}

#[test]
fn weld_swap_removes_and_drops_degenerate_polygons() {
    // three points of one triangle: the polygon collapses and goes
    let m = PolyMesh {
        points: vec![DVec3::ZERO, DVec3::X, DVec3::Y, DVec3::Z, DVec3::ONE],
        polys: vec![vec![0, 1, 2], vec![2, 1, 3], vec![1, 4, 3]],
    };
    let out = run("weld", &m, &points(&[0, 1]), json!({})).unwrap();
    assert_eq!(out.points.len(), 4);
    // point 4 moved into slot 1, the first polygon became a segment and was dropped
    assert_eq!(out.points[1], DVec3::ONE);
    assert_eq!(out.points[0], DVec3::new(0.5, 0.0, 0.0));
    // polygon 0 collapsed and was swap-removed (polygon 2 took its slot)
    assert_eq!(out.polys, vec![vec![0, 1, 3], vec![2, 0, 3]]);
    assert_eq!(
        run("weld", &m, &points(&[0]), json!({})),
        Err(ToolError::Invalid("select at least two points".into()))
    );
    assert_eq!(
        run("weld", &m, &polys(&[0]), json!({})),
        Err(ToolError::EmptySelection)
    );
    assert_eq!(
        run("weld", &m, &points(&[0, 9]), json!({})),
        Err(ToolError::BadPoint(9))
    );
}

#[test]
fn melt_removes_interior_points_and_refuses_holes() {
    // the four polygons around the middle point of a 2 x 2 grid become one square
    let g = grid(2);
    let out = run("melt", &g, &polys(&[0, 1, 2, 3]), json!({})).unwrap();
    assert_eq!(out.polys.len(), 1);
    assert_eq!(out.polys[0].len(), 8, "boundary of the 2 x 2 grid");
    assert_eq!(out.points.len(), 8, "the middle point is gone");
    out.validate().unwrap();
    // the ring around the middle polygon of a 3 x 3 grid has a hole
    let g3 = grid(3);
    let ring = polys(&[0, 1, 2, 3, 5, 6, 7, 8]);
    assert!(matches!(
        run("melt", &g3, &ring, json!({})),
        Err(ToolError::Unsupported(_))
    ));
    // two separate pairs make two groups
    let g4 = grid(4);
    let out = run("melt", &g4, &polys(&[0, 1, 14, 15]), json!({})).unwrap();
    assert_eq!(out.polys.len(), 14);
    // a lone polygon, or polygons that only touch at a corner, are refused
    assert!(matches!(
        run("melt", &g, &polys(&[0]), json!({})),
        Err(ToolError::Invalid(_))
    ));
    assert!(matches!(
        run("melt", &g, &polys(&[0, 3]), json!({})),
        Err(ToolError::Invalid(_))
    ));
    // a closed surface has no boundary
    let all = polys(&[0, 1, 2, 3, 4, 5]);
    assert!(matches!(
        run("melt", &cube(), &all, json!({})),
        Err(ToolError::Unsupported(_))
    ));
}

#[test]
fn dissolve_edges_points_and_polygons() {
    let g = grid(2);
    let edges = g.edges();
    let id = |a, b| {
        let e = forge_geometry::Edge::new(a, b);
        edges.iter().position(|x| *x == e).unwrap() as u32
    };
    let mut sel = Selection::default();
    sel.edges.insert(id(1, 4));
    let out = run("dissolve", &g, &sel, json!({})).unwrap();
    assert_eq!(out.polys.len(), 3);
    assert!(out.polys.iter().any(|p| p.len() == 6));
    // a boundary edge has one polygon: nothing to do
    let mut border = Selection::default();
    border.edges.insert(id(0, 1));
    assert!(matches!(
        run("dissolve", &g, &border, json!({})),
        Err(ToolError::Invalid(_))
    ));
    // the middle point: its four polygons join and the point goes
    let out = run("dissolve", &g, &points(&[4]), json!({})).unwrap();
    assert_eq!((out.polys.len(), out.points.len()), (1, 8));
    // a corner point used by one quad leaves a triangle
    let out = run("dissolve", &g, &points(&[0]), json!({})).unwrap();
    assert_eq!(out.polys[0].len(), 3);
    assert_eq!(out.points.len(), 8);
    // polygons: the same as Melt
    let out = run("dissolve", &g, &polys(&[0, 1]), json!({})).unwrap();
    assert_eq!(out.polys.len(), 3);
    // a triangle cannot lose a corner
    let tri = PolyMesh {
        points: vec![DVec3::ZERO, DVec3::X, DVec3::Y],
        polys: vec![vec![0, 1, 2]],
    };
    assert!(matches!(
        run("dissolve", &tri, &points(&[0]), json!({})),
        Err(ToolError::Unsupported(_))
    ));
    assert_eq!(
        run("dissolve", &g, &Selection::default(), json!({})),
        Err(ToolError::EmptySelection)
    );
}

#[test]
fn split_copies_the_selected_polygons() {
    let g = grid(2);
    let copy = forge_modeling::split_copy(&g, &polys(&[1, 3])).unwrap();
    // points of polygons 1 and 3, ascending: 1, 2, 4, 5 and 3, 4, 6, 7 -> 1 2 3 4 5 6 7 (4 shared)
    assert_eq!(copy.points.len(), 6);
    assert_eq!(copy.polys.len(), 2);
    copy.validate().unwrap();
    assert_eq!(copy.points[0], g.points[1]);
    assert!(forge_modeling::split_copy(&g, &Selection::default()).is_err());
    // the tool function leaves the source whole
    let (same, sel) =
        tool_fn("split").unwrap()(&g, &polys(&[1]), &settings("split", json!({}))).unwrap();
    assert_eq!(same, g);
    assert_eq!(sel.polys.len(), 1);
}

#[test]
fn disconnect_groups_and_unshared_points() {
    let g = grid(2);
    // two neighbours: grouped they share their points, alone they do not
    let grouped = run("disconnect", &g, &polys(&[0, 1]), json!({})).unwrap();
    let single = run(
        "disconnect",
        &g,
        &polys(&[0, 1]),
        json!({"preserve_groups": false}),
    )
    .unwrap();
    // points 3, 4, 5 are also used by the polygons below; the group copies them once
    assert_eq!(grouped.points.len(), 9 + 3);
    // alone, each polygon copies the points it shares with any other polygon (1, 3, 4 / 1, 4, 5)
    assert_eq!(single.points.len(), 9 + 6);
    // every polygon selected: no point is shared with an outside polygon
    let all = run(
        "disconnect",
        &cube(),
        &polys(&[0, 1, 2, 3, 4, 5]),
        json!({}),
    )
    .unwrap();
    assert_eq!(all, cube());
    // the first polygon after disconnecting a group touches the rest only along nothing
    let c = run("disconnect", &cube(), &polys(&[4]), json!({})).unwrap();
    let used: std::collections::HashSet<u32> = c.polys[4].iter().copied().collect();
    for (i, p) in c.polys.iter().enumerate() {
        if i != 4 {
            assert!(
                p.iter().all(|x| !used.contains(x)),
                "polygon {i} shares a point"
            );
        }
    }
}

#[test]
fn connect_points_cuts_ngons_and_refuses_the_rest() {
    let hex = PolyMesh {
        points: (0..6)
            .map(|k| {
                let a = k as f64 * std::f64::consts::TAU / 6.0;
                DVec3::new(a.cos(), a.sin(), 0.0)
            })
            .collect(),
        polys: vec![vec![0, 1, 2, 3, 4, 5]],
    };
    let out = run("connect_points", &hex, &points(&[1, 4]), json!({})).unwrap();
    assert_eq!(out.polys, vec![vec![0, 1, 4, 5], vec![1, 2, 3, 4]]);
    // neighbours: nothing to cut
    assert!(matches!(
        run("connect_points", &hex, &points(&[0, 5]), json!({})),
        Err(ToolError::Invalid(_))
    ));
    assert!(matches!(
        run("connect_points", &hex, &points(&[0, 1]), json!({})),
        Err(ToolError::Invalid(_))
    ));
    assert!(matches!(
        run("connect_points", &hex, &points(&[0, 2, 4]), json!({})),
        Err(ToolError::Unsupported(_))
    ));
    // the diagonal of the cube's faces: only the polygons that hold both points are cut
    let out = run("connect_points", &cube(), &points(&[0, 3]), json!({})).unwrap();
    assert_eq!(out.polys.len(), 7);
    assert_closed(&out);
}

#[test]
fn mirror_reverses_and_duplicates() {
    let asym = PolyMesh {
        points: vec![
            DVec3::new(1.0, 0.0, 0.0),
            DVec3::new(3.0, 0.0, 0.0),
            DVec3::new(3.0, 2.0, 0.0),
            DVec3::new(1.0, 2.0, 0.0),
        ],
        polys: vec![vec![0, 1, 2, 3]],
    };
    let out = run("mirror", &asym, &Selection::default(), json!({})).unwrap();
    assert_eq!(out.points[1], DVec3::new(-3.0, 0.0, 0.0));
    assert_eq!(out.polys[0], vec![0, 3, 2, 1]);
    // still facing +z: the mirrored polygon was reversed
    let n = forge_geometry::normals::poly_normal(&out.points, &out.polys[0]);
    assert!(n.z > 0.0);
    // a plane through a point and a tilted normal
    let out = run(
        "mirror",
        &asym,
        &Selection::default(),
        json!({"plane_point": [0.0, 1.0, 0.0], "plane_normal": [0.0, 3.0, 0.0]}),
    )
    .unwrap();
    assert_eq!(out.points[0], DVec3::new(1.0, 2.0, 0.0));
    // duplicate: the point on the plane is shared with welding, copied without
    let dup = json!({"duplicate": true, "plane_point": [1.0, 0.0, 0.0]});
    let out = run("mirror", &asym, &Selection::default(), dup.clone()).unwrap();
    assert_eq!((out.points.len(), out.polys.len()), (6, 2));
    // points 0 and 3 lie on the plane x = 1 and are shared; 1 and 2 are reflected to 4 and 5
    assert_eq!(out.polys[1], vec![0, 3, 5, 4]);
    assert_eq!(out.points[4], DVec3::new(-1.0, 0.0, 0.0));
    let n = forge_geometry::normals::poly_normal(&out.points, &out.polys[1]);
    assert!(n.z > 0.0);
    let loose = run(
        "mirror",
        &asym,
        &Selection::default(),
        json!({"duplicate": true, "plane_point": [1.0, 0.0, 0.0], "weld": false}),
    )
    .unwrap();
    assert_eq!(loose.points.len(), 8);
    // a zero normal is refused
    let r = run(
        "mirror",
        &asym,
        &Selection::default(),
        json!({"plane_normal": [0.0, 0.0, 0.0]}),
    );
    assert!(matches!(r, Err(ToolError::Invalid(_))));
}

#[test]
fn center_axis_modes() {
    let mut m = cube();
    for p in &mut m.points {
        *p += DVec3::new(10.0, 0.0, 0.0);
    }
    m.points[0].x += 4.0;
    let bbox = forge_modeling::center_axis(&m, &settings("center_axis", json!({}))).unwrap();
    assert_eq!(bbox.1, DVec3::new(11.0, 0.0, 0.0));
    let avg = forge_modeling::center_axis(
        &m,
        &settings("center_axis", json!({"mode": "point_average"})),
    )
    .unwrap();
    assert_eq!(avg.1, DVec3::new(10.5, 0.0, 0.0));
    // c + point is the old point
    assert_eq!(bbox.0.points[3] + bbox.1, m.points[3]);
    let empty = PolyMesh::default();
    assert!(forge_modeling::center_axis(&empty, &settings("center_axis", json!({}))).is_err());
}

#[test]
fn settings_reject_bad_values() {
    let desc = tool_desc("mirror").unwrap();
    let bad = |v: serde_json::Value| Settings::from_json(&desc, v.as_object().unwrap());
    assert!(bad(json!({"plane_normal": [1, 2]})).is_err());
    assert!(bad(json!({"plane_normal": [1, "a", 3]})).is_err());
    assert!(bad(json!({"plane_normal": 1})).is_err());
    let ok = bad(json!({"plane_normal": [0, 0, 5]})).unwrap();
    assert_eq!(ok.vec3("plane_normal"), DVec3::new(0.0, 0.0, 5.0));
    assert_eq!(ok.to_json()["plane_normal"], json!([0.0, 0.0, 5.0]));
    // subdivisions are clamped into 1..8
    let d = tool_desc("subdivide").unwrap();
    let s = Settings::from_json(&d, json!({"subdivisions": 99}).as_object().unwrap()).unwrap();
    assert_eq!(s.int("subdivisions"), 8);
}

#[test]
fn every_edge_is_still_manifold_after_closed_commands() {
    // triangulate and subdivide keep a closed mesh closed
    let t = run("triangulate", &cube(), &Selection::default(), json!({})).unwrap();
    assert_closed(&t);
    let s = run(
        "subdivide",
        &t,
        &Selection::default(),
        json!({"subdivisions": 2}),
    )
    .unwrap();
    assert_closed(&s);
    let _ = poly_edges(&s.polys[0]).count();
}
