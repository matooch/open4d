//! The Volume Builder and Volume Mesher node types and the scene evaluation of volumes
//! (docs/specs/VOLUME.md sections 6, 7).

use std::collections::BTreeMap;
use std::sync::Arc;

use forge_core::{
    Category, EvalCtx, MeshData, NodeEvaluator, NodeId, NodeType, ParamDesc, Plugin, Registry,
    RegistryError, Scene, Unit, Value,
};
use forge_geometry::PolyMesh;
use glam::{DMat4, DVec3, IVec3};

use crate::csg::{CsgOp, combine};
use crate::filters::{self, Band, FilterKind, SmoothOp};
use crate::grid::SparseGrid;
use crate::sdf::{SdfParams, mesh_to_sdf, spline_to_sdf};

pub const BUILDER: &str = "forge.volume_builder";
pub const MESHER: &str = "forge.volume_mesher";
pub const FILTER: &str = "forge.volume_filter";

/// Why a volume could not be built.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum VolumeBuildError {
    #[error("not supported yet: {0}")]
    Unsupported(&'static str),
    #[error("{0}")]
    Failed(String),
}

/// What the values of a [`Volume`] mean.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VolumeKind {
    /// Signed distance, negative inside.
    Sdf,
    /// Density: 1 deep inside, 0 outside.
    Fog,
}

/// A built volume in the builder's local space.
#[derive(Debug, Clone)]
pub struct Volume {
    pub grid: SparseGrid<f32>,
    pub kind: VolumeKind,
    /// Builder space to the space of the builder's parent.
    pub to_parent: DMat4,
}

fn builder_type() -> NodeType {
    let range = |id: &str, label: &str| ParamDesc::int(id, label, 2).range(1, i64::from(i32::MAX));
    NodeType::new(BUILDER, "Volume Builder", Category::Generator)
        .param(
            ParamDesc::float("voxel_size", "Voxel Size", 10.0)
                .range(0.001, 1e20)
                .step(0.1)
                .unit(Unit::Distance),
        )
        .param(range("interior_voxel_range", "Interior Voxel Range"))
        .param(range("exterior_voxel_range", "Exterior Voxel Range"))
        .param(range("spline_voxel_range", "Spline Voxel Range"))
        .param(range("particle_voxel_range", "Particle Voxel Range"))
        .param(ParamDesc::enumeration(
            "volume_type",
            "Volume Type",
            "sdf",
            &[
                ("sdf", "Signed Distance Field"),
                ("fog", "Fog"),
                ("vector", "Vector"),
            ],
        ))
        .param(ParamDesc::bool(
            "auto_update_settings",
            "Auto Update Settings",
            true,
        ))
        .param(ParamDesc::bool(
            "override_grid_matrix",
            "Override Grid Matrix",
            false,
        ))
        .param(ParamDesc::link_rows(
            "objects",
            "Objects",
            &[],
            &[
                ("union", "Union"),
                ("subtract", "Subtract"),
                ("intersect", "Intersect"),
            ],
        ))
}

fn filter_type() -> NodeType {
    NodeType::new(FILTER, "Volume Filter", Category::Generator)
        .param(ParamDesc::enumeration(
            "kind",
            "Type",
            "smooth",
            &[
                ("offset", "Offset"),
                ("reshape", "Reshape"),
                ("smooth", "Smooth"),
            ],
        ))
        .param(ParamDesc::enumeration(
            "smooth_operator",
            "Operator",
            "gaussian",
            &[
                ("gaussian", "Gaussian"),
                ("mean", "Mean"),
                ("median", "Median"),
                ("laplacian", "Laplacian Flow"),
            ],
        ))
        .param(ParamDesc::float("offset", "Offset", 5.0).unit(Unit::Distance))
        .param(ParamDesc::int("iterations", "Iterations", 1).range(1, 1000))
        .param(ParamDesc::float("strength", "Strength", 1.0).range(0.0, 1.0))
        .param(ParamDesc::int("voxel_distance", "Voxel Distance", 2).range(1, 16))
        .param(ParamDesc::int("voxel_range", "Voxel Range", 3).range(1, 16))
}

