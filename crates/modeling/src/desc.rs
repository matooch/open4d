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

/// A mesh command (docs/specs/MODELING.md §4): no drag parameter, `components` when it reads
/// points or edges.
fn command_desc(
    id: &'static str,
    label: &'static str,
    command: &'static str,
    components: bool,
    params: Vec<ParamDesc>,
) -> ToolDesc {
    ToolDesc {
        id,
        label,
        command,
        drag_param: "",
        params,
        components,
    }
}

fn command_descs() -> Vec<ToolDesc> {
    vec![
        command_desc(
            "subdivide",
            "Subdivide",
            "mesh.subdivide",
            false,
            vec![
                ParamDesc::int("subdivisions", "Subdivisions", 1).range(1, 8),
                ParamDesc::bool("hyper", "Hyper", false),
            ],
        ),
        command_desc(
            "triangulate",
            "Triangulate",
            "mesh.triangulate",
            false,
            vec![],
        ),
        command_desc(
            "untriangulate",
            "Untriangulate",
            "mesh.untriangulate",
            false,
            vec![
                ParamDesc::float("angle_deg", "Angle", 1.0).range(0.0, 90.0),
                ParamDesc::bool("ngons", "N-gons", false),
            ],
        ),
        command_desc(
            "reverse_normals",
            "Reverse Normals",
            "mesh.reverse_normals",
            false,
            vec![],
        ),
        command_desc(
            "optimize",
            "Optimize",
            "mesh.optimize",
            false,
            vec![
                ParamDesc::float("tolerance", "Tolerance", 0.01).range(0.0, BIG),
                ParamDesc::bool("optimize_points", "Optimize Points", true),
                ParamDesc::bool("optimize_polygons", "Optimize Polygons", true),
                ParamDesc::bool("delete_unused_points", "Delete Unused Points", true),
            ],
        ),
        command_desc("weld", "Weld", "mesh.weld", true, vec![]),
        command_desc("melt", "Melt", "mesh.melt", false, vec![]),
        command_desc("dissolve", "Dissolve", "mesh.dissolve", true, vec![]),
        command_desc("split", "Split", "mesh.split", false, vec![]),
        command_desc(
            "disconnect",
            "Disconnect",
            "mesh.disconnect",
            false,
            vec![ParamDesc::bool("preserve_groups", "Preserve Groups", true)],
        ),
        command_desc(
            "connect_points",
            "Connect Points/Edges",
            "mesh.connect_points",
            true,
            vec![],
        ),
        command_desc(
            "mirror",
            "Mirror",
            "mesh.mirror",
            false,
            vec![
                ParamDesc::vec3("plane_point", "Plane Point", [0.0; 3]).range(-BIG, BIG),
                ParamDesc::vec3("plane_normal", "Plane Normal", [1.0, 0.0, 0.0]).range(-BIG, BIG),
                ParamDesc::bool("duplicate", "Duplicate", false),
                ParamDesc::bool("weld", "Weld", true),
                ParamDesc::float("tolerance", "Tolerance", 0.01).range(0.0, BIG),
            ],
        ),
        command_desc(
            "center_axis",
            "Center Axis to",
            "mesh.center_axis",
            false,
            vec![ParamDesc::enumeration(
                "mode",
                "Center",
                "bounding_box",
                &[
                    ("bounding_box", "Bounding Box"),
                    ("point_average", "Point Average"),
                ],
            )],
        ),
    ]
}

/// The cutting tools that run as plain tool commands (`mesh.slide`; MODELING.md §5.3). They are
/// kept apart from [`tool_descs`] so the brief 020-022 tool list stays as it was.
pub fn cutting_descs() -> Vec<ToolDesc> {
    vec![ToolDesc {
        id: "slide",
        label: "Slide",
        command: "mesh.slide",
        drag_param: "amount",
        params: vec![
            ParamDesc::float("amount", "Amount", 0.0).range(-1.0, 1.0),
            ParamDesc::int("toward", "Slide Toward Point", -1).range(-1, 1_000_000_000),
        ],
        components: true,
    }]
}

