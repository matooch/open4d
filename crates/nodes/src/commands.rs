//! The `graph.*` commands (docs/specs/NODES.md section 6). Every edit rewrites the tag's
//! `graph` parameter, so it is one undo step and goes through the normal validation.

use std::collections::BTreeMap;

use forge_command::{CommandContext, CommandError, CommandRegistry, CommandSpec};
use forge_core::{NodeId, Value};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value as Json, json};

use crate::expression::{GRAPH_PARAM, load_graph};
use crate::graph::{Graph, GraphError, GraphNodeId, PortRef};
use crate::nodes::value_to_port;
use crate::ports::{PortType, PortValue};
use crate::registry::{EvalEnv, NodeRegistry};

fn spec(
    id: &'static str,
    label: &'static str,
    undoable: bool,
    sample: fn() -> Json,
) -> CommandSpec {
    CommandSpec {
        id,
        label,
        undoable,
        sample,
    }
}

fn graph_error(e: GraphError) -> CommandError {
    CommandError::Failed(e.to_string())
}

fn read(ctx: &dyn CommandContext, tag: u64) -> Result<Graph, CommandError> {
    load_graph(ctx.doc().scene(), NodeId(tag)).map_err(CommandError::Failed)
}

/// Stores the edited graph as one undo step.
fn commit(
    ctx: &mut dyn CommandContext,
    tag: u64,
    label: &str,
    graph: &Graph,
) -> Result<(), CommandError> {
    let text = graph.to_json();
    ctx.doc_mut()
        .transact(label, |tx| {
            tx.set_param(NodeId(tag), GRAPH_PARAM, Value::String(text))
        })
        .map_err(CommandError::from)
}

fn registry() -> &'static NodeRegistry {
    NodeRegistry::builtin()
}