fn mesher_type() -> NodeType {
    NodeType::new(MESHER, "Volume Mesher", Category::Generator)
        .param(
            ParamDesc::float("surface_threshold", "Surface Threshold", 0.01)
                .range(-1e20, 1e20)
                .step(0.01),
        )
        .param(
            ParamDesc::float("voxel_range_threshold", "Voxel Range Threshold", 0.5)
                .range(0.0, 1.0)
                .step(0.01),
        )
        .param(ParamDesc::bool(
            "use_absolute_value_iso",
            "Use Absolute Value (ISO)",
            false,
        ))
        .param(
            ParamDesc::float("adaptive", "Adaptive", 0.0)
                .range(0.0, 1.0)
                .step(0.001),
        )
        .param(ParamDesc::bool(
            "create_curvature_map",
            "Create Curvature Map",
            false,
        ))
        .param(ParamDesc::enumeration(
            "curvature_direction",
            "Curvature Direction",
            "both",
            &[
                ("both", "Both"),
                ("convex", "Convex"),
                ("concave", "Concave"),
            ],
        ))
}

/// The plugin that registers the volume node types and the mesher's evaluator.
pub struct VolumePlugin;

impl Plugin for VolumePlugin {
    fn id(&self) -> &str {
        "forge.volume"
    }

    fn register(&self, r: &mut Registry) -> Result<(), RegistryError> {
        r.register(builder_type())?;
        r.register(filter_type())?;
        r.register(mesher_type())?;
        r.set_evaluator(MESHER, Arc::new(MesherEvaluator))
    }
}

/// Typed access to a node's params with the descriptor defaults.
struct P<'a> {
    ty: NodeType,
    values: &'a BTreeMap<String, Value>,
}

impl P<'_> {
    fn get(&self, id: &str) -> Value {
        let d = self.ty.find_param(id).expect("own param");
        self.values
            .get(id)
            .and_then(|v| d.validate(v.clone()).ok())
            .unwrap_or_else(|| d.default.clone())
    }
    fn float(&self, id: &str) -> f64 {
        match self.get(id) {
            Value::Float(f) => f,
            v => unreachable!("{id}: {v:?}"),
        }
    }
    fn int(&self, id: &str) -> i64 {
        match self.get(id) {
            Value::Int(i) => i,
            v => unreachable!("{id}: {v:?}"),
        }
    }
    fn bool(&self, id: &str) -> bool {
        matches!(self.get(id), Value::Bool(true))
    }
    fn enumeration(&self, id: &str) -> String {
        match self.get(id) {
            Value::Enum(s) => s,
            v => unreachable!("{id}: {v:?}"),
        }
    }
}

/// How one list entry combines into the accumulator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Blend {
    Union,
    Subtract,
    Intersect,
}

impl Blend {
    fn op(self) -> CsgOp {
        match self {
            Blend::Union => CsgOp::Union,
            Blend::Subtract => CsgOp::Subtract,
            Blend::Intersect => CsgOp::Intersect,
        }
    }
}

/// One source of the list, in builder space.
enum Source {
    Mesh(PolyMesh),
    Spline { points: Vec<DVec3>, closed: bool },
    Volume(Box<Volume>),
    Filter(FilterSpec),
    Field(Box<dyn forge_fields::Field>),
}

/// A filter layer as read from its node.
struct FilterSpec {
    kind: FilterKind,
    iterations: u32,
}

struct Settings {
    sdf: SdfParams,
    spline_range: f64,
    fog: bool,
}

fn rows(values: &BTreeMap<String, Value>) -> Vec<forge_core::LinkRow> {
    match values.get("objects") {
        Some(Value::LinkRows(r)) => r.clone(),
        _ => Vec::new(),
    }
}

fn blend_of(row: Option<&forge_core::LinkRow>) -> Option<Blend> {
    match row {
        None => Some(Blend::Union),
        Some(r) if !r.enabled => None,
        Some(r) => Some(match r.blend.as_str() {
            "subtract" => Blend::Subtract,
            "intersect" => Blend::Intersect,
            _ => Blend::Union,
        }),
    }
}

/// Tube radius per voxel of spline range, fitted to O33 `spline-source` and `spline-radius`
/// (both give 14.7 at range 2, voxel size 10); parity-pending.
pub const SPLINE_RADIUS_PER_RANGE: f64 = 0.735;

