//! docs/specs/GEOMETRY.md §1–2.

mod common;

use common::{cube, grid, mesh, tetra};
use forge_geometry::{Edge, HalfEdgeMesh, MeshError, TopologyError};

#[test]
fn validate_reports_each_problem() {
    let pts = &[[0.0; 3]; 4];
    assert_eq!(mesh(pts, &[&[0, 1, 2]]).validate(), Ok(()));
    assert_eq!(
        mesh(pts, &[&[0, 1, 2], &[0, 1, 9]]).validate(),
        Err(MeshError::IndexOutOfRange { poly: 1, index: 9 })
    );
    assert_eq!(
        mesh(pts, &[&[0, 1]]).validate(),
        Err(MeshError::TooFewVertices { poly: 0 })
    );
    assert_eq!(
        mesh(pts, &[&[0, 1, 2, 1]]).validate(),
        Err(MeshError::RepeatedVertex { poly: 0, vertex: 1 })
    );
}

#[test]
fn edges_are_unique_in_first_appearance_order() {
    let e = cube().edges();
    assert_eq!(e.len(), 12);
    assert_eq!(
        e[..6],
        [
            Edge(0, 3),
            Edge(2, 3),
            Edge(1, 2),
            Edge(0, 1),
            Edge(4, 5),
            Edge(5, 6)
        ]
    );
    assert_eq!(Edge::new(7, 2), Edge(2, 7));
    assert_eq!(grid().edges().len(), 12);
}

#[test]
fn closed_meshes() {
    for (m, e) in [(cube(), 12), (tetra(), 6)] {
        let he = HalfEdgeMesh::build(&m).unwrap();
        assert!(he.is_closed());
        assert!(he.boundary_loops().is_empty());
        assert_eq!(he.euler_characteristic(), 2);
        assert_eq!(he.half_edge_count(), 2 * e);
    }
}

#[test]
fn half_edge_links_are_consistent() {
    let m = cube();
    let he = HalfEdgeMesh::build(&m).unwrap();
    for h in 0..he.half_edge_count() {
        let t = he.twin(h).expect("closed mesh");
        assert_eq!(he.twin(t), Some(h));
        assert_eq!(he.origin(t), he.dest(h));
        assert_eq!(he.prev(he.next(h)), h);
        assert_eq!(he.face(he.next(h)), he.face(h));
    }
    // half-edge ids follow corner order
    assert_eq!(he.face_half_edges(1), vec![4, 5, 6, 7]);
    assert_eq!(he.origin(5), 5);
    assert_eq!(he.dest(5), 6);
}

#[test]
fn adjacency_queries_are_sorted() {
    let he = HalfEdgeMesh::build(&cube()).unwrap();
    assert_eq!(he.vertex_neighbors(0), vec![1, 3, 4]);
    assert_eq!(he.vertex_faces(0), vec![0, 2, 4]);
    assert_eq!(he.face_neighbors(1), vec![2, 3, 4, 5]);
}

#[test]
fn open_grid_has_one_ordered_boundary_loop() {
    let he = HalfEdgeMesh::build(&grid()).unwrap();
    assert!(!he.is_closed());
    assert_eq!(he.euler_characteristic(), 1);
    assert_eq!(he.boundary_loops(), vec![vec![0, 1, 2, 5, 8, 7, 6, 3]]);
    assert_eq!(he.vertex_neighbors(0), vec![1, 3]);
    assert_eq!(he.vertex_neighbors(4), vec![1, 3, 5, 7]);
    assert_eq!(he.face_neighbors(0), vec![1, 2]);
    assert!(he.is_boundary_edge(0)); // 0 -> 1
}

#[test]
fn bowtie_vertex_is_allowed() {
    let pts = &[[0.0; 3]; 5];
    let he = HalfEdgeMesh::build(&mesh(pts, &[&[0, 1, 2], &[0, 3, 4]])).unwrap();
    assert_eq!(he.vertex_neighbors(0), vec![1, 2, 3, 4]);
    assert_eq!(he.vertex_faces(0), vec![0, 1]);
    assert_eq!(he.boundary_loops(), vec![vec![0, 1, 2], vec![0, 3, 4]]);
}

#[test]
fn unused_points_do_not_count_in_euler() {
    let mut m = tetra();
    m.points.push(glam::DVec3::ZERO);
    let he = HalfEdgeMesh::build(&m).unwrap();
    assert_eq!(he.euler_characteristic(), 2);
    assert!(he.vertex_neighbors(4).is_empty());
}

#[test]
fn bad_topology_is_rejected() {
    let pts = &[[0.0; 3]; 5];
    assert_eq!(
        HalfEdgeMesh::build(&mesh(pts, &[&[0, 1, 2], &[1, 0, 3], &[1, 0, 4]])).unwrap_err(),
        TopologyError::NonManifoldEdge(Edge(0, 1))
    );
    assert_eq!(
        HalfEdgeMesh::build(&mesh(pts, &[&[0, 1, 2], &[0, 1, 3]])).unwrap_err(),
        TopologyError::InconsistentOrientation(Edge(0, 1))
    );
    assert_eq!(
        HalfEdgeMesh::build(&mesh(pts, &[&[0, 1]])).unwrap_err(),
        TopologyError::Mesh(MeshError::TooFewVertices { poly: 0 })
    );
}
