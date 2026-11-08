//! The Correction deformer (docs/specs/MODIFIERS.md §14.7): stored point offsets.

use glam::DVec3;

/// How a point finds its corrected point.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mapping {
    /// Same index.
    Uv,
    /// Nearest stored point (`Normals` uses the same rule, parity-pending).
    Nearest,
}

/// Adds `strength · offsets[k]` to every point; `k` is the reference point the mapping
/// finds. Lists of different length leave the points alone.
pub fn correct(
    points: &mut [DVec3],
    reference: &[DVec3],
    offsets: &[DVec3],
    mapping: Mapping,
    strength: f64,
) {
    if reference.is_empty() || reference.len() != offsets.len() {
        return;
    }
    for (i, p) in points.iter_mut().enumerate() {
        let k = match mapping {
            Mapping::Uv => {
                if i >= reference.len() {
                    continue;
                }
                i
            }
            Mapping::Nearest => {
                let mut best = (f64::INFINITY, 0);
                for (k, r) in reference.iter().enumerate() {
                    let d = (*r - *p).length_squared();
                    if d < best.0 {
                        best = (d, k);
                    }
                }
                best.1
            }
        };
        *p += offsets[k] * strength;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_nearest_stored_point_supplies_the_offset() {
        let reference = vec![DVec3::ZERO, DVec3::new(100.0, 0.0, 0.0)];
        let offsets = vec![DVec3::new(0.0, 5.0, 0.0), DVec3::ZERO];
        let mut p = vec![DVec3::new(1.0, 0.0, 0.0), DVec3::new(90.0, 0.0, 0.0)];
        correct(&mut p, &reference, &offsets, Mapping::Nearest, 1.0);
        assert_eq!(p[0], DVec3::new(1.0, 5.0, 0.0));
        assert_eq!(p[1], DVec3::new(90.0, 0.0, 0.0));
        let mut q = vec![DVec3::new(90.0, 0.0, 0.0), DVec3::new(1.0, 0.0, 0.0)];
        correct(&mut q, &reference, &offsets, Mapping::Uv, 0.5);
        assert_eq!(q[0], DVec3::new(90.0, 2.5, 0.0));
        correct(&mut q, &reference, &[], Mapping::Uv, 1.0);
        assert_eq!(q[0], DVec3::new(90.0, 2.5, 0.0));
    }
}
