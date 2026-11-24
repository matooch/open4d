//! Pure typed material graphs lowered to the Standard shading IR (MATERIAL.md M8).

use std::collections::BTreeSet;
use std::sync::OnceLock;

use forge_core::{Category, Node, NodeType, ParamDesc, Value};
use forge_nodes::{EvalEnv, Graph, GraphError, NodeDef, NodeRegistry, PortType, PortValue};

use crate::{MATERIAL, Material, MixMode, ShadingGraph, compile, mix};

pub const NODE_MATERIAL: &str = "forge.material.node";
pub const GRAPH: &str = "graph";
pub const SURFACE: &str = "material.surface";
pub const COLOR: &str = "material.color";
pub const SCALAR: &str = "material.scalar";
pub const TEXTURE: &str = "material.texture";
pub const MIX: &str = "material.mix";

const PORTS: [&str; 7] = [
    "base_color",
    "base_texture",
    "roughness",
    "metallic",
    "normal_texture",
    "normal_strength",
    "emission",
];

pub fn registry() -> &'static NodeRegistry {
    static REGISTRY: OnceLock<NodeRegistry> = OnceLock::new();
    REGISTRY.get_or_init(|| {
        use PortType as T;
        use PortValue as V;
        let mut registry = NodeRegistry::new();
        let color = NodeDef::new(COLOR, "Color", "Input", |cx| {
            Ok(vec![cx.value("value").clone()])
        })
        .input("value", T::Colour, V::Colour([0.8; 3]))
        .output("value", T::Colour);
        let scalar = NodeDef::new(SCALAR, "Scalar", "Input", |cx| {
            Ok(vec![cx.value("value").clone()])
        })
        .input("value", T::Float, V::Float(0.5))
        .output("value", T::Float);
        let texture = NodeDef::new(TEXTURE, "Image Texture", "Input", |cx| {
            Ok(vec![cx.value("path").clone()])
        })
        .input("path", T::String, V::String(String::new()))
        .output("path", T::String);
        let blend = NodeDef::new(MIX, "Mix Color", "Color", |cx| {
            Ok(vec![V::Colour(mix(
                cx.vec3("a"),
                cx.vec3("b"),
                MixMode::Normal,
                cx.float("factor").clamp(0.0, 1.0),
            ))])
        })
        .input("a", T::Colour, V::Colour([0.0; 3]))
        .input("b", T::Colour, V::Colour([1.0; 3]))
        .input("factor", T::Float, V::Float(0.5))
        .output("color", T::Colour);
        let mut surface = NodeDef::new(SURFACE, "Surface", "Output", |cx| {
            Ok(PORTS.iter().map(|p| cx.value(p).clone()).collect())
        })
        .sink();
        for (id, value) in PORTS.into_iter().zip([
            V::Colour([0.8; 3]),
            V::String(String::new()),
            V::Float(0.5),
            V::Float(0.0),
            V::String(String::new()),
            V::Float(1.0),
            V::Colour([0.0; 3]),
        ]) {
            surface = surface
                .input(id, value.ty(), value.clone())
                .output(id, value.ty());
        }
        for def in [color, scalar, texture, blend, surface] {
            registry
                .register(def)
                .expect("material node definition is valid");
        }
        registry
    })
}

pub fn default_graph() -> Graph {
    let mut graph = Graph::new();
    graph
        .add_node(registry(), SURFACE)
        .expect("registered Surface");
    graph
}

pub fn node_material_type() -> NodeType {
    NodeType::new(NODE_MATERIAL, "Node Material", Category::Material)
        .not_in_create_menu()
        .param(ParamDesc::color("icon_color", "Color", [0.65, 0.52, 0.95]))
        .param(ParamDesc::string(GRAPH, "Graph", &default_graph().to_json()).hidden())
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Graph(#[from] GraphError),
    #[error("{0}")]
    Invalid(String),
}

pub fn load_graph(node: &Node) -> Result<Graph, Error> {
    if node.type_id == NODE_MATERIAL
        && let Some(Value::String(graph)) = node.params.get(GRAPH)
    {
        return Ok(Graph::from_json(graph)?);
    }
    Err(Error::Invalid(format!(
        "node {} has no material graph",
        node.id
    )))
}

pub fn compile_hosted(node: &Node) -> Result<ShadingGraph, Error> {
    match node.type_id.as_str() {
        NODE_MATERIAL => compile_graph(&load_graph(node)?),
        MATERIAL => Ok(compile(&Material::from_node(node))),
        _ => Err(Error::Invalid(format!(
            "unsupported material type {}",
            node.type_id
        ))),
    }
}

/// Evaluate the typed surface and convert its inputs into the common source material.
pub fn compile_graph(graph: &Graph) -> Result<ShadingGraph, Error> {
    let r = registry();
    let mut surfaces = graph.nodes().filter(|n| n.type_id == SURFACE);
    let root = surfaces
        .next()
        .filter(|_| surfaces.next().is_none())
        .ok_or_else(|| Error::Invalid("a node material needs exactly one Surface".into()))?;
    for node in graph.nodes() {
        let def = r
            .get(&node.type_id)
            .ok_or_else(|| GraphError::UnknownType(node.type_id.clone()))?;
        if node.values.keys().any(|p| def.find_input(p).is_none()) {
            return Err(Error::Invalid(format!(
                "node {} has an unknown input constant",
                node.id
            )));
        }
    }
    let def = |id| graph.node(id).and_then(|n| r.get(&n.type_id));
    let mut targets = BTreeSet::new();
    for link in graph.links() {
        if !targets.insert(&link.to)
            || link.from.node == root.id
            || !def(link.from.node)
                .and_then(|d| d.find_output(&link.from.port))
                .zip(def(link.to.node).and_then(|d| d.find_input(&link.to.port)))
                .is_some_and(|(a, b)| forge_nodes::can_convert(a.ty, b.ty))
        {
            return Err(Error::Invalid("invalid material graph link".into()));
        }
    }
    let ev = graph.evaluate_all(r, &EvalEnv::headless(0.0))?;
    let value = |port| {
        ev.output(root.id, port)
            .expect("Surface declares all of these outputs")
    };
    let float = |port| match value(port) {
        PortValue::Float(v) => *v,
        _ => unreachable!(),
    };
    let color = |port| match value(port) {
        PortValue::Colour(v) => *v,
        _ => unreachable!(),
    };
    let text = |port| match value(port) {
        PortValue::String(v) => v.as_str(),
        _ => unreachable!(),
    };
    let mut material = Material::new();
    for (id, value) in [
        ("color.color", Value::Color(color("base_color"))),
        (
            "color.roughness",
            Value::Float(float("roughness").clamp(0.0, 1.0)),
        ),
    ] {
        material.set(id, value);
    }
    if float("metallic") >= 0.5 {
        material.set("reflectance.spec.attenuation", Value::Enum("metal".into()));
    }
    let base = text("base_texture");
    if !base.is_empty() {
        material.set("color.texture", Value::String(base.into()));
        material.set("color.mix_mode", Value::Enum("multiply".into()));
    }
    let normal = text("normal_texture");
    if !normal.is_empty() {
        material.set("enable.normal", Value::Bool(true));
        material.set("normal.texture", Value::String(normal.into()));
        material.set(
            "normal.strength",
            Value::Float(float("normal_strength").clamp(0.0, 1000.0)),
        );
    }
    let emission = color("emission");
    if emission != [0.0; 3] {
        material.set("enable.luminance", Value::Bool(true));
        material.set("luminance.color", Value::Color(emission));
    }
    Ok(compile(&material))
}
