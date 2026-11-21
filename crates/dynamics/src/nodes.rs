//! Node types of the dynamics toolset: the rigid body tag, the scene settings object and the
//! connector (docs/specs/DYNAMICS.md §1–§3). Every parameter is a descriptor.

use forge_core::{
    Category, NodeId, NodeType, ParamDesc, Plugin, Registry, RegistryError, Scene, Unit, Value,
};

/// The rigid body tag (also the soft body tag: its Soft Body switch).
pub const RIGID_BODY: &str = "forge.rigid_body";
/// The dynamics scene settings object (gravity, time scale, steps per frame).
pub const SETTINGS: &str = "forge.dynamics_settings";
/// The connector object (hinge, ball-and-socket, slider, fixed).
pub const CONNECTOR: &str = "forge.connector";
/// The spring object (a linear force pair).
pub const SPRING: &str = "forge.spring";

const BIG: f64 = 1e18;

fn en(id: &str, label: &str, default: &str, options: &[(&str, &str)], group: &str) -> ParamDesc {
    ParamDesc::enumeration(id, label, default, options).group(group)
}

fn pct(id: &str, label: &str, default: f64, group: &str) -> ParamDesc {
    ParamDesc::float(id, label, default)
        .range(0.0, BIG)
        .unit(Unit::Percent)
        .group(group)
}

/// The rigid / soft body tag.
pub fn rigid_body_type() -> NodeType {
    const BODY: &str = "Dynamics";
    const COL: &str = "Collision";
    const MASS: &str = "Mass";
    const FORCE: &str = "Damping";
    const SOFT: &str = "Soft Body";
    NodeType::new(RIGID_BODY, "Rigid Body", Category::Tag)
        .param(ParamDesc::bool("enabled", "Enabled", true).group(BODY))
        .param(en(
            "dynamic",
            "Dynamic",
            "on",
            &[
                ("off", "Off"),
                ("on", "On"),
                ("ghost", "Ghost"),
                ("kinematic", "Kinematic"),
            ],
            BODY,
        ))
        .param(
            ParamDesc::bool("custom_initial_velocity", "Custom Initial Velocity", false)
                .group(BODY),
        )
        .param(
            ParamDesc::vec3(
                "initial_linear_velocity",
                "Initial Linear Velocity",
                [0.0; 3],
            )
            .unit(Unit::Distance)
            .group(BODY),
        )
        .param(
            ParamDesc::vec3(
                "initial_angular_velocity",
                "Initial Angular Velocity",
                [0.0; 3],
            )
            .unit(Unit::Angle)
            .group(BODY),
        )
        .param(en(
            "shape",
            "Shape",
            "automatic",
            &[
                ("automatic", "Automatic"),
                ("box", "Box"),
                ("sphere", "Sphere"),
                ("convex_hull", "Convex Hull"),
                ("static_mesh", "Static Mesh"),
            ],
            COL,
        ))
        .param(
            ParamDesc::float("size_increment", "Size Increment", 0.0)
                .unit(Unit::Distance)
                .group(COL),
        )
        .param(
            ParamDesc::float("bounce", "Bounce", 0.05)
                .range(0.0, 1.0)
                .unit(Unit::Percent)
                .group(COL),
        )
        .param(pct("friction", "Friction", 0.9, COL))
        .param(en(
            "mass_mode",
            "Use Mass",
            "world_density",
            &[
                ("world_density", "World Density"),
                ("custom_density", "Custom Density"),
                ("custom_mass", "Custom Mass"),
            ],
            MASS,
        ))
        .param(
            ParamDesc::float("density", "Density", 1.0)
                .range(0.0, BIG)
                .group(MASS),
        )
        .param(
            ParamDesc::float("mass", "Mass", 10.0)
                .range(0.0, BIG)
                .group(MASS),
        )
        .param(pct("linear_damping", "Linear Damping", 0.05, FORCE))
        .param(pct("angular_damping", "Angular Damping", 0.05, FORCE))
        .param(en(
            "soft_body",
            "Soft Body",
            "off",
            &[("off", "Off"), ("polygons", "Made of Polygons/Lines")],
            SOFT,
        ))
        .param(ParamDesc::bool("keep_shape", "Keep Soft Body Shape", true).group(SOFT))
        .param(pct("structural", "Structural", 1.0, SOFT))
        .param(pct("structural_damping", "Structural Damping", 0.2, SOFT))
        .param(pct("shear", "Shear", 0.5, SOFT))
        .param(pct("shear_damping", "Shear Damping", 0.2, SOFT))
        .param(pct("flexion", "Flexion", 0.5, SOFT))
        .param(pct("flexion_damping", "Flexion Damping", 0.2, SOFT))
        .param(
            ParamDesc::float("pressure", "Pressure", 0.0)
                .range(0.0, BIG)
                .group(SOFT),
        )
}

