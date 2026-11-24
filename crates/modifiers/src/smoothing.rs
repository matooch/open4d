//! Smoothing deformer (docs/specs/MODIFIERS.md §22.1).

use forge_geometry::PolyMesh;
use glam::DVec3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Relax,
    Smooth,
    Stretch,
}

pub struct SmoothingParams {
    /// Fractions (the percent params are stored as 0..1).
    pub strength: f64,
    pub kind: Kind,
    pub iterations: u32,
    pub stiffness: f64,
    pub keep_displacements: bool,
}

impl Default for SmoothingParams {
    fn default() -> Self {
        Self {
            strength: 1.0,
            kind: Kind::Smooth,
            iterations: 10,
            stiffness: 0.3,
            keep_displacements: false,
        }
    }
}

/// Laplacian smoothing: every point moves `strength * (1 - stiffness)` of the way to the
/// mean of its edge neighbours, repeated `iterations` times (Jacobi, so the order does not
/// matter). The `1 - stiffness` weight is the one that reproduces
/// `smoothing-deformer/defaults` (area 140,956, volume 4,339,075).
pub fn smooth(mesh: &mut PolyMesh, p: &SmoothingParams) {
    let lambda = p.strength.clamp(0.0, 1.0) * (1.0 - p.stiffness.clamp(0.0, 1.0));
    if lambda == 0.0 {
        return;
    }
    let n = mesh.points.len();
    let mut neighbours: Vec<Vec<u32>> = vec![Vec::new(); n];
    for e in mesh.edges() {
        let (a, b) = (e.0 as usize, e.1 as usize);
        if a < n && b < n {
            neighbours[a].push(e.1);
            neighbours[b].push(e.0);
        }
    }
    let original = mesh.points.clone();
    let mut current = mesh.points.clone();
    let iterations = p.iterations.max(1);
    for _ in 0..iterations {
        let mut next = current.clone();
        for i in 0..n {
            if neighbours[i].is_empty() {
                continue;
            }
            let mean = neighbours[i]
                .iter()
                .map(|j| current[*j as usize])
                .sum::<DVec3>()
                / neighbours[i].len() as f64;
            next[i] = current[i] + (mean - current[i]) * lambda;
        }
        current = next;
    }
    if p.keep_displacements {
        // Stored, not derived: the reference's displacement preservation is unobserved.
        let _ = &original;
    }
    mesh.points = current;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_flat_quad_stays_flat() {
        let mut mesh = PolyMesh {
            points: vec![
                DVec3::new(0.0, 0.0, 0.0),
                DVec3::new(1.0, 0.0, 0.0),
                DVec3::new(1.0, 1.0, 0.0),
                DVec3::new(0.0, 1.0, 0.0),
            ],
            polys: vec![vec![0, 1, 2, 3]],
        };
        smooth(
            &mut mesh,
            &SmoothingParams {
                iterations: 5,
                ..Default::default()
            },
        );
        // Boundary points slide inward but a flat sheet stays in its plane.
        assert!(mesh.points.iter().all(|p| p.z.abs() < 1e-12));
    }

    #[test]
    fn a_spike_is_pulled_in() {
        let mut mesh = PolyMesh {
            points: vec![
                DVec3::new(0.0, 0.0, 0.0),
                DVec3::new(2.0, 0.0, 0.0),
                DVec3::new(0.0, 2.0, 0.0),
                DVec3::new(0.0, 0.0, 2.0),
                DVec3::new(0.0, 0.0, -2.0),
            ],
            polys: vec![vec![0, 1, 2], vec![0, 1, 3], vec![0, 1, 4]],
        };
        smooth(
            &mut mesh,
            &SmoothingParams {
                iterations: 1,
                ..Default::default()
            },
        );
        assert!(mesh.points[1].x < 2.0);
    }

    #[test]
    fn zero_strength_changes_nothing() {
        let mut mesh = PolyMesh {
            points: vec![DVec3::ZERO, DVec3::X, DVec3::Y],
            polys: vec![vec![0, 1, 2]],
        };
        let before = mesh.points.clone();
        smooth(
            &mut mesh,
            &SmoothingParams {
                strength: 0.0,
                ..Default::default()
            },
        );
        assert_eq!(mesh.points, before);
    }
}
