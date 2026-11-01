//! docs/specs/EDITABLE.md §2.8: convert, connected, edge grow/shrink, loop and ring.
//! Grid: 3x3 quads, point (x, y) = y * 4 + x, poly (col, row) = row * 3 + col.

mod common;

use common::{cube, mesh};
use forge_geometry::selection::{IndexSet, Mode};
use forge_geometry::selection_ops::*;
use forge_geometry::{Edge, PolyMesh};

fn set(v: &[u32]) -> IndexSet {
    v.iter().copied().collect()
}

fn ids(s: &IndexSet) -> Vec<u32> {
    s.iter().collect()
}

fn grid3() -> PolyMesh {
    let mut points = Vec::new();
    for y in 0..4 {
        for x in 0..4 {
            points.push([f64::from(x), f64::from(y), 0.0]);
        }
    }
    let mut polys: Vec<Vec<u32>> = Vec::new();
    for row in 0..3u32 {
        for col in 0..3u32 {
            let p = row * 4 + col;
            polys.push(vec![p, p + 1, p + 5, p + 4]);
        }
    }
    let refs: Vec<&[u32]> = polys.iter().map(Vec::as_slice).collect();
    mesh(&points, &refs)
}

/// Edge ids of the point pairs.
fn edges(m: &PolyMesh, pairs: &[(u32, u32)]) -> IndexSet {
    let ids = m.edge_ids();
    pairs.iter().map(|p| ids[&Edge::new(p.0, p.1)]).collect()
}

fn pairs(m: &PolyMesh, s: &IndexSet) -> Vec<(u32, u32)> {
    let all = m.edges();
    let mut v: Vec<_> = s
        .iter()
        .map(|i| (all[i as usize].0, all[i as usize].1))
        .collect();
    v.sort_unstable();
    v
}

#[test]
fn tolerant_convert_takes_any_part() {
    let m = grid3();
    // Point 5 is a corner of polys 0, 1, 3, 4.
    let p = set(&[5]);
    assert!(convert(&m, &p, Kind::Points, Kind::Polys, Mode::All).is_empty());
    assert_eq!(
        ids(&convert(&m, &p, Kind::Points, Kind::Polys, Mode::Any)),
        [0, 1, 3, 4]
    );
    let e = edges(&m, &[(5, 6)]);
    assert!(convert(&m, &e, Kind::Edges, Kind::Polys, Mode::All).is_empty());
    assert_eq!(
        ids(&convert(&m, &e, Kind::Edges, Kind::Polys, Mode::Any)),
        [1, 4]
    );
    // A polygon with all four edges selected is chosen strictly.
    let all4 = edges(&m, &[(0, 1), (1, 5), (4, 5), (0, 4)]);
    assert_eq!(
        ids(&convert(&m, &all4, Kind::Edges, Kind::Polys, Mode::All)),
        [0]
    );
    // Same kind is a no-op; polygons to points ignore the mode.
    assert_eq!(convert(&m, &p, Kind::Points, Kind::Points, Mode::All), p);
    assert_eq!(
        ids(&convert(
            &m,
            &set(&[0]),
            Kind::Polys,
            Kind::Points,
            Mode::All
        )),
        [0, 1, 4, 5]
    );
}

#[test]
fn connected_spreads_over_shared_points() {
    // Two quads sharing only the corner point 2, and a third quad apart.
    let m = mesh(
        &[
            [0., 0., 0.],
            [1., 0., 0.],
            [1., 1., 0.],
            [0., 1., 0.],
            [2., 1., 0.],
            [2., 2., 0.],
            [1., 2., 0.],
            [5., 0., 0.],
            [6., 0., 0.],
            [6., 1., 0.],
            [5., 1., 0.],
        ],
        &[&[0, 1, 2, 3], &[2, 4, 5, 6], &[7, 8, 9, 10]],
    );
    assert_eq!(ids(&connected(&m, Kind::Polys, &set(&[0]))), [0, 1]);
    assert_eq!(ids(&connected(&m, Kind::Polys, &set(&[2]))), [2]);
    assert_eq!(ids(&connected(&m, Kind::Polys, &set(&[0, 2]))), [0, 1, 2]);
    assert_eq!(ids(&connected(&m, Kind::Points, &set(&[8]))), [7, 8, 9, 10]);
    let e = edges(&m, &[(2, 4)]);
    let got = connected(&m, Kind::Edges, &e);
    assert_eq!(got.len(), 8);
    assert!(connected(&m, Kind::Polys, &IndexSet::new()).is_empty());
    assert!(connected(&m, Kind::Polys, &set(&[99])).is_empty());
}

