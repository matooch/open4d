//! Rig display: joint and bone draw lists and the weight colour ramp (docs/specs/VIEWPORT.md
//! section 13). Pure: no GPU, no UI. Option names follow the joint's display options
//! (docs/parity/rigging.md, O34); shapes, sizes and colours are our own design.

use glam::{DMat4, DVec3};

use crate::camera::Camera;
use crate::curves::{DrawLine, square};
use crate::render::Palette;

/// Default joint colour: the Color row of the O34 joint table (0.4235, 0.8980, 0.5098).
pub const JOINT_COLOR: [f32; 3] = [0.4235, 0.8980, 0.5098];
/// Side of a weight marker, in pixels.
pub const WEIGHT_PX: f64 = 5.0;
/// Radius of a bone's waist and of a joint marker as a fraction of the bone length.
pub const BONE_RADIUS: f64 = 0.1;
/// Radius of the stick style as a fraction of the bone length.
pub const STICK_RADIUS: f64 = 0.04;
/// Joint marker size as a fraction of the bone length.
pub const JOINT_SIZE: f64 = 0.15;
/// Segments of a drawn circle.
pub const CIRCLE_SEGMENTS: usize = 24;

/// How the bone between a joint and its child is drawn (O34 "Display (object)").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoneStyle {
    None,
    /// An octahedron, the waist at 0.1 of the length, radius 0.1 of the length.
    Standard,
    /// A box of cross-section 0.2 of the length along the bone.
    Box,
    /// A single line.
    Line,
    /// A thin octahedron (radius 0.04 of the length).
    Stick,
    /// As `Standard` (the filled faces of the shaded view are the renderer's; the wire is the same).
    Polygon,
}

/// How the joint itself is marked (O34 "Display (joint)").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JointStyle {
    None,
    /// Three short lines along the joint's axes (red, green, blue).
    Axis,
    /// Three great circles.
    SphereWire,
    /// One circle facing the camera.
    Circle,
    /// As `SphereWire` (the filled sphere of the shaded view is the renderer's).
    Sphere,
}

/// One joint as the viewport draws it, world space.
#[derive(Debug, Clone, PartialEq)]
pub struct JointDraw {
    /// The joint's world matrix.
    pub world: DMat4,
    /// World position of the first child joint; `None` draws the bone along the joint's z axis
    /// with `length`.
    pub end: Option<DVec3>,
    /// Bone length when there is no child.
    pub length: f64,
    pub selected: bool,
}

impl JointDraw {
    /// The joint position.
    pub fn start(&self) -> DVec3 {
        self.world.w_axis.truncate()
    }

    /// The far end of the bone.
    pub fn bone_end(&self) -> DVec3 {
        self.end
            .unwrap_or_else(|| self.world.transform_point3(DVec3::Z * self.length))
    }
}

fn octahedron_edges(a: DVec3, b: DVec3, radius: f64, out: &mut Vec<[DVec3; 2]>) {
    let axis = b - a;
    let len = axis.length();
    if len < 1e-12 {
        return;
    }
    let dir = axis / len;
    let helper = if dir.x.abs() < 0.9 {
        DVec3::X
    } else {
        DVec3::Y
    };
    let u = dir.cross(helper).normalize();
    let v = dir.cross(u);
    let c = a + dir * (BONE_RADIUS * len);
    let r = radius * len;
    let ring = [c + u * r, c + v * r, c - u * r, c - v * r];
    for i in 0..4 {
        out.push([ring[i], ring[(i + 1) % 4]]);
        out.push([a, ring[i]]);
        out.push([b, ring[i]]);
    }
}

fn box_edges(a: DVec3, b: DVec3, out: &mut Vec<[DVec3; 2]>) {
    let axis = b - a;
    let len = axis.length();
    if len < 1e-12 {
        return;
    }
    let dir = axis / len;
    let helper = if dir.x.abs() < 0.9 {
        DVec3::X
    } else {
        DVec3::Y
    };
    let u = dir.cross(helper).normalize() * (BONE_RADIUS * len);
    let v = dir.cross(u).normalize() * (BONE_RADIUS * len);
    let corner = |end: DVec3, su: f64, sv: f64| end + u * su + v * sv;
    let signs = [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)];
    for i in 0..4 {
        let (s0, t0) = signs[i];
        let (s1, t1) = signs[(i + 1) % 4];
        out.push([corner(a, s0, t0), corner(a, s1, t1)]);
        out.push([corner(b, s0, t0), corner(b, s1, t1)]);
        out.push([corner(a, s0, t0), corner(b, s0, t0)]);
    }
}

