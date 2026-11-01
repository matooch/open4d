//! forge-modeling: modeling tools as pure functions on polygon meshes. Spec:
//! docs/specs/MODELING.md.

mod desc;
mod extrude;
mod settings;

pub use desc::{ToolDesc, tool_desc, tool_descs};
pub use extrude::{extrude, extrude_inner};
pub use settings::{Settings, SettingsError};

/// The pure function of a tool (docs/specs/MODELING.md §0).
pub type ToolFn = fn(
    &forge_geometry::PolyMesh,
    &forge_geometry::selection::IndexSet,
    &Settings,
) -> Result<
    (
        forge_geometry::PolyMesh,
        forge_geometry::selection::Selection,
    ),
    ToolError,
>;

/// Why a tool did not run. Nothing is changed in that case.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum ToolError {
    #[error("select at least one polygon")]
    EmptySelection,
    #[error("polygon {0} does not exist")]
    BadPolygon(u32),
    #[error("unsupported: {0}")]
    Unsupported(String),
}
