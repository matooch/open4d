//! The Camera deformer (docs/specs/MODIFIERS.md §14.3): a grid in screen space.

use glam::{DMat4, DVec3};

/// Half extent of the grid in `x/z`, `y/z` (the grid is square).
pub const GRID_HALF: f64 = 0.75;
/// Distance at which the grid offsets are measured.
pub const REFERENCE_DEPTH: f64 = 1000.0;

/// Index of grid point `(i, j)` in the offset list.
pub fn grid_index(grid_x: usize, i: usize, j: usize) -> usize {
    j * (grid_x + 1) + i
}

/// Screen position `(u, v)` of grid point `(i, j)`.
pub fn grid_position(grid_x: usize, grid_y: usize, i: usize, j: usize) -> (f64, f64) {
    (
        -GRID_HALF + 2.0 * GRID_HALF * i as f64 / grid_x as f64,
        -GRID_HALF + 2.0 * GRID_HALF * j as f64 / grid_y as f64,
    )
}

/// Moves `points` (world) by the bilinear grid offset seen from `camera` (its world
/// matrix, looking along +z). `offsets` holds `(grid_x+1)·(grid_y+1)` entries or none.
pub fn camera_deform(
    points: &mut [DVec3],
    camera: DMat4,
    offsets: &[DVec3],
    grid: (usize, usize),
    strength: f64,
) {
    let (gx, gy) = grid;
    if gx == 0 || gy == 0 || offsets.len() != (gx + 1) * (gy + 1) {
        return;
    }
    let inv = camera.inverse();
    if !inv.is_finite() {
        return;
    }
    for p in points.iter_mut() {
        let c = inv.transform_point3(*p);
        if c.z <= 1e-9 {
            continue;
        }
        let gu = (c.x / c.z + GRID_HALF) / (2.0 * GRID_HALF) * gx as f64;
        let gv = (c.y / c.z + GRID_HALF) / (2.0 * GRID_HALF) * gy as f64;
        if !(0.0..=gx as f64).contains(&gu) || !(0.0..=gy as f64).contains(&gv) {
            continue;
        }
        let (i, j) = (
            (gu.floor() as usize).min(gx - 1),
            (gv.floor() as usize).min(gy - 1),
        );
        let (fx, fy) = (gu - i as f64, gv - j as f64);
        let o = |i: usize, j: usize| offsets[grid_index(gx, i, j)];
        let off = (o(i, j) * (1.0 - fx) + o(i + 1, j) * fx) * (1.0 - fy)
            + (o(i, j + 1) * (1.0 - fx) + o(i + 1, j + 1) * fx) * fy;
        let shift = DVec3::new(off.x, off.y, 0.0) * (c.z / REFERENCE_DEPTH) * strength;
        *p = camera.transform_point3(c + shift);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_grid_point_moves_the_points_at_its_position() {
        let (gx, gy) = (4, 4);
        let mut offsets = vec![DVec3::ZERO; 25];
        let centre = grid_index(gx, 2, 2);
        offsets[centre] = DVec3::new(50.0, 0.0, 0.0);
        let cam = DMat4::from_translation(DVec3::new(0.0, 0.0, -1000.0));
        let mut pts = vec![
            DVec3::new(0.0, 0.0, 0.0),
            DVec3::new(2000.0, 0.0, 0.0),
            DVec3::new(0.0, 0.0, -2000.0),
        ];
        camera_deform(&mut pts, cam, &offsets, (gx, gy), 1.0);
        // depth 1000: the offset applies in full; far outside the grid and behind: unchanged
        assert!((pts[0] - DVec3::new(50.0, 0.0, 0.0)).length() < 1e-9);
        assert_eq!(pts[1], DVec3::new(2000.0, 0.0, 0.0));
        assert_eq!(pts[2], DVec3::new(0.0, 0.0, -2000.0));
        let mut none = vec![DVec3::new(1.0, 2.0, 3.0)];
        camera_deform(&mut none, cam, &[], (gx, gy), 1.0);
        assert_eq!(none[0], DVec3::new(1.0, 2.0, 3.0));
    }
}
