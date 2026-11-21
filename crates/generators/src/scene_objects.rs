//! Camera, Light, Environment, Floor and Sky objects (docs/specs/SCENE-OBJECTS.md, brief 047).
//!
//! These nodes have no geometry. Their parameter descriptors follow the observation O29
//! (`docs/parity/scene-defaults.md` sections 2 and 3) for cameras and lights; environment,
//! floor and sky are our own design (O29 observed no such objects). The `*_data` functions
//! turn a node of a scene into plain data that the renderers and the viewport consume.

use std::collections::BTreeMap;

use forge_core::{Category, NodeId, NodeType, ParamDesc, Scene, Unit, Value, transform};
use glam::{DMat3, DMat4};

pub const CAMERA: &str = "forge.camera";
pub const LIGHT: &str = "forge.light";
pub const ENVIRONMENT: &str = "forge.environment";
pub const FLOOR: &str = "forge.floor";
pub const SKY: &str = "forge.sky";

/// Every scene object type id, in Create menu order.
pub const TYPES: [&str; 5] = [CAMERA, LIGHT, ENVIRONMENT, FLOOR, SKY];

/// Film aspect of a new document (1280 x 720), used for the vertical field of view readback.
pub const DEFAULT_FILM_ASPECT: f64 = 16.0 / 9.0;

fn deg(d: f64) -> f64 {
    d.to_radians()
}

fn dist(id: &str, label: &str, default: f64, group: &str) -> ParamDesc {
    ParamDesc::float(id, label, default)
        .range(0.0, 999_999_999.0)
        .unit(Unit::Distance)
        .group(group)
}

fn pct(id: &str, label: &str, default: f64, max_percent: f64, group: &str) -> ParamDesc {
    ParamDesc::float(id, label, default)
        .range(0.0, max_percent / 100.0)
        .unit(Unit::Percent)
        .group(group)
}

fn angle(
    id: &str,
    label: &str,
    default_deg: f64,
    min_deg: f64,
    max_deg: f64,
    g: &str,
) -> ParamDesc {
    ParamDesc::float(id, label, deg(default_deg))
        .range(deg(min_deg), deg(max_deg))
        .unit(Unit::Angle)
        .group(g)
}

fn flag(id: &str, label: &str, default: bool, group: &str) -> ParamDesc {
    ParamDesc::bool(id, label, default).group(group)
}

const PROJECTIONS: [(&str, &str); 14] = [
    ("perspective", "Perspective"),
    ("parallel", "Parallel"),
    ("left", "Left"),
    ("right", "Right"),
    ("front", "Front"),
    ("back", "Back"),
    ("top", "Top"),
    ("bottom", "Bottom"),
    ("military", "Military"),
    ("frog", "Frog"),
    ("bird", "Bird"),
    ("gentleman", "Gentleman"),
    ("isometric", "Isometric"),
    ("dimetric", "Dimetric"),
];

