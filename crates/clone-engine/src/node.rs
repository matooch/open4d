//! The cloner node type: parameter descriptors and their reading
//! (clone-toolset spec C2).

use std::collections::BTreeMap;
use std::f64::consts::TAU;

use forge_core::{Category, NodeType, ParamDesc, Plugin, Registry, RegistryError, Unit, Value};
use glam::DVec3;

use crate::MAX_CLONES;
use crate::settings::*;

/// Type id of the cloner.
pub const CLONER: &str = "forge.cloner";

const BIG: f64 = 1e20;
const COUNT_MAX: i64 = MAX_CLONES as i64;
const SEED_MAX: i64 = 2_147_483_647;

fn opts(list: &[(&str, &str)]) -> Vec<(String, String)> {
    list.iter()
        .map(|(a, b)| (a.to_string(), b.to_string()))
        .collect()
}

fn en(id: &str, label: &str, default: &str, options: &[(&str, &str)], group: &str) -> ParamDesc {
    let o = opts(options);
    let refs: Vec<(&str, &str)> = o.iter().map(|(a, b)| (a.as_str(), b.as_str())).collect();
    ParamDesc::enumeration(id, label, default, &refs).group(group)
}

/// The cloner's node type. Every parameter is a descriptor, so the attribute panel, undo,
/// animation and the file format derive from this list.
pub fn cloner_type() -> NodeType {
    const OBJ: &str = "Object";
    const TR: &str = "Transform";
    const LIN: &str = "Linear";
    const RAD: &str = "Radial";
    const GRID: &str = "Grid Array";
    const HONEY: &str = "Honeycomb";
    NodeType::new(CLONER, "Cloner", Category::Generator)
        // Common, Object tab.
        .param(en(
            "mode",
            "Mode",
            "grid",
            &[
                ("object", "Object"),
                ("linear", "Linear"),
                ("radial", "Radial"),
                ("grid", "Grid Array"),
                ("honeycomb", "Honeycomb"),
            ],
            OBJ,
        ))
        .param(en(
            "clones_mode",
            "Clones",
            "iterate",
            &[
                ("iterate", "Iterate"),
                ("random", "Random"),
                ("blend", "Blend"),
                ("sort", "Sort"),
            ],
            OBJ,
        ))
        .param(ParamDesc::bool("reset_coordinates", "Reset Coordinates", true).group(OBJ))
        .param(en(
            "fix_texture",
            "Fix Texture",
            "off",
            &[
                ("off", "Off"),
                ("straight", "Straight"),
                ("alternate_x", "Alternate X"),
                ("alternate_y", "Alternate Y"),
            ],
            OBJ,
        ))
        .param(en(
            "instance_mode",
            "Instance Mode",
            "instance",
            &[
                ("instance", "Instance"),
                ("render_instance", "Render Instance"),
                ("multi_instance", "Multi-Instance"),
            ],
            OBJ,
        ))
        .param(en(
            "viewport_mode",
            "Viewport Mode",
            "object",
            &[
                ("off", "Off"),
                ("points", "Points"),
                ("matrix", "Matrix"),
                ("bounding_box", "Bounding Box"),
                ("object", "Object"),
            ],
            OBJ,
        ))
        // Linear.
        .param(
            ParamDesc::int("linear_count", "Count", 3)
                .range(0, COUNT_MAX)
                .group(LIN),
        )
        .param(
            ParamDesc::float("linear_offset", "Offset", 0.0)
                .range(0.0, BIG)
                .group(LIN),
        )
        .param(en(
            "linear_mode",
            "Mode",
            "per_step",
            &[("end_point", "End Point"), ("per_step", "Per Step")],
            LIN,
        ))
        .param(
            ParamDesc::float("amount", "Amount", 1.0)
                .range(-BIG, BIG)
                .unit(Unit::Percent)
                .group(LIN),
        )
        .param(
            ParamDesc::vec3("transform_position", "P", [0.0, 50.0, 0.0])
                .range(-BIG, BIG)
                .unit(Unit::Distance)
                .group(LIN),
        )
        .param(
            ParamDesc::vec3("transform_scale", "S", [1.0; 3])
                .range(-BIG, BIG)
                .unit(Unit::Percent)
                .group(LIN),
        )
        .param(
            ParamDesc::vec3("transform_rotation", "R", [0.0; 3])
                .range(-BIG, BIG)
                .unit(Unit::Angle)
                .group(LIN),
        )
        .param(en(
            "step_mode",
            "Step Mode",
            "single",
            &[("single", "Single Value"), ("cumulative", "Cumulative")],
            LIN,
        ))
        .param(
            ParamDesc::float("step_size", "Step Size", 1.0)
                .range(0.0, BIG)
                .unit(Unit::Percent)
                .group(LIN),
        )
        .param(
            ParamDesc::vec3("step_rotation", "Step Rotation", [0.0; 3])
                .range(-BIG, BIG)
                .unit(Unit::Angle)
                .group(LIN),
        )
        // Radial.
        .param(
            ParamDesc::int("radial_count", "Count", 5)
                .range(0, COUNT_MAX)
                .group(RAD),
        )
        .param(
            ParamDesc::float("radius", "Radius", 50.0)
                .range(-BIG, BIG)
                .unit(Unit::Distance)
                .group(RAD),
        )
        .param(en(
            "plane",
            "Plane",
            "xz",
            &[("xy", "XY"), ("zy", "ZY"), ("xz", "XZ")],
            RAD,
        ))
        .param(ParamDesc::bool("align", "Align", true).group(RAD))
        .param(
            ParamDesc::float("start_angle", "Start Angle", 0.0)
                .range(-BIG, BIG)
                .unit(Unit::Angle)
                .group(RAD),
        )
        .param(
            ParamDesc::float("end_angle", "End Angle", TAU)
                .range(-BIG, BIG)
                .unit(Unit::Angle)
                .group(RAD),
        )
        .param(
            ParamDesc::float("radial_offset", "Offset", 0.0)
                .range(-BIG, BIG)
                .unit(Unit::Angle)
                .group(RAD),
        )
        .param(
            ParamDesc::float("radial_offset_variation", "Offset Variation", 0.0)
                .range(0.0, BIG)
                .unit(Unit::Percent)
                .group(RAD),
        )
        .param(
            ParamDesc::int("radial_offset_seed", "Offset Seed", 1_234_567)
                .range(0, SEED_MAX)
                .group(RAD),
        )
        // Grid Array.
        .param(
            ParamDesc::vec3("grid_count", "Count", [3.0, 1.0, 3.0])
                .range(1.0, MAX_CLONES as f64)
                .step(1.0)
                .group(GRID),
        )
        .param(en(
            "grid_mode",
            "Mode",
            "per_step",
            &[("per_step", "Per-Step"), ("endpoint", "Endpoint")],
            GRID,
        ))
        .param(
            ParamDesc::vec3("grid_size", "Size", [200.0; 3])
                .range(-BIG, BIG)
                .unit(Unit::Distance)
                .group(GRID),
        )
        .param(en(
            "grid_form",
            "Form",
            "cubic",
            &[
                ("cubic", "Cubic"),
                ("sphere", "Sphere"),
                ("cylinder", "Cylinder"),
                ("object", "Object"),
            ],
            GRID,
        ))
        .param(
            ParamDesc::float("fill", "Fill", 1.0)
                .range(0.0, 1.0)
                .unit(Unit::Percent)
                .group(GRID),
        )
        // Honeycomb.
        .param(en(
            "orientation",
            "Orientation",
            "z",
            &[("z", "Z (XY)"), ("x", "X (ZY)"), ("y", "Y (XZ)")],
            HONEY,
        ))
        .param(en(
            "offset_direction",
            "Offset Direction",
            "height",
            &[("width", "Width"), ("height", "Height")],
            HONEY,
        ))
        .param(
            ParamDesc::float("honeycomb_offset", "Offset", 0.5)
                .range(0.0, 1.0)
                .unit(Unit::Percent)
                .group(HONEY),
        )
        .param(
            ParamDesc::float("honeycomb_offset_variation", "Offset Variation", 0.0)
                .range(0.0, 1.0)
                .unit(Unit::Percent)
                .group(HONEY),
        )
        .param(
            ParamDesc::float("perpendicular_variation", "Perpendicular Variation", 0.0)
                .range(0.0, 1.0)
                .unit(Unit::Percent)
                .group(HONEY),
        )
        .param(
            ParamDesc::int("honeycomb_seed", "Seed", 123_456)
                .range(0, SEED_MAX)
                .group(HONEY),
        )
        .param(
            ParamDesc::int("count_width", "Count Width", 10)
                .range(1, COUNT_MAX)
                .group(HONEY),
        )
        .param(
            ParamDesc::int("count_height", "Count Height", 10)
                .range(1, COUNT_MAX)
                .group(HONEY),
        )
        .param(en(
            "honeycomb_mode",
            "Mode",
            "per_step",
            &[("per_step", "Per Step"), ("end_point", "End Point")],
            HONEY,
        ))
        .param(
            ParamDesc::float("size_width", "Size Width", 300.0)
                .range(0.0, BIG)
                .unit(Unit::Distance)
                .group(HONEY),
        )
        .param(
            ParamDesc::float("size_height", "Size Height", 346.410_161_6)
                .range(0.0, BIG)
                .unit(Unit::Distance)
                .group(HONEY),
        )
        .param(en(
            "honeycomb_form",
            "Form",
            "square",
            &[
                ("circle", "Circle"),
                ("square", "Square"),
                ("spline", "Spline"),
            ],
            HONEY,
        ))
        // Effectors (list order = application order, spec E1).
        .param(ParamDesc::link_list("effectors", "Effectors", &[Category::Effector]).group(OBJ))
        // Transform tab.
        .param(en(
            "display",
            "Display",
            "none",
            &[
                ("none", "None"),
                ("weight", "Weight"),
                ("uv", "UV"),
                ("color", "Color"),
                ("index", "Index"),
            ],
            TR,
        ))
        .param(
            ParamDesc::vec3("clone_position", "P", [0.0; 3])
                .range(-BIG, BIG)
                .unit(Unit::Distance)
                .group(TR),
        )
        .param(
            ParamDesc::vec3("clone_rotation", "R", [0.0; 3])
                .range(-BIG, BIG)
                .unit(Unit::Angle)
                .group(TR),
        )
        .param(
            ParamDesc::vec3("clone_scale", "S", [1.0; 3])
                .range(-BIG, BIG)
                .group(TR),
        )
        .param(ParamDesc::color("clone_color", "Color", [1.0; 3]).group(TR))
        .param(en(
            "uv_orientation",
            "W(UV)-Orientation",
            "z_plus",
            &[
                ("x_plus", "X+"),
                ("x_minus", "X-"),
                ("y_plus", "Y+"),
                ("y_minus", "Y-"),
                ("z_plus", "Z+"),
                ("z_minus", "Z-"),
            ],
            TR,
        ))
        .param(
            ParamDesc::float("clone_weight", "Weight", 0.0)
                .range(0.0, BIG)
                .unit(Unit::Percent)
                .group(TR),
        )
        .param(en(
            "animation_mode",
            "Animation Mode",
            "play",
            &[
                ("play", "Play"),
                ("loop", "Loop"),
                ("fixed", "Fixed"),
                ("fixed_loop", "Fixed Loop"),
            ],
            TR,
        ))
}