/// The scene-level settings object. Without one in the scene the defaults apply.
pub fn settings_type() -> NodeType {
    NodeType::new(SETTINGS, "Dynamics Settings", Category::Object)
        .param(ParamDesc::bool("enabled", "Enabled", true))
        .param(
            ParamDesc::float("gravity", "Gravity", crate::settings::DEFAULT_GRAVITY)
                .range(-BIG, BIG)
                .unit(Unit::Distance),
        )
        .param(ParamDesc::float("time_scale", "Time Scale", 1.0).range(0.0, 100.0))
        .param(
            ParamDesc::int(
                "steps_per_frame",
                "Steps per Frame",
                crate::settings::DEFAULT_STEPS,
            )
            .range(1, 100),
        )
}

/// The connector object: a joint between Object A and Object B at the connector's position.
pub fn connector_type() -> NodeType {
    let both = [Category::Object, Category::Generator];
    NodeType::new(CONNECTOR, "Connector", Category::Object)
        .param(en(
            "type",
            "Type",
            "hinge",
            &[
                ("hinge", "Hinge"),
                ("ball_socket", "Ball and Socket"),
                ("slider", "Slider"),
                ("fixed", "Fixed"),
            ],
            "Connector",
        ))
        .param(ParamDesc::link("object_a", "Object A", &both).group("Connector"))
        .param(ParamDesc::link("object_b", "Object B", &both).group("Connector"))
        .param(
            ParamDesc::vec3("offset", "Offset", [0.0; 3])
                .unit(Unit::Distance)
                .group("Connector"),
        )
        .param(ParamDesc::bool("ignore_collisions", "Ignore Collisions", true).group("Connector"))
        .param(ParamDesc::bool("angular_limit", "Angular Limit", false).group("Connector"))
        .param(
            ParamDesc::float("limit_from", "From", 0.0)
                .unit(Unit::Angle)
                .group("Connector"),
        )
        .param(
            ParamDesc::float("limit_to", "To", std::f64::consts::PI)
                .unit(Unit::Angle)
                .group("Connector"),
        )
}

/// The spring object: a force pair between Object A and Object B.
pub fn spring_type() -> NodeType {
    let both = [Category::Object, Category::Generator];
    NodeType::new(SPRING, "Spring", Category::Object)
        .param(en(
            "type",
            "Type",
            "linear",
            &[("linear", "Linear")],
            "Spring",
        ))
        .param(ParamDesc::link("object_a", "Object A", &both).group("Spring"))
        .param(ParamDesc::link("object_b", "Object B", &both).group("Spring"))
        .param(
            ParamDesc::float("rest_length", "Rest Length", 100.0)
                .range(0.0, BIG)
                .unit(Unit::Distance)
                .group("Spring"),
        )
        .param(
            ParamDesc::float("stiffness", "Stiffness", 1.0)
                .range(0.0, BIG)
                .group("Spring"),
        )
        .param(pct("damping", "Damping", 0.2, "Spring"))
}

/// Installs the dynamics node types.
pub struct DynamicsToolset;

impl Plugin for DynamicsToolset {
    fn id(&self) -> &str {
        "forge.dynamics-toolset"
    }

    fn register(&self, r: &mut Registry) -> Result<(), RegistryError> {
        r.register(rigid_body_type())?;
        r.register(settings_type())?;
        r.register(connector_type())?;
        r.register(spring_type())?;
        Ok(())
    }
}

// --- parameter reading with the descriptor default as fallback ---

pub(crate) fn float(scene: &Scene, id: NodeId, key: &str) -> f64 {
    match scene.param(id, key) {
        Some(Value::Float(v)) => *v,
        Some(Value::Int(i)) => *i as f64,
        _ => 0.0,
    }
}

pub(crate) fn boolean(scene: &Scene, id: NodeId, key: &str) -> bool {
    matches!(scene.param(id, key), Some(Value::Bool(true)))
}

pub(crate) fn choice<'a>(scene: &'a Scene, id: NodeId, key: &str) -> &'a str {
    match scene.param(id, key) {
        Some(Value::Enum(s)) => s.as_str(),
        _ => "",
    }
}

pub(crate) fn vec3(scene: &Scene, id: NodeId, key: &str) -> [f64; 3] {
    match scene.param(id, key) {
        Some(Value::Vec3(v)) => *v,
        _ => [0.0; 3],
    }
}

pub(crate) fn link(scene: &Scene, id: NodeId, key: &str) -> Option<NodeId> {
    match scene.param(id, key) {
        Some(Value::Link(l)) => *l,
        _ => None,
    }
}
