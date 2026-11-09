//! Splines in the viewport: the draw list of a spline object and picking of its control points
//! and handles (docs/specs/VIEWPORT.md section 11). Pure: no GPU, no UI.

use glam::DVec3;

use crate::camera::{Camera, Projection};
use crate::render::{Palette, Shading};

/// Side of a control point square, in pixels.
pub const POINT_PX: f64 = 7.0;
/// Side of a handle end square, in pixels.
pub const HANDLE_PX: f64 = 4.0;
/// Default pick radius of control points and handle ends, in pixels.
pub const PICK_RADIUS: f64 = 8.0;
/// Colour of unselected control points.
pub const POINT_COLOR: [f32; 3] = [0.85, 0.85, 0.85];
/// Colour of handle lines and handle end squares.
pub const HANDLE_COLOR: [f32; 3] = [0.45, 0.75, 0.95];

/// One sampled loop of a spline, world space.
#[derive(Debug, Clone, PartialEq)]
pub struct Loop {
    pub points: Vec<DVec3>,
    pub closed: bool,
}

/// The control points of an edited spline, world space (section 11.1).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Controls {
    pub points: Vec<DVec3>,
    /// World position of each left handle end (`point + offset`).
    pub left: Vec<DVec3>,
    pub right: Vec<DVec3>,
    pub selected: Vec<bool>,
    /// Whether handles exist and are shown (the Bezier type).
    pub handles: bool,
}

/// One spline object as the viewport draws it.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SplineDraw {
    pub loops: Vec<Loop>,
    pub selected: bool,
    pub controls: Option<Controls>,
}

/// One coloured line segment, world space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DrawLine {
    pub a: DVec3,
    pub b: DVec3,
    pub color: [f32; 3],
}

/// A control point or handle end under the cursor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlHit {
    Point(usize),
    Left(usize),
    Right(usize),
}

impl SplineDraw {
    /// The box of the sampled points.
    pub fn bounds(&self) -> Option<(DVec3, DVec3)> {
        self.loops
            .iter()
            .flat_map(|l| l.points.iter().copied())
            .fold(None, |acc, p| {
                Some(acc.map_or((p, p), |(lo, hi): (DVec3, DVec3)| (lo.min(p), hi.max(p))))
            })
    }
}

/// World size of a pixel at `p`; `None` behind the camera.
fn pixel_world(cam: &Camera, p: DVec3, h: f64) -> Option<f64> {
    match cam.projection {
        Projection::Perspective { fov_y } => {
            let depth = (p - cam.eye_world()).dot(cam.forward_world());
            (depth > 1e-9).then(|| 2.0 * depth * (fov_y / 2.0).tan() / h)
        }
        Projection::Orthographic { height } => Some(height / h),
    }
}

/// A square of `px` pixels centred on `c`, in the camera plane (four lines).
pub(crate) fn square(
    out: &mut Vec<DrawLine>,
    cam: &Camera,
    size: [f64; 2],
    c: DVec3,
    px: f64,
    color: [f32; 3],
) {
    let Some(wpp) = pixel_world(cam, c, size[1]) else {
        return;
    };
    let r = cam.right_world() * (px / 2.0 * wpp);
    let u = cam.up_world() * (px / 2.0 * wpp);
    let corners = [c - r - u, c + r - u, c + r + u, c - r + u];
    for i in 0..4 {
        out.push(DrawLine {
            a: corners[i],
            b: corners[(i + 1) % 4],
            color,
        });
    }
}

fn box_lines(out: &mut Vec<DrawLine>, (lo, hi): (DVec3, DVec3), color: [f32; 3]) {
    let corner = |i: usize| {
        DVec3::new(
            if i & 1 == 1 { hi.x } else { lo.x },
            if i & 2 == 2 { hi.y } else { lo.y },
            if i & 4 == 4 { hi.z } else { lo.z },
        )
    };
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
}

/// The lines of one spline for `shading` seen by `cam` in a viewport of `size` pixels
/// (section 11.1).
pub fn draw_list(
    d: &SplineDraw,
    shading: Shading,
    pal: &Palette,
    cam: &Camera,
    size: [f64; 2],
) -> Vec<DrawLine> {
    let color = if d.selected {
        pal.selection
    } else {
        pal.object
    };
    let mut out = Vec::new();
    if shading == Shading::BoundingBox {
        if let Some(b) = d.bounds() {
            box_lines(&mut out, b, color);
        }
    } else {
        for l in &d.loops {
            for w in l.points.windows(2) {
                out.push(DrawLine {
                    a: w[0],
                    b: w[1],
                    color,
                });
            }
            if l.closed
                && l.points.len() > 2
                && let (Some(first), Some(last)) = (l.points.first(), l.points.last())
                && first != last
            {
                out.push(DrawLine {
                    a: *last,
                    b: *first,
                    color,
                });
            }
        }
    }
    let Some(c) = &d.controls else {
        return out;
    };
    let ends = |c: &Controls, i: usize| [(c.left[i], c.points[i]), (c.right[i], c.points[i])];
    let shown = |end: DVec3, p: DVec3| c.handles && (end - p).length_squared() > 1e-18;
    for i in 0..c.points.len() {
        for (end, p) in ends(c, i) {
            if shown(end, p) {
                out.push(DrawLine {
                    a: p,
                    b: end,
                    color: HANDLE_COLOR,
                });
            }
        }
    }
    for i in 0..c.points.len() {
        for (end, p) in ends(c, i) {
            if shown(end, p) {
                square(&mut out, cam, size, end, HANDLE_PX, HANDLE_COLOR);
            }
        }
    }
    for (i, p) in c.points.iter().enumerate() {
        let color = if c.selected.get(i).copied().unwrap_or(false) {
            pal.selection
        } else {
            POINT_COLOR
        };
        square(&mut out, cam, size, *p, POINT_PX, color);
    }
    out
}

/// The control point or handle end nearest the cursor within `radius` pixels (section 11.1).
pub fn pick(
    d: &SplineDraw,
    cam: &Camera,
    cursor: (f64, f64),
    size: (f64, f64),
    radius: f64,
) -> Option<ControlHit> {
    let c = d.controls.as_ref()?;
    let dist = |p: DVec3| {
        let s = cam.project(p, size.0, size.1)?;
        let dd = ((s.x - cursor.0).powi(2) + (s.y - cursor.1).powi(2)).sqrt();
        (dd <= radius).then_some(dd)
    };
    // (distance, kind rank, index): Point < Right < Left
    let mut best: Option<(f64, u8, usize, ControlHit)> = None;
    let mut offer = |dd: f64, rank: u8, i: usize, hit: ControlHit| {
        let better = best.is_none_or(|(bd, br, bi, _)| (dd, rank, i) < (bd, br, bi));
        if better {
            best = Some((dd, rank, i, hit));
        }
    };
    for (i, p) in c.points.iter().enumerate() {
        if let Some(dd) = dist(*p) {
            offer(dd, 0, i, ControlHit::Point(i));
        }
        if c.handles {
            let far = |end: DVec3| (end - *p).length_squared() > 1e-18;
            if far(c.right[i])
                && let Some(dd) = dist(c.right[i])
            {
                offer(dd, 1, i, ControlHit::Right(i));
            }
            if far(c.left[i])
                && let Some(dd) = dist(c.left[i])
            {
                offer(dd, 2, i, ControlHit::Left(i));
            }
        }
    }
    best.map(|b| b.3)
}