/// The clone toolset plugin: registers the cloner (and, in later briefs, the effectors).
pub struct CloneToolset;

impl Plugin for CloneToolset {
    fn id(&self) -> &str {
        "forge.clone-toolset"
    }

    fn register(&self, r: &mut Registry) -> Result<(), RegistryError> {
        r.register(cloner_type())?;
        r.register(crate::plain::plain_type())
    }
}

/// Typed parameter access with defaults from the descriptors.
pub(crate) struct Params<'a> {
    pub(crate) ty: &'a NodeType,
    pub(crate) values: &'a BTreeMap<String, Value>,
}

impl Params<'_> {
    fn get(&self, id: &str) -> Value {
        let desc = self.ty.find_param(id).expect("cloner reads its own params");
        self.values
            .get(id)
            .and_then(|v| desc.validate(v.clone()).ok())
            .unwrap_or_else(|| desc.default.clone())
    }

    pub(crate) fn float(&self, id: &str) -> f64 {
        match self.get(id) {
            Value::Float(f) => f,
            other => unreachable!("{id} is not a float: {other:?}"),
        }
    }

    fn int(&self, id: &str) -> i64 {
        match self.get(id) {
            Value::Int(i) => i,
            other => unreachable!("{id} is not an int: {other:?}"),
        }
    }

    fn count(&self, id: &str) -> usize {
        self.int(id).max(0) as usize
    }

    pub(crate) fn bool(&self, id: &str) -> bool {
        matches!(self.get(id), Value::Bool(true))
    }

    pub(crate) fn vec3(&self, id: &str) -> DVec3 {
        match self.get(id) {
            Value::Vec3(v) => DVec3::from_array(v),
            other => unreachable!("{id} is not a vec3: {other:?}"),
        }
    }

    pub(crate) fn color(&self, id: &str) -> [f64; 3] {
        match self.get(id) {
            Value::Color(c) => c,
            other => unreachable!("{id} is not a colour: {other:?}"),
        }
    }

    pub(crate) fn option(&self, id: &str) -> String {
        match self.get(id) {
            Value::Enum(s) => s,
            other => unreachable!("{id} is not an enum: {other:?}"),
        }
    }
}

