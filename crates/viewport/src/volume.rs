//! Volume display: the box, slice markers and the value colour ramp as draw lists
//! (docs/specs/VOLUME.md section 11). Pure: no GPU, no UI; the iso-surface preview is an
//! ordinary mesh the renderer already draws. Colours and sizes are our own design
//! (GUI session G4 is unanswered): **parity-pending**.

use glam::DVec3;

use crate::curves::DrawLine;

/// Colour of the box of a volume.
pub const BOX_COLOR: [f32; 3] = [0.55, 0.75, 0.95];

/// What the viewport shows of a volume (the builder's Display setting).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VolumeDisplay {
    /// Only the box of the grid.
    Box,
    /// The box and the coarse iso-surface.
    Preview,
    /// The box and one slice of samples: `axis` 0, 1, 2, lattice `index`.
    Slice { axis: usize, index: i32 },
}

/// The 12 edges of the axis-aligned box `min`, `max`.
pub fn box_lines(min: DVec3, max: DVec3, color: [f32; 3]) -> Vec<DrawLine> {
    let corner = |i: usize| {
        DVec3::new(
            if i & 1 == 0 { min.x } else { max.x },
            if i & 2 == 0 { min.y } else { max.y },
            if i & 4 == 0 { min.z } else { max.z },
        )
    };
    let mut out = Vec::with_capacity(12);
    for i in 0..8usize {
        for bit in [1usize, 2, 4] {
            if i & bit == 0 {
                out.push(DrawLine {
                    a: corner(i),
                    b: corner(i | bit),
                    color,
                });
            }
        }
    }
    out
}

/// Colour of a field value `v` for a band of half width `band`: blue inside (negative), orange
/// outside, white on the surface, saturating at the band.
pub fn value_color(v: f32, band: f32) -> [f32; 3] {
    let t = if band > 0.0 {
        (v / band).clamp(-1.0, 1.0)
    } else {
        0.0
    };
    let (target, k) = if t < 0.0 {
        ([0.15, 0.35, 0.95], -t)
    } else {
        ([0.95, 0.55, 0.1], t)
    };
    [
        1.0 + (target[0] - 1.0) * k,
        1.0 + (target[1] - 1.0) * k,
        1.0 + (target[2] - 1.0) * k,
    ]
}

/// A small cross (three axis-aligned segments of half length `half`) at every sample, coloured
/// by [`value_color`].
pub fn slice_markers(samples: &[(DVec3, f32)], half: f64, band: f32) -> Vec<DrawLine> {
    let mut out = Vec::with_capacity(samples.len() * 3);
    for (p, v) in samples {
        let color = value_color(*v, band);
        for axis in [DVec3::X, DVec3::Y, DVec3::Z] {
            out.push(DrawLine {
                a: *p - axis * half,
                b: *p + axis * half,
                color,
            });
        }
    }
    out
}

/// Everything a volume draws besides its mesh: the box, and the markers of a slice.
pub fn display_lines(
    bounds: Option<(DVec3, DVec3)>,
    display: VolumeDisplay,
    slice: &[(DVec3, f32)],
    voxel_size: f64,
    band: f32,
) -> Vec<DrawLine> {
    let mut out = bounds
        .map(|(lo, hi)| box_lines(lo, hi, BOX_COLOR))
        .unwrap_or_default();
    if matches!(display, VolumeDisplay::Slice { .. }) {
        out.extend(slice_markers(slice, voxel_size * 0.2, band));
    }
    out
}
