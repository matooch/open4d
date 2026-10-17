//! forge-geometry: polygon meshes, half-edge connectivity, normals, selections and
//! splines. Spec: docs/specs/GEOMETRY.md.

pub mod halfedge;
pub mod mesh;
pub mod normals;
pub mod selection;
pub mod spline;
pub mod triangulate;

pub use halfedge::{HalfEdgeMesh, HeId, TopologyError};
pub use mesh::{Edge, MeshError, PolyMesh};