fn port_json(node: u64, port: &str) -> Json {
    json!({"node": node, "port": port})
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct AddNodeArgs {
    /// The expression tag that owns the graph.
    tag: u64,
    /// Node type id, e.g. `math.add` (see `graph.node_types`).
    #[serde(rename = "type")]
    type_id: String,
    /// Constants of unconnected inputs by port id.
    values: Option<BTreeMap<String, Json>>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct RemoveArgs {
    tag: u64,
    /// Graph node id.
    node: u64,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct ConnectArgs {
    tag: u64,
    from_node: u64,
    /// Output port id.
    from_port: String,
    to_node: u64,
    /// Input port id.
    to_port: String,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct DisconnectArgs {
    tag: u64,
    to_node: u64,
    to_port: String,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct SetValueArgs {
    tag: u64,
    node: u64,
    /// Input port id.
    port: String,
    /// A number, bool, `[x, y, z]`, 16 numbers, string or node id, by the port type.
    value: Json,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct TagArgs {
    tag: u64,
}

/// Registers the `graph.*` commands.
pub fn register_commands(r: &mut CommandRegistry) -> Result<(), CommandError> {
    r.register_fn(
        spec(
            "graph.add_node",
            "Add Graph Node",
            true,
            || json!({"tag": 1, "type": "math.add", "values": {"b": 2.0}}),
        ),
        add_node,
    )?;
    r.register_fn(
        spec(
            "graph.remove",
            "Remove Graph Node",
            true,
            || json!({"tag": 1, "node": 1}),
        ),
        remove,
    )?;
    r.register_fn(
        spec(
            "graph.connect",
            "Connect Graph Ports",
            true,
            || json!({"tag": 1, "from_node": 1, "from_port": "out", "to_node": 2, "to_port": "a"}),
        ),
        connect,
    )?;
    r.register_fn(
        spec(
            "graph.disconnect",
            "Disconnect Graph Port",
            true,
            || json!({"tag": 1, "to_node": 2, "to_port": "a"}),
        ),
        disconnect,
    )?;
    r.register_fn(
        spec(
            "graph.set_value",
            "Set Graph Value",
            true,
            || json!({"tag": 1, "node": 1, "port": "b", "value": 2.5}),
        ),
        set_value,
    )?;
    r.register_fn(
        spec("graph.get", "Get Graph", false, || json!({"tag": 1})),
        get,
    )?;
    r.register_fn(
        spec("graph.node_types", "List Graph Node Types", false, || {
            json!({})
        }),
        |_, _: NoArgs| Ok(node_types()),
    )?;
    r.register_fn(
        spec(
            "graph.evaluate",
            "Evaluate Graph",
            false,
            || json!({"tag": 1}),
        ),
        evaluate,
    )
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct NoArgs {}

fn input_type(graph_node_type: &str, port: &str) -> Result<PortType, CommandError> {
    let def = registry()
        .get(graph_node_type)
        .ok_or_else(|| CommandError::Failed(format!("unknown node type {graph_node_type:?}")))?;
    def.find_input(port)
        .map(|p| p.ty)
        .ok_or_else(|| CommandError::Failed(format!("{graph_node_type} has no input {port:?}")))
}

fn add_node(ctx: &mut dyn CommandContext, a: AddNodeArgs) -> Result<Json, CommandError> {
    let mut graph = read(ctx, a.tag)?;
    let id = graph
        .add_node(registry(), &a.type_id)
        .map_err(graph_error)?;
    for (port, raw) in a.values.iter().flatten() {
        let ty = input_type(&a.type_id, port)?;
        let value = PortValue::from_json(ty, raw)
            .map_err(|e| CommandError::InvalidArgs(format!("{port}: {e}")))?;
        graph
            .set_value(registry(), id, port, value)
            .map_err(graph_error)?;
    }
    commit(ctx, a.tag, "Add Graph Node", &graph)?;
    Ok(json!({"node": id.0}))
}

fn remove(ctx: &mut dyn CommandContext, a: RemoveArgs) -> Result<Json, CommandError> {
    let mut graph = read(ctx, a.tag)?;
    let links = graph
        .remove_node(GraphNodeId(a.node))
        .map_err(graph_error)?;
    commit(ctx, a.tag, "Remove Graph Node", &graph)?;
    Ok(json!({"removed_links": links}))
}

fn connect(ctx: &mut dyn CommandContext, a: ConnectArgs) -> Result<Json, CommandError> {
    let mut graph = read(ctx, a.tag)?;
    let from = PortRef::new(GraphNodeId(a.from_node), &a.from_port);
    let to = PortRef::new(GraphNodeId(a.to_node), &a.to_port);
    let old = graph.connect(registry(), from, to).map_err(graph_error)?;
    commit(ctx, a.tag, "Connect Graph Ports", &graph)?;
    Ok(json!({"replaced": old.is_some()}))
}

fn disconnect(ctx: &mut dyn CommandContext, a: DisconnectArgs) -> Result<Json, CommandError> {
    let mut graph = read(ctx, a.tag)?;
    graph
        .disconnect(&PortRef::new(GraphNodeId(a.to_node), &a.to_port))
        .map_err(graph_error)?;
    commit(ctx, a.tag, "Disconnect Graph Port", &graph)?;
    Ok(Json::Null)
}

fn set_value(ctx: &mut dyn CommandContext, a: SetValueArgs) -> Result<Json, CommandError> {
    let mut graph = read(ctx, a.tag)?;
    let node = graph
        .node(GraphNodeId(a.node))
        .ok_or_else(|| CommandError::Failed(format!("graph node {} does not exist", a.node)))?;
    let ty = input_type(&node.type_id, &a.port)?;
    let value = PortValue::from_json(ty, &a.value)
        .map_err(|e| CommandError::InvalidArgs(format!("{}: {e}", a.port)))?;
    graph
        .set_value(registry(), GraphNodeId(a.node), &a.port, value)
        .map_err(graph_error)?;
    commit(ctx, a.tag, "Set Graph Value", &graph)?;
    Ok(Json::Null)
}

fn get(ctx: &mut dyn CommandContext, a: TagArgs) -> Result<Json, CommandError> {
    let graph = read(ctx, a.tag)?;
    let nodes: Vec<Json> = graph
        .nodes()
        .map(|n| {
            let values: serde_json::Map<String, Json> = n
                .values
                .iter()
                .map(|(k, v)| (k.clone(), v.to_json()))
                .collect();
            json!({"id": n.id.0, "type": n.type_id, "values": values})
        })
        .collect();
    let links: Vec<Json> = graph
        .links()
        .iter()
        .map(|l| {
            json!({
                "from": port_json(l.from.node.0, &l.from.port),
                "to": port_json(l.to.node.0, &l.to.port),
            })
        })
        .collect();
    let order = graph
        .topo_order()
        .ok()
        .map(|o| o.into_iter().map(|n| n.0).collect::<Vec<_>>());
    Ok(json!({"nodes": nodes, "links": links, "order": order}))
}

fn node_types() -> Json {
    let port = |p: &crate::registry::PortDecl, with_default: bool| {
        let mut o = json!({"id": p.id, "label": p.label, "type": p.ty.name()});
        if with_default {
            o["default"] = p.default.to_json();
        }
        o
    };
    let types: Vec<Json> = registry()
        .defs()
        .map(|d| {
            json!({
                "id": d.id,
                "label": d.label,
                "category": d.category,
                "sink": d.sink,
                "inputs": d.inputs.iter().map(|p| port(p, true)).collect::<Vec<_>>(),
                "outputs": d.outputs.iter().map(|p| port(p, false)).collect::<Vec<_>>(),
            })
        })
        .collect();
    json!({ "types": types })
}

/// A dry run at the timeline's current frame on the evaluated scene; changes nothing.
fn evaluate(ctx: &mut dyn CommandContext, a: TagArgs) -> Result<Json, CommandError> {
    let graph = read(ctx, a.tag)?;
    let scene = ctx.evaluated_scene();
    let fps = f64::from(scene.settings().fps);
    let frame = ctx.timeline().current;
    let host = scene.node(NodeId(a.tag)).and_then(|n| n.host);
    let env = EvalEnv {
        scene: Some(&scene),
        seconds: frame as f64 / fps,
        fps,
        host,
        limits: Default::default(),
    };
    let result = graph.evaluate_all(registry(), &env).map_err(graph_error)?;
    let outputs: serde_json::Map<String, Json> = result
        .values
        .iter()
        .map(|((n, p), v)| (format!("{n}.{p}"), v.to_json()))
        .collect();
    let writes: Vec<Json> = result
        .writes
        .iter()
        .map(|w| {
            let value = value_to_port(&w.value).map_or(Json::Null, |v| v.to_json());
            json!({"node": w.node.0, "param": w.param, "value": value})
        })
        .collect();
    let ran: Vec<u64> = result.ran.iter().map(|n| n.0).collect();
    Ok(json!({"outputs": outputs, "writes": writes, "ran": ran}))
}
