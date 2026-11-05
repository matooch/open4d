//! Arrangement rules: the copy step shared by Duplicate and the Array command, Arrange
//! positions, Randomize draws and the Array command itself (docs/specs/MODELING.md §9).
//! Own design, parity-pending.

use forge_core::transform::Transform;
use forge_geometry::PolyMesh;
use forge_geometry::selection::{IndexSet, Selection};
use glam::DVec3;

use crate::{Settings, ToolError};

/// A copy's scale factor never falls below this.
pub const MIN_SCALE: f64 = 0.001;

/// Most points an Array command may produce.
pub const MAX_ARRAY_POINTS: usize = 4_000_000;

/// The placement delta of one copy (§9.1).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Step {
    /// Added to the position.
    pub pos: DVec3,
    /// Added to the heading, pitch and bank, in degrees.
    pub rot_deg: DVec3,
    /// Multiplies the scale per axis.
    pub scale: DVec3,
}

impl Step {
    /// The step as a transform (rotation in radians).
    pub fn transform(&self) -> Transform {
        Transform {
            pos: self.pos,
            rot: self.rot_deg.map(f64::to_radians),
            scale: self.scale,
        }
    }
}

/// Copy `k` (1-based) of `copies`. `mode` is `step` (each copy adds the settings once more)
/// or `total` (the last copy reaches the settings); anything else counts as `step`.
pub fn copy_step(
    k: usize,
    copies: usize,
    mode: &str,
    offset: DVec3,
    rotation_deg: DVec3,
    scale: DVec3,
) -> Step {
    let f = if mode == "total" {
        k as f64 / copies.max(1) as f64
    } else {
        k as f64
    };
    Step {
        pos: offset * f,
        rot_deg: rotation_deg * f,
        scale: scale.map(|s| (1.0 + f * (s - 1.0)).max(MIN_SCALE)),
    }
}

/// The step of copy `k` from the settings of Duplicate or the Array command.
pub fn step_of(k: usize, s: &Settings) -> Step {
    copy_step(
        k,
        s.int("copies").max(1) as usize,
        s.choice("mode"),
        s.vec3("offset"),
        s.vec3("rotation_deg"),
        s.vec3("scale"),
    )
}

/// The Array command (§9.6): the mesh followed by `copies` transformed copies of itself.
/// The selection that comes back holds the polygons of the copies.
pub fn array(
    mesh: &PolyMesh,
    _sel: &Selection,
    s: &Settings,
) -> Result<(PolyMesh, Selection), ToolError> {
    let copies = s.int("copies").max(1) as usize;
    if mesh.points.is_empty() || mesh.polys.is_empty() {
        return Err(ToolError::Invalid("the mesh is empty".into()));
    }
    if mesh.points.len().saturating_mul(copies + 1) > MAX_ARRAY_POINTS {
        return Err(ToolError::Invalid(format!(
            "the result would have more than {MAX_ARRAY_POINTS} points"
        )));
    }
    let mut out = mesh.clone();
    let mut selected = IndexSet::new();
    for k in 1..=copies {
        let m = step_of(k, s).transform().matrix();
        let base = out.points.len() as u32;
        out.points
            .extend(mesh.points.iter().map(|p| m.transform_point3(*p)));
        for poly in &mesh.polys {
            selected.insert(out.polys.len() as u32);
            out.polys.push(poly.iter().map(|c| c + base).collect());
        }
    }
    Ok((
        out,
        Selection {
            polys: selected,
            ..Selection::default()
        },
    ))
}

/// New local positions for the nodes at `positions`, in the order given (§9.3).
pub fn arrange_positions(positions: &[DVec3], s: &Settings) -> Vec<DVec3> {
    let n = positions.len();
    let Some(first) = positions.first().copied() else {
        return Vec::new();
    };
    match s.choice("mode") {
        "grid" => {
            let columns = s.int("columns").max(1) as usize;
            let (col, row) = (s.vec3("spacing"), s.vec3("row_spacing"));
            (0..n)
                .map(|i| first + col * (i % columns) as f64 + row * (i / columns) as f64)
                .collect()
        }
        "circle" => {
            let centre = positions.iter().copied().sum::<DVec3>() / n as f64;
            let (u, v) = match s.choice("plane") {
                "xy" => (DVec3::X, DVec3::Y),
                "zy" => (DVec3::Z, DVec3::Y),
                _ => (DVec3::X, DVec3::Z),
            };
            let (radius, start) = (s.float("radius"), s.float("start_angle_deg"));
            (0..n)
                .map(|i| {
                    let a = (start + 360.0 * i as f64 / n as f64).to_radians();
                    centre + radius * (a.cos() * u + a.sin() * v)
                })
                .collect()
        }
        _ => {
            let step = s.vec3("spacing");
            (0..n).map(|i| first + step * i as f64).collect()
        }
    }
}

const GOLDEN: u64 = 0x9E37_79B9_7F4A_7C15;

/// The nine draws of one node in `[-1, 1)` (§9.4): position x y z, rotation h p b, scale x y z.
/// SplitMix64 (public-domain algorithm) from `seed ^ node_id * GOLDEN`.
pub fn randomize_draw(seed: i64, node_id: u64) -> [f64; 9] {
    let mut state = (seed as u64) ^ node_id.wrapping_mul(GOLDEN);
    let mut out = [0.0; 9];
    for slot in &mut out {
        state = state.wrapping_add(GOLDEN);
        let mut z = state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^= z >> 31;
        *slot = 2.0 * ((z >> 11) as f64 / (1u64 << 53) as f64) - 1.0;
    }
    out
}
