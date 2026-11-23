//! Instance Over Time: an Object reference or a sequence of sampled child transforms.
//! See the clone-toolset spec I1.

use std::cell::RefCell;

use forge_clone_engine::{CloneData, CloneItem};
use forge_core::{Category, EvalCtx, NodeId, NodeType, ParamDesc, Scene, Value};
use forge_geometry::PolyMesh;
use glam::{DMat4, DVec3};

use crate::{BuildError, PARALLEL_POINTS, Params, evaluate, expand_clones};

pub const TYPE: &str = "forge.instance_time";

pub(crate) fn node_type() -> NodeType {
    NodeType::new(TYPE, "Instance Over Time", Category::Generator)
        .menu_path("Generator", 0)
        .param(ParamDesc::enumeration(
            "mode",
            "Mode",
            "object",
            &[("object", "Object"), ("trail", "Trail")],
        ))
        .param(ParamDesc::link(
            "reference",
            "Reference",
            &[Category::Object, Category::Generator],
        ))
        .param(ParamDesc::int("start_frame", "Start Frame", 0).range(-1_000_000, 1_000_000))
        .param(ParamDesc::int("end_frame", "End Frame", 90).range(-1_000_000, 1_000_000))
        .param(ParamDesc::int("step", "Step", 1).range(1, 100_000))
}

pub fn reads_time(scene: &Scene, id: NodeId) -> bool {
    scene.node(id).is_some_and(|n| n.type_id == TYPE)
        && matches!(scene.param(id, "mode"), Some(Value::Enum(mode)) if mode == "trail")
}

pub(crate) fn trail_child(scene: &Scene, id: NodeId) -> Option<NodeId> {
    if !reads_time(scene, id) {
        return None;
    }
    scene.node(id)?.children.iter().copied().find(|child| {
        scene
            .node_type(*child)
            .is_some_and(|ty| matches!(ty.category, Category::Object | Category::Generator))
    })
}

thread_local! {
    static ACTIVE: RefCell<Vec<NodeId>> = const { RefCell::new(Vec::new()) };
}

struct Active;

impl Active {
    fn enter(id: NodeId) -> Option<Self> {
        ACTIVE.with_borrow_mut(|seen| {
            if seen.contains(&id) {
                return None;
            }
            seen.push(id);
            Some(Self)
        })
    }
}

impl Drop for Active {
    fn drop(&mut self) {
        ACTIVE.with_borrow_mut(|seen| {
            seen.pop();
        });
    }
}

fn placement(matrix: DMat4, seconds: f64) -> CloneItem {
    CloneItem {
        weight: 1.0,
        time: seconds,
        ..CloneItem::at(matrix, DVec3::ZERO)
    }
}

fn trail(
    scene: &Scene,
    id: NodeId,
    source: NodeId,
    params: &Params<'_>,
    ctx: &EvalCtx,
) -> Result<CloneData, BuildError> {
    let current = ctx.time * ctx.fps;
    if !current.is_finite() || !ctx.fps.is_finite() || ctx.fps <= 0.0 {
        return Err(BuildError::Unsupported(
            "invalid Instance Over Time time or FPS",
        ));
    }
    let frame = |key| match params.get(key) {
        Value::Int(frame) => frame,
        _ => unreachable!("integer frame parameter"),
    };
    let start = frame("start_frame");
    // Compare in seconds so division/multiplication round-off cannot drop a sample at
    // an exact frame or introduce the next sample during sub-frame evaluation.
    let next = current.ceil();
    let current_frame = if next / ctx.fps <= ctx.time {
        next
    } else {
        next - 1.0
    };
    let end = frame("end_frame").min(current_frame as i64);
    let step = params.int("step");
    let mut data = CloneData::default();
    if end >= start && (end - start) / step as i64 >= 10_000 {
        return Err(BuildError::Unsupported(
            "more than 10000 Instance Over Time samples",
        ));
    }
    let generator_world = scene.world_matrix(id).expect("generator exists");
    let to_local = generator_world.inverse();
    if !to_local.is_finite() {
        return Ok(data);
    }
    // Only the source and its ancestors can change the source's world matrix.
    let mut tracked = Vec::new();
    let mut cursor = Some(source);
    while let Some(node) = cursor.and_then(|id| scene.node(id)) {
        if !node.tracks.is_empty() {
            tracked.push(node.id);
        }
        cursor = node.parent;
    }
    for frame in (start..=end).step_by(step) {
        let time = frame as f64 / ctx.fps;
        if !time.is_finite() || !(time - ctx.time).is_finite() {
            return Err(BuildError::Unsupported(
                "invalid Instance Over Time sample time",
            ));
        }
        let (at, _) = scene.with_params(forge_anim::overrides_exact(scene, &tracked, time));
        if let Some(world) = at.world_matrix(source) {
            data.push(placement(to_local * world, time - ctx.time));
        }
    }
    Ok(data)
}

fn instances(
    scene: &Scene,
    id: NodeId,
    ctx: &EvalCtx,
) -> Result<Option<(PolyMesh, CloneData)>, BuildError> {
    let Some(node) = scene.node(id).filter(|n| n.type_id == TYPE) else {
        return Ok(None);
    };
    let Some(_active) = Active::enter(id) else {
        return Ok(None);
    };
    let ty = node_type();
    let params = Params {
        ty: &ty,
        values: &node.params,
    };
    let is_trail = reads_time(scene, id);
    let source = if is_trail {
        trail_child(scene, id)
    } else {
        params.link("reference")
    };
    let Some(source) = source else {
        return Ok(None);
    };
    let mut data = if is_trail {
        trail(scene, id, source, &params, ctx)?
    } else {
        CloneData::default()
    };
    if !is_trail {
        data.push(placement(DMat4::IDENTITY, 0.0));
    }
    if data.is_empty() {
        return Ok(None);
    }
    Ok(evaluate(scene, source, ctx)?.map(|prototype| (prototype, data)))
}

/// Instance placements in generator-local space, without expanding the prototype.
pub fn clone_data(
    scene: &Scene,
    id: NodeId,
    ctx: &EvalCtx,
) -> Result<Option<CloneData>, BuildError> {
    Ok(instances(scene, id, ctx)?.map(|(_, data)| data))
}

pub(crate) fn geometry(
    scene: &Scene,
    id: NodeId,
    ctx: &EvalCtx,
) -> Result<Option<PolyMesh>, BuildError> {
    let Some((prototype, data)) = instances(scene, id, ctx)? else {
        return Ok(None);
    };
    let points = prototype.points.len().saturating_mul(data.len());
    if points > u32::MAX as usize {
        return Err(BuildError::Unsupported(
            "too many Instance Over Time vertices",
        ));
    }
    Ok(Some(expand_clones(
        &[prototype],
        &data,
        points >= PARALLEL_POINTS,
    )))
}
