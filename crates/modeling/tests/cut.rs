//! Loop cut, Knife and Slide (docs/specs/MODELING.md §5). No golden data exists for these
//! tools, so these tests are the proof of the rules.

use std::collections::HashMap;

use forge_geometry::selection::{IndexSet, Selection};
use forge_geometry::{Edge, PolyMesh};
use forge_modeling::{
    KnifeLine, Plane, Settings, ToolError, View, even_offsets, knife_line, knife_plane, loop_cut,
    slide, tool_desc,
};
use glam::DVec3;
use serde_json::json;

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

/// A prism with `n` sides: points 0..n on the bottom ring, n..2n on the top; two n-gon caps.
fn prism(n: u32) -> PolyMesh {
    let mut points = Vec::new();
    for z in [0.0, 2.0] {
        for i in 0..n {
            let a = f64::from(i) / f64::from(n) * std::f64::consts::TAU;
            points.push(DVec3::new(a.cos(), a.sin(), z));
        }
    }
    let mut polys: Vec<Vec<u32>> = (0..n)
        .map(|i| vec![i, (i + 1) % n, n + (i + 1) % n, n + i])
        .collect();
    polys.push((0..n).rev().collect());
    polys.push((n..2 * n).collect());
    PolyMesh { points, polys }
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

// ------------------------------------------------------------------ loop cut

#[test]
fn loop_cut_around_a_cube() {
    let c = cube();
    let out = loop_cut(&c, edge_id(&c, 0, 2), &[0.5]).unwrap();
    assert_eq!((out.mesh.points.len(), out.mesh.polys.len()), (12, 10));
    assert_closed(&out.mesh);
    assert!(near(volume(&out.mesh), volume(&c)));
    // the four new points lie on the plane x = 0
    for p in &out.mesh.points[8..] {
        assert!(near(p.x, 0.0), "{p:?}");
    }
    assert_eq!(out.new_edges.len(), 4);
    assert_eq!(out.new_edge_ids().len(), 4);
    // every polygon is a quad
    assert!(out.mesh.polys.iter().all(|p| p.len() == 4));
}

#[test]
fn offsets_follow_the_start_edge_in_every_quad() {
    // 0.25 along the start edge (x from -1 to 1): all four ring points at x = -0.5
    let c = cube();
    let out = loop_cut(&c, edge_id(&c, 0, 2), &[0.25]).unwrap();
    for p in &out.mesh.points[8..] {
        assert!(near(p.x, -0.5), "{p:?}");
    }
    // starting from the parallel edge on the other side gives the same, from its own direction
    let out = loop_cut(&c, edge_id(&c, 3, 1), &[0.25]).unwrap();
    assert!(out.mesh.points[8..].iter().all(|p| near(p.x, -0.5)));
}

#[test]
fn several_cuts_and_even_spacing() {
    let c = cube();
    let out = loop_cut(&c, edge_id(&c, 0, 2), &[0.75, 0.25]).unwrap();
    assert_eq!((out.mesh.points.len(), out.mesh.polys.len()), (16, 14));
    assert_closed(&out.mesh);
    let mut xs: Vec<f64> = out.mesh.points[8..].iter().map(|p| p.x).collect();
    xs.sort_by(f64::total_cmp);
    assert!(near(xs[0], -0.5) && near(xs[7], 0.5));
    assert_eq!(out.new_edges.len(), 8);
    assert_eq!(even_offsets(3), vec![0.25, 0.5, 0.75]);
    let three = loop_cut(&c, edge_id(&c, 0, 2), &even_offsets(3)).unwrap();
    assert_eq!(three.mesh.polys.len(), 6 - 4 + 16);
}

#[test]
fn ring_stops_at_the_border_of_an_open_mesh() {
    let g = grid(3);
    // the edge (5, 6) of the middle row: the ring runs up and down the column, 3 quads
    let out = loop_cut(&g, edge_id(&g, 5, 6), &[0.5]).unwrap();
    assert_eq!((out.mesh.points.len(), out.mesh.polys.len()), (20, 12));
    out.mesh.validate().unwrap();
    // the four new points sit at x = 1.5 on the rows 0..3
    let mut ys: Vec<f64> = out.mesh.points[16..].iter().map(|p| p.y).collect();
    ys.sort_by(f64::total_cmp);
    assert_eq!(ys, vec![0.0, 1.0, 2.0, 3.0]);
    assert!(out.mesh.points[16..].iter().all(|p| near(p.x, 1.5)));
}

#[test]
fn a_neighbour_that_is_not_a_quad_gets_the_point() {
    // quad 0 1 2 3 and a triangle 3 2 4 on its top edge; cut across the bottom edge 0 1
    let m = PolyMesh {
        points: vec![
            DVec3::new(0.0, 0.0, 0.0),
            DVec3::new(1.0, 0.0, 0.0),
            DVec3::new(1.0, 1.0, 0.0),
            DVec3::new(0.0, 1.0, 0.0),
            DVec3::new(0.5, 2.0, 0.0),
        ],
        polys: vec![vec![0, 1, 2, 3], vec![3, 2, 4]],
    };
    let out = loop_cut(&m, edge_id(&m, 0, 1), &[0.5]).unwrap();
    assert_eq!(out.mesh.points.len(), 7);
    assert_eq!(out.mesh.polys.len(), 3);
    // the triangle became a quad holding the new point of the top edge
    assert_eq!(out.mesh.polys[1].len(), 4);
    let top = out.mesh.points[out.mesh.polys[1][1] as usize];
    assert!(near(top.x, 0.5) && near(top.y, 1.0), "{top:?}");
    out.mesh.validate().unwrap();
}

#[test]
fn cylinder_barrel_and_rim() {
    let p = prism(8);
    assert_closed(&p);
    // a vertical edge: the ring is the 8 side quads, closed
    let around = loop_cut(&p, edge_id(&p, 0, 8), &[0.5]).unwrap();
    assert_eq!(
        (around.mesh.points.len(), around.mesh.polys.len()),
        (24, 18)
    );
    assert_closed(&around.mesh);
    assert!(around.mesh.points[16..].iter().all(|q| near(q.z, 1.0)));
    assert_eq!(around.new_edges.len(), 8);
    // a rim edge: one side quad is cut, the n-gon caps get a point each
    let rim = loop_cut(&p, edge_id(&p, 0, 1), &[0.5]).unwrap();
    assert_eq!(rim.mesh.polys.len(), 11);
    assert_eq!(rim.mesh.polys[8].len(), 9);
    assert_eq!(rim.mesh.polys[9].len(), 9);
    assert_closed(&rim.mesh);
    assert!(near(volume(&rim.mesh), volume(&p)));
}

#[test]
fn loop_cut_errors() {
    let c = cube();
    let e = edge_id(&c, 0, 2);
    for bad in [vec![], vec![0.0], vec![1.0], vec![f64::NAN], vec![0.5, 0.5]] {
        assert!(
            matches!(loop_cut(&c, e, &bad), Err(ToolError::Invalid(_))),
            "{bad:?}"
        );
    }
    assert_eq!(loop_cut(&c, 99, &[0.5]), Err(ToolError::BadEdge(99)));
    assert!(loop_cut(&c, e, &vec![0.5; 65]).is_err());
    // an edge that borders only a triangle has no ring
    let tri = PolyMesh {
        points: vec![DVec3::ZERO, DVec3::X, DVec3::Y],
        polys: vec![vec![0, 1, 2]],
    };
    assert!(matches!(
        loop_cut(&tri, 0, &[0.5]),
        Err(ToolError::Invalid(_))
    ));
}

// ------------------------------------------------------------------ knife

fn plane_x(x: f64) -> Plane {
    Plane {
        point: DVec3::new(x, 0.0, 0.0),
        normal: DVec3::X,
    }
}

#[test]
fn knife_plane_through_a_cube() {
    let c = cube();
    let out = knife_plane(&c, &IndexSet::new(), plane_x(0.0)).unwrap();
    assert_eq!((out.mesh.points.len(), out.mesh.polys.len()), (12, 10));
    assert_closed(&out.mesh);
    assert!(near(volume(&out.mesh), volume(&c)));
    assert!(out.mesh.points[8..].iter().all(|p| near(p.x, 0.0)));
    assert_eq!(out.new_edge_ids().len(), 4);
    // the same cut as the loop cut at the middle
    let lc = loop_cut(&c, edge_id(&c, 0, 2), &[0.5]).unwrap();
    assert_eq!(lc.mesh.polys.len(), out.mesh.polys.len());
}

#[test]
fn knife_plane_through_corners_adds_no_points() {
    let c = cube();
    let plane = Plane {
        point: DVec3::ZERO,
        normal: DVec3::new(1.0, -1.0, 0.0),
    };
    let out = knife_plane(&c, &IndexSet::new(), plane).unwrap();
    // the front and back faces are cut along their diagonal; faces holding an edge on the plane
    // are not
    assert_eq!((out.mesh.points.len(), out.mesh.polys.len()), (8, 8));
    assert_closed(&out.mesh);
    assert_eq!(out.mesh.polys.iter().filter(|p| p.len() == 3).count(), 4);
}

#[test]
fn knife_misses() {
    let c = cube();
    assert!(matches!(
        knife_plane(&c, &IndexSet::new(), plane_x(5.0)),
        Err(ToolError::Invalid(_))
    ));
    let zero = Plane {
        point: DVec3::ZERO,
        normal: DVec3::ZERO,
    };
    assert!(knife_plane(&c, &IndexSet::new(), zero).is_err());
    let bad: IndexSet = [99].into_iter().collect();
    assert_eq!(
        knife_plane(&c, &bad, plane_x(0.0)),
        Err(ToolError::BadPolygon(99))
    );
}

#[test]
fn knife_concave_polygon_gets_two_chords() {
    // a U opening up; the plane y = 1.5 crosses it four times
    let pts = [
        (0.0, 0.0),
        (3.0, 0.0),
        (3.0, 2.0),
        (2.0, 2.0),
        (2.0, 1.0),
        (1.0, 1.0),
        (1.0, 2.0),
        (0.0, 2.0),
    ];
    let m = PolyMesh {
        points: pts.iter().map(|(x, y)| DVec3::new(*x, *y, 0.0)).collect(),
        polys: vec![(0..8).collect()],
    };
    let plane = Plane {
        point: DVec3::new(0.0, 1.5, 0.0),
        normal: DVec3::Y,
    };
    let out = knife_plane(&m, &IndexSet::new(), plane).unwrap();
    assert_eq!((out.mesh.points.len(), out.mesh.polys.len()), (12, 3));
    out.mesh.validate().unwrap();
    assert_eq!(out.new_edges.len(), 2);
}

#[test]
fn knife_restricted_to_polygons_leaves_no_gap() {
    let c = cube();
    let top: IndexSet = [4].into_iter().collect();
    let out = knife_plane(&c, &top, plane_x(0.0)).unwrap();
    assert_eq!((out.mesh.points.len(), out.mesh.polys.len()), (10, 7));
    assert_closed(&out.mesh);
    // the neighbours across the two cut edges gained a corner
    let five = out.mesh.polys.iter().filter(|p| p.len() == 5).count();
    assert_eq!(five, 2);
}

#[test]
fn knife_line_in_a_parallel_view() {
    let c = cube();
    assert!(volume(&c) > 0.0, "outward winding");
    let mk = |a: f64, b: f64, visible_only| KnifeLine {
        from: DVec3::new(0.0, a, 0.0),
        to: DVec3::new(0.0, b, 0.0),
        view: View::Direction(DVec3::Z),
        visible_only,
    };
    let all = IndexSet::new();
    // seen along +z: the front face (z = -1) is visible, the back one and the edge-on faces not
    let out = knife_line(&c, &all, &mk(-5.0, 5.0, true)).unwrap();
    assert_eq!((out.mesh.points.len(), out.mesh.polys.len()), (10, 7));
    out.mesh.validate().unwrap();
    // without the visibility test the plane x = 0 cuts all four faces
    let out = knife_line(&c, &all, &mk(-5.0, 5.0, false)).unwrap();
    assert_eq!((out.mesh.points.len(), out.mesh.polys.len()), (12, 10));
    assert_closed(&out.mesh);
    // a segment that ends inside the faces makes no chord there: only the bottom face (y = -1),
    // whose chord projects to s = 0.8, is cut
    let half = knife_line(&c, &all, &mk(-5.0, 0.0, false)).unwrap();
    assert_eq!((half.mesh.points.len(), half.mesh.polys.len()), (10, 7));
    assert!(matches!(
        knife_line(&c, &all, &mk(-5.0, -3.0, false)),
        Err(ToolError::Invalid(_))
    ));
    // degenerate segment
    assert!(knife_line(&c, &all, &mk(1.0, 1.0, false)).is_err());
}

#[test]
fn knife_line_from_the_eye_matches_the_plane() {
    let c = cube();
    let line = KnifeLine {
        from: DVec3::new(0.0, -5.0, 0.0),
        to: DVec3::new(0.0, 5.0, 0.0),
        view: View::Eye(DVec3::new(0.0, 0.0, -10.0)),
        visible_only: false,
    };
    let out = knife_line(&c, &IndexSet::new(), &line).unwrap();
    assert_eq!((out.mesh.points.len(), out.mesh.polys.len()), (12, 10));
    // seen from (0, 0, -10) the segment's extent is exact: a chord of the front face at x = 0
    // reaches y = +-1, which projects to s = 0.611 / 0.389
    let short = KnifeLine {
        to: DVec3::new(0.0, 0.0, 0.0),
        ..line
    };
    let part = knife_line(&c, &IndexSet::new(), &short).unwrap();
    assert!(part.mesh.polys.len() < out.mesh.polys.len());
    // an eye on the segment's line makes the plane degenerate
    let flat = KnifeLine {
        view: View::Eye(DVec3::new(0.0, 7.0, 0.0)),
        ..line
    };
    assert!(knife_line(&c, &IndexSet::new(), &flat).is_err());
}

// ------------------------------------------------------------------ slide

fn slide_settings(amount: f64, toward: i64) -> Settings {
    let desc = tool_desc("slide").unwrap();
    Settings::from_json(
        &desc,
        json!({"amount": amount, "toward": toward})
            .as_object()
            .unwrap(),
    )
    .unwrap()
}

fn edges_sel(m: &PolyMesh, pairs: &[(u32, u32)]) -> Selection {
    let mut s = Selection::default();
    for (a, b) in pairs {
        s.edges.insert(edge_id(m, *a, *b));
    }
    s
}

#[test]
fn slide_a_chain_of_edges() {
    let g = grid(3);
    let sel = edges_sel(&g, &[(5, 6), (6, 7)]);
    let (m, kept) = slide(&g, &sel, &slide_settings(0.5, -1)).unwrap();
    assert_eq!(kept, sel);
    assert_eq!(m.polys, g.polys);
    for i in [5usize, 6, 7] {
        assert!(near(m.points[i].y, 1.5), "{i}: {:?}", m.points[i]);
        assert!(near(m.points[i].x, g.points[i].x));
    }
    assert_eq!(m.points[4], g.points[4]);
    assert_eq!(m.points[8], g.points[8]);
    // the other direction moves down to y = 0.5, a quarter of the rail: 0.75
    let (m, _) = slide(&g, &sel, &slide_settings(-0.25, -1)).unwrap();
    assert!(near(m.points[6].y, 0.75));
    // selection order does not matter
    let rev = edges_sel(&g, &[(6, 7), (5, 6)]);
    assert_eq!(
        slide(&g, &rev, &slide_settings(0.5, -1)).unwrap().0,
        slide(&g, &sel, &slide_settings(0.5, -1)).unwrap().0
    );
}

#[test]
fn slide_a_cube_loop_keeps_the_volume_closed() {
    // the four vertical edges of the 8-prism barrel cannot slide up (rails are the rim); use
    // the loop of horizontal edges of a cut cube instead
    let c = cube();
    let cut = loop_cut(&c, edge_id(&c, 0, 2), &[0.5]).unwrap().mesh;
    let ids: Vec<u32> = (0..cut.edges().len() as u32).collect();
    let loop_edges: Vec<(u32, u32)> = {
        let e = cut.edges();
        ids.iter()
            .map(|i| e[*i as usize])
            .filter(|e| e.0 >= 8 && e.1 >= 8)
            .map(|e| (e.0, e.1))
            .collect()
    };
    assert_eq!(loop_edges.len(), 4);
    let sel = edges_sel(&cut, &loop_edges);
    let (m, _) = slide(&cut, &sel, &slide_settings(0.5, -1)).unwrap();
    assert_closed(&m);
    // all four points moved along x by a quarter of the cube's width (toward x = -1 or 1,
    // consistently for the whole loop)
    let xs: Vec<f64> = m.points[8..].iter().map(|p| p.x).collect();
    assert!(xs.iter().all(|x| near(*x, xs[0])), "{xs:?}");
    assert!(near(xs[0].abs(), 0.5), "{xs:?}");
}

#[test]
fn slide_needs_a_side() {
    let g = grid(2);
    // the border edge (1, 2) runs left to right with its polygon on the left
    let sel = edges_sel(&g, &[(1, 2)]);
    assert!(slide(&g, &sel, &slide_settings(0.5, -1)).is_ok());
    assert!(matches!(
        slide(&g, &sel, &slide_settings(-0.5, -1)),
        Err(ToolError::Invalid(_))
    ));
}

#[test]
fn slide_points_toward_a_neighbour() {
    let g = grid(3);
    let mut sel = Selection::default();
    sel.points.insert(5);
    let (m, _) = slide(&g, &sel, &slide_settings(0.25, 6)).unwrap();
    assert!(near(m.points[5].x, 1.25));
    // 7 is not joined to 5 by an edge
    assert!(slide(&g, &sel, &slide_settings(0.25, 7)).is_err());
    assert!(slide(&g, &sel, &slide_settings(0.25, -1)).is_err());
    assert!(slide(&g, &sel, &slide_settings(-0.25, 6)).is_err());
    assert_eq!(
        slide(&g, &sel, &slide_settings(0.25, 999)),
        Err(ToolError::BadPoint(999))
    );
}

#[test]
fn slide_errors() {
    let g = grid(2);
    assert_eq!(
        slide(&g, &Selection::default(), &slide_settings(0.5, -1)),
        Err(ToolError::EmptySelection)
    );
    let sel = edges_sel(&g, &[(1, 2)]);
    assert!(slide(&g, &sel, &slide_settings(0.0, -1)).is_err());
    let mut bad = Selection::default();
    bad.edges.insert(99);
    assert_eq!(
        slide(&g, &bad, &slide_settings(0.5, -1)),
        Err(ToolError::BadEdge(99))
    );
}
