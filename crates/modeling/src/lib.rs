//! forge-modeling: modeling tools as pure functions on polygon meshes. Spec:
//! docs/specs/MODELING.md.

mod bevel;
mod desc;
mod extrude;
mod settings;

pub use bevel::bevel;
pub use desc::{ToolDesc, tool_desc, tool_descs};
pub use extrude::{extrude, extrude_inner};
pub use settings::{Settings, SettingsError};

/// The pure function of a tool (docs/specs/MODELING.md §0). It reads the part of the
/// selection it works on (polygons for Extrude, edges or polygons for Bevel).
pub type ToolFn = fn(
    &forge_geometry::PolyMesh,
    &forge_geometry::selection::Selection,
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
    #[error("nothing is selected")]
    EmptySelection,
    #[error("polygon {0} does not exist")]
    BadPolygon(u32),
    #[error("edge {0} does not exist")]
    BadEdge(u32),
    #[error("point {0} does not exist")]
    BadPoint(u32),
    #[error("unsupported: {0}")]
    Unsupported(String),
}

fn run_extrude(
    mesh: &forge_geometry::PolyMesh,
    sel: &forge_geometry::selection::Selection,
    s: &Settings,
) -> Result<
    (
        forge_geometry::PolyMesh,
        forge_geometry::selection::Selection,
    ),
    ToolError,
> {
    extrude(mesh, &sel.polys, s)
}

fn run_extrude_inner(
    mesh: &forge_geometry::PolyMesh,
    sel: &forge_geometry::selection::Selection,
    s: &Settings,
) -> Result<
    (
        forge_geometry::PolyMesh,
        forge_geometry::selection::Selection,
    ),
    ToolError,
> {
    extrude_inner(mesh, &sel.polys, s)
}

/// The pure function of the tool with this id (`inset` is accepted for `extrude_inner`).
pub fn tool_fn(id: &str) -> Option<ToolFn> {
    match id {
        "extrude" => Some(run_extrude),
        "extrude_inner" | "inset" => Some(run_extrude_inner),
        "bevel" => Some(bevel),
        _ => None,
    }
}
