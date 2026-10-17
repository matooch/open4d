//! forge-generators: parametric primitives. Spec: docs/specs/PRIMITIVES.md
//! (vertex and face orders come from docs/parity/{plane,cube,sphere}.md).

mod cube;
mod plane;
mod sphere;

use std::collections::BTreeMap;

use forge_core::{NodeType, Plugin, Registry, RegistryError, Value};
use forge_geometry::PolyMesh;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum BuildError {
    #[error("{0} is not a generator type")]
    UnknownType(String),
    /// An observed option whose geometry is not derived yet.
    #[error("not supported yet: {0}")]
    Unsupported(&'static str),
}

/// The primitive node types.
pub struct Primitives;

fn types() -> [NodeType; 3] {
    [plane::node_type(), cube::node_type(), sphere::node_type()]
}

impl Plugin for Primitives {
    fn id(&self) -> &str {
        "forge.primitives"
    }

    fn register(&self, r: &mut Registry) -> Result<(), RegistryError> {
        types().into_iter().try_for_each(|t| r.register(t))
    }
}

/// Typed parameter access with defaults and validation from the descriptors.
pub(crate) struct Params<'a> {
    ty: &'a NodeType,
    values: &'a BTreeMap<String, Value>,
}

impl Params<'_> {
    fn get(&self, id: &str) -> Value {
        let desc = self
            .ty
            .find_param(id)
            .expect("generator reads its own params");
        self.values
            .get(id)
            .and_then(|v| desc.validate(v.clone()).ok())
            .unwrap_or_else(|| desc.default.clone())
    }

    pub(crate) fn float(&self, id: &str) -> f64 {
        match self.get(id) {
            Value::Float(f) => f,
            other => unreachable!("{id} is not a float: {other:?}"),
        }
    }

    pub(crate) fn int(&self, id: &str) -> usize {
        match self.get(id) {
            Value::Int(i) => i.max(0) as usize,
            other => unreachable!("{id} is not an int: {other:?}"),
        }
    }

    pub(crate) fn bool(&self, id: &str) -> bool {
        matches!(self.get(id), Value::Bool(true))
    }

    pub(crate) fn enumeration(&self, id: &str) -> String {
        match self.get(id) {
            Value::Enum(s) => s,
            other => unreachable!("{id} is not an enum: {other:?}"),
        }
    }
}

/// Builds the local-space mesh of a generator node from its params.
pub fn build(type_id: &str, values: &BTreeMap<String, Value>) -> Result<PolyMesh, BuildError> {
    let ty = types()
        .into_iter()
        .find(|t| t.id == type_id)
        .ok_or_else(|| BuildError::UnknownType(type_id.to_string()))?;
    let p = Params { ty: &ty, values };
    match type_id {
        "forge.plane" => Ok(plane::build(&p)),
        "forge.cube" => cube::build(&p),
        "forge.sphere" => sphere::build(&p),
        _ => unreachable!("types() and build() list the same ids"),
    }
}
