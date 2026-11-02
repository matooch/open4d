//! Bridge, Close Polygon Hole, Stitch and Sew, Polygon Pen (docs/specs/MODELING.md §6). No golden
//! data exists for these tools (O25 could not run them), so these tests are the proof of the rules.

use std::collections::HashMap;

use forge_geometry::selection::Selection;
use forge_geometry::{Edge, PolyMesh};
use forge_modeling::{
    BridgeEnd, PenCorner, Settings, StitchTo, ToolError, border_loop, bridge, close_hole,
    disconnect, pen, stitch, stitch_edges, tool_desc,
};
use glam::DVec3;

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

/// The cube without its polygons 0 and 2: an open tube with two border loops.
fn tube() -> PolyMesh {
    let mut c = cube();
    c.polys = vec![
        c.polys[1].clone(),
        c.polys[3].clone(),
        c.polys[4].clone(),
        c.polys[5].clone(),
    ];
    c
}

/// The cube without its polygon 4 (the hole is the 1, 7, 5, 3 face).
fn open_cube() -> PolyMesh {
    let mut c = cube();
    c.polys.remove(4);
    c
}

fn edge_id(mesh: &PolyMesh, a: u32, b: u32) -> u32 {
    mesh.edge_ids()[&Edge::new(a, b)]
}

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