/// The interactive cutting tools whose commands take their own arguments (`mesh.loop_cut`,
/// `mesh.knife`; MODELING.md §5.1, §5.2, §5.5). Their params are the viewport tool's settings.
pub fn interactive_cut_descs() -> Vec<ToolDesc> {
    let tool = |id, label, command, params| ToolDesc {
        id,
        label,
        command,
        drag_param: "",
        params,
        components: false,
    };
    vec![
        tool(
            "loop_cut",
            "Loop/Path Cut",
            "mesh.loop_cut",
            vec![
                ParamDesc::int("count", "Number of Cuts", 1).range(1, 64),
                ParamDesc::bool("lock_offset", "Fixed Offset", false),
                ParamDesc::float("offset", "Offset", 0.5).range(0.001, 0.999),
            ],
        ),
        tool(
            "knife_line",
            "Knife Line",
            "mesh.knife",
            vec![ParamDesc::bool("visible_only", "Visible Only", true)],
        ),
        tool(
            "knife_plane",
            "Knife Plane",
            "mesh.knife",
            vec![
                ParamDesc::vec3("plane_point", "Plane Point", [0.0; 3]).range(-BIG, BIG),
                ParamDesc::vec3("plane_normal", "Plane Normal", [1.0, 0.0, 0.0]).range(-BIG, BIG),
            ],
        ),
        tool(
            "knife_loop",
            "Knife Loop",
            "mesh.knife",
            vec![
                ParamDesc::bool("lock_offset", "Fixed Offset", false),
                ParamDesc::float("offset", "Offset", 0.5).range(0.001, 0.999),
            ],
        ),
    ]
}

/// The topology tools (`mesh.bridge`, `mesh.close_hole`, `mesh.stitch`, `mesh.pen`; MODELING.md
/// §6). Their params are the viewport tool's settings; the commands take explicit arguments.
pub fn building_descs() -> Vec<ToolDesc> {
    let tool = |id, label, command, params| ToolDesc {
        id,
        label,
        command,
        drag_param: "",
        params,
        components: false,
    };
    vec![
        tool(
            "bridge",
            "Bridge",
            "mesh.bridge",
            vec![ParamDesc::int("segments", "Segments", 1).range(1, 64)],
        ),
        tool(
            "close_hole",
            "Close Polygon Hole",
            "mesh.close_hole",
            vec![],
        ),
        tool(
            "stitch",
            "Stitch and Sew",
            "mesh.stitch",
            vec![ParamDesc::enumeration(
                "position",
                "Merge At",
                "middle",
                &[("middle", "Middle"), ("first", "First Point")],
            )],
        ),
        tool("polygon_pen", "Polygon Pen", "mesh.pen", vec![]),
    ]
}

fn step_params(copies: i64) -> Vec<ParamDesc> {
    vec![
        ParamDesc::int("copies", "Copies", copies).range(1, 1000),
        ParamDesc::vec3("offset", "Offset", [200.0, 0.0, 0.0]).range(-BIG, BIG),
        ParamDesc::vec3("rotation_deg", "Rotation", [0.0; 3]).range(-BIG, BIG),
        ParamDesc::vec3("scale", "Scale", [1.0; 3]).range(0.001, 1000.0),
        ParamDesc::enumeration(
            "mode",
            "Mode",
            "step",
            &[("step", "Per Copy"), ("total", "Total")],
        ),
    ]
}

/// The Array command (`mesh.array_command`; MODELING.md §9.6), a mesh tool: a grid of
/// instances (the original included), `offset`, `rotation_deg` and `scale` are the totals
/// reached by the last instance.
pub fn array_descs() -> Vec<ToolDesc> {
    vec![command_desc(
        "array",
        "Array",
        "mesh.array_command",
        false,
        vec![
            ParamDesc::int("count_x", "Count X", 3).range(1, 1000),
            ParamDesc::int("count_y", "Count Y", 1).range(1, 1000),
            ParamDesc::int("count_z", "Count Z", 1).range(1, 1000),
            ParamDesc::vec3("offset", "Offset", [400.0, 0.0, 0.0]).range(-BIG, BIG),
            ParamDesc::vec3("rotation_deg", "Rotation", [0.0; 3]).range(-BIG, BIG),
            ParamDesc::vec3("scale", "Scale", [1.0; 3]).range(0.001, 1000.0),
        ],
    )]
}

