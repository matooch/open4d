//! forge-nodes: a typed node-graph engine for expressions (docs/specs/NODES.md).
//!
//! Typed ports with implicit and explicit conversions, node types registered in a
//! [`NodeRegistry`], a graph with stable ids, cycle checks and topological evaluation with
//! per-evaluation caching, the `forge.expression` tag that writes scene parameters, and the
//! `graph.*` commands. The editor UI is a separate crate.

mod commands;
mod expression;
mod graph;
mod nodes;
mod ports;
mod registry;
mod script;

pub use commands::register_commands;
pub use expression::{
    Diagnostic, ExpressionRun, GRAPH_PARAM, NodesToolset, TAG, apply_expressions,
    apply_expressions_in, apply_expressions_with, evaluate_tag, expression_tags, load_graph,
    tag_reads_time, tag_type,
};
pub use graph::{Evaluation, Graph, GraphError, GraphNode, GraphNodeId, Link, PortRef};
pub use nodes::{BuiltinNodes, noise_kind};
pub use ports::{
    ConvertError, IDENTITY, PortType, PortValue, can_convert, convert, convert_explicit,
};
pub use registry::{
    EvalEnv, EvalError, EvalFn, NodeCtx, NodeDef, NodePlugin, NodeRegistry, ParamWrite, PortDecl,
    RegistryError,
};
pub use script::SandboxLimits;