fn near(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

/// Two cubes, the second moved up by 3: its bottom polygon is 6 and the first one's top is 2.
fn stack() -> PolyMesh {
    let c = cube();
    let mut m = c.clone();
    m.points
        .extend(c.points.iter().map(|p| *p + DVec3::new(0.0, 0.0, 3.0)));
    m.polys.extend(
        c.polys
            .iter()
            .map(|p| p.iter().map(|i| i + 8).collect::<Vec<u32>>()),
    );
    m
}

// ------------------------------------------------------------------ border loops

#[test]
fn a_border_loop_runs_in_the_direction_of_its_polygon() {
    let t = tube();
    let lp = border_loop(&t, edge_id(&t, 0, 1)).unwrap();
    assert_eq!(lp.len(), 4);
    let lp2 = border_loop(&t, edge_id(&t, 6, 4)).unwrap();
    assert_eq!(lp2.len(), 4);
    // the loop starts at the start of the given edge in its polygon: polygon [6,7,1,0] runs 1 -> 0
    assert_eq!(&lp[..2], &[1, 0]);
    // a second polygon around the loop would have to run the opposite way
    assert_eq!(border_loop(&t, edge_id(&t, 1, 0)).unwrap(), lp);
}

#[test]
fn only_border_edges_have_a_loop() {
    let t = tube();
    assert!(matches!(
        border_loop(&t, edge_id(&t, 3, 5)),
        Err(ToolError::Invalid(_))
    ));
    assert_eq!(border_loop(&t, 999), Err(ToolError::BadEdge(999)));
    // the closed cube has no border at all
    let c = cube();
    assert!(matches!(border_loop(&c, 0), Err(ToolError::Invalid(_))));
}

#[test]
fn a_pinched_border_is_refused() {
    // two triangles that meet at one point: the border visits point 0 twice
    let m = PolyMesh {
        points: vec![DVec3::ZERO, DVec3::X, DVec3::Y, DVec3::NEG_X, DVec3::NEG_Y],
        polys: vec![vec![0, 1, 2], vec![0, 3, 4]],
    };
    assert!(matches!(border_loop(&m, 0), Err(ToolError::Invalid(_))));
}

// ------------------------------------------------------------------ bridge

#[test]
fn bridging_two_open_cube_ends_gives_a_closed_tube() {
    // two cubes, the facing polygons (2 of the lower, 6 of the upper) deleted: two open ends
    let mut s = stack();
    s.polys.remove(6);
    s.polys.remove(2);
    let lower = edge_id(&s, 4, 5);
    let upper = edge_id(&s, 8, 9);
    let out = bridge(&s, BridgeEnd::Edge(lower), BridgeEnd::Edge(upper), 1).unwrap();
    assert_eq!((out.mesh.points.len(), out.mesh.polys.len()), (16, 10 + 4));
    assert_eq!(out.new_polys, vec![10, 11, 12, 13]);
    assert!(out.new_points.is_empty());
    assert_closed(&out.mesh);
    assert!(near(volume(&out.mesh), 20.0), "{}", volume(&out.mesh));
    // every edge is used by exactly two polygons
    let mut uses: HashMap<Edge, u32> = HashMap::new();
    for p in &out.mesh.polys {
        for e in forge_geometry::mesh::poly_edges(p) {
            *uses.entry(e).or_default() += 1;
        }
    }
    assert!(uses.values().all(|n| *n == 2));
}

#[test]
fn bridging_across_a_tube_adds_a_second_skin() {
    // both loops of one open tube: the quads connect the rings (the sides stay)
    let t = tube();
    let out = bridge(
        &t,
        BridgeEnd::Edge(edge_id(&t, 0, 1)),
        BridgeEnd::Edge(edge_id(&t, 4, 6)),
        1,
    )
    .unwrap();
    assert_eq!((out.mesh.points.len(), out.mesh.polys.len()), (8, 8));
    assert_eq!(out.new_polys, vec![4, 5, 6, 7]);
}

#[test]
fn bridging_two_polygons_joins_a_stack_of_cubes_into_one_block() {
    let s = stack();
    // top of the lower cube is polygon 2, bottom of the upper cube is polygon 6
    let out = bridge(&s, BridgeEnd::Poly(2), BridgeEnd::Poly(6), 1).unwrap();
    assert_eq!((out.mesh.points.len(), out.mesh.polys.len()), (16, 10 + 4));
    assert_closed(&out.mesh);
    // 2 x 2 x 2 + 2 x 2 x 2 + the 2 x 2 x 1 gap
    assert!(near(volume(&out.mesh), 20.0), "{}", volume(&out.mesh));
    assert_eq!(out.new_polys.len(), 4);
    assert!(
        out.new_polys
            .iter()
            .all(|p| (*p as usize) < out.mesh.polys.len())
    );
    // the order of the two ends does not matter for the result's validity
    let flipped = bridge(&s, BridgeEnd::Poly(6), BridgeEnd::Poly(2), 1).unwrap();
    assert_closed(&flipped.mesh);
    assert!(near(volume(&flipped.mesh), 20.0));
}

#[test]
fn segments_add_rings_between_the_loops() {
    let s = stack();
    let out = bridge(&s, BridgeEnd::Poly(2), BridgeEnd::Poly(6), 3).unwrap();
    // two rings of four new points, twelve new quads
    assert_eq!(
        (out.mesh.points.len(), out.mesh.polys.len()),
        (16 + 8, 10 + 12)
    );
    assert_eq!(out.new_points, (16..24).collect::<Vec<u32>>());
    assert_closed(&out.mesh);
    assert!(near(volume(&out.mesh), 20.0));
    // ring 1 sits a third of the way, ring 2 two thirds of the way from z = 1 to z = 2
    for p in &out.mesh.points[16..20] {
        assert!(near(p.z, 1.0 + 1.0 / 3.0), "{p:?}");
    }
    for p in &out.mesh.points[20..24] {
        assert!(near(p.z, 1.0 + 2.0 / 3.0), "{p:?}");
    }
}

#[test]
fn the_rotation_with_the_shortest_total_edge_length_is_chosen() {
    // the same stack but the upper cube's bottom polygon starts at another corner
    let mut s = stack();
    s.polys[6].rotate_left(2);
    let out = bridge(&s, BridgeEnd::Poly(2), BridgeEnd::Poly(6), 1).unwrap();
    assert_closed(&out.mesh);
    // each bridge quad is a straight 1-high wall: its two vertical edges are the shortest
    for q in &out.new_polys {
        let poly = &out.mesh.polys[*q as usize];
        let (b, c) = (poly[1], poly[2]);
        let side = out.mesh.points[b as usize] - out.mesh.points[c as usize];
        assert!(near(side.length(), 1.0), "{poly:?}");
    }
    assert!(near(volume(&out.mesh), 20.0));
}

#[test]
fn a_tie_chooses_the_smallest_rotation() {
    // a square and the same square turned by 45 degrees one unit above: two rotations tie
    let ring = |turn: f64, z: f64| -> Vec<DVec3> {
        (0..4)
            .map(|i| {
                let a = turn + f64::from(i) * std::f64::consts::FRAC_PI_2;
                DVec3::new(a.cos(), a.sin(), z)
            })
            .collect()
    };
    let mut points = ring(0.0, 0.0);
    points.extend(ring(std::f64::consts::FRAC_PI_4, 1.0));
    let m = PolyMesh {
        points,
        polys: vec![vec![0, 1, 2, 3], vec![4, 5, 6, 7]],
    };
    let out = bridge(&m, BridgeEnd::Poly(0), BridgeEnd::Poly(1), 1).unwrap();
    // loop A is polygon 0 reversed, the pairing walks polygon 1 forward from its rotation k
    let la: Vec<u32> = vec![3, 2, 1, 0];
    let lb = [4u32, 5, 6, 7];
    let cost = |k: usize| -> f64 {
        (0..4)
            .map(|i| (m.points[la[i] as usize] - m.points[lb[(i + k) % 4] as usize]).length())
            .sum()
    };
    let costs: Vec<f64> = (0..4).map(cost).collect();
    let min = costs.iter().copied().fold(f64::MAX, f64::min);
    let ties: Vec<usize> = (0..4).filter(|k| (costs[*k] - min).abs() < 1e-12).collect();
    assert!(ties.len() >= 2, "{costs:?}");
    // new polygons are [r[i+1], r[i], partner[i], partner[i+1]]: polygon 0 shows partner[0]
    let first = &out.mesh.polys[out.new_polys[0] as usize];
    assert_eq!(first[2], lb[ties[0]]);
    out.mesh.validate().unwrap();
}

#[test]
fn bridge_refuses_what_it_cannot_do() {
    let t = tube();
    let e = |a, b| edge_id(&t, a, b);
    // same loop
    assert!(matches!(
        bridge(&t, BridgeEnd::Edge(e(0, 1)), BridgeEnd::Edge(e(1, 3)), 1),
        Err(ToolError::Invalid(_))
    ));
    // not a border edge
    assert!(matches!(
        bridge(&t, BridgeEnd::Edge(e(3, 5)), BridgeEnd::Edge(e(0, 1)), 1),
        Err(ToolError::Invalid(_))
    ));
    // segments
    for s in [0, 65] {
        assert!(matches!(
            bridge(&t, BridgeEnd::Edge(e(0, 1)), BridgeEnd::Edge(e(4, 6)), s),
            Err(ToolError::Invalid(_))
        ));
    }
    // different sizes: a triangle against the square
    let mut m = stack();
    m.points.push(DVec3::new(5.0, 5.0, 5.0));
    m.polys.push(vec![8, 9, 16]);
    assert!(matches!(
        bridge(
            &m,
            BridgeEnd::Poly(2),
            BridgeEnd::Poly(m.polys.len() as u32 - 1),
            1
        ),
        Err(ToolError::Invalid(_))
    ));
    // the same polygon twice, and a missing one
    let s = stack();
    assert!(matches!(
        bridge(&s, BridgeEnd::Poly(2), BridgeEnd::Poly(2), 1),
        Err(ToolError::Invalid(_))
    ));
    assert_eq!(
        bridge(&s, BridgeEnd::Poly(2), BridgeEnd::Poly(99), 1),
        Err(ToolError::BadPolygon(99))
    );
}

// ------------------------------------------------------------------ close hole

#[test]
fn closing_the_hole_of_an_open_cube_restores_the_polygon() {
    let m = open_cube();
    assert_eq!(m.polys.len(), 5);
    let out = close_hole(&m, edge_id(&m, 1, 7)).unwrap();
    assert_eq!(out.mesh.polys.len(), 6);
    assert_eq!(out.mesh.points.len(), 8);
    assert_eq!(out.new_polys, vec![5]);
    assert_closed(&out.mesh);
    assert!(near(volume(&out.mesh), 8.0));
    // the same four points as the deleted polygon [1, 7, 5, 3], as a rotation of it
    let mut got = out.mesh.polys[5].clone();
    while got[0] != 1 {
        got.rotate_left(1);
    }
    assert_eq!(got, vec![1, 7, 5, 3]);
}

#[test]
fn the_new_polygon_starts_at_the_start_of_the_given_edge() {
    let m = open_cube();
    for (a, b) in [(1, 7), (7, 5), (5, 3), (3, 1)] {
        let id = edge_id(&m, a, b);
        let lp = border_loop(&m, id).unwrap();
        let out = close_hole(&m, id).unwrap();
        let poly = &out.mesh.polys[5];
        assert_eq!(poly[0], lp[0]);
        assert_eq!(poly[1], *lp.last().unwrap());
        assert_closed(&out.mesh);
    }
}

#[test]
fn a_large_hole_becomes_one_ngon() {
    // a six-sided prism without its top cap
    let mut points = Vec::new();
    for z in [0.0, 2.0] {
        for i in 0..6 {
            let a = f64::from(i) / 6.0 * std::f64::consts::TAU;
            points.push(DVec3::new(a.cos(), a.sin(), z));
        }
    }
    let mut polys: Vec<Vec<u32>> = (0..6u32)
        .map(|i| vec![i, (i + 1) % 6, 6 + (i + 1) % 6, 6 + i])
        .collect();
    polys.push((0..6u32).rev().collect());
    let m = PolyMesh { points, polys };
    let id = edge_id(&m, 6, 7);
    let out = close_hole(&m, id).unwrap();
    assert_eq!(out.mesh.polys.len(), 8);
    assert_eq!(out.mesh.polys[7].len(), 6);
    assert_closed(&out.mesh);
    // a closed mesh and an interior edge have no hole
    assert!(close_hole(&out.mesh, 0).is_err());
    assert!(matches!(
        close_hole(&m, edge_id(&m, 0, 6)),
        Err(ToolError::Invalid(_))
    ));
}

// ------------------------------------------------------------------ stitch

/// The cube with polygon 4 disconnected: it has its own four points (8..12).
fn split_cube() -> PolyMesh {
    let c = cube();
    let mut sel = Selection::default();
    sel.polys.insert(4);
    let desc = tool_desc("disconnect").unwrap();
    let s = Settings::defaults(&desc);
    disconnect(&c, &sel, &s).unwrap().0
}

#[test]
fn stitching_the_seam_of_a_disconnected_polygon_restores_the_cube() {
    let m = split_cube();
    assert_eq!(m.points.len(), 12);
    // the disconnected copy of polygon 4 [1, 7, 5, 3] has its own points
    let copy = m.polys[4].clone();
    assert!(copy.iter().all(|p| *p >= 8));
    let pairs: Vec<(u32, u32)> = [1u32, 7, 5, 3]
        .iter()
        .zip(&copy)
        .map(|(a, b)| (*a, *b))
        .collect();
    let out = stitch(&m, &pairs, StitchTo::Middle).unwrap();
    assert_eq!((out.mesh.points.len(), out.mesh.polys.len()), (8, 6));
    assert_closed(&out.mesh);
    assert!(near(volume(&out.mesh), 8.0));
    assert_eq!(out.new_points.len(), 4);
}

#[test]
fn stitch_edges_pairs_the_facing_corners() {
    let m = split_cube();
    // every border edge of the copy faces the border edge of the cube with the same two points
    let copy = m.polys[4].clone();
    let orig = [1u32, 7, 5, 3];
    // the copy edge (copy[0], copy[1]) is the edge (1, 7); the open side of the cube has (7, 1)
    let a = edge_id(&m, copy[0], copy[1]);
    let b = edge_id(&m, orig[0], orig[1]);
    let pairs = stitch_edges(&m, &[(a, b)]).unwrap();
    // the copy edge runs copy[0] -> copy[1], the cube edge 7 -> 1 as its polygon [6,7,1,0] says
    assert!(
        pairs.contains(&(copy[0], orig[0])) && pairs.contains(&(copy[1], orig[1])),
        "{pairs:?}"
    );
    let out = stitch(&m, &pairs, StitchTo::First).unwrap();
    assert_eq!(out.mesh.points.len(), 10);
}

#[test]
fn stitch_middle_and_first_choose_the_position() {
    let m = PolyMesh {
        points: vec![
            DVec3::new(0.0, 0.0, 0.0),
            DVec3::new(1.0, 0.0, 0.0),
            DVec3::new(0.0, 1.0, 0.0),
            DVec3::new(1.1, 0.0, 0.0),
            DVec3::new(2.0, 0.0, 0.0),
            DVec3::new(2.0, 1.0, 0.0),
        ],
        polys: vec![vec![0, 1, 2], vec![3, 4, 5]],
    };
    let mid = stitch(&m, &[(1, 3)], StitchTo::Middle).unwrap();
    assert_eq!(mid.mesh.points.len(), 5);
    assert!(near(mid.mesh.points[1].x, 1.05));
    assert_eq!(mid.new_points, vec![1]);
    let first = stitch(&m, &[(3, 1)], StitchTo::First).unwrap();
    // the lowest index of the set (1) keeps its position, whatever the pair order
    assert!(near(first.mesh.points[1].x, 1.0));
    // the swap-remove: the last point (5) took the freed slot 3
    assert_eq!(first.mesh.polys[1], vec![1, 4, 3]);
}

#[test]
fn chains_of_pairs_merge_into_the_lowest_index() {
    let m = PolyMesh {
        points: vec![
            DVec3::ZERO,
            DVec3::X,
            DVec3::new(2.0, 0.0, 0.0),
            DVec3::Y,
            DVec3::new(0.0, 2.0, 0.0),
        ],
        polys: vec![vec![0, 1, 3], vec![2, 4, 3]],
    };
    let out = stitch(&m, &[(2, 1), (1, 0)], StitchTo::Middle).unwrap();
    // 0, 1, 2 become one point at their average
    assert_eq!(out.mesh.points.len(), 3);
    assert!(near(out.mesh.points[0].x, 1.0));
    assert_eq!(out.new_points, vec![0]);
}

#[test]
fn a_polygon_that_collapses_is_removed() {
    let m = PolyMesh {
        points: vec![DVec3::ZERO, DVec3::X, DVec3::Y, DVec3::new(0.0, 0.0, 1.0)],
        polys: vec![vec![0, 1, 2], vec![0, 1, 3]],
    };
    let out = stitch(&m, &[(2, 3)], StitchTo::Middle).unwrap();
    // the two triangles become the same triangle twice (kept); joining 0 and 1 collapses both
    assert_eq!(out.mesh.polys.len(), 2);
    let out = stitch(&m, &[(0, 1)], StitchTo::Middle).unwrap();
    assert!(out.mesh.polys.is_empty());
    assert_eq!(out.mesh.points.len(), 3);
}

#[test]
fn stitch_errors() {
    let m = split_cube();
    assert!(matches!(
        stitch(&m, &[], StitchTo::Middle),
        Err(ToolError::Invalid(_))
    ));
    assert!(matches!(
        stitch(&m, &[(3, 3)], StitchTo::Middle),
        Err(ToolError::Invalid(_))
    ));
    assert_eq!(
        stitch(&m, &[(0, 99)], StitchTo::Middle),
        Err(ToolError::BadPoint(99))
    );
    // interior edges, the same edge twice, a missing edge, and sewing an edge to its own points
    let interior = edge_id(&m, 0, 2);
    assert!(matches!(
        stitch_edges(&m, &[(interior, interior)]),
        Err(ToolError::Invalid(_))
    ));
    let copy = m.polys[4].clone();
    let border = edge_id(&m, copy[0], copy[1]);
    assert!(matches!(
        stitch_edges(&m, &[(border, interior)]),
        Err(ToolError::Invalid(_))
    ));
    assert_eq!(
        stitch_edges(&m, &[(border, 999)]),
        Err(ToolError::BadEdge(999))
    );
}

// ------------------------------------------------------------------ pen

#[test]
fn pen_adds_a_polygon_with_new_points_appended_in_order() {
    let c = cube();
    let out = pen(
        &c,
        &[
            PenCorner::New(DVec3::new(5.0, 0.0, 0.0)),
            PenCorner::New(DVec3::new(6.0, 0.0, 0.0)),
            PenCorner::New(DVec3::new(6.0, 1.0, 0.0)),
        ],
    )
    .unwrap();
    assert_eq!((out.mesh.points.len(), out.mesh.polys.len()), (11, 7));
    assert_eq!(out.mesh.polys[6], vec![8, 9, 10]);
    assert_eq!(out.new_polys, vec![6]);
    assert_eq!(out.new_points, vec![8, 9, 10]);
}

#[test]
fn pen_snaps_to_existing_points() {
    let c = cube();
    let out = pen(
        &c,
        &[
            PenCorner::Point(0),
            PenCorner::Point(1),
            PenCorner::New(DVec3::new(-3.0, 0.0, -1.0)),
        ],
    )
    .unwrap();
    assert_eq!(out.mesh.points.len(), 9);
    assert_eq!(out.mesh.polys[6], vec![0, 1, 8]);
    assert_eq!(out.new_points, vec![8]);
    // four existing corners make a quad over the open face of an open cube
    let m = open_cube();
    let fill = pen(
        &m,
        &[
            PenCorner::Point(1),
            PenCorner::Point(7),
            PenCorner::Point(5),
            PenCorner::Point(3),
        ],
    )
    .unwrap();
    assert!(fill.new_points.is_empty());
    assert_closed(&fill.mesh);
}

#[test]
fn pen_with_one_corner_adds_a_point() {
    let c = cube();
    let out = pen(&c, &[PenCorner::New(DVec3::splat(4.0))]).unwrap();
    assert_eq!((out.mesh.points.len(), out.mesh.polys.len()), (9, 6));
    assert_eq!(out.new_points, vec![8]);
    assert!(out.new_polys.is_empty());
}

#[test]
fn pen_errors() {
    let c = cube();
    let n = |x| PenCorner::New(DVec3::new(x, 0.0, 0.0));
    assert_eq!(pen(&c, &[]), Err(ToolError::EmptySelection));
    assert!(matches!(
        pen(&c, &[PenCorner::Point(0)]),
        Err(ToolError::Invalid(_))
    ));
    assert!(matches!(
        pen(&c, &[n(1.0), n(2.0)]),
        Err(ToolError::Invalid(_))
    ));
    assert_eq!(
        pen(
            &c,
            &[
                PenCorner::Point(0),
                PenCorner::Point(1),
                PenCorner::Point(99)
            ]
        ),
        Err(ToolError::BadPoint(99))
    );
    assert!(matches!(
        pen(
            &c,
            &[
                PenCorner::Point(0),
                PenCorner::Point(1),
                PenCorner::Point(0)
            ]
        ),
        Err(ToolError::Invalid(_))
    ));
    // collinear: no area
    assert!(matches!(
        pen(&c, &[n(1.0), n(2.0), n(3.0)]),
        Err(ToolError::Invalid(_))
    ));
    assert!(matches!(
        pen(&c, &[n(f64::NAN)]),
        Err(ToolError::Invalid(_))
    ));
}