/// Reads a cloner's parameters (defaults and clamping from the descriptors).
pub fn settings_from_params(values: &BTreeMap<String, Value>) -> Settings {
    let ty = cloner_type();
    let p = Params { ty: &ty, values };
    let mode = match p.option("mode").as_str() {
        "object" => Mode::Object,
        "linear" => Mode::Linear,
        "radial" => Mode::Radial,
        "honeycomb" => Mode::Honeycomb,
        _ => Mode::Grid,
    };
    let clones_mode = match p.option("clones_mode").as_str() {
        "random" => ClonesMode::Random,
        "blend" => ClonesMode::Blend,
        "sort" => ClonesMode::Sort,
        _ => ClonesMode::Iterate,
    };
    let instance_mode = match p.option("instance_mode").as_str() {
        "render_instance" => InstanceMode::RenderInstance,
        "multi_instance" => InstanceMode::MultiInstance,
        _ => InstanceMode::Instance,
    };
    let counts = p.vec3("grid_count");
    let whole = |v: f64| v.round().max(1.0) as usize;
    Settings {
        mode,
        clones_mode,
        reset_coordinates: p.bool("reset_coordinates"),
        instance_mode,
        transform: TransformTab {
            position: p.vec3("clone_position"),
            rotation: p.vec3("clone_rotation"),
            scale: p.vec3("clone_scale"),
            color: p.color("clone_color"),
            weight: p.float("clone_weight"),
        },
        linear: Linear {
            count: p.count("linear_count"),
            offset: p.float("linear_offset"),
            mode: match p.option("linear_mode").as_str() {
                "end_point" => LinearMode::EndPoint,
                _ => LinearMode::PerStep,
            },
            amount: p.float("amount"),
            position: p.vec3("transform_position"),
            scale: p.vec3("transform_scale"),
            rotation: p.vec3("transform_rotation"),
            step_mode: match p.option("step_mode").as_str() {
                "cumulative" => StepMode::Cumulative,
                _ => StepMode::Single,
            },
            step_size: p.float("step_size"),
            step_rotation: p.vec3("step_rotation"),
        },
        radial: Radial {
            count: p.count("radial_count"),
            radius: p.float("radius"),
            plane: match p.option("plane").as_str() {
                "xy" => Plane::Xy,
                "zy" => Plane::Zy,
                _ => Plane::Xz,
            },
            align: p.bool("align"),
            start_angle: p.float("start_angle"),
            end_angle: p.float("end_angle"),
            offset: p.float("radial_offset"),
            offset_variation: p.float("radial_offset_variation"),
            offset_seed: p.int("radial_offset_seed").max(0) as u64,
        },
        grid: Grid {
            count: [whole(counts.x), whole(counts.y), whole(counts.z)],
            mode: match p.option("grid_mode").as_str() {
                "endpoint" => GridMode::Endpoint,
                _ => GridMode::PerStep,
            },
            size: p.vec3("grid_size"),
            form: match p.option("grid_form").as_str() {
                "sphere" => GridForm::Sphere,
                "cylinder" => GridForm::Cylinder,
                "object" => GridForm::Object,
                _ => GridForm::Cubic,
            },
            fill: p.float("fill"),
        },
        honeycomb: Honeycomb {
            orientation: match p.option("orientation").as_str() {
                "x" => Orientation::X,
                "y" => Orientation::Y,
                _ => Orientation::Z,
            },
            offset_direction: match p.option("offset_direction").as_str() {
                "width" => OffsetDirection::Width,
                _ => OffsetDirection::Height,
            },
            offset: p.float("honeycomb_offset"),
            offset_variation: p.float("honeycomb_offset_variation"),
            perpendicular_variation: p.float("perpendicular_variation"),
            seed: p.int("honeycomb_seed").max(0) as u64,
            count_width: p.count("count_width").max(1),
            count_height: p.count("count_height").max(1),
            mode: match p.option("honeycomb_mode").as_str() {
                "end_point" => HoneycombMode::EndPoint,
                _ => HoneycombMode::PerStep,
            },
            size_width: p.float("size_width"),
            size_height: p.float("size_height"),
            form: match p.option("honeycomb_form").as_str() {
                "circle" => HoneycombForm::Circle,
                "spline" => HoneycombForm::Spline,
                _ => HoneycombForm::Square,
            },
        },
    }
}
