//! World-axis translate and scale gizmo (docs/specs/VIEWPORT.md §5).
//! Rotation and object-local axes wait for docs/parity/transform.md.

use glam::{DVec2, DVec3};

use crate::camera::{Camera, Projection};
use crate::snap::{line_param_closest_to_ray, ray_plane, snap_grid, snap_scalar};

const CENTER_PX: f64 = 8.0;
const AXIS_TOL_PX: f64 = 6.0;
const PLANE_LO: f64 = 0.2;
const PLANE_HI: f64 = 0.45;
/// Handles seen (nearly) edge-on are not pickable.
const MIN_PLANE_AREA_PX: f64 = 25.0;
const MIN_AXIS_LEN_PX: f64 = 6.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GizmoMode {
    Translate,
    Scale,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Axis {
    X,
    Y,
    Z,
}

impl Axis {
    pub const ALL: [Axis; 3] = [Axis::X, Axis::Y, Axis::Z];

    pub fn vec(self) -> DVec3 {
        match self {
            Axis::X => DVec3::X,
            Axis::Y => DVec3::Y,
            Axis::Z => DVec3::Z,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Plane {
    XY,
    YZ,
    XZ,
}

impl Plane {
    pub const ALL: [Plane; 3] = [Plane::XY, Plane::YZ, Plane::XZ];

    pub fn axes(self) -> (Axis, Axis) {
        match self {
            Plane::XY => (Axis::X, Axis::Y),
            Plane::YZ => (Axis::Y, Axis::Z),
            Plane::XZ => (Axis::X, Axis::Z),
        }
    }

    pub fn normal(self) -> DVec3 {
        match self {
            Plane::XY => DVec3::Z,
            Plane::YZ => DVec3::X,
            Plane::XZ => DVec3::Y,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Handle {
    Axis(Axis),
    Plane(Plane),
    Center,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Gizmo {
    pub origin: DVec3,
    pub mode: GizmoMode,
    pub size_px: f64,
}

impl Default for Gizmo {
    fn default() -> Self {
        Gizmo {
            origin: DVec3::ZERO,
            mode: GizmoMode::Translate,
            size_px: 100.0,
        }
    }
}

/// World size of a pixel at `p`'s depth.
pub fn world_per_pixel_at(cam: &Camera, p: DVec3, viewport_h: f64) -> f64 {
    match cam.projection {
        Projection::Perspective { fov_y } => {
            let depth = (p - cam.eye_world()).dot(cam.forward_world());
            2.0 * depth * (fov_y / 2.0).tan() / viewport_h
        }
        Projection::Orthographic { height } => height / viewport_h,
    }
}

fn seg_distance(p: DVec2, a: DVec2, b: DVec2) -> f64 {
    let ab = b - a;
    let len2 = ab.length_squared();
    let s = if len2 > 0.0 {
        ((p - a).dot(ab) / len2).clamp(0.0, 1.0)
    } else {
        0.0
    };
    (a + ab * s - p).length()
}

/// Point-in-quad for a convex screen quad; edge-on (tiny-area) quads never hit.
fn in_convex_quad(p: DVec2, q: [DVec2; 4]) -> bool {
    let area = 0.5 * ((q[2] - q[0]).perp_dot(q[3] - q[1])).abs();
    if area < MIN_PLANE_AREA_PX {
        return false;
    }
    let sides: Vec<f64> = (0..4)
        .map(|i| {
            let (a, b) = (q[i], q[(i + 1) % 4]);
            (b - a).perp_dot(p - a)
        })
        .collect();
    sides.iter().all(|s| *s >= 0.0) || sides.iter().all(|s| *s <= 0.0)
}

impl Gizmo {
    /// World length of an axis handle.
    pub fn world_length(&self, cam: &Camera, viewport_h: f64) -> f64 {
        self.size_px * world_per_pixel_at(cam, self.origin, viewport_h)
    }

    /// World-space corners of a plane handle.
    pub fn plane_corners(&self, plane: Plane, l: f64) -> [DVec3; 4] {
        let (a, b) = plane.axes();
        let (a, b) = (a.vec() * l, b.vec() * l);
        let o = self.origin;
        [
            o + a * PLANE_LO + b * PLANE_LO,
            o + a * PLANE_HI + b * PLANE_LO,
            o + a * PLANE_HI + b * PLANE_HI,
            o + a * PLANE_LO + b * PLANE_HI,
        ]
    }

    pub fn hit_test(&self, cam: &Camera, x: f64, y: f64, w: f64, h: f64) -> Option<Handle> {
        let cursor = DVec2::new(x, y);
        let screen = |p: DVec3| cam.project(p, w, h).map(|s| s.truncate());
        let o = screen(self.origin)?;
        if (cursor - o).length() <= CENTER_PX {
            return Some(Handle::Center);
        }
        let l = self.world_length(cam, h);
        for plane in Plane::ALL {
            let c = self.plane_corners(plane, l).map(screen);
            if let [Some(a), Some(b), Some(cc), Some(d)] = c
                && in_convex_quad(cursor, [a, b, cc, d])
            {
                return Some(Handle::Plane(plane));
            }
        }
        Axis::ALL
            .iter()
            .filter_map(|&axis| {
                let end = screen(self.origin + axis.vec() * l)?;
                if (end - o).length() < MIN_AXIS_LEN_PX {
                    return None; // pointing at the camera
                }
                let d = seg_distance(cursor, o, end);
                (d <= AXIS_TOL_PX).then_some((d, axis))
            })
            .min_by(|a, b| a.0.total_cmp(&b.0))
            .map(|(_, axis)| Handle::Axis(axis))
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DragDelta {
    Translate(DVec3),
    Scale(DVec3),
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Start {
    Line {
        axis: Axis,
        s0: f64,
    },
    Plane {
        plane_normal: DVec3,
        axes: Option<(Axis, Axis)>,
        hit0: DVec3,
    },
    Screen {
        x0: f64,
    },
}

/// An in-progress gizmo drag.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Drag {
    gizmo: Gizmo,
    start: Start,
    last: DragDelta,
}

fn snap1(v: f64, snap: Option<f64>) -> f64 {
    snap.map_or(v, |inc| snap_scalar(v, inc))
}

impl Drag {
    pub fn begin(
        gizmo: &Gizmo,
        handle: Handle,
        cam: &Camera,
        x: f64,
        y: f64,
        w: f64,
        h: f64,
    ) -> Option<Drag> {
        let ray = cam.ray(x, y, w, h);
        let o = gizmo.origin;
        let scale = gizmo.mode == GizmoMode::Scale;
        let start = match handle {
            Handle::Axis(axis) => {
                let s0 = line_param_closest_to_ray(o, axis.vec(), &ray)?;
                if scale && s0.abs() < 1e-9 {
                    return None;
                }
                Start::Line { axis, s0 }
            }
            Handle::Plane(plane) => {
                let hit0 = ray.at(ray_plane(&ray, o, plane.normal())?);
                if scale && (hit0 - o).length() < 1e-9 {
                    return None;
                }
                Start::Plane {
                    plane_normal: plane.normal(),
                    axes: Some(plane.axes()),
                    hit0,
                }
            }
            Handle::Center if scale => Start::Screen { x0: x },
            Handle::Center => {
                let n = cam.forward_world();
                let hit0 = ray.at(ray_plane(&ray, o, n)?);
                Start::Plane {
                    plane_normal: n,
                    axes: None,
                    hit0,
                }
            }
        };
        let last = if scale {
            DragDelta::Scale(DVec3::ONE)
        } else {
            DragDelta::Translate(DVec3::ZERO)
        };
        Some(Drag {
            gizmo: *gizmo,
            start,
            last,
        })
    }

    #[allow(clippy::too_many_arguments)] // camera, cursor, viewport and snap are independent
    pub fn update(
        &mut self,
        cam: &Camera,
        x: f64,
        y: f64,
        w: f64,
        h: f64,
        snap: Option<f64>,
    ) -> DragDelta {
        let ray = cam.ray(x, y, w, h);
        let o = self.gizmo.origin;
        let scale = self.gizmo.mode == GizmoMode::Scale;
        let next = match self.start {
            Start::Line { axis, s0 } => line_param_closest_to_ray(o, axis.vec(), &ray).map(|s| {
                if scale {
                    let mut v = DVec3::ONE;
                    v[axis as usize] = snap1(s / s0, snap);
                    DragDelta::Scale(v)
                } else {
                    DragDelta::Translate(axis.vec() * snap1(s - s0, snap))
                }
            }),
            Start::Plane {
                plane_normal,
                axes,
                hit0,
            } => ray_plane(&ray, o, plane_normal).map(|t| {
                let hit = ray.at(t);
                match (scale, axes) {
                    (true, Some((a, b))) => {
                        let f = snap1((hit - o).length() / (hit0 - o).length(), snap);
                        let mut v = DVec3::ONE;
                        v[a as usize] = f;
                        v[b as usize] = f;
                        DragDelta::Scale(v)
                    }
                    _ => DragDelta::Translate(
                        snap.map_or(hit - hit0, |inc| snap_grid(hit - hit0, inc)),
                    ),
                }
            }),
            Start::Screen { x0 } => Some(DragDelta::Scale(DVec3::splat(snap1(
                1.0 + (x - x0) / self.gizmo.size_px,
                snap,
            )))),
        };
        if let Some(d) = next {
            self.last = d;
        }
        self.last
    }
}
