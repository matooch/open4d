//! Node type definitions and the node registry (docs/specs/NODES.md section 1).

use std::collections::BTreeMap;
use std::sync::{Arc, OnceLock};

use forge_core::{NodeId, Scene, Value};

use crate::ports::{ConvertError, PortType, PortValue, convert};
use crate::script::SandboxLimits;

/// Why a node failed to run.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{0}")]
pub struct EvalError(pub String);

impl From<String> for EvalError {
    fn from(s: String) -> Self {
        EvalError(s)
    }
}

impl From<&str> for EvalError {
    fn from(s: &str) -> Self {
        EvalError(s.to_string())
    }
}

impl From<ConvertError> for EvalError {
    fn from(e: ConvertError) -> Self {
        EvalError(e.to_string())
    }
}

/// The read-only inputs of one evaluation.
#[derive(Clone)]
pub struct EvalEnv<'a> {
    /// The scene to read objects from (already animated); `None` for pure graphs.
    pub scene: Option<&'a Scene>,
    pub seconds: f64,
    pub fps: f64,
    /// The object that carries the expression; empty `object` inputs mean this node.
    pub host: Option<NodeId>,
    pub limits: SandboxLimits,
}

impl<'a> EvalEnv<'a> {
    /// No scene, 30 fps.
    pub fn headless(seconds: f64) -> EvalEnv<'a> {
        EvalEnv {
            scene: None,
            seconds,
            fps: 30.0,
            host: None,
            limits: SandboxLimits::default(),
        }
    }

    pub fn frame(&self) -> f64 {
        self.seconds * self.fps
    }
}

/// A parameter write requested by a node (applied by the caller, not by the graph).
#[derive(Debug, Clone, PartialEq)]
pub struct ParamWrite {
    pub node: NodeId,
    pub param: String,
    pub value: Value,
}

/// One input or output port of a node type.
#[derive(Debug, Clone, PartialEq)]
pub struct PortDecl {
    /// Stable id (`[a-z0-9_]+`).
    pub id: String,
    pub label: String,
    pub ty: PortType,
    /// The constant of an unconnected input (ignored for outputs).
    pub default: PortValue,
}

/// What a node's eval function sees: its inputs (already converted to the declared port
/// types) and the environment.
pub struct NodeCtx<'a> {
    pub env: &'a EvalEnv<'a>,
    def: &'a NodeDef,
    inputs: Vec<PortValue>,
    pub(crate) writes: &'a mut Vec<ParamWrite>,
}

impl<'a> NodeCtx<'a> {
    pub(crate) fn new(
        env: &'a EvalEnv<'a>,
        def: &'a NodeDef,
        inputs: Vec<PortValue>,
        writes: &'a mut Vec<ParamWrite>,
    ) -> NodeCtx<'a> {
        NodeCtx {
            env,
            def,
            inputs,
            writes,
        }
    }

    /// The value of input `id`.
    ///
    /// # Panics
    /// If the node type has no such input (a bug in the node definition).
    pub fn value(&self, id: &str) -> &PortValue {
        let i = self
            .def
            .inputs
            .iter()
            .position(|p| p.id == id)
            .unwrap_or_else(|| panic!("{}: no input {id:?}", self.def.id));
        &self.inputs[i]
    }

    pub fn float(&self, id: &str) -> f64 {
        match self.value(id) {
            PortValue::Float(f) => *f,
            PortValue::Int(i) => *i as f64,
            PortValue::Bool(b) => f64::from(u8::from(*b)),
            other => panic!("{}.{id}: {} is not a number", self.def.id, other.ty()),
        }
    }

    pub fn int(&self, id: &str) -> i64 {
        match self.value(id) {
            PortValue::Int(i) => *i,
            other => panic!("{}.{id}: {} is not an int", self.def.id, other.ty()),
        }
    }

    pub fn bool(&self, id: &str) -> bool {
        match self.value(id) {
            PortValue::Bool(b) => *b,
            other => panic!("{}.{id}: {} is not a bool", self.def.id, other.ty()),
        }
    }

    pub fn vec3(&self, id: &str) -> [f64; 3] {
        match self.value(id) {
            PortValue::Vec3(v) | PortValue::Colour(v) => *v,
            other => panic!("{}.{id}: {} is not a vec3", self.def.id, other.ty()),
        }
    }

    pub fn matrix(&self, id: &str) -> [f64; 16] {
        match self.value(id) {
            PortValue::Matrix(m) => *m,
            other => panic!("{}.{id}: {} is not a matrix", self.def.id, other.ty()),
        }
    }

    pub fn string(&self, id: &str) -> &str {
        match self.value(id) {
            PortValue::String(s) => s,
            other => panic!("{}.{id}: {} is not a string", self.def.id, other.ty()),
        }
    }

    pub fn node(&self, id: &str) -> Option<NodeId> {
        match self.value(id) {
            PortValue::Node(n) => *n,
            other => panic!("{}.{id}: {} is not a node", self.def.id, other.ty()),
        }
    }

    /// The scene, or an error for graphs evaluated without one.
    pub fn scene(&self) -> Result<&'a Scene, EvalError> {
        self.env
            .scene
            .ok_or_else(|| EvalError("this node needs a scene".into()))
    }

    /// The node in the `object` input, or the host when it is empty.
    pub fn object_or_host(&self, id: &str) -> Result<NodeId, EvalError> {
        self.node(id)
            .or(self.env.host)
            .ok_or_else(|| EvalError("no object given and no host object".into()))
    }

    /// Queues a parameter write.
    pub fn write_param(&mut self, node: NodeId, param: &str, value: Value) {
        self.writes.push(ParamWrite {
            node,
            param: param.to_string(),
            value,
        });
    }
}