/// The radius of the tube a spline becomes (VOLUME.md section 6.3).
fn spline_radius(s: &Settings) -> f64 {
    SPLINE_RADIUS_PER_RANGE * s.spline_range * s.sdf.voxel_size
}

fn gather(
    scene: &Scene,
    node: NodeId,
    ctx: &EvalCtx,
    to_builder: &DMat4,
    out: &mut Vec<(NodeId, Source)>,
    depth: u32,
) -> Result<(), VolumeBuildError> {
    let Some(n) = scene.node(node) else {
        return Ok(());
    };
    if !forge_core::visibility::enabled(scene, node) {
        return Ok(());
    }
    if n.type_id == FILTER {
        out.push((node, Source::Filter(filter_spec(&n.params))));
        return Ok(());
    }
    if scene
        .node_type(node)
        .is_some_and(|t| t.category == Category::Field)
    {
        let field = forge_fields::field_from_node(scene, node).map_err(|e| match e {
            forge_fields::FieldError::Unsupported(w) => VolumeBuildError::Unsupported(w),
            e => VolumeBuildError::Failed(e.to_string()),
        })?;
        out.push((node, Source::Field(field)));
        return Ok(());
    }
    let world = scene
        .world_matrix(node)
        .ok_or_else(|| VolumeBuildError::Failed("a list entry has no transform".into()))?;
    let m = *to_builder * world;
    if n.type_id == BUILDER {
        if depth > 8 {
            return Err(VolumeBuildError::Failed("builders nested too deep".into()));
        }
        let vol = build_volume_inner(scene, node, ctx, depth + 1)?;
        out.push((node, Source::Volume(Box::new(vol))));
        return Ok(());
    }
    if let Some(line) = forge_generators::splines::polyline_mesh(scene, node) {
        let closed = line.points.len() > 2 && line.polys.len() == line.points.len();
        let points = line.points.iter().map(|p| m.transform_point3(*p)).collect();
        out.push((node, Source::Spline { points, closed }));
        return Ok(());
    }
    match forge_generators::evaluate(scene, node, ctx) {
        Ok(Some(mut mesh)) => {
            for p in &mut mesh.points {
                *p = m.transform_point3(*p);
            }
            out.push((node, Source::Mesh(mesh)));
            Ok(())
        }
        Ok(None) => {
            // no geometry of its own (a Null): its children are entries too
            for c in n.children.clone() {
                gather(scene, c, ctx, to_builder, out, depth)?;
            }
            Ok(())
        }
        Err(forge_generators::BuildError::Unsupported(w)) => Err(VolumeBuildError::Unsupported(w)),
        Err(e) => Err(VolumeBuildError::Failed(e.to_string())),
    }
}

fn filter_spec(values: &BTreeMap<String, Value>) -> FilterSpec {
    let p = P {
        ty: filter_type(),
        values,
    };
    let distance = p.float("offset");
    let kind = match p.enumeration("kind").as_str() {
        "offset" => FilterKind::Offset { distance },
        "reshape" => FilterKind::Reshape { distance },
        _ => FilterKind::Smooth {
            op: match p.enumeration("smooth_operator").as_str() {
                "mean" => SmoothOp::Mean,
                "median" => SmoothOp::Median,
                "laplacian" => SmoothOp::Laplacian,
                _ => SmoothOp::Gaussian,
            },
            width: p.int("voxel_distance") as u32,
            strength: p.float("strength"),
        },
    };
    FilterSpec {
        kind,
        iterations: p.int("iterations") as u32,
    }
}

/// Applies a field layer to the accumulated distance field (VOLUME.md section 9).
fn apply_field(
    grid: &SparseGrid<f32>,
    field: &dyn forge_fields::Field,
    mode: filters::FieldMode,
    strength: f64,
    builder_world: &DMat4,
    ctx: &EvalCtx,
    s: &Settings,
) -> SparseGrid<f32> {
    let voxels = grid.active_voxels();
    let points: Vec<DVec3> = voxels
        .iter()
        .map(|(i, _)| builder_world.transform_point3(grid.index_to_world(*i)))
        .collect();
    let values: Vec<f64> = field
        .sample_many(&points, ctx)
        .iter()
        .map(|v| v.value)
        .collect();
    filters::apply_field_values(grid, &voxels, &values, mode, strength, s.sdf.voxel_size)
}

