//! Adaptive ground grid on the XZ plane (docs/specs/VIEWPORT.md §4).

use glam::DVec3;

use crate::camera::Camera;

const HALF_CELLS: i64 = 50;
const MIN_PX: f64 = 10.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineKind {
    Minor,
    Major,
    /// The line along X at z = 0.
    AxisX,
    /// The line along Z at x = 0.
    AxisZ,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GridLine {
    pub a: DVec3,
    pub b: DVec3,
    pub kind: LineKind,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Grid {
    pub spacing: f64,
    pub lines: Vec<GridLine>,
}

/// Smallest power of ten ≥ `v` (v > 0).
fn ceil_pow10(v: f64) -> f64 {
    let mut e = v.log10().ceil() as i32;
    if 10f64.powi(e - 1) >= v {
        e -= 1;
    }
    10f64.powi(e)
}

pub fn grid_lines(cam: &Camera, viewport_h: f64) -> Grid {
    let spacing = ceil_pow10(MIN_PX * cam.world_per_pixel(viewport_h));
    let major = 10.0 * spacing;
    let cx = (cam.target_world().x / major).round() * major;
    let cz = (cam.target_world().z / major).round() * major;
    let half = HALF_CELLS as f64 * spacing;
    let kind = |coord: f64, axis: LineKind| {
        let index = (coord / spacing).round() as i64;
        if index == 0 {
            axis
        } else if index % 10 == 0 {
            LineKind::Major
        } else {
            LineKind::Minor
        }
    };
    let mut lines = Vec::with_capacity(2 * (2 * HALF_CELLS as usize + 1));
    for k in -HALF_CELLS..=HALF_CELLS {
        let z = cz + k as f64 * spacing;
        lines.push(GridLine {
            a: DVec3::new(cx - half, 0.0, z),
            b: DVec3::new(cx + half, 0.0, z),
            kind: kind(z, LineKind::AxisX),
        });
    }
    for k in -HALF_CELLS..=HALF_CELLS {
        let x = cx + k as f64 * spacing;
        lines.push(GridLine {
            a: DVec3::new(x, 0.0, cz - half),
            b: DVec3::new(x, 0.0, cz + half),
            kind: kind(x, LineKind::AxisZ),
        });
    }
    Grid { spacing, lines }
}
