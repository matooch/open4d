//! Dynamics: rigid bodies on rapier, connectors and springs, soft bodies (docs/specs/DYNAMICS.md).

pub mod apply;
pub mod build;
pub mod cache;
pub mod clones;
pub mod joints;
pub mod nodes;
pub mod settings;
pub mod sim;
pub mod soft;

pub use cache::DynamicsCache;
pub use nodes::{
    CONNECTOR, DynamicsToolset, RIGID_BODY, SETTINGS, SPRING, connector_type, rigid_body_type,
    settings_type, spring_type,
};
pub use settings::DynamicsSettings;
pub use sim::{BodyKey, BodyKind, BodySpec, DynamicsError, MassSpec, ShapeKind, Simulation};