/// `forge.camera` (O29 section 2: the Object, Physical and Details tabs; the Stereoscopic,
/// Composition Helpers and Spherical Camera tabs are not parameters yet).
pub fn camera_type() -> NodeType {
    let (o, p, d) = ("Object", "Physical", "Details");
    NodeType::new(CAMERA, "Camera", Category::Object)
        .param(
            ParamDesc::enumeration("projection", "Projection", "perspective", &PROJECTIONS)
                .group(o),
        )
        .param(
            ParamDesc::float("sensor_size", "Sensor Size (Film Gate)", 36.0)
                .range(1.0, 2000.0)
                .group(o),
        )
        // the focal length has no field of its own: the horizontal FOV drives it (O57)
        .param(angle(
            "fov_horizontal",
            "Field of View (Horizontal)",
            2.0 * 0.5f64.atan().to_degrees(),
            0.2,
            174.0,
            o,
        ))
        .param(
            ParamDesc::float("zoom", "Zoom", 1.0)
                .range(0.0001, 1_000_000.0)
                .group(o),
        )
        .param(
            ParamDesc::float("film_offset_x", "Film Offset X", 0.0)
                .range(-999_999_999.0, 999_999_999.0)
                .unit(Unit::Percent)
                .group(o),
        )
        .param(
            ParamDesc::float("film_offset_y", "Film Offset Y", 0.0)
                .range(-999_999_999.0, 999_999_999.0)
                .unit(Unit::Percent)
                .group(o),
        )
        .param(dist("focus_distance", "Focus Distance", 2000.0, o))
        .param(flag("use_target_object", "Use Target Object", false, o))
        .param(ParamDesc::link("focus_object", "Focus Object", &[Category::Object]).group(o))
        .param(
            ParamDesc::float("white_balance_k", "White Balance (K)", 6500.0)
                .range(1000.0, 10000.0)
                .group(o),
        )
        .param(flag("affect_lights_only", "Affect Lights Only", false, o))
        .param(flag(
            "export_to_compositing",
            "Export to Compositing",
            true,
            o,
        ))
        .param(flag("movie_camera", "Movie Camera", false, p))
        .param(
            ParamDesc::float("f_stop", "F-Stop (f/#)", 8.0)
                .range(0.01, 256.0)
                .group(p),
        )
        .param(flag("exposure", "Exposure", false, p))
        .param(
            ParamDesc::float("iso", "ISO", 200.0)
                .range(10.0, 10000.0)
                .group(p),
        )
        .param(
            ParamDesc::float("gain_db", "Gain (dB)", 0.0)
                .range(-100.0, 100.0)
                .group(p),
        )
        .param(
            ParamDesc::float("shutter_speed", "Shutter Speed (s)", 1.0 / 30.0)
                .range(0.0001, 100.0)
                .group(p),
        )
        .param(angle(
            "shutter_angle",
            "Shutter Angle",
            180.0,
            10.0,
            1080.0,
            p,
        ))
        .param(angle(
            "shutter_offset",
            "Shutter Offset",
            0.0,
            0.0,
            360.0,
            p,
        ))
        .param(pct(
            "shutter_efficiency",
            "Shutter Efficiency",
            0.7,
            100.0,
            p,
        ))
        .param(
            ParamDesc::float(
                "lens_distortion_quadratic",
                "Lens Distortion - Quadratic",
                0.0,
            )
            .range(-1.0, 1.0)
            .unit(Unit::Percent)
            .group(p),
        )
        .param(
            ParamDesc::float("lens_distortion_cubic", "Lens Distortion - Cubic", 0.0)
                .range(-1.0, 1.0)
                .unit(Unit::Percent)
                .group(p),
        )
        .param(pct(
            "vignetting_intensity",
            "Vignetting Intensity",
            0.0,
            500.0,
            p,
        ))
        .param(pct("vignetting_offset", "Vignetting Offset", 0.0, 100.0, p))
        .param(
            ParamDesc::float("chromatic_aberration", "Chromatic Aberration", 0.0)
                .range(-10.0, 10.0)
                .unit(Unit::Percent)
                .group(p),
        )
        .param(flag("diaphragm_shape", "Diaphragm Shape", false, p))
        .param(ParamDesc::int("blades", "Blades", 6).range(3, 32).group(p))
        .param(angle("diaphragm_angle", "Angle", 0.0, 0.0, 3600.0, p))
        .param(
            ParamDesc::float("bias", "Bias", 0.0)
                .range(-1.0, 1.0)
                .group(p),
        )
        .param(
            ParamDesc::float("anisotropy", "Anisotropy", 0.0)
                .range(-1.0, 1.0)
                .unit(Unit::Percent)
                .group(p),
        )
        .param(flag(
            "enable_near_clipping",
            "Enable Near Clipping",
            true,
            d,
        ))
        .param(dist("near_clipping", "Near Clipping", 0.0, d))
        .param(flag("enable_far_clipping", "Enable Far Clipping", false, d))
        .param(dist("far_clipping", "Far Clipping", 10000.0, d))
        .param(flag("show_cone", "Show Cone", true, d))
}

const LIGHT_TYPES: [(&str, &str); 9] = [
    ("omni", "Omni"),
    ("spot", "Spot"),
    ("infinite", "Infinite"),
    ("area", "Area"),
    ("square_spot", "Square Spot"),
    ("parallel", "Parallel"),
    ("parallel_spot", "Parallel Spot"),
    ("square_parallel_spot", "Square Parallel Spot"),
    ("ies", "IES"),
];

