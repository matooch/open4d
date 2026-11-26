//! The Null's viewport display (docs/specs/SCENE-OBJECTS.md S9, brief 046, observation O20b):
//! the drawn geometry of each display shape as pure functions of Radius, Aspect Ratio and
//! Orientation, and the reader of a Null node. The Null has no geometry; nothing here reaches
//! a render or an export.

use std::f64::consts::{FRAC_PI_2, TAU};

use forge_core::{NULL_SHAPES, NodeId, Scene, Value, layers};
use glam::{DMat4, DVec3};

/// Type id of the Null.
pub const TYPE: &str = "forge.null";

/// Segments of every drawn circle (own design, parity-pending O20b: S9.3).
pub const CIRCLE_SEGMENTS: usize = 64;
/// Inner radius of the star as a fraction of the radius (own design, parity-pending O20b).
pub const STAR_INNER: f64 = 0.5;

/// One line segment.
pub type Segment = [DVec3; 2];

/// What a Null draws.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NullShape {
    None,
    Dot,
    Locator,
    Circle,
    Rectangle,
    Diamond,
    Triangle,
    Pentagon,
    Hexagon,
    Octagon,
    Star,
    Axis,
    Cube,
    Pyramid,
    Sphere,
}

impl NullShape {
    /// Every shape, in the order of `forge_core::NULL_SHAPES`.
    pub const ALL: [NullShape; 15] = [
        NullShape::None,
        NullShape::Dot,
        NullShape::Locator,
        NullShape::Circle,
        NullShape::Rectangle,
        NullShape::Diamond,
        NullShape::Triangle,
        NullShape::Pentagon,
        NullShape::Hexagon,
        NullShape::Octagon,
        NullShape::Star,
        NullShape::Axis,
        NullShape::Cube,
        NullShape::Pyramid,
        NullShape::Sphere,
    ];

    /// The param option id.
    pub fn id(self) -> &'static str {
        NULL_SHAPES[self as usize].0
    }

    pub fn from_id(id: &str) -> Option<NullShape> {
        NULL_SHAPES
            .iter()
            .position(|(i, _)| *i == id)
            .map(|k| NullShape::ALL[k])
    }

    /// Cube, pyramid and sphere: drawn in three dimensions, ignoring Aspect Ratio.
    pub fn is_solid(self) -> bool {
        matches!(
            self,
            NullShape::Cube | NullShape::Pyramid | NullShape::Sphere
        )
    }
}

/// The plane a shape lies in (S9.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NullOrientation {
    Camera,
    X,
    Y,
    Z,
}

impl NullOrientation {
    pub fn from_id(id: &str) -> Option<NullOrientation> {
        match id {
            "camera" => Some(NullOrientation::Camera),
            "x" => Some(NullOrientation::X),
            "y" => Some(NullOrientation::Y),
            "z" => Some(NullOrientation::Z),
            _ => None,
        }
    }
}

/// The viewport camera's axes in the space the lines are wanted in.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ViewFrame {
    pub right: DVec3,
    pub up: DVec3,
    /// Points from the scene toward the viewer.
    pub toward: DVec3,
}

impl ViewFrame {
    /// Looking along +Z with X to the right and Y up (the Front view).
    pub const FRONT: ViewFrame = ViewFrame {
        right: DVec3::X,
        up: DVec3::Y,
        toward: DVec3::NEG_Z,
    };
}

/// True iff the drawing is placed by the viewport camera (a flat shape with Camera orientation):
/// the caller then draws it at the Null's world position without the Null's rotation and scale.
/// Every other drawing is in the Null's local frame.
pub fn follows_view(shape: NullShape, orientation: NullOrientation) -> bool {
    orientation == NullOrientation::Camera && !shape.is_solid()
}

/// The frame (u, v, n) of S9.2.
fn frame(shape: NullShape, orientation: NullOrientation, view: &ViewFrame) -> [DVec3; 3] {
    match orientation {
        NullOrientation::Camera if !shape.is_solid() => [view.right, view.up, view.toward],
        NullOrientation::Camera | NullOrientation::Z => [DVec3::X, DVec3::Y, DVec3::Z],
        NullOrientation::X => [DVec3::Z, DVec3::Y, DVec3::X],
        NullOrientation::Y => [DVec3::X, DVec3::Z, DVec3::Y],
    }
}

fn closed(points: &[DVec3]) -> Vec<Segment> {
    (0..points.len())
        .map(|i| [points[i], points[(i + 1) % points.len()]])
        .collect()
}

/// `n` vertices on the circle of radius `r` starting at `start` radians, counter-clockwise.
fn ring(n: usize, r: f64, start: f64, u: DVec3, v: DVec3) -> Vec<DVec3> {
    (0..n)
        .map(|k| {
            let a = start + TAU * k as f64 / n as f64;
            u * (r * a.cos()) + v * (r * a.sin())
        })
        .collect()
}