#[test]
fn edge_grow_and_shrink() {
    let m = grid3();
    let e = edges(&m, &[(5, 6)]);
    // Neighbours share point 5 or 6: 3 more at each end.
    let grown = grow_edges(&m, &e);
    assert_eq!(grown.len(), 7);
    assert_eq!(shrink_edges(&m, &grown), e);
    assert!(shrink_edges(&m, &e).is_empty());
}

#[test]
fn edge_loop_on_a_grid() {
    let m = grid3();
    // An interior edge runs to the border points and stops there.
    let l = edge_loop(&m, &edges(&m, &[(5, 6)]));
    assert_eq!(pairs(&m, &l), [(4, 5), (5, 6), (6, 7)]);
    // A border edge follows the border but not around the corner.
    let b = edge_loop(&m, &edges(&m, &[(1, 2)]));
    assert_eq!(pairs(&m, &b), [(0, 1), (1, 2), (2, 3)]);
    // Selecting a loop edge again gives the same loop.
    assert_eq!(edge_loop(&m, &l), l);
    assert!(edge_loop(&m, &set(&[999])).is_empty());
}

#[test]
fn edge_loop_stops_at_poles_and_closes_on_a_cube() {
    // The plain cube has valence-3 corners: a loop is the edge itself.
    let c = cube();
    let one = set(&[0]);
    assert_eq!(edge_loop(&c, &one), one);

    // A triangle fan point (valence 5, triangles) stops loops.
    let fan = mesh(
        &[[0., 0., 0.], [1., 0., 0.], [0., 1., 0.], [-1., 0., 0.]],
        &[&[0, 1, 2], &[0, 2, 3]],
    );
    let e = edges(&fan, &[(0, 2)]);
    assert_eq!(edge_loop(&fan, &e), e);

    // On the 2x2x2 cube the loop starting on the equator goes all the way round and closes.
    // Top border edges end in valence-3 corners and stay short.
    let src =
        std::fs::read_to_string("../../tests/golden/selection-ops/convert-edge-to-poly/mesh.obj")
            .unwrap();
    let g = forge_parity_harness::parse_obj(&src).unwrap();
    let m = mesh(
        &g.positions,
        &g.faces.iter().map(Vec::as_slice).collect::<Vec<_>>(),
    );
    assert_eq!(
        pairs(&m, &edge_loop(&m, &edges(&m, &[(2, 5)]))),
        [(2, 5), (5, 8)]
    );
    let on_equator =
        |e: &Edge| g.positions[e.0 as usize][1] == 0.0 && g.positions[e.1 as usize][1] == 0.0;
    let start = m.edges().iter().position(on_equator).unwrap() as u32;
    let l = edge_loop(&m, &set(&[start]));
    assert_eq!(l.len(), 8);
    let all = m.edges();
    assert!(l.iter().all(|i| on_equator(&all[i as usize])));
}

#[test]
fn edge_ring_and_poly_strip_on_a_grid() {
    let m = grid3();
    let r = edge_ring(&m, &edges(&m, &[(5, 6)]));
    assert_eq!(pairs(&m, &r), [(1, 2), (5, 6), (9, 10), (13, 14)]);
    // A ring edge gives the same ring.
    assert_eq!(edge_ring(&m, &r).len(), 4);
    // Vertical strip (across the bottom and top edges) and horizontal strip.
    assert_eq!(ids(&poly_strip(&m, &set(&[4]), 0)), [1, 4, 7]);
    assert_eq!(ids(&poly_strip(&m, &set(&[4]), 1)), [3, 4, 5]);
    assert_eq!(ids(&poly_strip(&m, &set(&[0]), 1)), [0, 1, 2]);
    // Triangles contribute only themselves.
    let tri = mesh(&[[0., 0., 0.], [1., 0., 0.], [0., 1., 0.]], &[&[0, 1, 2]]);
    assert_eq!(ids(&poly_strip(&tri, &set(&[0]), 0)), [0]);
}

#[test]
fn rings_and_strips_close_around_a_cube() {
    let c = cube();
    // A strip around the cube: 4 quads, closed.
    let s = poly_strip(&c, &set(&[2]), 0);
    assert_eq!(ids(&s), [0, 1, 2, 3]);
    // The ring of a cube edge is the 4 parallel edges.
    assert_eq!(edge_ring(&c, &set(&[0])).len(), 4);
}
