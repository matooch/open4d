//! The Tracer: a spline made from the positions of other objects over time (clone-toolset
//! spec T1-T3). Trace Paths follows every vertex (or origin, or clone) of the linked objects
//! through the frames stepped so far; Connect All Objects joins their current positions.

use forge_core::{Category, EvalCtx, NodeId, NodeType, ParamDesc, Scene, Value};
use forge_particles::PointSource;
use glam::{DMat4, DVec3};

use crate::spline_sampling::{Interpolation, Sampling, add_params};
use crate::splines::Curve;
use crate::{BuildError, Params, evaluate};

pub const TYPE: &str = "forge.tracer";

pub(crate) fn node_type() -> NodeType {
    let t = NodeType::new(TYPE, "Tracer", Category::Generator)
        .menu_path("Generator", 0)
        .param(ParamDesc::enumeration(
            "tracing_mode",
            "Tracing mode",
            "trace_paths",
            &[
                ("trace_paths", "Trace Paths"),
                ("trace_single", "Trace Single Object"),
                ("connect_all", "Connect All Objects"),
            ],
        ))
        .param(ParamDesc::int("sample_step", "Sample step", 1).range(1, 100_000))
        .param(ParamDesc::bool("trace_active", "Trace active", true))
        .param(ParamDesc::bool("trace_vertices", "Trace vertices", true))
        .param(ParamDesc::bool("use_subgroups", "Use subgroups", true))
        .param(ParamDesc::enumeration(
            "handle_cloners",
            "Handle cloners",
            "nodes_only",
            &[
                ("nodes_only", "Nodes Only"),
                ("nodes_and_edges", "Nodes and Edges"),
            ],
        ))
        .param(ParamDesc::bool("include_cloner", "Include cloner", false))
        .param(ParamDesc::enumeration(
            "space",
            "Space",
            "global",
            &[("global", "Global"), ("local", "Local")],
        ))
        .param(ParamDesc::enumeration(
            "limit",
            "Limit",
            "none",
            &[("none", "None"), ("from", "From"), ("to", "To")],
        ))
        .param(ParamDesc::enumeration(
            "spline_type",
            "Type",
            "linear",
            &[
                ("linear", "Linear"),
                ("cubic", "Cubic"),
                ("bezier", "Bezier"),
                ("b_spline", "B-Spline"),
                ("akima", "Akima"),
            ],
        ))
        .param(ParamDesc::bool("close_spline", "Close spline", false))
        .param(ParamDesc::bool(
            "reverse_sequence",
            "Reverse sequence",
            false,
        ))
        .param(ParamDesc::link_list(
            "trace_links",
            "Trace links",
            &[Category::Object, Category::Generator],
        ));
    add_params(t, Interpolation::None)
}

/// The nodes of a traced object at the scene state `scene` (world space): its vertices
/// (Trace Vertices), its origin, or its clones.
fn nodes_at(
    scene: &Scene,
    id: NodeId,
    vertices: bool,
    ctx: &EvalCtx,
) -> Result<Vec<DVec3>, BuildError> {
    let world = scene.world_matrix(id).unwrap_or(DMat4::IDENTITY);
    let type_id = scene
        .node(id)
        .map(|n| n.type_id.clone())
        .unwrap_or_default();
    if type_id == forge_clone_engine::MATRIX || type_id == forge_clone_engine::CLONER {
        let data = if type_id == forge_clone_engine::MATRIX {
            crate::matrix_data(scene, id, ctx)?
        } else {
            crate::clone_data(scene, id, ctx)?
        };
        return Ok(data
            .map(|d| {
                (0..d.len())
                    .filter(|i| d.visible[*i])
                    .map(|i| world.transform_point3(d.matrices[i].w_axis.truncate()))
                    .collect()
            })
            .unwrap_or_default());
    }
    if vertices && let Some(mesh) = evaluate(scene, id, ctx)? {
        return Ok(mesh
            .points
            .iter()
            .map(|p| world.transform_point3(*p))
            .collect());
    }
    Ok(vec![world.transform_point3(DVec3::ZERO)])
}

/// The frames Trace Paths samples when the document is at `frame`: the stepped frames 1..=frame
/// that are the first or a multiple of Sample Step (`tracer/interval`: 1, 5, 10, ... 30).
pub fn sampled_frames(frame: i64, step: i64) -> Vec<i64> {
    (1..=frame).filter(|f| *f == 1 || f % step == 0).collect()
}

