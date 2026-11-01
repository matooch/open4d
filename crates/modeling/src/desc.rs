//! Tool descriptors: one definition for the command schema, the attribute panel, the
//! defaults and the range checks (docs/specs/MODELING.md §0, §1.1, §2.1).

use forge_core::ParamDesc;

/// A tool: its names and its settings.
#[derive(Debug, Clone)]
pub struct ToolDesc {
    /// Short id (`extrude`, `extrude_inner`).
    pub id: &'static str,
    pub label: &'static str,
    /// The registry command that applies the tool.
    pub command: &'static str,
    /// The setting an interactive drag edits.
    pub drag_param: &'static str,
    pub params: Vec<ParamDesc>,
    /// The tool also works on edges and points (the command takes `edges` and `points`).
    pub components: bool,
}

const BIG: f64 = 1e20;

fn offset() -> ParamDesc {
    ParamDesc::float("offset", "Offset", 5.0).range(-BIG, BIG)
}

fn common_tail() -> Vec<ParamDesc> {
    vec![
        ParamDesc::float("offset_variation", "Offset Variation", 0.0).range(0.0, 1e18),
        ParamDesc::int("subdivision", "Subdivision", 0).range(0, 1_000_000),
        ParamDesc::bool("create_ngons", "Create N-gons", false),
        ParamDesc::bool("preserve_groups", "Preserve Groups", true),
    ]
}

fn extrude_desc() -> ToolDesc {
    let mut params = vec![offset()];
    params.extend(common_tail());
    params.extend([
        ParamDesc::bool("create_caps", "Create Caps", false),
        ParamDesc::float("maximum_angle_deg", "Maximum Angle", 91.0).range(0.0, 180.0),
        ParamDesc::float("rotation_deg", "Rotation", 0.0).range(-BIG, BIG),
        ParamDesc::bool("quantize_rotation", "Quantize Rotation", true),
        ParamDesc::float("quantize_angle_deg", "Quantize Angle", 5.0).range(0.0, 180.0),
        ParamDesc::float("bevel", "Bevel", 5.0).range(-BIG, BIG),
        ParamDesc::float("bevel_variation", "Bevel Variation", 0.0).range(0.0, 1e18),
        ParamDesc::bool("realtime_update", "Realtime Update", true),
    ]);
    ToolDesc {
        id: "extrude",
        label: "Extrude",
        command: "mesh.extrude",
        drag_param: "offset",
        params,
        components: false,
    }
}

fn extrude_inner_desc() -> ToolDesc {
    let mut params = vec![offset()];
    params.extend(common_tail());
    params.extend([
        ParamDesc::float("maximum_angle_deg", "Maximum Angle", 89.0).range(0.0, 180.0),
        ParamDesc::bool("realtime_update", "Realtime Update", true),
    ]);
    ToolDesc {
        id: "extrude_inner",
        label: "Extrude Inner",
        command: "mesh.extrude_inner",
        drag_param: "offset",
        params,
        components: false,
    }
}

fn bevel_desc() -> ToolDesc {
    let on = |id: &str, label: &str, default: bool| ParamDesc::bool(id, label, default);
    let params = vec![
        ParamDesc::enumeration(
            "bevel_mode",
            "Bevel Mode",
            "chamfer",
            &[("chamfer", "Chamfer"), ("solid", "Solid")],
        ),
        ParamDesc::enumeration(
            "offset_mode",
            "Offset Mode",
            "fixed_distance",
            &[
                ("fixed_distance", "Fixed Distance"),
                ("radial", "Radial"),
                ("proportional", "Proportional"),
            ],
        ),
        ParamDesc::float("offset", "Offset", 10.0).range(-BIG, BIG),
        ParamDesc::int("subdivision", "Subdivision", 0).range(0, 1_000_000),
        ParamDesc::float("depth", "Depth", 1.0).range(-1e18, 1e18),
        on("limit", "Limit", true),
        ParamDesc::enumeration(
            "shape",
            "Shape",
            "round",
            &[("round", "Round"), ("user", "User"), ("profile", "Profile")],
        ),
        ParamDesc::float("tension", "Tension", 1.0).range(-1e18, 1e18),
        ParamDesc::enumeration(
            "mitering",
            "Mitering",
            "default",
            &[
                ("default", "Default"),
                ("uniform", "Uniform"),
                ("radial", "Radial"),
                ("patch", "Patch"),
            ],
        ),
        ParamDesc::enumeration(
            "ending",
            "Ending",
            "default",
            &[
                ("default", "Default"),
                ("extend", "Extend"),
                ("inset", "Inset"),
            ],
        ),
        ParamDesc::enumeration(
            "partial_rounding",
            "Partial Rounding",
            "none",
            &[("none", "None"), ("full", "Full"), ("convex", "Convex")],
        ),
        on("corner_ngons", "Corner N-gons", false),
        on("rounding_ngons", "Rounding N-gons", false),
        on("phong_break_rounding", "Phong Break Rounding", false),
        on("phong_break_miters", "Phong Break Miters", false),
        on("realtime_update", "Realtime Update", true),
        ParamDesc::float("extrusion", "Extrusion", 10.0).range(-BIG, BIG),
    ];
    ToolDesc {
        id: "bevel",
        label: "Bevel",
        command: "mesh.bevel",
        drag_param: "offset",
        params,
        components: true,
    }
}

/// Every tool, in menu order.
pub fn tool_descs() -> Vec<ToolDesc> {
    vec![extrude_desc(), extrude_inner_desc(), bevel_desc()]
}

/// The tool with this id (`inset` is accepted as a spelling of `extrude_inner`).
pub fn tool_desc(id: &str) -> Option<ToolDesc> {
    let id = if id == "inset" { "extrude_inner" } else { id };
    tool_descs().into_iter().find(|t| t.id == id)
}