const AREA_SHAPES: [(&str, &str); 9] = [
    ("disc", "Disc"),
    ("rectangle", "Rectangle"),
    ("line", "Line"),
    ("sphere", "Sphere"),
    ("cylinder", "Cylinder"),
    ("cylinder_perpendicular", "Cylinder (Perpendicular)"),
    ("cube", "Cube"),
    ("hemisphere", "Hemisphere"),
    ("object", "Object / Spline"),
];

const FALLOFFS: [(&str, &str); 5] = [
    ("none", "None"),
    ("inverse_square", "Inverse Square (Physically Accurate)"),
    ("linear", "Linear"),
    ("step", "Step"),
    ("inverse_square_clamped", "Inverse Square Clamped"),
];

/// `forge.light`: one node type for all nine Type options, as O29 lists the same parameters
/// for each (only the visibility of some controls differs). The General, Details and Shadow
/// tabs are parameters; Visibility (volumetric), Photometric, Caustics, Noise, Lens Effects and
/// the Project include list are not yet.
pub fn light_type() -> NodeType {
    let (g, d, s) = ("General", "Details", "Shadow");
    NodeType::new(LIGHT, "Light", Category::Object)
        .param(ParamDesc::color("color", "Color", [1.0; 3]).group(g))
        .param(flag("use_temperature", "Use Temperature", false, g))
        .param(
            ParamDesc::float("color_temperature", "Color Temperature", 6500.0)
                .range(1000.0, 10000.0)
                .group(g),
        )
        .param(
            ParamDesc::float("intensity", "Intensity", 1.0)
                .range(-100_000.0, 100_000.0)
                .unit(Unit::Percent)
                .group(g),
        )
        .param(ParamDesc::enumeration("type", "Type", "omni", &LIGHT_TYPES).group(g))
        .param(
            ParamDesc::enumeration(
                "shadow",
                "Shadow",
                "none",
                &[
                    ("none", "None"),
                    ("shadow_maps", "Shadow Maps (Soft)"),
                    ("raytraced", "Raytraced (Hard)"),
                    ("area", "Area"),
                ],
            )
            .group(g),
        )
        .param(
            ParamDesc::enumeration(
                "visible_light",
                "Visible Light",
                "none",
                &[
                    ("none", "None"),
                    ("visible", "Visible"),
                    ("volumetric", "Volumetric"),
                    ("inverse_volumetric", "Inverse Volumetric"),
                ],
            )
            .group(g),
        )
        .param(flag("no_illumination", "No Illumination", false, g))
        .param(flag(
            "ambient_illumination",
            "Ambient Illumination",
            false,
            g,
        ))
        .param(flag("diffuse", "Diffuse", true, g))
        .param(flag("specular", "Specular", true, g))
        .param(flag("separate_pass", "Separate Pass", false, g))
        .param(flag("gi_illumination", "GI Illumination", true, g))
        .param(flag(
            "export_to_compositing",
            "Export to Compositing",
            true,
            g,
        ))
        .param(flag("use_inner_value", "Use Inner Value", true, d))
        .param(angle("inner_angle", "Inner Angle", 0.0, 0.0, 175.0, d))
        .param(angle("outer_angle", "Outer Angle", 30.0, 0.0, 175.0, d))
        .param(
            ParamDesc::float("aspect_ratio", "Aspect Ratio", 1.0)
                .range(0.01, 100.0)
                .group(d),
        )
        .param(
            ParamDesc::float("contrast", "Contrast", 0.0)
                .range(-10.0, 10.0)
                .unit(Unit::Percent)
                .group(d),
        )
        .param(flag("shadow_caster", "Shadow Caster", false, d))
        .param(
            ParamDesc::enumeration("area_shape", "Area Shape", "rectangle", &AREA_SHAPES).group(d),
        )
        .param(ParamDesc::link("object", "Object", &[Category::Object]).group(d))
        .param(
            ParamDesc::float("size_x", "Size X", 200.0)
                .range(2e-5, 999_999_999.0)
                .unit(Unit::Distance)
                .group(d),
        )
        .param(dist("size_y", "Size Y", 200.0, d))
        .param(dist("size_z", "Size Z", 200.0, d))
        .param(angle(
            "falloff_angle",
            "Falloff Angle",
            180.0,
            0.0,
            180.0,
            d,
        ))
        .param(angle("infinite_angle", "Infinite Angle", 0.5, 0.0, 90.0, d))
        .param(
            ParamDesc::int("samples", "Samples", 40)
                .range(16, 1000)
                .group(d),
        )
        .param(flag("show_in_render", "Show in Render", false, d))
        .param(flag(
            "show_as_solid_in_viewport",
            "Show as Solid in Viewport",
            false,
            d,
        ))
        .param(flag("show_in_specular", "Show in Specular", true, d))
        .param(flag("show_in_reflection", "Show in Reflection", false, d))
        .param(ParamDesc::enumeration("falloff", "Falloff", "none", &FALLOFFS).group(d))
        .param(dist("falloff_inner_radius", "Falloff Inner Radius", 0.0, d))
        .param(dist("falloff_radius", "Falloff Radius / Decay", 500.0, d))
        .param(flag(
            "colored_edge_falloff",
            "Colored Edge Falloff",
            false,
            d,
        ))
        .param(flag("z_direction_only", "Z Direction Only", false, d))
        .param(flag("use_near_clipping", "Use Near Clipping", false, d))
        .param(dist("near_clipping_from", "Near Clipping from", 0.0, d))
        .param(dist("near_clipping_to", "Near Clipping to", 10.0, d))
        .param(flag("use_far_clipping", "Use Far Clipping", false, d))
        .param(dist("far_clipping_from", "Far Clipping from", 90.0, d))
        .param(dist("far_clipping_to", "Far Clipping to", 100.0, d))
        .param(pct("shadow_density", "Shadow Density", 1.0, 10000.0, s))
        .param(ParamDesc::color("shadow_color", "Shadow Color", [0.0; 3]).group(s))
}

