//! Motion paths and onion-skin ghosts: draw lists (docs/specs/VIEWPORT.md section 12).
//! Pure: no GPU, no UI. Colours and sizes are our own design (parity-pending G5).

use forge_geometry::PolyMesh;
use glam::DVec3;

use crate::camera::Camera;
use crate::curves::{DrawLine, square};
use crate::render::Palette;

/// Side of a key square, in pixels.
pub const KEY_PX: f64 = 7.0;
/// Side of the current-frame square, in pixels.
pub const CURRENT_PX: f64 = 9.0;
/// Path segments up to the current frame.
pub const PATH_PAST: [f32; 3] = [0.55, 0.62, 0.78];
/// Path segments after the current frame.
pub const PATH_FUTURE: [f32; 3] = [0.35, 0.85, 0.95];
pub const KEY_COLOR: [f32; 3] = [1.0, 0.90, 0.30];
/// Ghosts of earlier frames.
pub const GHOST_BEFORE: [f32; 3] = [0.90, 0.35, 0.35];
/// Ghosts of later frames.
pub const GHOST_AFTER: [f32; 3] = [0.35, 0.80, 0.45];
/// A ghost mesh with more polygons is drawn as its bounding box.
pub const MAX_GHOST_POLYS: usize = 5000;

/// One object's motion path, world space.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PathDraw {
    /// (frame, position), ascending frames.
    pub samples: Vec<(f64, DVec3)>,
    pub keys: Vec<DVec3>,
    /// The current frame and the object's position there, when it lies in the path range.
    pub current: Option<(f64, DVec3)>,
}

/// One ghost of one object: wire segments in world space.
#[derive(Debug, Clone, PartialEq)]
pub struct GhostDraw {
    /// After the current frame.
    pub forward: bool,
    /// 1 is the ghost nearest the current frame.
    pub rank: u32,
    /// Number of ghosts on this side.
    pub of: u32,
    pub lines: Vec<[DVec3; 2]>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct MotionDraw {
    pub paths: Vec<PathDraw>,
    pub ghosts: Vec<GhostDraw>,
}

impl MotionDraw {
    pub fn is_empty(&self) -> bool {
        self.paths.is_empty() && self.ghosts.is_empty()
    }
}

/// The wire of a ghost: unique polygon edges, or the 12 edges of the bounding box for a mesh
/// over [`MAX_GHOST_POLYS`] polygons. Empty for an empty mesh.
pub fn ghost_geometry(mesh: &PolyMesh) -> Vec<[DVec3; 2]> {
    if mesh.points.is_empty() || mesh.polys.is_empty() {
        return Vec::new();
    }
    if mesh.polys.len() > MAX_GHOST_POLYS {
        let (lo, hi) = mesh
            .points
            .iter()
            .fold((DVec3::MAX, DVec3::MIN), |(lo, hi), p| {
                (lo.min(*p), hi.max(*p))
            });
        let corner = |i: usize| {
            DVec3::new(
                if i & 1 == 1 { hi.x } else { lo.x },
                if i & 2 == 2 { hi.y } else { lo.y },
                if i & 4 == 4 { hi.z } else { lo.z },
            )
        };
        let mut out = Vec::new();
        for i in 0..8usize {
            for bit in [1usize, 2, 4] {
                if i & bit == 0 {
                    out.push([corner(i), corner(i | bit)]);
                }
            }
        }
        return out;
    }
    mesh.edges()
        .into_iter()
        .filter_map(|e| {
            Some([
                *mesh.points.get(e.0 as usize)?,
                *mesh.points.get(e.1 as usize)?,
            ])
        })
        .collect()
}

/// How much of the ghost tint shows: `0.8` for the nearest ghost, down to `0.3` for the
/// farthest of `of`.
pub fn ghost_strength(rank: u32, of: u32) -> f32 {
    if of <= 1 {
        return 0.8;
    }
    let k = (rank.clamp(1, of) - 1) as f32 / (of - 1) as f32;
    0.8 - 0.5 * k
}

/// The ghost's line colour: its side tint mixed with the background by [`ghost_strength`].
pub fn ghost_color(forward: bool, rank: u32, of: u32, pal: &Palette) -> [f32; 3] {
    let tint = if forward { GHOST_AFTER } else { GHOST_BEFORE };
    let s = ghost_strength(rank, of);
    std::array::from_fn(|i| pal.background[i] + (tint[i] - pal.background[i]) * s)
}

/// All lines of `d` seen by `cam` in a viewport of `size` pixels. Order: ghosts, then per
/// path its segments, key squares and the current-frame square.
pub fn overlay_lines(d: &MotionDraw, pal: &Palette, cam: &Camera, size: [f64; 2]) -> Vec<DrawLine> {
    let mut out = Vec::new();
    for g in &d.ghosts {
        let color = ghost_color(g.forward, g.rank, g.of, pal);
        out.extend(g.lines.iter().map(|l| DrawLine {
            a: l[0],
            b: l[1],
            color,
        }));
    }
    for p in &d.paths {
        let now = p.current.map(|c| c.0);
        for w in p.samples.windows(2) {
            let past = now.is_some_and(|n| w[1].0 <= n);
            out.push(DrawLine {
                a: w[0].1,
                b: w[1].1,
                color: if past { PATH_PAST } else { PATH_FUTURE },
            });
        }
        for k in &p.keys {
            square(&mut out, cam, size, *k, KEY_PX, KEY_COLOR);
        }
        if let Some((_, pos)) = p.current {
            square(&mut out, cam, size, pos, CURRENT_PX, pal.selection);
        }
    }
    out
}
