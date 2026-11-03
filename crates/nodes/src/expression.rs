//! The expression tag: a scene node that owns a graph and writes parameters each
//! evaluation (docs/specs/NODES.md section 5).

use forge_core::{
    Category, NodeId, NodeType, ParamDesc, Plugin, Registry, RegistryError, Scene, Value,
};

use crate::graph::{Graph, GraphError};
use crate::registry::{EvalEnv, NodeRegistry, ParamWrite};

/// Node type id of the expression tag.
pub const TAG: &str = "forge.expression";
/// The parameter that holds the graph JSON.
pub const GRAPH_PARAM: &str = "graph";

pub fn tag_type() -> NodeType {
    NodeType::new(TAG, "Expression", Category::Tag)
        .param(ParamDesc::bool("enabled", "Enabled", true))
        .param(ParamDesc::string(GRAPH_PARAM, "Graph", &Graph::new().to_json()).hidden())
}

/// Installs the expression tag type.
pub struct NodesToolset;

impl Plugin for NodesToolset {
    fn id(&self) -> &str {
        "forge.nodes"
    }

    fn register(&self, registry: &mut Registry) -> Result<(), RegistryError> {
        registry.register(tag_type())
    }
}

/// Something that went wrong with one expression tag during a pass. Never fatal.
#[derive(Debug, Clone, PartialEq)]
pub struct Diagnostic {
    pub tag: NodeId,
    pub message: String,
}

/// The graph stored in an expression tag.
pub fn load_graph(scene: &Scene, tag: NodeId) -> Result<Graph, String> {
    let node = scene
        .node(tag)
        .ok_or_else(|| format!("node {tag} does not exist"))?;
    if node.type_id != TAG {
        return Err(format!("node {tag} is not an expression tag"));
    }
    match node.params.get(GRAPH_PARAM) {
        Some(Value::String(text)) => Graph::from_json(text).map_err(|e: GraphError| e.to_string()),
        _ => Err(format!("node {tag} has no graph parameter")),
    }
}

/// True if the expression tag's graph can read the time: it has a `time.now` or a script
/// node (CORE.md section 5b). A tag whose graph cannot be read counts as not reading it.
pub fn tag_reads_time(scene: &Scene, tag: NodeId) -> bool {
    load_graph(scene, tag).is_ok_and(|g| {
        g.nodes()
            .any(|n| n.type_id == "time.now" || n.type_id == "script.rhai")
    })
}

/// Tags of a scene in document order: an object, its tags in order, then its children.
pub fn expression_tags(scene: &Scene) -> Vec<NodeId> {
    let mut out = Vec::new();
    let mut stack: Vec<NodeId> = scene.roots().iter().rev().copied().collect();
    while let Some(id) = stack.pop() {
        let Some(node) = scene.node(id) else { continue };
        out.extend(
            node.tags
                .iter()
                .copied()
                .filter(|t| scene.node(*t).is_some_and(|n| n.type_id == TAG)),
        );
        stack.extend(node.children.iter().rev());
    }
    out
}

/// Evaluates one tag against `scene`; the writes and diagnostics it produced.
pub fn evaluate_tag(
    scene: &Scene,
    registry: &NodeRegistry,
    tag: NodeId,
    seconds: f64,
) -> Result<Vec<ParamWrite>, String> {
    let graph = load_graph(scene, tag)?;
    let host = scene.node(tag).and_then(|n| n.host);
    let env = EvalEnv {
        scene: Some(scene),
        seconds,
        fps: f64::from(scene.settings().fps),
        host,
        limits: Default::default(),
    };
    graph
        .evaluate(registry, &env)
        .map(|e| e.writes)
        .map_err(|e| e.to_string())
}

/// Runs every enabled expression tag in document order, applying each tag's writes before
/// the next tag runs. Returns the changed copy of the scene and what went wrong.
pub fn apply_expressions_with(
    scene: &Scene,
    registry: &NodeRegistry,
    seconds: f64,
) -> (Scene, Vec<Diagnostic>) {
    let run = apply_expressions_in(scene.clone(), registry, seconds, &expression_tags(scene));
    (run.scene, run.diagnostics)
}

/// The outcome of [`apply_expressions_in`].
pub struct ExpressionRun {
    pub scene: Scene,
    pub diagnostics: Vec<Diagnostic>,
    /// Every (node, param) an enabled tag tried to write, in write order: the change set of
    /// the stage (a write may leave the value as it was).
    pub touched: Vec<(NodeId, String)>,
}

/// [`apply_expressions_with`] for a scene the caller owns, with the tags (in document
/// order, see [`expression_tags`]) given and the writes reported. Nodes no tag writes to
/// stay shared with the input scene.
pub fn apply_expressions_in(
    mut out: Scene,
    registry: &NodeRegistry,
    seconds: f64,
    tags: &[NodeId],
) -> ExpressionRun {
    let mut diagnostics = Vec::new();
    let mut touched = Vec::new();
    for &tag in tags {
        let enabled = !matches!(out.param(tag, "enabled"), Some(Value::Bool(false)));
        if !enabled {
            continue;
        }
        match evaluate_tag(&out, registry, tag, seconds) {
            Err(message) => diagnostics.push(Diagnostic { tag, message }),
            Ok(writes) if writes.is_empty() => {}
            Ok(writes) => {
                touched.extend(writes.iter().map(|w| (w.node, w.param.clone())));
                let overrides = writes.into_iter().map(|w| (w.node, w.param, w.value));
                for (node, param, error) in out.apply_params(overrides) {
                    diagnostics.push(Diagnostic {
                        tag,
                        message: format!("write to {param:?} of node {node} skipped: {error}"),
                    });
                }
            }
        }
    }
    ExpressionRun {
        scene: out,
        diagnostics,
        touched,
    }
}

/// [`apply_expressions_with`] using the built-in node registry.
pub fn apply_expressions(scene: &Scene, seconds: f64) -> (Scene, Vec<Diagnostic>) {
    apply_expressions_with(scene, NodeRegistry::builtin(), seconds)
}
