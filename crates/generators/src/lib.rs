//! forge-generators: parametric primitives. Spec: docs/specs/PRIMITIVES.md
//! (vertex and face orders come from docs/parity/{plane,cube,sphere}.md).

mod boole;
mod cube;
mod lathe;
mod plane;
mod platonic;
mod pyramid;
mod round;
mod sphere;
mod subdivision;

pub use boole::{boole, inputs_of as boole_inputs_of};
pub use subdivision::{input_of, subdivide, subdivide_with_creases};

use std::collections::BTreeMap;

use forge_core::{NodeId, NodeType, Plugin, Registry, RegistryError, Scene, Value};
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

fn types() -> [NodeType; 10] {
    [
        plane::node_type(),
        cube::node_type(),
        sphere::node_type(),
        round::cylinder_type(),
        round::cone_type(),
        round::disc_type(),
        round::torus_type(),
        round::tube_type(),
        round::capsule_type(),
        pyramid::node_type(),
    ]
}

impl Plugin for Primitives {
    fn id(&self) -> &str {
        "forge.primitives"
    }

    fn register(&self, r: &mut Registry) -> Result<(), RegistryError> {
        types().into_iter().try_for_each(|t| r.register(t))?;
        r.register(subdivision::node_type())?;
        r.register(boole::node_type())
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
        "forge.cylinder" => round::build_cylinder(&p),
        "forge.cone" => round::build_cone(&p),
        "forge.disc" => round::build_disc(&p),
        "forge.torus" => round::build_torus(&p),
        "forge.tube" => round::build_tube(&p),
        "forge.capsule" => round::build_capsule(&p),
        "forge.pyramid" => pyramid::build(&p),
        _ => unreachable!("types() and build() list the same ids"),
    }
}

/// Local-space mesh of any generator node in a scene (docs/specs/SUBDIVISION.md §4):
/// primitives from their params, the subdivision generator from its input child, the
/// boolean generator from its two inputs (docs/specs/BOOLE.md §1).
/// `Ok(None)` for nodes without geometry.
pub fn evaluate(scene: &Scene, id: NodeId) -> Result<Option<PolyMesh>, BuildError> {
    let Some(node) = scene.node(id) else {
        return Ok(None);
    };
    if node.type_id == subdivision::TYPE {
        let ty = subdivision::node_type();
        let p = Params {
            ty: &ty,
            values: &node.params,
        };
        return subdivision::evaluate_subdivision(scene, id, &p);
    }
    if node.type_id == boole::TYPE {
        return boole::evaluate_boole(scene, id);
    }
    match build(&node.type_id, &node.params) {
        Ok(mesh) => Ok(Some(mesh)),
        Err(BuildError::UnknownType(_)) => Ok(None),
        Err(e) => Err(e),
    }
}

/// True if the node is an input a parent generator consumes (not drawn itself):
/// the subdivision input (SUBDIVISION.md §1) or a boolean's A or B (BOOLE.md §1).
pub fn is_consumed(scene: &Scene, id: NodeId) -> bool {
    let Some(p) = scene
        .node(id)
        .and_then(|n| n.parent)
        .and_then(|p| scene.node(p))
    else {
        return false;
    };
    match p.type_id.as_str() {
        subdivision::TYPE => input_of(scene, p.id) == Some(id),
        boole::TYPE => boole::inputs_of(scene, p.id).contains(&id),
        _ => false,
    }
}
