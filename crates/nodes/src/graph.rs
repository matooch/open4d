//! The graph model, edits, topological order and evaluation (docs/specs/NODES.md section 3).

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::ports::{ConvertError, PortType, PortValue, can_convert, convert};
use crate::registry::{EvalEnv, NodeCtx, NodeRegistry, ParamWrite};

/// Stable id of a graph node; allocated from a counter that is never reused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct GraphNodeId(pub u64);

impl std::fmt::Display for GraphNodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// A port of a graph node.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct PortRef {
    pub node: GraphNodeId,
    pub port: String,
}

impl PortRef {
    pub fn new(node: GraphNodeId, port: &str) -> PortRef {
        PortRef {
            node,
            port: port.to_string(),
        }
    }
}

/// An output port feeding an input port.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Link {
    pub from: PortRef,
    pub to: PortRef,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GraphNode {
    pub id: GraphNodeId,
    pub type_id: String,
    /// Constants of unconnected inputs.
    #[serde(default)]
    pub values: BTreeMap<String, PortValue>,
}

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum GraphError {
    #[error("unknown node type {0:?}")]
    UnknownType(String),
    #[error("graph node {0} does not exist")]
    UnknownNode(GraphNodeId),
    #[error("graph node {node} has no {kind} port {port:?}")]
    UnknownPort {
        node: GraphNodeId,
        port: String,
        kind: &'static str,
    },
    #[error("cannot connect {from} to {to}")]
    TypeMismatch { from: PortType, to: PortType },
    #[error("the link would close a cycle through graph nodes {0:?}")]
    Cycle(Vec<GraphNodeId>),
    #[error("input {0:?} has no link")]
    NotLinked(String),
    #[error("invalid value: {0}")]
    InvalidValue(String),
    #[error("graph node {node}: {message}")]
    Eval { node: GraphNodeId, message: String },
    #[error("invalid graph: {0}")]
    Json(String),
}

impl From<ConvertError> for GraphError {
    fn from(e: ConvertError) -> Self {
        GraphError::InvalidValue(e.to_string())
    }
}

/// The saved form: nodes as a list ordered by id.
#[derive(Serialize, Deserialize)]
struct GraphData {
    next_id: u64,
    #[serde(default)]
    nodes: Vec<GraphNode>,
    #[serde(default)]
    links: Vec<Link>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(from = "GraphData", into = "GraphData")]
pub struct Graph {
    next_id: u64,
    nodes: BTreeMap<GraphNodeId, GraphNode>,
    /// Sorted by `to`; at most one per input.
    links: Vec<Link>,
}

impl From<GraphData> for Graph {
    fn from(d: GraphData) -> Graph {
        let nodes: BTreeMap<_, _> = d.nodes.into_iter().map(|n| (n.id, n)).collect();
        let next = nodes.keys().map(|k| k.0 + 1).max().unwrap_or(1);
        let mut g = Graph {
            next_id: d.next_id.max(next),
            nodes,
            links: d.links,
        };
        g.links.sort_by(|a, b| a.to.cmp(&b.to));
        g
    }
}

impl From<Graph> for GraphData {
    fn from(g: Graph) -> GraphData {
        GraphData {
            next_id: g.next_id,
            nodes: g.nodes.into_values().collect(),
            links: g.links,
        }
    }
}

impl Default for Graph {
    fn default() -> Graph {
        Graph::new()
    }
}

/// The result of one evaluation.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Evaluation {
    /// Output values by (node, port).
    pub values: BTreeMap<(GraphNodeId, String), PortValue>,
    /// Parameter writes requested by the nodes, in the order the nodes ran.
    pub writes: Vec<ParamWrite>,
    /// The nodes that ran, in order; each at most once.
    pub ran: Vec<GraphNodeId>,
}

impl Evaluation {
    pub fn output(&self, node: GraphNodeId, port: &str) -> Option<&PortValue> {
        self.values.get(&(node, port.to_string()))
    }

    pub fn float(&self, node: GraphNodeId, port: &str) -> Option<f64> {
        match self.output(node, port)? {
            PortValue::Float(f) => Some(*f),
            _ => None,
        }
    }
}

impl Graph {
    pub fn new() -> Graph {
        Graph {
            next_id: 1,
            nodes: BTreeMap::new(),
            links: Vec::new(),
        }
    }

    pub fn nodes(&self) -> impl Iterator<Item = &GraphNode> {
        self.nodes.values()
    }

    pub fn node(&self, id: GraphNodeId) -> Option<&GraphNode> {
        self.nodes.get(&id)
    }

