//! Polygon mesh and edges (docs/specs/GEOMETRY.md §1).

use std::collections::BTreeMap;

use glam::DVec3;

/// N-gon mesh; polygons index into `points`, wound counter-clockwise.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PolyMesh {
    pub points: Vec<DVec3>,
    pub polys: Vec<Vec<u32>>,
}

/// Unordered vertex pair, stored with `.0 < .1`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Edge(pub u32, pub u32);

impl Edge {
    pub fn new(a: u32, b: u32) -> Edge {
        if a < b { Edge(a, b) } else { Edge(b, a) }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum MeshError {
    #[error("polygon {poly} references point {index}, which does not exist")]
    IndexOutOfRange { poly: usize, index: u32 },
    #[error("polygon {poly} has fewer than 3 vertices")]
    TooFewVertices { poly: usize },
    #[error("polygon {poly} uses vertex {vertex} more than once")]
    RepeatedVertex { poly: usize, vertex: u32 },
}

/// Edges of one polygon, corner `i` to `i + 1` (wrapping).
pub fn poly_edges(poly: &[u32]) -> impl Iterator<Item = Edge> + '_ {
    (0..poly.len()).map(move |i| Edge::new(poly[i], poly[(i + 1) % poly.len()]))
}

impl PolyMesh {
    pub fn validate(&self) -> Result<(), MeshError> {
        for (poly, verts) in self.polys.iter().enumerate() {
            if verts.len() < 3 {
                return Err(MeshError::TooFewVertices { poly });
            }
            for (i, &index) in verts.iter().enumerate() {
                if index as usize >= self.points.len() {
                    return Err(MeshError::IndexOutOfRange { poly, index });
                }
                if verts[..i].contains(&index) {
                    return Err(MeshError::RepeatedVertex {
                        poly,
                        vertex: index,
                    });
                }
            }
        }
        Ok(())
    }

    /// Unique edges in first-appearance order; edge ids are indices into this list.
    pub fn edges(&self) -> Vec<Edge> {
        let mut seen = BTreeMap::new();
        let mut out = Vec::new();
        for poly in &self.polys {
            for e in poly_edges(poly) {
                seen.entry(e).or_insert_with(|| {
                    out.push(e);
                    out.len() - 1
                });
            }
        }
        out
    }

    /// Edge → edge id, for the ids of [`PolyMesh::edges`].
    pub fn edge_ids(&self) -> BTreeMap<Edge, u32> {
        self.edges()
            .into_iter()
            .enumerate()
            .map(|(i, e)| (e, i as u32))
            .collect()
    }
}
