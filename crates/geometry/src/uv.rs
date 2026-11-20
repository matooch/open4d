//! Corner UVs: one `(u, v)` per polygon corner, parallel to [`PolyMesh::polys`]
//! (docs/specs/UV.md section U1).

use crate::mesh::PolyMesh;

/// Texture coordinates per polygon corner. `corners[p][c]` belongs to corner `c` of polygon
/// `p`; a mesh and its UVs stay valid together while the polygon sizes agree.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CornerUvs {
    pub corners: Vec<Vec<[f64; 2]>>,
}

impl CornerUvs {
    /// True if there is one UV per corner of `mesh`.
    pub fn matches(&self, mesh: &PolyMesh) -> bool {
        self.corners.len() == mesh.polys.len()
            && self
                .corners
                .iter()
                .zip(&mesh.polys)
                .all(|(c, p)| c.len() == p.len())
    }

    /// The OBJ view: a deduplicated table (first-use order, values equal within `tol`) and the
    /// table index of every corner.
    pub fn indexed(&self, tol: f64) -> (Vec<[f64; 2]>, Vec<Vec<u32>>) {
        let mut table: Vec<[f64; 2]> = Vec::new();
        let mut lookup = std::collections::HashMap::new();
        let key = |v: f64| (v / tol).round() as i64;
        let idx = self
            .corners
            .iter()
            .map(|poly| {
                poly.iter()
                    .map(|uv| {
                        *lookup.entry((key(uv[0]), key(uv[1]))).or_insert_with(|| {
                            table.push(*uv);
                            (table.len() - 1) as u32
                        })
                    })
                    .collect()
            })
            .collect();
        (table, idx)
    }
}