/// The Tracer's curve in its own frame; `None` when there is nothing to trace yet.
pub fn curve_of(scene: &Scene, id: NodeId, ctx: &EvalCtx) -> Result<Option<Curve>, BuildError> {
    let Some(node) = scene.node(id) else {
        return Ok(None);
    };
    let ty = node_type();
    let p = Params {
        ty: &ty,
        values: &node.params,
    };
    let sampling = Sampling::from_params(&p);
    match p.enumeration("tracing_mode").as_str() {
        "trace_single" => return Err(BuildError::Unsupported("Tracer mode Trace Single Object")),
        "connect_all" | "trace_paths" => {}
        _ => unreachable!("enum"),
    }
    if p.enumeration("spline_type") != "linear" {
        return Err(BuildError::Unsupported(
            "Tracer spline types other than Linear",
        ));
    }
    if p.enumeration("space") != "global" {
        return Err(BuildError::Unsupported("Tracer space Local"));
    }
    if p.enumeration("limit") != "none" {
        return Err(BuildError::Unsupported("Tracer Limit From and To"));
    }
    if p.enumeration("handle_cloners") != "nodes_only" {
        return Err(BuildError::Unsupported("Tracer cloner edges"));
    }
    let links: Vec<NodeId> = match node.params.get("trace_links") {
        Some(Value::LinkList(l)) => l
            .iter()
            .copied()
            .filter(|l| *l != id && scene.node(*l).is_some())
            .collect(),
        _ => Vec::new(),
    };
    // Emitters are traced by particle id (DYNAMICS.md section 11), the other links by node.
    let (emitters, links): (Vec<NodeId>, Vec<NodeId>) = links
        .into_iter()
        .partition(|l| forge_particles::is_emitter(scene, *l));
    let vertices = p.bool("trace_vertices");
    let closed = p.bool("close_spline");
    let reverse = p.bool("reverse_sequence");
    // The curve is placed by the Tracer's local transform afterwards, so the points are
    // expressed in its local frame: the world positions of the traced nodes come out unchanged.
    let to_local = {
        let local = scene.local_transform(id).expect("node").matrix();
        let parent = node
            .parent
            .and_then(|q| scene.world_matrix(q))
            .unwrap_or(DMat4::IDENTITY);
        local.inverse() * parent.inverse()
    };
    let mut chains: Vec<Vec<DVec3>> = Vec::new();
    let tracing_paths = p.enumeration("tracing_mode") != "connect_all";
    if !tracing_paths {
        let mut line = Vec::new();
        for l in &links {
            line.extend(nodes_at(scene, *l, false, ctx)?);
        }
        let frame = (ctx.time * ctx.fps).round() as i64;
        for e in &emitters {
            if let Some(mut source) = forge_particles::EmitterPoints::new(scene, *e) {
                line.extend(source.points_at(frame).iter().map(|q| q.position));
            }
        }
        chains.push(line);
    } else {
        let frame = (ctx.time * ctx.fps).round() as i64;
        let step = p.int("sample_step").max(1);
        for f in sampled_frames(frame, step as i64) {
            let c = EvalCtx::at_frame(f, ctx.fps);
            let at = scene_at_exact(scene, c.time);
            for (k, l) in links.iter().enumerate() {
                // The nodes of every link, in link order, one chain per node.
                let nodes = nodes_at(&at, *l, vertices, &c)?;
                let base: usize = links[..k]
                    .iter()
                    .map(|q| chain_count(scene, *q, vertices))
                    .sum();
                if chains.len() < base + nodes.len() {
                    chains.resize(base + nodes.len(), Vec::new());
                }
                for (i, n) in nodes.into_iter().enumerate() {
                    chains[base + i].push(n);
                }
            }
        }
    }
    if tracing_paths {
        let frame = (ctx.time * ctx.fps).round() as i64;
        for e in &emitters {
            let Some(mut source) = forge_particles::EmitterPoints::new(scene, *e) else {
                continue;
            };
            let mut by_id: std::collections::BTreeMap<u64, Vec<DVec3>> = Default::default();
            for f in sampled_frames(frame, p.int("sample_step").max(1) as i64) {
                for q in source.points_at(f) {
                    by_id.entry(q.id).or_default().push(q.position);
                }
            }
            chains.extend(by_id.into_values());
        }
    }
    let mut curves: Vec<Curve> = chains
        .into_iter()
        .filter(|c| c.len() >= 2)
        .map(|mut c| {
            if reverse {
                c.reverse();
            }
            let c: Vec<DVec3> = c.iter().map(|q| to_local.transform_point3(*q)).collect();
            Curve::polyline(&c, closed, sampling)
        })
        .collect();
    if curves.is_empty() {
        return Ok(None);
    }
    let mut main = curves.remove(0);
    main.extra = curves;
    Ok(Some(main))
}

/// How many chains a link makes (its node count at frame 0 of the unanimated scene).
fn chain_count(scene: &Scene, id: NodeId, vertices: bool) -> usize {
    nodes_at(scene, id, vertices, &EvalCtx::default()).map_or(1, |n| n.len())
}

/// The scene at `time` with float and vector-component tracks read at exactly that time:
/// the Tracer samples the stepped frames themselves, not the document time rounded to a
/// millisecond (`tracer/vertices`: the cube is at 10.000106 at frame 1, not at 9.9).
fn scene_at_exact(scene: &Scene, time: f64) -> Scene {
    use forge_core::{TrackTarget, parse_track_target};
    let mut at = forge_anim::scene_at(scene, time);
    let mut overrides: Vec<(NodeId, String, Value)> = Vec::new();
    for id in forge_anim::track_nodes(scene) {
        let (Some(node), Some(ty)) = (scene.node(id), scene.node_type(id)) else {
            continue;
        };
        for (key, track) in &node.tracks {
            if track.quaternion {
                continue;
            }
            match parse_track_target(key, |q| ty.find_param(q)) {
                Ok(TrackTarget::Float(q)) => overrides.push((
                    id,
                    q,
                    Value::Float(forge_anim::value_at_exact(track, &[], time)),
                )),
                Ok(TrackTarget::Component(q, axis)) => {
                    let current = overrides
                        .iter()
                        .rev()
                        .find(|(i, n, _)| *i == id && *n == q)
                        .map(|(_, _, v)| v.clone())
                        .or_else(|| at.param(id, &q).cloned());
                    if let Some(Value::Vec3(mut c)) = current {
                        c[axis] = forge_anim::value_at_exact(track, &[], time);
                        overrides.push((id, q, Value::Vec3(c)));
                    }
                }
                _ => {}
            }
        }
    }
    at.apply_params(overrides);
    at
}
