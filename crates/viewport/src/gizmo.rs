//! Translate / rotate / scale gizmo in a world or object frame
//! (docs/specs/VIEWPORT.md §5, docs/specs/MANIPULATE.md §2).

use std::f64::consts::TAU;

use glam::{DVec2, DVec3};

use crate::camera::{Camera, Projection};
use crate::snap::{line_param_closest_to_ray, ray_plane, snap_scalar};

const CENTER_PX: f64 = 8.0;
const AXIS_TOL_PX: f64 = 6.0;
const PLANE_LO: f64 = 0.2;
const PLANE_HI: f64 = 0.45;
/// Handles seen (nearly) edge-on are not pickable.
const MIN_PLANE_AREA_PX: f64 = 25.0;
const MIN_AXIS_LEN_PX: f64 = 6.0;
/// Polyline resolution of rotate rings.
pub const RING_SEGMENTS: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GizmoMode {
    Translate,
    Rotate,
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

    pub fn index(self) -> usize {
        self as usize
    }

    /// The world axis of the same name.
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

    /// The axis perpendicular to the plane.
    pub fn normal_axis(self) -> Axis {
        match self {
            Plane::XY => Axis::Z,
            Plane::YZ => Axis::X,
            Plane::XZ => Axis::Y,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Handle {
    Axis(Axis),
    Plane(Plane),
    Center,
}

/// Which frame the gizmo's axes follow (chosen by the editor).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Space {
    #[default]
    Object,
    World,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Gizmo {
    pub origin: DVec3,
    pub mode: GizmoMode,
    pub size_px: f64,
    /// Orthonormal frame: the gizmo's X, Y and Z handles point along these.
    pub axes: [DVec3; 3],
}

impl Default for Gizmo {
    fn default() -> Self {
        Gizmo {
            origin: DVec3::ZERO,
            mode: GizmoMode::Translate,
            size_px: 100.0,
            axes: [DVec3::X, DVec3::Y, DVec3::Z],
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
    pub fn axis(&self, a: Axis) -> DVec3 {
        self.axes[a.index()]
    }

    /// World length of an axis handle (and the rotate ring radius).
    pub fn world_length(&self, cam: &Camera, viewport_h: f64) -> f64 {
        self.size_px * world_per_pixel_at(cam, self.origin, viewport_h)
    }

    /// World-space corners of a plane handle.
    pub fn plane_corners(&self, plane: Plane, l: f64) -> [DVec3; 4] {
        let (a, b) = plane.axes();
        let (a, b) = (self.axis(a) * l, self.axis(b) * l);
        let o = self.origin;
        [
            o + a * PLANE_LO + b * PLANE_LO,
            o + a * PLANE_HI + b * PLANE_LO,
            o + a * PLANE_HI + b * PLANE_HI,
            o + a * PLANE_LO + b * PLANE_HI,
        ]
    }

    /// `RING_SEGMENTS` world points of the rotate ring around `a` (radius `l`).
    pub fn ring_points(&self, a: Axis, l: f64) -> Vec<DVec3> {
        let i = a.index();
        let (u, v) = (self.axes[(i + 1) % 3], self.axes[(i + 2) % 3]);
        (0..RING_SEGMENTS)
            .map(|k| {
                let t = TAU * k as f64 / RING_SEGMENTS as f64;
                self.origin + (u * t.cos() + v * t.sin()) * l
            })
            .collect()
    }

    pub fn hit_test(&self, cam: &Camera, x: f64, y: f64, w: f64, h: f64) -> Option<Handle> {
        let cursor = DVec2::new(x, y);
        let screen = |p: DVec3| cam.project(p, w, h).map(|s| s.truncate());
        let o = screen(self.origin)?;
        let l = self.world_length(cam, h);
        if self.mode == GizmoMode::Rotate {
            return Axis::ALL
                .iter()
                .filter_map(|&a| {
                    let pts: Option<Vec<DVec2>> =
                        self.ring_points(a, l).into_iter().map(screen).collect();
                    let pts = pts?;
                    let d = (0..pts.len())
                        .map(|k| seg_distance(cursor, pts[k], pts[(k + 1) % pts.len()]))
                        .fold(f64::INFINITY, f64::min);
                    (d <= AXIS_TOL_PX).then_some((d, a))
                })
                .min_by(|a, b| a.0.total_cmp(&b.0))
                .map(|(_, a)| Handle::Axis(a));
        }
        if (cursor - o).length() <= CENTER_PX {
            return Some(Handle::Center);
        }
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
                let end = screen(self.origin + self.axis(axis) * l)?;
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
    /// World-space offset.
    Translate(DVec3),
    /// Right-handed rotation about a world-space unit axis.
    Rotate { axis: DVec3, angle: f64 },
    /// Factors along the gizmo axes.
    Scale(DVec3),
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Start {
    Line {
        axis: Axis,
        s0: f64,
    },
    Plane {
        normal: DVec3,
        axes: Option<(Axis, Axis)>,
        hit0: DVec3,
    },
    Ring {
        normal: DVec3,
        s: DVec3,
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
        let start = match (gizmo.mode, handle) {
            (GizmoMode::Rotate, Handle::Axis(a)) => {
                let n = gizmo.axis(a);
                let s = ray.at(ray_plane(&ray, o, n)?) - o;
                if s.length() < 1e-9 {
                    return None;
                }
                Start::Ring { normal: n, s }
            }
            (GizmoMode::Rotate, _) => return None,
            (mode, Handle::Axis(axis)) => {
                let s0 = line_param_closest_to_ray(o, gizmo.axis(axis), &ray)?;
                if mode == GizmoMode::Scale && s0.abs() < 1e-9 {
                    return None;
                }
                Start::Line { axis, s0 }
            }
            (mode, Handle::Plane(plane)) => {
                let normal = gizmo.axis(plane.normal_axis());
                let hit0 = ray.at(ray_plane(&ray, o, normal)?);
                if mode == GizmoMode::Scale && (hit0 - o).length() < 1e-9 {
                    return None;
                }
                Start::Plane {
                    normal,
                    axes: Some(plane.axes()),
                    hit0,
                }
            }
            (GizmoMode::Scale, Handle::Center) => Start::Screen { x0: x },
            (_, Handle::Center) => {
                let normal = cam.forward_world();
                let hit0 = ray.at(ray_plane(&ray, o, normal)?);
                Start::Plane {
                    normal,
                    axes: None,
                    hit0,
                }
            }
        };
        let last = match gizmo.mode {
            GizmoMode::Translate => DragDelta::Translate(DVec3::ZERO),
            GizmoMode::Rotate => DragDelta::Rotate {
                axis: gizmo.axes[0],
                angle: 0.0,
            },
            GizmoMode::Scale => DragDelta::Scale(DVec3::ONE),
        };
        let last = match start {
            Start::Ring { normal, .. } => DragDelta::Rotate {
                axis: normal,
                angle: 0.0,
            },
            _ => last,
        };
        Some(Drag {
            gizmo: *gizmo,
            start,
            last,
        })
    }

    /// `snap`: units for translate, radians for rotate, factor steps for scale.
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
        let g = self.gizmo;
        let o = g.origin;
        let scale = g.mode == GizmoMode::Scale;
        let next = match self.start {
            Start::Ring { normal, s } => ray_plane(&ray, o, normal).map(|t| {
                let c = ray.at(t) - o;
                let angle = s.cross(c).dot(normal).atan2(s.dot(c));
                DragDelta::Rotate {
                    axis: normal,
                    angle: snap1(angle, snap),
                }
            }),
            Start::Line { axis, s0 } => line_param_closest_to_ray(o, g.axis(axis), &ray).map(|s| {
                if scale {
                    let mut v = DVec3::ONE;
                    v[axis.index()] = snap1(s / s0, snap);
                    DragDelta::Scale(v)
                } else {
                    DragDelta::Translate(g.axis(axis) * snap1(s - s0, snap))
                }
            }),
            Start::Plane { normal, axes, hit0 } => ray_plane(&ray, o, normal).map(|t| {
                let hit = ray.at(t);
                match (scale, axes) {
                    (true, Some((a, b))) => {
                        let f = snap1((hit - o).length() / (hit0 - o).length(), snap);
                        let mut v = DVec3::ONE;
                        v[a.index()] = f;
                        v[b.index()] = f;
                        DragDelta::Scale(v)
                    }
                    (_, Some((a, b))) => {
                        let d = hit - hit0;
                        let along = |ax: Axis| g.axis(ax) * snap1(d.dot(g.axis(ax)), snap);
                        DragDelta::Translate(along(a) + along(b))
                    }
                    (_, None) => {
                        let d = hit - hit0;
                        DragDelta::Translate(snap.map_or(d, |inc| d.map(|c| snap_scalar(c, inc))))
                    }
                }
            }),
            Start::Screen { x0 } => Some(DragDelta::Scale(DVec3::splat(snap1(
                1.0 + (x - x0) / g.size_px,
                snap,
            )))),
        };
        if let Some(d) = next {
            self.last = d;
        }
        self.last
    }
}
