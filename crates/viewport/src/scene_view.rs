//! Camera and light objects in the viewport (docs/specs/SCENE-OBJECTS.md S7, brief 047):
//! "look through camera" and the line gizmos of cameras, lights, floors and skies. Pure maths
//! on world matrices and lens numbers (the viewport does not depend on the scene crate).
//!
//! A world matrix is camera/light to world, rigid; the object looks along its local +Z.

use glam::{DMat4, DVec3};

use crate::camera::{Camera, Handedness, Projection};
use crate::curves::DrawLine;

/// The lens of a camera object.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Lens {
    /// Focal length and film gate width in mm; the gate spans the image width.
    Perspective { focal_mm: f64, sensor_mm: f64 },
    /// World width that spans the image width.
    Orthographic { width: f64 },
}

/// Horizontal field of view of a perspective lens.
pub fn fov_horizontal(focal_mm: f64, sensor_mm: f64) -> f64 {
    2.0 * (sensor_mm / (2.0 * focal_mm)).atan()
}

/// The viewport camera that sees what a camera object sees ("look through camera"), for a
/// viewport of `aspect` = width / height. The orbit `distance` is only the pivot depth the
/// navigation tools use; the clip planes follow it (`distance * 1e-3` to `distance * 1e4`).
/// Film offset and bank (roll) are not representable by the orbit camera and are dropped.
pub fn look_through(world: DMat4, lens: Lens, aspect: f64, distance: f64) -> Camera {
    let mirror = DVec3::new(1.0, 1.0, -1.0);
    let eye = world.w_axis.truncate() * mirror;
    let fwd = world.transform_vector3(DVec3::Z).normalize() * mirror;
    let distance = distance.max(1e-6);
    let projection = match lens {
        Lens::Perspective {
            focal_mm,
            sensor_mm,
        } => Projection::Perspective {
            fov_y: 2.0 * ((fov_horizontal(focal_mm, sensor_mm) * 0.5).tan() / aspect).atan(),
        },
        Lens::Orthographic { width } => Projection::Orthographic {
            height: width / aspect,
        },
    };
    Camera {
        target: eye + fwd * distance,
        distance,
        // offset_dir = (cos p sin y, sin p, cos p cos y) points from the target to the eye
        yaw: (-fwd.x).atan2(-fwd.z),
        pitch: (-fwd.y).clamp(-1.0, 1.0).asin(),
        projection,
        handedness: Handedness::Left,
    }
}

/// Colour of an unselected camera or light gizmo.
pub const OBJECT_COLOR: [f32; 3] = [0.85, 0.85, 0.85];
/// Colour of a selected one.
pub const SELECTED_COLOR: [f32; 3] = [1.0, 0.65, 0.2];

fn line(a: DVec3, b: DVec3, color: [f32; 3]) -> DrawLine {
    DrawLine { a, b, color }
}

fn closed(points: &[DVec3], color: [f32; 3]) -> Vec<DrawLine> {
    (0..points.len())
        .map(|i| line(points[i], points[(i + 1) % points.len()], color))
        .collect()
}

/// The frustum of a camera object drawn `depth` units deep: four edges from the eye to the
/// corners of the frame at `depth` and the frame itself; an up marker above the frame. For
/// orthographic lenses the frame is a box of the lens width out to `depth`.
pub fn camera_lines(
    world: DMat4,
    lens: Lens,
    aspect: f64,
    depth: f64,
    selected: bool,
) -> Vec<DrawLine> {
    let color = if selected {
        SELECTED_COLOR
    } else {
        OBJECT_COLOR
    };
    let p = |x: f64, y: f64, z: f64| world.transform_point3(DVec3::new(x, y, z));
    let (hw, hh) = match lens {
        Lens::Perspective {
            focal_mm,
            sensor_mm,
        } => {
            let hw = depth * sensor_mm * 0.5 / focal_mm;
            (hw, hw / aspect)
        }
        Lens::Orthographic { width } => (width * 0.5, width * 0.5 / aspect),
    };
    let eye = p(0.0, 0.0, 0.0);
    let corners = [
        p(-hw, -hh, depth),
        p(hw, -hh, depth),
        p(hw, hh, depth),
        p(-hw, hh, depth),
    ];
    let mut out = closed(&corners, color);
    match lens {
        Lens::Perspective { .. } => {
            out.extend(corners.iter().map(|c| line(eye, *c, color)));
        }
        Lens::Orthographic { .. } => {
            let near = [
                p(-hw, -hh, 0.0),
                p(hw, -hh, 0.0),
                p(hw, hh, 0.0),
                p(-hw, hh, 0.0),
            ];
            out.extend(closed(&near, color));
            out.extend((0..4).map(|i| line(near[i], corners[i], color)));
        }
    }
    // the up marker: a triangle above the frame
    let top = p(0.0, hh * 1.35, depth);
    out.push(line(corners[3], top, color));
    out.push(line(corners[2], top, color));
    out
}