    pub fn links(&self) -> &[Link] {
        &self.links
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// The link into an input, if any.
    pub fn link_into(&self, to: &PortRef) -> Option<&Link> {
        self.links.iter().find(|l| &l.to == to)
    }

    pub fn add_node(
        &mut self,
        registry: &NodeRegistry,
        type_id: &str,
    ) -> Result<GraphNodeId, GraphError> {
        registry
            .get(type_id)
            .ok_or_else(|| GraphError::UnknownType(type_id.to_string()))?;
        let id = GraphNodeId(self.next_id);
        self.next_id += 1;
        self.nodes.insert(
            id,
            GraphNode {
                id,
                type_id: type_id.to_string(),
                values: BTreeMap::new(),
            },
        );
        Ok(id)
    }

    /// Removes a node and every link that touches it; returns the number of links removed.
    pub fn remove_node(&mut self, id: GraphNodeId) -> Result<usize, GraphError> {
        self.nodes.remove(&id).ok_or(GraphError::UnknownNode(id))?;
        let before = self.links.len();
        self.links.retain(|l| l.from.node != id && l.to.node != id);
        Ok(before - self.links.len())
    }

    fn def_of<'r>(
        &self,
        registry: &'r NodeRegistry,
        id: GraphNodeId,
    ) -> Result<&'r std::sync::Arc<crate::NodeDef>, GraphError> {
        let node = self.nodes.get(&id).ok_or(GraphError::UnknownNode(id))?;
        registry
            .get(&node.type_id)
            .ok_or_else(|| GraphError::UnknownType(node.type_id.clone()))
    }

    /// Links `from` (an output) to `to` (an input), replacing any link into `to`. Returns
    /// the replaced link.
    pub fn connect(
        &mut self,
        registry: &NodeRegistry,
        from: PortRef,
        to: PortRef,
    ) -> Result<Option<Link>, GraphError> {
        let out = self
            .def_of(registry, from.node)?
            .find_output(&from.port)
            .ok_or_else(|| GraphError::UnknownPort {
                node: from.node,
                port: from.port.clone(),
                kind: "output",
            })?
            .ty;
        let inp = self
            .def_of(registry, to.node)?
            .find_input(&to.port)
            .ok_or_else(|| GraphError::UnknownPort {
                node: to.node,
                port: to.port.clone(),
                kind: "input",
            })?
            .ty;
        if !can_convert(out, inp) {
            return Err(GraphError::TypeMismatch { from: out, to: inp });
        }
        if let Some(path) = self.path(to.node, from.node) {
            return Err(GraphError::Cycle(path));
        }
        let old = self.links.iter().position(|l| l.to == to);
        let old = old.map(|i| self.links.remove(i));
        let link = Link { from, to };
        let at = self.links.partition_point(|l| l.to < link.to);
        self.links.insert(at, link);
        Ok(old)
    }

    /// A path of graph nodes following links downstream from `start` to `goal` (inclusive),
    /// if one exists. `start == goal` is the path `[start]`.
    fn path(&self, start: GraphNodeId, goal: GraphNodeId) -> Option<Vec<GraphNodeId>> {
        let mut prev: BTreeMap<GraphNodeId, GraphNodeId> = BTreeMap::new();
        let mut seen = BTreeSet::from([start]);
        let mut stack = vec![start];
        while let Some(n) = stack.pop() {
            if n == goal {
                let mut path = vec![n];
                let mut cur = n;
                while let Some(p) = prev.get(&cur) {
                    path.push(*p);
                    cur = *p;
                }
                path.reverse();
                return Some(path);
            }
            for l in self.links.iter().filter(|l| l.from.node == n) {
                if seen.insert(l.to.node) {
                    prev.insert(l.to.node, n);
                    stack.push(l.to.node);
                }
            }
        }
        None
    }

    /// Removes the link into `to`; returns it.
    pub fn disconnect(&mut self, to: &PortRef) -> Result<Link, GraphError> {
        let i = self
            .links
            .iter()
            .position(|l| &l.to == to)
            .ok_or_else(|| GraphError::NotLinked(format!("{}.{}", to.node, to.port)))?;
        Ok(self.links.remove(i))
    }

    /// Sets the constant of an input (converted implicitly to the port type).
    pub fn set_value(
        &mut self,
        registry: &NodeRegistry,
        node: GraphNodeId,
        port: &str,
        value: PortValue,
    ) -> Result<(), GraphError> {
        let ty = self
            .def_of(registry, node)?
            .find_input(port)
            .ok_or_else(|| GraphError::UnknownPort {
                node,
                port: port.to_string(),
                kind: "input",
            })?
            .ty;
        if !value.is_finite() {
            return Err(GraphError::InvalidValue("value is not finite".into()));
        }
        let value = convert(&value, ty)?;
        self.nodes
            .get_mut(&node)
            .expect("checked")
            .values
            .insert(port.to_string(), value);
        Ok(())
    }

    /// The value an unconnected input would use: its constant, else the default.
    pub fn constant_of(
        &self,
        registry: &NodeRegistry,
        node: GraphNodeId,
        port: &str,
    ) -> Result<PortValue, GraphError> {
        let decl = self
            .def_of(registry, node)?
            .find_input(port)
            .ok_or_else(|| GraphError::UnknownPort {
                node,
                port: port.to_string(),
                kind: "input",
            })?;
        Ok(self
            .nodes
            .get(&node)
            .and_then(|n| n.values.get(port))
            .cloned()
            .unwrap_or_else(|| decl.default.clone()))
    }

    /// Nodes in dependency order (sources first); ties go to the smaller id.
    pub fn topo_order(&self) -> Result<Vec<GraphNodeId>, GraphError> {
        let mut indegree: BTreeMap<GraphNodeId, usize> =
            self.nodes.keys().map(|k| (*k, 0)).collect();
        for l in &self.links {
            if let Some(d) = indegree.get_mut(&l.to.node) {
                *d += 1;
            }
        }
        let mut ready: BTreeSet<GraphNodeId> = indegree
            .iter()
            .filter(|(_, d)| **d == 0)
            .map(|(k, _)| *k)
            .collect();
        let mut order = Vec::with_capacity(self.nodes.len());
        while let Some(n) = ready.pop_first() {
            order.push(n);
            for l in self.links.iter().filter(|l| l.from.node == n) {
                if let Some(d) = indegree.get_mut(&l.to.node) {
                    *d -= 1;
                    if *d == 0 {
                        ready.insert(l.to.node);
                    }
                }
            }
        }
        if order.len() < self.nodes.len() {
            let done: BTreeSet<_> = order.iter().copied().collect();
            let stuck = self.nodes.keys().filter(|k| !done.contains(k)).copied();
            return Err(GraphError::Cycle(stuck.collect()));
        }
        Ok(order)
    }

    /// Runs the sink nodes and everything they need.
    pub fn evaluate(
        &self,
        registry: &NodeRegistry,
        env: &EvalEnv,
    ) -> Result<Evaluation, GraphError> {
        let mut sinks = Vec::new();
        for n in self.nodes.values() {
            if registry.get(&n.type_id).is_some_and(|d| d.sink) {
                sinks.push(n.id);
            }
        }
        self.run(registry, env, &sinks)
    }

    /// Runs every node.
    pub fn evaluate_all(
        &self,
        registry: &NodeRegistry,
        env: &EvalEnv,
    ) -> Result<Evaluation, GraphError> {
        let all: Vec<_> = self.nodes.keys().copied().collect();
        self.run(registry, env, &all)
    }

    /// Runs the given nodes and their dependencies.
    pub fn evaluate_for(
        &self,
        registry: &NodeRegistry,
        env: &EvalEnv,
        targets: &[GraphNodeId],
    ) -> Result<Evaluation, GraphError> {
        for t in targets {
            if !self.nodes.contains_key(t) {
                return Err(GraphError::UnknownNode(*t));
            }
        }
        self.run(registry, env, targets)
    }

    fn run(
        &self,
        registry: &NodeRegistry,
        env: &EvalEnv,
        roots: &[GraphNodeId],
    ) -> Result<Evaluation, GraphError> {
        let order = self.topo_order()?;
        let mut needed: BTreeSet<GraphNodeId> = BTreeSet::new();
        let mut stack: Vec<GraphNodeId> = roots.to_vec();
        while let Some(n) = stack.pop() {
            if needed.insert(n) {
                stack.extend(
                    self.links
                        .iter()
                        .filter(|l| l.to.node == n)
                        .map(|l| l.from.node),
                );
            }
        }
        let mut result = Evaluation::default();
        for id in order.into_iter().filter(|n| needed.contains(n)) {
            let fail = |message: String| GraphError::Eval { node: id, message };
            let node = &self.nodes[&id];
            let def = registry
                .get(&node.type_id)
                .ok_or_else(|| GraphError::UnknownType(node.type_id.clone()))?;
            let mut inputs = Vec::with_capacity(def.inputs.len());
            for decl in &def.inputs {
                let to = PortRef::new(id, &decl.id);
                let raw = match self.link_into(&to) {
                    Some(l) => result
                        .values
                        .get(&(l.from.node, l.from.port.clone()))
                        .cloned()
                        .ok_or_else(|| GraphError::UnknownPort {
                            node: l.from.node,
                            port: l.from.port.clone(),
                            kind: "output",
                        })?,
                    None => self.constant_of(registry, id, &decl.id)?,
                };
                let value = convert(&raw, decl.ty)
                    .map_err(|e| fail(format!("input {:?}: {e}", decl.id)))?;
                inputs.push(value);
            }
            let mut ctx = NodeCtx::new(env, def, inputs, &mut result.writes);
            let outputs = (def.eval)(&mut ctx).map_err(|e| fail(e.0))?;
            if outputs.len() != def.outputs.len() {
                return Err(fail(format!(
                    "node produced {} outputs, expected {}",
                    outputs.len(),
                    def.outputs.len()
                )));
            }
            for (decl, value) in def.outputs.iter().zip(outputs) {
                let value = convert(&value, decl.ty)
                    .map_err(|e| fail(format!("output {:?}: {e}", decl.id)))?;
                if !value.is_finite() {
                    return Err(fail(format!("output {:?} is not finite", decl.id)));
                }
                result.values.insert((id, decl.id.clone()), value);
            }
            result.ran.push(id);
        }
        Ok(result)
    }

    /// The deterministic JSON text stored in the expression tag's `graph` parameter.
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).expect("graphs serialize")
    }

    pub fn from_json(text: &str) -> Result<Graph, GraphError> {
        serde_json::from_str(text).map_err(|e| GraphError::Json(e.to_string()))
    }
}