/// `forge.environment`: uniform ambient radiance (our design; the O29 material Environment
/// group supplies the defaults: colour white, brightness 100 %).
pub fn environment_type() -> NodeType {
    let g = "Environment";
    NodeType::new(ENVIRONMENT, "Environment", Category::Object)
        .param(ParamDesc::color("color", "Color", [1.0; 3]).group(g))
        .param(pct("brightness", "Brightness", 1.0, 10000.0, g))
        .param(flag(
            "visible_in_background",
            "Visible in Background",
            false,
            g,
        ))
        .param(ParamDesc::file_path("image", "HDRI Image", "").group(g))
        .param(angle(
            "image_rotation",
            "Image Rotation",
            0.0,
            -360.0,
            360.0,
            g,
        ))
}

/// `forge.sky`: a vertical colour gradient about world +Y (our design).
pub fn sky_type() -> NodeType {
    let g = "Sky";
    NodeType::new(SKY, "Sky", Category::Object)
        .param(ParamDesc::color("zenith_color", "Zenith Color", [0.25, 0.45, 0.85]).group(g))
        .param(ParamDesc::color("horizon_color", "Horizon Color", [0.85, 0.9, 0.95]).group(g))
        .param(ParamDesc::color("ground_color", "Ground Color", [0.25, 0.22, 0.2]).group(g))
        .param(pct("brightness", "Brightness", 1.0, 10000.0, g))
        .param(flag(
            "visible_in_background",
            "Visible in Background",
            true,
            g,
        ))
}

/// `forge.floor`: an endless horizontal plane through the object's origin, with the object's
/// +Y as its normal (our design; rendered as a very large square).
pub fn floor_type() -> NodeType {
    let g = "Floor";
    NodeType::new(FLOOR, "Floor", Category::Object)
        .param(ParamDesc::color("color", "Color", [0.8, 0.8, 0.8]).group(g))
        .param(
            ParamDesc::float("extent", "Extent", 100_000.0)
                .range(1.0, 1.0e9)
                .unit(Unit::Distance)
                .group(g),
        )
}

/// The five scene object types.
pub fn node_types() -> [NodeType; 5] {
    [
        camera_type(),
        light_type(),
        environment_type(),
        floor_type(),
        sky_type(),
    ]
}

// --- reading a node ---

struct Reader<'a> {
    ty: std::sync::Arc<NodeType>,
    values: &'a BTreeMap<String, Value>,
}

