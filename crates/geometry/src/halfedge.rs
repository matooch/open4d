//! Half-edge connectivity (docs/specs/GEOMETRY.md §2).
//! Refs: Mäntylä 1988; Kettner, CGTA 13 (1999).

use std::collections::BTreeMap;

use crate::mesh::{Edge, MeshError, PolyMesh};

/// Half-edge id: the running corner index over all polygons.
pub type HeId = usize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum TopologyError {
    #[error(transparent)]
    Mesh(#[from] MeshError),
    #[error("edge {0:?} is used by more than two polygons")]
    NonManifoldEdge(Edge),
    #[error("edge {0:?} is traversed in the same direction by two polygons")]
    InconsistentOrientation(Edge),
}

#[derive(Debug, Clone)]
pub struct HalfEdgeMesh {
    origin: Vec<u32>,
    next: Vec<HeId>,
    prev: Vec<HeId>,
    face: Vec<u32>,
    twin: Vec<Option<HeId>>,
    /// `face_start[f]..face_start[f + 1]` are face `f`'s half-edges.
    face_start: Vec<HeId>,
    vert_out: Vec<Vec<HeId>>,
    vert_in: Vec<Vec<HeId>>,
    edge_count: usize,
}

impl HalfEdgeMesh {
    pub fn build(mesh: &PolyMesh) -> Result<HalfEdgeMesh, TopologyError> {
        mesh.validate()?;
        let corners: usize = mesh.polys.iter().map(Vec::len).sum();
        let mut he = HalfEdgeMesh {
            origin: Vec::with_capacity(corners),
            next: Vec::with_capacity(corners),
            prev: Vec::with_capacity(corners),
            face: Vec::with_capacity(corners),
            twin: vec![None; corners],
            face_start: Vec::with_capacity(mesh.polys.len() + 1),
            vert_out: vec![Vec::new(); mesh.points.len()],
            vert_in: vec![Vec::new(); mesh.points.len()],
            edge_count: 0,
        };
        let mut by_edge: BTreeMap<Edge, Vec<HeId>> = BTreeMap::new();
        for (f, poly) in mesh.polys.iter().enumerate() {
            let start = he.origin.len();
            he.face_start.push(start);
            let k = poly.len();
            for i in 0..k {
                let id = start + i;
                let (a, b) = (poly[i], poly[(i + 1) % k]);
                he.origin.push(a);
                he.next.push(start + (i + 1) % k);
                he.prev.push(start + (i + k - 1) % k);
                he.face.push(f as u32);
                he.vert_out[a as usize].push(id);
                he.vert_in[b as usize].push(id);
                by_edge.entry(Edge::new(a, b)).or_default().push(id);
            }
        }
        he.face_start.push(he.origin.len());
        for (edge, hes) in &by_edge {
            match hes.as_slice() {
                [_] => {}
                [a, b] => {
                    if he.origin[*a] == he.origin[*b] {
                        return Err(TopologyError::InconsistentOrientation(*edge));
                    }
                    he.twin[*a] = Some(*b);
                    he.twin[*b] = Some(*a);
                }
                _ => return Err(TopologyError::NonManifoldEdge(*edge)),
            }
        }
        he.edge_count = by_edge.len();
        Ok(he)
    }

    pub fn half_edge_count(&self) -> usize {
        self.origin.len()
    }

    pub fn edge_count(&self) -> usize {
        self.edge_count
    }

    pub fn face_count(&self) -> usize {
        self.face_start.len() - 1
    }

    pub fn origin(&self, h: HeId) -> u32 {
        self.origin[h]
    }

    pub fn dest(&self, h: HeId) -> u32 {
        self.origin[self.next[h]]
    }

    pub fn next(&self, h: HeId) -> HeId {
        self.next[h]
    }

    pub fn prev(&self, h: HeId) -> HeId {
        self.prev[h]
    }

    pub fn face(&self, h: HeId) -> u32 {
        self.face[h]
    }

    pub fn twin(&self, h: HeId) -> Option<HeId> {
        self.twin[h]
    }

    pub fn is_boundary_edge(&self, h: HeId) -> bool {
        self.twin[h].is_none()
    }

    pub fn is_closed(&self) -> bool {
        self.twin.iter().all(Option::is_some)
    }

    pub fn face_half_edges(&self, f: usize) -> Vec<HeId> {
        (self.face_start[f]..self.face_start[f + 1]).collect()
    }

    pub fn vertex_neighbors(&self, v: u32) -> Vec<u32> {
        let v = v as usize;
        let mut out: Vec<u32> = self.vert_out[v]
            .iter()
            .map(|&h| self.dest(h))
            .chain(self.vert_in[v].iter().map(|&h| self.origin(h)))
            .collect();
        out.sort_unstable();
        out.dedup();
        out
    }

    pub fn vertex_faces(&self, v: u32) -> Vec<u32> {
        let mut out: Vec<u32> = self.vert_out[v as usize]
            .iter()
            .map(|&h| self.face(h))
            .collect();
        out.sort_unstable();
        out.dedup();
        out
    }

    pub fn face_neighbors(&self, f: usize) -> Vec<u32> {
        let mut out: Vec<u32> = self
            .face_half_edges(f)
            .into_iter()
            .filter_map(|h| self.twin(h).map(|t| self.face(t)))
            .filter(|&g| g as usize != f)
            .collect();
        out.sort_unstable();
        out.dedup();
        out
    }

    /// V − E + F, counting only vertices used by a polygon.
    pub fn euler_characteristic(&self) -> i64 {
        let v = self.vert_out.iter().filter(|o| !o.is_empty()).count() as i64;
        v - self.edge_count as i64 + self.face_count() as i64
    }

    /// The boundary half-edge following `h` along its boundary loop.
    fn next_boundary(&self, h: HeId) -> HeId {
        let mut cur = self.next[h];
        // A fan has at most as many half-edges as the mesh; bound the walk.
        for _ in 0..self.half_edge_count() {
            match self.twin[cur] {
                None => return cur,
                Some(t) => cur = self.next[t],
            }
        }
        unreachable!("boundary walk did not terminate");
    }

    /// Ordered vertex loops along the boundary; see the spec for ordering.
    pub fn boundary_loops(&self) -> Vec<Vec<u32>> {
        let mut visited = vec![false; self.half_edge_count()];
        let mut loops = Vec::new();
        for start in 0..self.half_edge_count() {
            if visited[start] || !self.is_boundary_edge(start) {
                continue;
            }
            let mut lp = Vec::new();
            let mut h = start;
            while !visited[h] {
                visited[h] = true;
                lp.push(self.origin(h));
                h = self.next_boundary(h);
            }
            let min = lp
                .iter()
                .enumerate()
                .min_by_key(|(_, v)| **v)
                .map_or(0, |(i, _)| i);
            lp.rotate_left(min);
            loops.push(lp);
        }
        loops.sort();
        loops
    }
}