/// The line segments of a shape, centred on the origin (S9.3). `aspect` scales the vertical
/// axis of the flat shapes only; the solids ignore it. A closed outline is a chain whose last
/// segment ends where the first starts. The Dot has no lines (see [`null_display_points`]).
pub fn null_display_lines(
    shape: NullShape,
    radius: f64,
    aspect: f64,
    orientation: NullOrientation,
    view: &ViewFrame,
) -> Vec<Segment> {
    let [u, v, n] = frame(shape, orientation, view);
    let r = radius;
    // flat shapes: the vertical axis carries the aspect
    let va = v * aspect;
    let polygon =
        |sides: usize, start_deg: f64| closed(&ring(sides, r, start_deg.to_radians(), u, va));
    match shape {
        NullShape::None | NullShape::Dot => Vec::new(),
        NullShape::Locator => vec![[-u * r, u * r], [-va * r, va * r]],
        NullShape::Circle => polygon(CIRCLE_SEGMENTS, 0.0),
        NullShape::Rectangle => polygon(4, 45.0),
        NullShape::Diamond => polygon(4, 0.0),
        NullShape::Triangle => polygon(3, 90.0),
        NullShape::Pentagon => polygon(5, 90.0),
        NullShape::Hexagon => polygon(6, 0.0),
        NullShape::Octagon => polygon(8, 22.5),
        NullShape::Star => {
            let pts: Vec<DVec3> = (0..16)
                .map(|k| {
                    let a = FRAC_PI_2 + TAU * k as f64 / 16.0;
                    let rr = if k % 2 == 0 { r } else { r * STAR_INNER };
                    u * (rr * a.cos()) + va * (rr * a.sin())
                })
                .collect();
            closed(&pts)
        }
        NullShape::Axis => vec![[DVec3::ZERO, u * r], [DVec3::ZERO, va * r]],
        NullShape::Cube => {
            let corner = |i: usize| {
                let s = |bit: usize| if i >> bit & 1 == 1 { r } else { -r };
                u * s(0) + v * s(1) + n * s(2)
            };
            let mut out = Vec::with_capacity(12);
            for i in 0..8usize {
                for bit in 0..3 {
                    if i >> bit & 1 == 0 {
                        out.push([corner(i), corner(i | 1 << bit)]);
                    }
                }
            }
            out
        }
        NullShape::Pyramid => {
            let base = [(-r, -r), (r, -r), (r, r), (-r, r)].map(|(a, b)| u * a + v * b - n * r);
            let apex = n * r;
            let mut out = closed(&base);
            out.extend(base.iter().map(|c| [*c, apex]));
            out
        }
        NullShape::Sphere => {
            let mut out = closed(&ring(CIRCLE_SEGMENTS, r, 0.0, u, v));
            out.extend(closed(&ring(CIRCLE_SEGMENTS, r, 0.0, u, n)));
            out.extend(closed(&ring(CIRCLE_SEGMENTS, r, 0.0, v, n)));
            out
        }
    }
}

/// The points a shape draws: the Dot is one point at the origin, every other shape has none.
pub fn null_display_points(shape: NullShape) -> Vec<DVec3> {
    if shape == NullShape::Dot {
        vec![DVec3::ZERO]
    } else {
        Vec::new()
    }
}

/// How a Null picks its colour (S9.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NullColorMode {
    Material,
    Layer,
    Automatic,
    Custom,
}

/// Everything the viewport needs of one Null.
#[derive(Debug, Clone, PartialEq)]
pub struct NullData {
    pub node: NodeId,
    pub shape: NullShape,
    pub radius: f64,
    pub aspect: f64,
    pub orientation: NullOrientation,
    pub color_mode: NullColorMode,
    pub color: [f64; 3],
    /// The object's layer colour, if it has a layer.
    pub layer_color: Option<[f64; 3]>,
    pub xray: bool,
    pub world: DMat4,
}

fn value(scene: &Scene, id: NodeId, key: &str) -> Value {
    let desc = scene
        .param_desc(id, key)
        .expect("the Null declares its params");
    scene
        .param(id, key)
        .and_then(|v| desc.validate(v.clone()).ok())
        .unwrap_or_else(|| desc.default.clone())
}

/// Reads a Null node; a missing param takes its default (old files have none).
pub fn null_data(scene: &Scene, id: NodeId) -> Option<NullData> {
    if scene.node(id)?.type_id != TYPE {
        return None;
    }
    let float = |k: &str| match value(scene, id, k) {
        Value::Float(f) => f,
        _ => 0.0,
    };
    let en = |k: &str| match value(scene, id, k) {
        Value::Enum(s) => s,
        _ => String::new(),
    };
    let color = match value(scene, id, "color") {
        Value::Color(c) => c,
        _ => [1.0; 3],
    };
    let color_mode = match en("display_color").as_str() {
        "layer" => NullColorMode::Layer,
        "automatic" => NullColorMode::Automatic,
        "custom" => NullColorMode::Custom,
        _ => NullColorMode::Material,
    };
    Some(NullData {
        node: id,
        shape: NullShape::from_id(&en("shape")).unwrap_or(NullShape::Dot),
        radius: float("radius"),
        aspect: float("aspect"),
        orientation: NullOrientation::from_id(&en("orientation"))
            .unwrap_or(NullOrientation::Camera),
        color_mode,
        color,
        layer_color: layers::of(scene, id).map(|l| l.color),
        xray: matches!(value(scene, id, "xray"), Value::Bool(true)),
        world: scene.world_matrix(id)?,
    })
}

/// The Nulls of a scene in tree order (callers filter by visibility).
pub fn null_objects(scene: &Scene) -> Vec<NullData> {
    let mut out = Vec::new();
    for r in scene.roots() {
        for id in std::iter::once(*r).chain(scene.descendants(*r)) {
            out.extend(null_data(scene, id));
        }
    }
    out
}