/// The wire segments of a bone in `style`.
pub fn bone_edges(a: DVec3, b: DVec3, style: BoneStyle) -> Vec<[DVec3; 2]> {
    let mut out = Vec::new();
    match style {
        BoneStyle::None => {}
        BoneStyle::Line => out.push([a, b]),
        BoneStyle::Standard | BoneStyle::Polygon => octahedron_edges(a, b, BONE_RADIUS, &mut out),
        BoneStyle::Stick => octahedron_edges(a, b, STICK_RADIUS, &mut out),
        BoneStyle::Box => box_edges(a, b, &mut out),
    }
    out
}

fn circle(out: &mut Vec<DrawLine>, centre: DVec3, u: DVec3, v: DVec3, r: f64, color: [f32; 3]) {
    let at = |k: usize| {
        let t = std::f64::consts::TAU * k as f64 / CIRCLE_SEGMENTS as f64;
        centre + u * (r * t.cos()) + v * (r * t.sin())
    };
    for k in 0..CIRCLE_SEGMENTS {
        out.push(DrawLine {
            a: at(k),
            b: at(k + 1),
            color,
        });
    }
}

/// All lines of one joint: its bone, then its marker. A selected joint uses the selection
/// colour for both, others [`JOINT_COLOR`]; the axis marker keeps its axis colours. `cam` is
/// needed only for [`JointStyle::Circle`].
pub fn joint_lines(
    d: &JointDraw,
    bone: BoneStyle,
    joint: JointStyle,
    pal: &Palette,
    cam: &Camera,
) -> Vec<DrawLine> {
    let color = if d.selected {
        pal.selection
    } else {
        JOINT_COLOR
    };
    let (a, b) = (d.start(), d.bone_end());
    let len = (b - a).length();
    let mut out: Vec<DrawLine> = bone_edges(a, b, bone)
        .into_iter()
        .map(|e| DrawLine {
            a: e[0],
            b: e[1],
            color,
        })
        .collect();
    let size = len * JOINT_SIZE;
    let axes = [
        d.world.transform_vector3(DVec3::X).normalize_or_zero(),
        d.world.transform_vector3(DVec3::Y).normalize_or_zero(),
        d.world.transform_vector3(DVec3::Z).normalize_or_zero(),
    ];
    match joint {
        JointStyle::None => {}
        JointStyle::Axis => {
            let colors = [pal.gizmo_x, pal.gizmo_y, pal.gizmo_z];
            for (axis, c) in axes.iter().zip(colors) {
                out.push(DrawLine {
                    a,
                    b: a + *axis * size,
                    color: c,
                });
            }
        }
        JointStyle::SphereWire | JointStyle::Sphere => {
            circle(&mut out, a, axes[0], axes[1], size, color);
            circle(&mut out, a, axes[1], axes[2], size, color);
            circle(&mut out, a, axes[2], axes[0], size, color);
        }
        JointStyle::Circle => {
            circle(&mut out, a, cam.right_world(), cam.up_world(), size, color);
        }
    }
    out
}

/// The weight colour ramp: 0 blue, 0.25 cyan, 0.5 green, 0.75 yellow, 1 red, linear between;
/// values outside 0..=1 clamp.
pub fn weight_color(w: f64) -> [f32; 3] {
    const STOPS: [[f32; 3]; 5] = [
        [0.0, 0.0, 1.0],
        [0.0, 1.0, 1.0],
        [0.0, 1.0, 0.0],
        [1.0, 1.0, 0.0],
        [1.0, 0.0, 0.0],
    ];
    let t = (w.clamp(0.0, 1.0) * 4.0) as f32;
    let i = (t.floor() as usize).min(3);
    let f = t - i as f32;
    std::array::from_fn(|c| STOPS[i][c] + (STOPS[i + 1][c] - STOPS[i][c]) * f)
}

/// One colour per weight ([`weight_color`]).
pub fn weight_colors(weights: &[f64]) -> Vec<[f32; 3]> {
    weights.iter().map(|w| weight_color(*w)).collect()
}

/// The weight overlay of one joint: a [`WEIGHT_PX`] square at every vertex, coloured by its
/// weight. `points` are world positions, `weights` the joint's weight per point (a missing
/// weight is 0). With `hide_zero` vertices of weight 0 get no square.
pub fn weight_overlay(
    points: &[DVec3],
    weights: &[f64],
    hide_zero: bool,
    cam: &Camera,
    size: [f64; 2],
) -> Vec<DrawLine> {
    let mut out = Vec::new();
    for (i, p) in points.iter().enumerate() {
        let w = weights.get(i).copied().unwrap_or(0.0);
        if hide_zero && w <= 0.0 {
            continue;
        }
        square(&mut out, cam, size, *p, WEIGHT_PX, weight_color(w));
    }
    out
}