/// The shape of a light gizmo.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LightGizmo {
    /// A point: three crossing circles of `radius`.
    Omni { radius: f64 },
    /// A cone of full angle `outer` (radians), `length` long.
    Spot { outer: f64, length: f64 },
    /// Parallel rays: a circle with five lines of `length` along +Z.
    Infinite { radius: f64, length: f64 },
    /// A rectangle `width` x `height` in the local XY plane, with a normal tick toward +Z.
    AreaRect { width: f64, height: f64 },
    /// A disc of `radius` in the local XY plane, with a normal tick toward +Z.
    AreaDisc { radius: f64 },
    /// A sphere of `radius`.
    AreaSphere { radius: f64 },
}

const CIRCLE_SEGMENTS: usize = 24;

fn circle(
    world: DMat4,
    centre_z: f64,
    radius: f64,
    plane: usize,
    color: [f32; 3],
) -> Vec<DrawLine> {
    let pts: Vec<DVec3> = (0..CIRCLE_SEGMENTS)
        .map(|i| {
            let (s, c) = (i as f64 / CIRCLE_SEGMENTS as f64 * std::f64::consts::TAU).sin_cos();
            let v = match plane {
                0 => DVec3::new(c * radius, s * radius, centre_z),
                1 => DVec3::new(c * radius, centre_z, s * radius),
                _ => DVec3::new(centre_z, c * radius, s * radius),
            };
            world.transform_point3(v)
        })
        .collect();
    closed(&pts, color)
}

/// The line gizmo of a light object.
pub fn light_lines(world: DMat4, shape: LightGizmo, selected: bool) -> Vec<DrawLine> {
    let color = if selected {
        SELECTED_COLOR
    } else {
        OBJECT_COLOR
    };
    let p = |v: DVec3| world.transform_point3(v);
    let mut out = Vec::new();
    let normal = |length: f64| {
        let end = p(DVec3::Z * length);
        let barb = length * 0.23;
        [
            line(p(DVec3::ZERO), end, color),
            line(p(DVec3::new(-barb, 0.0, length - barb)), end, color),
            line(p(DVec3::new(barb, 0.0, length - barb)), end, color),
        ]
    };
    match shape {
        LightGizmo::Omni { radius } => {
            for plane in 0..3 {
                out.extend(circle(world, 0.0, radius, plane, color));
            }
        }
        LightGizmo::Spot { outer, length } => {
            let r = length * (outer * 0.5).tan();
            out.extend(circle(world, length, r, 0, color));
            for (x, y) in [(r, 0.0), (-r, 0.0), (0.0, r), (0.0, -r)] {
                out.push(line(p(DVec3::ZERO), p(DVec3::new(x, y, length)), color));
            }
        }
        LightGizmo::Infinite { radius, length } => {
            out.extend(circle(world, 0.0, radius, 0, color));
            out.push(line(p(DVec3::ZERO), p(DVec3::new(0.0, 0.0, length)), color));
            for k in 0..4 {
                let a = k as f64 * std::f64::consts::FRAC_PI_2;
                let o = DVec3::new(a.cos() * radius, a.sin() * radius, 0.0);
                out.push(line(p(o), p(o + DVec3::new(0.0, 0.0, length)), color));
            }
        }
        LightGizmo::AreaRect { width, height } => {
            let (hw, hh) = (width * 0.5, height * 0.5);
            let c = [
                p(DVec3::new(-hw, -hh, 0.0)),
                p(DVec3::new(hw, -hh, 0.0)),
                p(DVec3::new(hw, hh, 0.0)),
                p(DVec3::new(-hw, hh, 0.0)),
            ];
            out.extend(closed(&c, color));
            let tick = hw.min(hh).max(1e-6);
            out.extend(normal(tick));
        }
        LightGizmo::AreaDisc { radius } => {
            out.extend(circle(world, 0.0, radius, 0, color));
            out.extend(normal(radius));
        }
        LightGizmo::AreaSphere { radius } => {
            for plane in 0..3 {
                out.extend(circle(world, 0.0, radius, plane, color));
            }
        }
    }
    out
}

/// A square grid of `cells` x `cells` lines of the floor plane (local XZ, +Y up) out to
/// `half` units, clipped to a finite size so the gizmo stays readable for an endless floor.
pub fn floor_lines(world: DMat4, half: f64, cells: usize, color: [f32; 3]) -> Vec<DrawLine> {
    let cells = cells.max(1);
    let step = 2.0 * half / cells as f64;
    let p = |x: f64, z: f64| world.transform_point3(DVec3::new(x, 0.0, z));
    let mut out = Vec::new();
    for i in 0..=cells {
        let t = -half + i as f64 * step;
        out.push(line(p(t, -half), p(t, half), color));
        out.push(line(p(-half, t), p(half, t), color));
    }
    out
}