/// Resamples a volume onto another lattice (trilinear) within its active bounds.
fn resample(v: &Volume, to: &SdfParams) -> SparseGrid<f32> {
    let mut out = to.empty_grid();
    let src = &v.grid;
    let Some((lo, hi)) = src.bounds() else {
        return out;
    };
    let wlo = src.index_to_world(lo);
    let whi = src.index_to_world(hi);
    let a = ((wlo - to.origin) / to.voxel_size).floor().as_ivec3();
    let b = ((whi - to.origin) / to.voxel_size).ceil().as_ivec3();
    for x in a.x..=b.x {
        for y in a.y..=b.y {
            for z in a.z..=b.z {
                let i = IVec3::new(x, y, z);
                let w = out.index_to_world(i);
                let val = src.trilinear(w);
                if val.abs() <= to.exterior_band().max(to.interior_band()) as f32
                    && val != src.background()
                {
                    out.set(i, val);
                }
            }
        }
    }
    out.signed_flood_fill(to.interior_band() as f32);
    out
}

fn settings(p: &P) -> Result<Settings, VolumeBuildError> {
    if p.bool("override_grid_matrix") {
        return Err(VolumeBuildError::Unsupported("Override Grid Matrix"));
    }
    let fog = match p.enumeration("volume_type").as_str() {
        "sdf" => false,
        "fog" => true,
        _ => return Err(VolumeBuildError::Unsupported("Vector volumes")),
    };
    Ok(Settings {
        sdf: SdfParams {
            voxel_size: p.float("voxel_size"),
            origin: DVec3::ZERO,
            exterior_range: p.int("exterior_voxel_range") as f64,
            interior_range: p.int("interior_voxel_range") as f64,
        },
        spline_range: p.int("spline_voxel_range") as f64,
        fog,
    })
}

/// Fog density of a distance field (VOLUME.md section 6.4): 0 up to `FOG_OFFSET` voxels inside the
/// surface, rising linearly over `FOG_RAMP` voxels to 1. Fitted to O33 `fog-input` (one case,
/// two constants); parity-pending.
pub fn sdf_to_fog(grid: &SparseGrid<f32>, interior_band: f32) -> SparseGrid<f32> {
    let vox = grid.voxel_size() as f32;
    let (off, ramp) = (FOG_OFFSET as f32 * vox, FOG_RAMP as f32 * vox);
    // samples at the interior background (deeper than the band) are full density
    grid.map(0.0, |v| {
        if v <= -interior_band {
            1.0
        } else {
            ((-v - off) / ramp).clamp(0.0, 1.0)
        }
    })
}

/// Inside distance, in voxels, where fog density starts.
pub const FOG_OFFSET: f64 = 0.7;
/// Voxels over which fog density rises from 0 to 1.
pub const FOG_RAMP: f64 = 4.0;