/// A node's evaluation function: outputs in declaration order.
pub type EvalFn = fn(&mut NodeCtx<'_>) -> Result<Vec<PortValue>, EvalError>;

#[derive(Clone)]
pub struct NodeDef {
    /// `namespace.name`.
    pub id: String,
    pub label: String,
    /// Menu grouping (`math`, `vector`, ...).
    pub category: String,
    pub inputs: Vec<PortDecl>,
    pub outputs: Vec<PortDecl>,
    /// A sink runs even if nothing consumes it (the parameter write).
    pub sink: bool,
    pub eval: EvalFn,
}

impl std::fmt::Debug for NodeDef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NodeDef")
            .field("id", &self.id)
            .field("inputs", &self.inputs)
            .field("outputs", &self.outputs)
            .field("sink", &self.sink)
            .finish()
    }
}

impl NodeDef {
    pub fn new(id: &str, label: &str, category: &str, eval: EvalFn) -> NodeDef {
        NodeDef {
            id: id.to_string(),
            label: label.to_string(),
            category: category.to_string(),
            inputs: Vec::new(),
            outputs: Vec::new(),
            sink: false,
            eval,
        }
    }

    pub fn input(mut self, id: &str, ty: PortType, default: PortValue) -> NodeDef {
        self.inputs.push(PortDecl {
            id: id.to_string(),
            label: label_of(id),
            ty,
            default,
        });
        self
    }

    pub fn output(mut self, id: &str, ty: PortType) -> NodeDef {
        self.outputs.push(PortDecl {
            id: id.to_string(),
            label: label_of(id),
            ty,
            default: PortValue::zero(ty),
        });
        self
    }

    pub fn sink(mut self) -> NodeDef {
        self.sink = true;
        self
    }

    pub fn find_input(&self, id: &str) -> Option<&PortDecl> {
        self.inputs.iter().find(|p| p.id == id)
    }

    pub fn find_output(&self, id: &str) -> Option<&PortDecl> {
        self.outputs.iter().find(|p| p.id == id)
    }
}

fn label_of(id: &str) -> String {
    let mut s = id.replace('_', " ");
    if let Some(c) = s.get_mut(0..1) {
        c.make_ascii_uppercase();
    }
    s
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RegistryError {
    #[error("invalid node type id {0:?} (expected namespace.name, [a-z0-9_.])")]
    InvalidTypeId(String),
    #[error("node type {0:?} is already registered")]
    Duplicate(String),
    #[error("{type_id}: invalid or duplicate port id {port:?}")]
    InvalidPort { type_id: String, port: String },
    #[error("{type_id}: default of input {port:?} does not fit its type")]
    InvalidDefault { type_id: String, port: String },
}

/// A bundle of node types registered together.
pub trait NodePlugin {
    fn id(&self) -> &str;
    fn register(&self, registry: &mut NodeRegistry) -> Result<(), RegistryError>;
}

#[derive(Debug, Clone, Default)]
pub struct NodeRegistry {
    defs: BTreeMap<String, Arc<NodeDef>>,
}

fn valid_id(id: &str, dotted: bool) -> bool {
    let ok = |s: &str| {
        !s.is_empty()
            && s.chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
    };
    if dotted {
        id.contains('.') && id.split('.').all(ok)
    } else {
        ok(id)
    }
}

impl NodeRegistry {
    pub fn new() -> NodeRegistry {
        NodeRegistry::default()
    }

    /// A registry with the built-in nodes.
    pub fn with_builtins() -> NodeRegistry {
        let mut r = NodeRegistry::new();
        r.install(&crate::nodes::BuiltinNodes)
            .expect("built-in node types are valid");
        r
    }

    /// The shared registry of built-in nodes, used by the commands and the expression tag.
    pub fn builtin() -> &'static NodeRegistry {
        static BUILTIN: OnceLock<NodeRegistry> = OnceLock::new();
        BUILTIN.get_or_init(NodeRegistry::with_builtins)
    }

    pub fn register(&mut self, def: NodeDef) -> Result<(), RegistryError> {
        if !valid_id(&def.id, true) {
            return Err(RegistryError::InvalidTypeId(def.id));
        }
        if self.defs.contains_key(&def.id) {
            return Err(RegistryError::Duplicate(def.id));
        }
        let mut seen = std::collections::BTreeSet::new();
        for p in def.inputs.iter().chain(&def.outputs) {
            // inputs and outputs have separate namespaces; check each list below
            if !valid_id(&p.id, false) {
                return Err(RegistryError::InvalidPort {
                    type_id: def.id,
                    port: p.id.clone(),
                });
            }
        }
        for list in [&def.inputs, &def.outputs] {
            seen.clear();
            for p in list {
                if !seen.insert(p.id.as_str()) {
                    return Err(RegistryError::InvalidPort {
                        type_id: def.id.clone(),
                        port: p.id.clone(),
                    });
                }
            }
        }
        for p in &def.inputs {
            if convert(&p.default, p.ty).is_err() {
                return Err(RegistryError::InvalidDefault {
                    type_id: def.id.clone(),
                    port: p.id.clone(),
                });
            }
        }
        self.defs.insert(def.id.clone(), Arc::new(def));
        Ok(())
    }

    /// Registers all of a plugin's types, or none of them.
    pub fn install(&mut self, plugin: &dyn NodePlugin) -> Result<(), RegistryError> {
        let mut staged = self.clone();
        plugin.register(&mut staged)?;
        *self = staged;
        Ok(())
    }

    pub fn get(&self, id: &str) -> Option<&Arc<NodeDef>> {
        self.defs.get(id)
    }

    /// All definitions, sorted by id.
    pub fn defs(&self) -> impl Iterator<Item = &Arc<NodeDef>> {
        self.defs.values()
    }
}