impl Reader<'_> {
    fn get(&self, id: &str) -> Value {
        let desc = self
            .ty
            .find_param(id)
            .expect("scene object reads its own params");
        self.values
            .get(id)
            .and_then(|v| desc.validate(v.clone()).ok())
            .unwrap_or_else(|| desc.default.clone())
    }
    fn f(&self, id: &str) -> f64 {
        match self.get(id) {
            Value::Float(f) => f,
            other => unreachable!("{id}: {other:?}"),
        }
    }
    fn b(&self, id: &str) -> bool {
        matches!(self.get(id), Value::Bool(true))
    }
    fn e(&self, id: &str) -> String {
        match self.get(id) {
            Value::Enum(s) => s,
            other => unreachable!("{id}: {other:?}"),
        }
    }
    fn c(&self, id: &str) -> [f64; 3] {
        match self.get(id) {
            Value::Color(c) => c,
            other => unreachable!("{id}: {other:?}"),
        }
    }
    fn s(&self, id: &str) -> String {
        match self.get(id) {
            Value::String(s) => s,
            other => unreachable!("{id}: {other:?}"),
        }
    }
}

fn reader<'a>(scene: &'a Scene, id: NodeId, type_id: &str) -> Option<Reader<'a>> {
    let n = scene.node(id)?;
    (n.type_id == type_id).then(|| Reader {
        ty: scene.registry().get(type_id).expect("registered").clone(),
        values: &n.params,
    })
}

/// World matrix of a node with scale and shear removed (a camera or light has none).
pub fn rigid_world(scene: &Scene, id: NodeId) -> Option<DMat4> {
    let w = scene.world_matrix(id)?;
    let r = transform::rotation_part(&w);
    Some(DMat4::from_mat3_translation(
        DMat3::from_cols(r.x_axis, r.y_axis, r.z_axis),
        w.w_axis.truncate(),
    ))
}

/// Lens of a camera, in the units of the parameters.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Lens {
    Perspective {
        focal_mm: f64,
        sensor_mm: f64,
        /// Shift of the principal point, +x right (a fraction of the image width), +y up (a
        /// fraction of the image height): minus Film Offset X, plus Film Offset Y.
        film_offset: [f64; 2],
    },
    /// World width that spans the image width.
    Orthographic { width: f64 },
}

/// Everything a renderer or the viewport needs from a camera object.
#[derive(Debug, Clone, PartialEq)]
pub struct CameraData {
    pub node: NodeId,
    pub name: String,
    /// Camera to world, rigid; the camera looks along +Z, +X right, +Y up.
    pub world: DMat4,
    pub projection: String,
    pub lens: Lens,
    pub sensor_mm: f64,
    pub zoom: f64,
    pub focus_distance: f64,
    /// Radius of the thin lens in scene units (`focal / (2 f-stop)`, mm converted to cm), 0 for
    /// orthographic projections (RENDER.md R15).
    pub lens_radius: f64,
    pub near_clip: Option<f64>,
    pub far_clip: Option<f64>,
    pub f_stop: f64,
    pub shutter_speed: f64,
    pub shutter_angle: f64,
    pub show_cone: bool,
    /// Approximations made for this camera (a projection that is not modelled).
    pub notes: Vec<String>,
}

/// Scene units per millimetre: O29 lists every distance in centimetres, focal lengths and the
/// film gate in millimetres.
pub const MM_TO_SCENE: f64 = 0.1;

/// Radius of the entrance pupil, in scene units, of a lens of `focal_mm` at `f_stop`
/// (`diameter = focal / f-stop`).
pub fn aperture_radius(focal_mm: f64, f_stop: f64) -> f64 {
    focal_mm / (2.0 * f_stop) * MM_TO_SCENE
}

/// Focal length in mm that gives the horizontal field of view `fov` (radians) on `sensor`.
pub fn focal_from_fov(fov: f64, sensor_mm: f64) -> f64 {
    sensor_mm * 0.5 / (fov * 0.5).tan()
}

/// Vertical field of view (radians) for the horizontal `fov` and a film `aspect` (width/height).
pub fn vertical_fov(fov: f64, aspect: f64) -> f64 {
    2.0 * ((fov * 0.5).tan() / aspect).atan()
}

