//! The cloner's settings as plain data, read from node parameters
//! (clone-toolset spec C2).

use glam::DVec3;

/// Distribution mode of the cloner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Object,
    Linear,
    Radial,
    Grid,
    Honeycomb,
}

/// How clones pick their prototype (brief 067 implements all but Iterate).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClonesMode {
    Iterate,
    Random,
    Blend,
    Sort,
}

/// How the renderer and viewport consume clones (brief 079); stored only.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstanceMode {
    Instance,
    RenderInstance,
    MultiInstance,
}

/// The Transform tab shared by all modes (clone-toolset spec C3.1).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TransformTab {
    pub position: DVec3,
    /// Heading, pitch, bank in radians.
    pub rotation: DVec3,
    pub scale: DVec3,
    pub color: [f64; 3],
    /// Fraction (1.0 = 100 %).
    pub weight: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinearMode {
    EndPoint,
    PerStep,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepMode {
    Single,
    Cumulative,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Linear {
    pub count: usize,
    pub offset: f64,
    pub mode: LinearMode,
    /// Fraction.
    pub amount: f64,
    pub position: DVec3,
    /// Per-axis scale as a fraction (1 = unchanged).
    pub scale: DVec3,
    /// Radians (h, p, b).
    pub rotation: DVec3,
    pub step_mode: StepMode,
    /// Fraction.
    pub step_size: f64,
    /// Radians (h, p, b).
    pub step_rotation: DVec3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Plane {
    Xy,
    Zy,
    Xz,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Radial {
    pub count: usize,
    pub radius: f64,
    pub plane: Plane,
    pub align: bool,
    pub start_angle: f64,
    pub end_angle: f64,
    pub offset: f64,
    /// Fraction.
    pub offset_variation: f64,
    pub offset_seed: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GridMode {
    PerStep,
    Endpoint,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GridForm {
    Cubic,
    Sphere,
    Cylinder,
    Object,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Grid {
    pub count: [usize; 3],
    pub mode: GridMode,
    pub size: DVec3,
    pub form: GridForm,
    /// Fraction.
    pub fill: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Orientation {
    /// Z (XY)
    Z,
    /// X (ZY)
    X,
    /// Y (XZ)
    Y,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OffsetDirection {
    Width,
    Height,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HoneycombMode {
    PerStep,
    EndPoint,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HoneycombForm {
    Circle,
    Square,
    Spline,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Honeycomb {
    pub orientation: Orientation,
    pub offset_direction: OffsetDirection,
    /// Fraction.
    pub offset: f64,
    /// Fractions.
    pub offset_variation: f64,
    pub perpendicular_variation: f64,
    pub seed: u64,
    pub count_width: usize,
    pub count_height: usize,
    pub mode: HoneycombMode,
    pub size_width: f64,
    pub size_height: f64,
    pub form: HoneycombForm,
}

/// Everything the clone generation reads.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Settings {
    pub mode: Mode,
    pub clones_mode: ClonesMode,
    pub reset_coordinates: bool,
    pub instance_mode: InstanceMode,
    pub transform: TransformTab,
    pub linear: Linear,
    pub radial: Radial,
    pub grid: Grid,
    pub honeycomb: Honeycomb,
}