/// The object commands of MODELING.md §9 (`object.duplicate` ...). Their settings are the
/// command's numeric arguments, flat; `nodes`, `source` and the like are not settings.
pub fn object_descs() -> Vec<ToolDesc> {
    let plane = &[("xz", "XZ"), ("xy", "XY"), ("zy", "ZY")];
    let on = |id: &str, label: &str| ParamDesc::bool(id, label, true);
    let mut duplicate = step_params(1);
    duplicate[1] = ParamDesc::vec3("offset", "Offset", [0.0; 3]).range(-BIG, BIG);
    vec![
        command_desc(
            "duplicate",
            "Duplicate",
            "object.duplicate",
            false,
            duplicate,
        ),
        command_desc(
            "arrange",
            "Arrange",
            "object.arrange",
            false,
            vec![
                ParamDesc::enumeration(
                    "mode",
                    "Mode",
                    "line",
                    &[("line", "Line"), ("grid", "Grid"), ("circle", "Circle")],
                ),
                ParamDesc::vec3("spacing", "Spacing", [200.0, 0.0, 0.0]).range(-BIG, BIG),
                ParamDesc::vec3("row_spacing", "Row Spacing", [0.0, 0.0, 200.0]).range(-BIG, BIG),
                ParamDesc::int("columns", "Columns", 3).range(1, 1_000_000),
                ParamDesc::float("radius", "Radius", 200.0).range(0.0, BIG),
                ParamDesc::enumeration("plane", "Plane", "xz", plane),
                ParamDesc::float("start_angle_deg", "Start Angle", 0.0).range(-BIG, BIG),
            ],
        ),
        command_desc(
            "randomize",
            "Randomize",
            "object.randomize",
            false,
            vec![
                ParamDesc::int("seed", "Seed", 1).range(0, 1_000_000_000_000_i64),
                ParamDesc::vec3("position", "Position", [50.0; 3]).range(0.0, BIG),
                ParamDesc::vec3("rotation_deg", "Rotation", [0.0; 3]).range(0.0, BIG),
                ParamDesc::vec3("scale", "Scale", [0.0; 3]).range(0.0, 0.999),
                ParamDesc::bool("uniform_scale", "Uniform Scale", false),
            ],
        ),
        command_desc(
            "transfer",
            "Transfer",
            "object.transfer",
            false,
            vec![
                ParamDesc::int("target", "Target (node id)", -1).range(-1, 1_000_000_000_000_i64),
                on("position", "Position"),
                on("rotation", "Rotation"),
                on("scale", "Scale"),
                ParamDesc::enumeration(
                    "space",
                    "Space",
                    "world",
                    &[("world", "World"), ("local", "Local")],
                ),
            ],
        ),
    ]
}

/// The object command descriptor with this id (`duplicate`, `arrange`, `randomize`, `transfer`).
pub fn object_desc(id: &str) -> Option<ToolDesc> {
    object_descs().into_iter().find(|t| t.id == id)
}

/// Every tool, in menu order.
pub fn tool_descs() -> Vec<ToolDesc> {
    let mut all = vec![extrude_desc(), extrude_inner_desc(), bevel_desc()];
    all.extend(command_descs());
    all
}

/// The tool with this id (`inset` is accepted as a spelling of `extrude_inner`).
pub fn tool_desc(id: &str) -> Option<ToolDesc> {
    let id = if id == "inset" { "extrude_inner" } else { id };
    tool_descs()
        .into_iter()
        .chain(cutting_descs())
        .chain(interactive_cut_descs())
        .chain(building_descs())
        .chain(array_descs())
        .chain(object_descs())
        .find(|t| t.id == id)
}