fn build_volume_inner(
    scene: &Scene,
    builder: NodeId,
    ctx: &EvalCtx,
    depth: u32,
) -> Result<Volume, VolumeBuildError> {
    let node = scene
        .node(builder)
        .filter(|n| n.type_id == BUILDER)
        .ok_or_else(|| VolumeBuildError::Failed("not a volume builder".into()))?;
    let p = P {
        ty: builder_type(),
        values: &node.params,
    };
    let s = settings(&p)?;
    let world = scene
        .world_matrix(builder)
        .ok_or_else(|| VolumeBuildError::Failed("the builder has no transform".into()))?;
    let to_builder = world.inverse();
    let overrides = rows(&node.params);
    let mut entries = Vec::new();
    for c in node.children.clone() {
        gather(scene, c, ctx, &to_builder, &mut entries, depth)?;
    }
    let band = Band {
        exterior: s.sdf.exterior_band(),
        interior: s.sdf.interior_band(),
    };
    // bottom to top: a layer acts on everything below it (VOLUME.md section 6.2)
    let mut acc: Option<SparseGrid<f32>> = None;
    for (id, source) in entries.into_iter().rev() {
        // a row keyed by the direct child; entries below a Null have no row of their own
        let row = overrides.iter().find(|r| r.target == id);
        let Some(blend) = blend_of(row) else { continue };
        let grid = match source {
            Source::Filter(f) => {
                if let Some(a) = acc.take() {
                    acc = Some(filters::apply(&a, f.kind, f.iterations, band));
                }
                continue;
            }
            Source::Field(field) => {
                if let Some(a) = acc.take() {
                    let mode = match row.map(|r| r.blend.as_str()) {
                        Some("subtract") => filters::FieldMode::Subtract,
                        Some("intersect") => filters::FieldMode::Multiply,
                        _ => filters::FieldMode::Add,
                    };
                    let strength = row.map_or(1.0, |r| r.strength);
                    acc = Some(apply_field(
                        &a,
                        field.as_ref(),
                        mode,
                        strength,
                        &world,
                        ctx,
                        &s,
                    ));
                }
                continue;
            }
            Source::Mesh(m) => mesh_to_sdf(&m, &s.sdf),
            Source::Spline { points, closed } => {
                spline_to_sdf(&points, closed, spline_radius(&s), &s.sdf)
            }
            Source::Volume(v) => {
                let same =
                    v.grid.same_lattice(&s.sdf.empty_grid()) && v.to_parent == DMat4::IDENTITY;
                if same { v.grid } else { resample(&v, &s.sdf) }
            }
        };
        acc = Some(match acc {
            None => grid,
            Some(a) => combine(&a, &grid, blend.op())
                .map_err(|e| VolumeBuildError::Failed(e.to_string()))?,
        });
    }
    let sdf = acc.unwrap_or_else(|| s.sdf.empty_grid());
    let (grid, kind) = if s.fog {
        (
            sdf_to_fog(&sdf, s.sdf.interior_band() as f32),
            VolumeKind::Fog,
        )
    } else {
        (sdf, VolumeKind::Sdf)
    };
    let to_parent = scene
        .local_transform(builder)
        .map(|t| t.matrix())
        .unwrap_or(DMat4::IDENTITY);
    Ok(Volume {
        grid,
        kind,
        to_parent,
    })
}

/// Builds the volume of a builder node (VOLUME.md section 6).
pub fn build_volume(
    scene: &Scene,
    builder: NodeId,
    ctx: &EvalCtx,
) -> Result<Volume, VolumeBuildError> {
    build_volume_inner(scene, builder, ctx, 0)
}

#[derive(Debug)]
struct MesherEvaluator;

impl NodeEvaluator for MesherEvaluator {
    fn evaluate(
        &self,
        scene: &Scene,
        id: NodeId,
        ctx: &EvalCtx,
    ) -> Result<Option<MeshData>, String> {
        mesh_volume_node(scene, id, ctx)
            .map(|m| {
                m.map(|m| MeshData {
                    points: m.points.iter().map(|p| p.to_array()).collect(),
                    polys: m.polys,
                })
            })
            .map_err(|e| e.to_string())
    }
}

/// The mesh of a mesher node: its first builder child, built and meshed, in the mesher's
/// space. `Ok(None)` without a builder child.
pub fn mesh_volume_node(
    scene: &Scene,
    mesher: NodeId,
    ctx: &EvalCtx,
) -> Result<Option<PolyMesh>, VolumeBuildError> {
    let Some(node) = scene.node(mesher) else {
        return Ok(None);
    };
    let Some(builder) = node
        .children
        .iter()
        .copied()
        .find(|c| scene.node(*c).is_some_and(|n| n.type_id == BUILDER))
    else {
        return Ok(None);
    };
    let p = P {
        ty: mesher_type(),
        values: &node.params,
    };
    if p.bool("create_curvature_map") {
        return Err(VolumeBuildError::Unsupported("Create Curvature Map"));
    }
    let volume = build_volume(scene, builder, ctx)?;
    let opts = crate::mesher::MeshOptions {
        iso: p.float("surface_threshold"),
        absolute: p.bool("use_absolute_value_iso"),
        adaptive: p.float("adaptive"),
    };
    let mut mesh = crate::mesher::mesh_volume(&volume, &opts);
    for pt in &mut mesh.points {
        *pt = volume.to_parent.transform_point3(*pt);
    }
    Ok(Some(mesh))
}
