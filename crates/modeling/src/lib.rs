//! forge-modeling: modeling tools as pure functions on polygon meshes. Spec:
//! docs/specs/MODELING.md.

mod bevel;
mod build;
mod cut;
mod desc;
mod extrude;
mod merge;
mod points;
mod settings;
mod slide;
mod soft;
mod surface;
mod util;

pub use bevel::bevel;
pub use build::{
    BridgeEnd, BuildOutput, MAX_SEGMENTS, PenCorner, StitchTo, border_loop, bridge, close_hole,
    pen, stitch, stitch_edges,
};
pub use cut::{
    CutOutput, KnifeLine, MAX_CUTS, Plane, View, even_offsets, knife_line, knife_plane, loop_cut,
};
pub use desc::{
    ToolDesc, building_descs, cutting_descs, interactive_cut_descs, tool_desc, tool_descs,
};
pub use extrude::{extrude, extrude_inner};
pub use merge::{dissolve, melt};
pub use points::{
    center_axis, center_axis_tool, connect_points, disconnect, mirror, split, split_copy, weld,
};
pub use settings::{Settings, SettingsError};
pub use slide::slide;
pub use soft::{
    Axis, CURVE_LEN, ComponentPlan, ComponentSettings, Distance, Falloff, SoftSelection, Symmetry,
    distances, partners, transform_components, weights,
};
pub use surface::{optimize, reverse_normals, subdivide, triangulate, untriangulate};

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
    /// The selection or settings do not allow the command to do anything.
    #[error("{0}")]
    Invalid(String),
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
        "subdivide" => Some(subdivide),
        "triangulate" => Some(triangulate),
        "untriangulate" => Some(untriangulate),
        "reverse_normals" => Some(reverse_normals),
        "optimize" => Some(optimize),
        "weld" => Some(weld),
        "melt" => Some(melt),
        "dissolve" => Some(dissolve),
        "split" => Some(split),
        "disconnect" => Some(disconnect),
        "connect_points" => Some(connect_points),
        "mirror" => Some(mirror),
        "center_axis" => Some(center_axis_tool),
        "slide" => Some(slide),
        _ => None,
    }
}