/// World width spanned by an orthographic camera at zoom 1 (O57 `ortho-front`: 1000 units
/// across 320 px); **parity-pending** for other zoom values.
pub const ORTHO_WIDTH: f64 = 1000.0;

/// Fixed orientation of the six axis projections: the camera looks along +Z (Front), -Z (Back),
/// +X (Left: the camera stands on the left), -X (Right), -Y (Top, screen up = +Z) or +Y
/// (Bottom, screen up = -Z). Source: O57 `ortho-front`, `ortho-top`; the other four follow from
/// them and are parity-pending.
pub fn axis_view_rotation(projection: &str) -> Option<DMat3> {
    use std::f64::consts::{FRAC_PI_2, PI};
    Some(match projection {
        "front" => DMat3::IDENTITY,
        "back" => DMat3::from_rotation_y(PI),
        "left" => DMat3::from_rotation_y(FRAC_PI_2),
        "right" => DMat3::from_rotation_y(-FRAC_PI_2),
        "top" => DMat3::from_rotation_x(FRAC_PI_2),
        "bottom" => DMat3::from_rotation_x(-FRAC_PI_2),
        _ => return None,
    })
}

pub fn camera_data(scene: &Scene, id: NodeId) -> Option<CameraData> {
    let r = reader(scene, id, CAMERA)?;
    let sensor = r.f("sensor_size");
    let zoom = r.f("zoom");
    let projection = r.e("projection");
    let lens = if projection == "perspective" {
        Lens::Perspective {
            focal_mm: focal_from_fov(r.f("fov_horizontal"), sensor),
            sensor_mm: sensor,
            // the parameters shift the frame, the lens moves the principal point the other way
            // (O57 `film-offset`: +20 % X moves the picture left by 20 % of the width;
            // `film-offset-y`: +20 % Y moves it up by 20 % of the height, so Y is not negated)
            film_offset: [-r.f("film_offset_x"), r.f("film_offset_y")],
        }
    } else {
        Lens::Orthographic {
            width: ORTHO_WIDTH / zoom,
        }
    };
    let mut world = rigid_world(scene, id)?;
    let mut notes = Vec::new();
    if let Some(rot) = axis_view_rotation(&projection) {
        // the axis views ignore the camera's rotation (O57 `ortho-top`: a camera with a
        // rotation set still looks straight down)
        world = DMat4::from_mat3_translation(rot, world.w_axis.truncate());
    } else if !matches!(projection.as_str(), "perspective" | "parallel") {
        notes.push(format!(
            "projection {projection} is not modelled: rendered as a parallel view along the camera"
        ));
    }
    Some(CameraData {
        node: id,
        name: scene.node(id)?.name.clone(),
        world,
        projection,
        lens,
        sensor_mm: sensor,
        zoom,
        focus_distance: r.f("focus_distance"),
        lens_radius: match lens {
            Lens::Perspective { focal_mm, .. } => aperture_radius(focal_mm, r.f("f_stop")),
            Lens::Orthographic { .. } => 0.0,
        },
        near_clip: r.b("enable_near_clipping").then(|| r.f("near_clipping")),
        far_clip: r.b("enable_far_clipping").then(|| r.f("far_clipping")),
        f_stop: r.f("f_stop"),
        shutter_speed: r.f("shutter_speed"),
        shutter_angle: r.f("shutter_angle"),
        show_cone: r.b("show_cone"),
        notes,
    })
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AreaKind {
    Disc {
        radius: f64,
    },
    Rectangle {
        width: f64,
        height: f64,
    },
    Sphere {
        radius: f64,
    },
    /// An area shape that is not rendered yet (its id).
    Unsupported(&'static str),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LightShape {
    Omni,
    Spot,
    Infinite,
    Area,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FalloffKind {
    None,
    Linear,
    InverseSquare,
    Step,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShadowKind {
    None,
    Hard,
    Soft,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LightData {
    pub node: NodeId,
    pub name: String,
    /// Light to world, rigid; the light shines along +Z.
    pub world: DMat4,
    pub type_id: String,
    pub shape: LightShape,
    /// Colour with the colour temperature applied.
    pub color: [f64; 3],
    pub intensity: f64,
    /// Full cone angles in radians (spot types).
    pub inner: f64,
    pub outer: f64,
    pub infinite_angle: f64,
    pub area: AreaKind,
    pub two_sided: bool,
    pub falloff: FalloffKind,
    pub falloff_radius: f64,
    pub shadow: ShadowKind,
    pub shadow_density: f64,
    pub shadow_color: [f64; 3],
    pub diffuse: bool,
    pub specular: bool,
    /// The light does not illuminate (No Illumination).
    pub off: bool,
    /// An area light's surface is seen by camera rays (Show in Render).
    pub visible: bool,
    /// Rendering approximations made for this light (an option that is not modelled).
    pub notes: Vec<String>,
}

/// Linear RGB tint of a black body of `kelvin`, normalised so 6500 K is white. A public
/// polynomial approximation (Tanner Helland's fit of the Planckian locus) clamped to 0..1.
pub fn kelvin_tint(kelvin: f64) -> [f64; 3] {
    fn raw(k: f64) -> [f64; 3] {
        let t = k.clamp(1000.0, 40000.0) / 100.0;
        let r = if t <= 66.0 {
            255.0
        } else {
            329.698727446 * (t - 60.0).powf(-0.1332047592)
        };
        let g = if t <= 66.0 {
            99.4708025861 * t.ln() - 161.1195681661
        } else {
            288.1221695283 * (t - 60.0).powf(-0.0755148492)
        };
        let b = if t >= 66.0 {
            255.0
        } else if t <= 19.0 {
            0.0
        } else {
            138.5177312231 * (t - 10.0).ln() - 305.0447927307
        };
        let c = |v: f64| (v / 255.0).clamp(0.0, 1.0);
        [c(r), c(g), c(b)]
    }
    let (k, white) = (raw(kelvin), raw(6500.0));
    [
        (k[0] / white[0]).min(1.0),
        (k[1] / white[1]).min(1.0),
        (k[2] / white[2]).min(1.0),
    ]
}

pub fn light_data(scene: &Scene, id: NodeId) -> Option<LightData> {
    let r = reader(scene, id, LIGHT)?;
    let type_id = r.e("type");
    let mut notes = Vec::new();
    let shape = match type_id.as_str() {
        "omni" | "ies" => LightShape::Omni,
        "spot" | "square_spot" => LightShape::Spot,
        "infinite" | "parallel" | "parallel_spot" | "square_parallel_spot" => LightShape::Infinite,
        _ => LightShape::Area,
    };
    match type_id.as_str() {
        "ies" => notes.push("IES profile ignored: rendered as an omni light".into()),
        "square_spot" => notes.push("square cone not modelled: rendered as a round spot".into()),
        "parallel_spot" | "square_parallel_spot" => {
            notes.push("parallel cone not modelled: rendered as an infinite light".into())
        }
        _ => {}
    }
    let (sx, sy) = (r.f("size_x"), r.f("size_y"));
    let area = match r.e("area_shape").as_str() {
        "disc" => AreaKind::Disc { radius: sx * 0.5 },
        "rectangle" => AreaKind::Rectangle {
            width: sx,
            height: sy,
        },
        "sphere" => AreaKind::Sphere { radius: sx * 0.5 },
        "line" => AreaKind::Unsupported("line"),
        "cylinder" => AreaKind::Unsupported("cylinder"),
        "cylinder_perpendicular" => AreaKind::Unsupported("cylinder_perpendicular"),
        "cube" => AreaKind::Unsupported("cube"),
        "hemisphere" => AreaKind::Unsupported("hemisphere"),
        _ => AreaKind::Unsupported("object"),
    };
    let falloff = match r.e("falloff").as_str() {
        "linear" => FalloffKind::Linear,
        "inverse_square" | "inverse_square_clamped" => FalloffKind::InverseSquare,
        "step" => FalloffKind::Step,
        _ => FalloffKind::None,
    };
    let shadow = match r.e("shadow").as_str() {
        "raytraced" => ShadowKind::Hard,
        "shadow_maps" | "area" => ShadowKind::Soft,
        _ => ShadowKind::None,
    };
    let mut color = r.c("color");
    if r.b("use_temperature") {
        let t = kelvin_tint(r.f("color_temperature"));
        for (c, t) in color.iter_mut().zip(t) {
            *c *= t;
        }
    }
    Some(LightData {
        node: id,
        name: scene.node(id)?.name.clone(),
        world: rigid_world(scene, id)?,
        type_id,
        shape,
        color,
        intensity: r.f("intensity"),
        inner: if r.b("use_inner_value") {
            r.f("inner_angle")
        } else {
            0.0
        },
        outer: r.f("outer_angle"),
        infinite_angle: r.f("infinite_angle"),
        area,
        two_sided: false,
        falloff,
        falloff_radius: r.f("falloff_radius"),
        shadow,
        shadow_density: r.f("shadow_density"),
        shadow_color: r.c("shadow_color"),
        diffuse: r.b("diffuse"),
        specular: r.b("specular"),
        off: r.b("no_illumination"),
        visible: r.b("show_in_render"),
        notes,
    })
}

#[derive(Debug, Clone, PartialEq)]
pub struct EnvironmentData {
    pub node: NodeId,
    pub color: [f64; 3],
    pub brightness: f64,
    pub visible_in_background: bool,
    /// HDRI file path as stored (relative to the scene file); empty = uniform colour.
    pub image: String,
    /// Rotation about +Y in radians.
    pub image_rotation: f64,
}

pub fn environment_data(scene: &Scene, id: NodeId) -> Option<EnvironmentData> {
    let r = reader(scene, id, ENVIRONMENT)?;
    Some(EnvironmentData {
        node: id,
        color: r.c("color"),
        brightness: r.f("brightness"),
        visible_in_background: r.b("visible_in_background"),
        image: r.s("image"),
        image_rotation: r.f("image_rotation"),
    })
}

#[derive(Debug, Clone, PartialEq)]
pub struct SkyData {
    pub node: NodeId,
    pub zenith: [f64; 3],
    pub horizon: [f64; 3],
    pub ground: [f64; 3],
    pub brightness: f64,
    pub visible_in_background: bool,
}

pub fn sky_data(scene: &Scene, id: NodeId) -> Option<SkyData> {
    let r = reader(scene, id, SKY)?;
    Some(SkyData {
        node: id,
        zenith: r.c("zenith_color"),
        horizon: r.c("horizon_color"),
        ground: r.c("ground_color"),
        brightness: r.f("brightness"),
        visible_in_background: r.b("visible_in_background"),
    })
}

#[derive(Debug, Clone, PartialEq)]
pub struct FloorData {
    pub node: NodeId,
    pub color: [f64; 3],
    pub extent: f64,
    /// Floor plane to world; the floor is the local XZ plane, +Y up.
    pub world: DMat4,
}

pub fn floor_data(scene: &Scene, id: NodeId) -> Option<FloorData> {
    let r = reader(scene, id, FLOOR)?;
    Some(FloorData {
        node: id,
        color: r.c("color"),
        extent: r.f("extent"),
        world: rigid_world(scene, id)?,
    })
}

/// Every scene object of a scene, in tree order. Objects a layer or the editor/render
/// visibility hides are still listed; callers decide.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SceneObjects {
    pub cameras: Vec<CameraData>,
    pub lights: Vec<LightData>,
    pub environments: Vec<EnvironmentData>,
    pub skies: Vec<SkyData>,
    pub floors: Vec<FloorData>,
}

pub fn scene_objects(scene: &Scene) -> SceneObjects {
    let mut out = SceneObjects::default();
    let mut order = Vec::new();
    for r in scene.roots() {
        order.push(*r);
        order.extend(scene.descendants(*r));
    }
    for id in order {
        let Some(n) = scene.node(id) else { continue };
        match n.type_id.as_str() {
            CAMERA => out.cameras.extend(camera_data(scene, id)),
            LIGHT => out.lights.extend(light_data(scene, id)),
            ENVIRONMENT => out.environments.extend(environment_data(scene, id)),
            SKY => out.skies.extend(sky_data(scene, id)),
            FLOOR => out.floors.extend(floor_data(scene, id)),
            _ => {}
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn white_at_6500k() {
        let t = kelvin_tint(6500.0);
        assert!(t.iter().all(|c| (c - 1.0).abs() < 1e-9));
        let warm = kelvin_tint(2700.0);
        assert!(warm[0] > warm[2]);
    }
}
