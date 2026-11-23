//! Native Dynamics Cache tag data and playback (DYNAMICS.md §5.2).

use forge_core::{Category, NodeId, NodeType, ParamDesc, Scene, Value, transform};

pub const TYPE: &str = "forge.dynamics_cache";
pub const MAX_FRAMES: i64 = 10_000;
pub const MAX_VALUES: usize = 8_000_000;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Rigid,
    Clones,
    Soft,
}

impl Kind {
    pub fn id(self) -> &'static str {
        match self {
            Self::Rigid => "rigid",
            Self::Clones => "clones",
            Self::Soft => "soft",
        }
    }
}

pub(crate) fn node_type() -> NodeType {
    NodeType::new(TYPE, "Dynamics Cache", Category::Tag)
        .param(ParamDesc::bool("enabled", "Enabled", true).animatable(false))
        .param(
            ParamDesc::enumeration(
                "kind",
                "Shape",
                "rigid",
                &[("rigid", "Rigid"), ("clones", "Clones"), ("soft", "Soft")],
            )
            .hidden(),
        )
        .param(
            ParamDesc::int("start_frame", "Start Frame", 0)
                .animatable(false)
                .hidden(),
        )
        .param(
            ParamDesc::int("count", "Frame Count", 0)
                .range(0, MAX_FRAMES)
                .animatable(false)
                .hidden(),
        )
        .param(
            ParamDesc::int("fps", "FPS", 30)
                .range(1, 240)
                .animatable(false)
                .hidden(),
        )
        .param(ParamDesc::float_list("samples", "Samples"))
}

pub fn tag(scene: &Scene, host: NodeId) -> Option<NodeId> {
    scene
        .node(host)?
        .tags
        .iter()
        .copied()
        .find(|id| scene.node(*id).is_some_and(|node| node.type_id == TYPE))
}

pub fn enabled(scene: &Scene, tag: NodeId) -> bool {
    matches!(scene.param(tag, "enabled"), Some(Value::Bool(true)))
        && matches!(scene.param(tag, "samples"), Some(Value::FloatList(list)) if !list.is_empty())
}

/// Per-frame data as it appears in the evaluated scene, including solver local-space conversion.
pub fn sample(scene: &Scene, host: NodeId, body: NodeId, kind: Kind) -> Option<Vec<f64>> {
    let mut frame = Vec::new();
    match kind {
        Kind::Rigid => {
            let transform = scene.local_transform(host)?;
            frame.extend(transform.pos.to_array());
            frame.extend(transform.rot.to_array());
        }
        Kind::Clones => {
            let Value::Vec3List(p) = scene.param(host, "dynamics_pos")? else {
                return None;
            };
            let Value::Vec3List(r) = scene.param(host, "dynamics_rot")? else {
                return None;
            };
            if p.is_empty() || p.len() != r.len() {
                return None;
            }
            frame.extend(p.iter().flatten().copied());
            frame.extend(r.iter().flatten().copied());
        }
        Kind::Soft => {
            let Value::Vec3List(points) = scene.param(body, "soft_points")? else {
                return None;
            };
            if points.is_empty() {
                return None;
            }
            frame.extend(points.iter().flatten().copied());
        }
    }
    frame.iter().all(|v| v.is_finite()).then_some(frame)
}

/// Overrides for one valid recorded frame. Invalid metadata/payload falls through to live.
pub fn playback(scene: &Scene, id: NodeId, frame: i64) -> Option<Vec<(NodeId, String, Value)>> {
    let tag = scene.node(id)?;
    let host = tag.host?;
    if !enabled(scene, id) {
        return None;
    }
    let int = |p| match tag.params.get(p) {
        Some(Value::Int(i)) => Some(*i),
        _ => None,
    };
    if int("fps")? != i64::from(scene.settings().fps) {
        return None;
    }
    let offset = frame.checked_sub(int("start_frame")?)?;
    let count = int("count")?;
    if count > MAX_FRAMES || !(0..count).contains(&offset) {
        return None;
    }
    let Value::FloatList(buf) = tag.params.get("samples")? else {
        return None;
    };
    let count = count as usize;
    if buf.len() > MAX_VALUES || !buf.len().is_multiple_of(count) {
        return None;
    }
    let width = buf.len() / count;
    let from = offset as usize * width;
    let values = &buf[from..from + width];
    let Value::Enum(kind) = tag.params.get("kind")? else {
        return None;
    };
    let out = |param: &str, value| (host, param.to_string(), value);
    let (vectors, remainder) = values.as_chunks::<3>();
    if vectors.is_empty() || !remainder.is_empty() || !values.iter().all(|v| v.is_finite()) {
        return None;
    }
    match kind.as_str() {
        "rigid" if vectors.len() == 2 => Some(vec![
            out(transform::POS, Value::Vec3(vectors[0])),
            out(transform::ROT, Value::Vec3(vectors[1])),
        ]),
        "clones" if vectors.len().is_multiple_of(2) => {
            let (p, r) = vectors.split_at(vectors.len() / 2);
            Some(vec![
                out("dynamics_pos", Value::Vec3List(p.to_vec())),
                out("dynamics_rot", Value::Vec3List(r.to_vec())),
            ])
        }
        "soft" => Some(vec![(
            crate::build::body_tag(scene, host)?,
            "soft_points".into(),
            Value::Vec3List(vectors.to_vec()),
        )]),
        _ => None,
    }
}
