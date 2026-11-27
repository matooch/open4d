//! Pure typed material graphs lowered to the Standard shading IR (MATERIAL.md M8).

use std::collections::{BTreeMap, BTreeSet};
use std::sync::OnceLock;

use forge_core::{Category, Node, NodeType, ParamDesc, Value};
use forge_nodes::{
    EvalEnv, Evaluation, Graph, GraphError, GraphNodeId, NodeDef, NodeRegistry, PortRef, PortType,
    PortValue,
};

use crate::ir::{LUMA, fresnel, ramp_factor};
use crate::{IrNode, MATERIAL, Material, MixMode, ShadingGraph, compile, mix};

pub const NODE_MATERIAL: &str = "forge.material.node";
pub const GRAPH: &str = "graph";
pub const SURFACE: &str = "material.surface";
pub const COLOR: &str = "material.color";
pub const SCALAR: &str = "material.scalar";
pub const TEXTURE: &str = "material.texture";
pub const MIX: &str = "material.mix";
pub const FRESNEL: &str = "material.fresnel";
pub const GRADIENT: &str = "material.gradient";
pub const LUMINANCE: &str = "material.luminance";
pub const VECTOR: &str = "material.vector";

// Keep Surface descriptors, defaults, and graph-to-value conversion in one table.
macro_rules! surface {
    ($($field:ident: $ty:ty = $default:expr => $port:ident),+ $(,)?) => {
        const PORTS: &[&str] = &[$(stringify!($field)),+];
        /// Evaluated inputs, including the values the shading IR cannot represent.
        #[derive(Debug, Clone, PartialEq)]
        pub struct SurfaceValues { $(pub $field: $ty),+ }

        impl Default for SurfaceValues {
            fn default() -> Self { Self { $($field: $default),+ } }
        }

        impl SurfaceValues {
            fn ports() -> impl Iterator<Item = (&'static str, PortValue)> {
                [$( (stringify!($field), PortValue::$port($default)) ),+].into_iter()
            }

            pub fn from_graph(graph: &Graph) -> Result<Self, Error> {
                let (id, eval) = evaluate_surface(graph)?;
                Ok(Self::from_evaluation(id, &eval))
            }

            fn from_evaluation(id: GraphNodeId, eval: &Evaluation) -> Self {
                Self {
                    $($field: match eval.output(id, stringify!($field)) {
                        Some(PortValue::$port(value)) => value.clone(),
                        _ => unreachable!("a Surface declares this typed output"),
                    }),+
                }
            }

            pub fn to_graph(&self) -> Result<Graph, Error> {
                let mut graph = default_graph();
                let id = graph.nodes().next().expect("default Surface").id;
                $(if self.$field != $default {
                    graph.set_value(registry(), id, stringify!($field), PortValue::$port(self.$field.to_owned()))?;
                })+
                Ok(graph)
            }
        }
    }
}

surface! {
    base_color: [f64; 3] = [0.8; 3] => Colour,
    base_texture: String = String::new() => String,
    roughness: f64 = 0.5 => Float,
    metallic: f64 = 0.0 => Float,
    metallic_roughness_texture: String = String::new() => String,
    normal_texture: String = String::new() => String,
    normal_strength: f64 = 1.0 => Float,
    emission: [f64; 3] = [0.0; 3] => Colour,
    emissive_texture: String = String::new() => String,
    occlusion_texture: String = String::new() => String,
    occlusion_strength: f64 = 1.0 => Float,
    alpha: f64 = 1.0 => Float,
    alpha_mode: String = "OPAQUE".to_owned() => String,
    alpha_cutoff: f64 = 0.5 => Float,
    double_sided: bool = false => Bool,
}

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
        let gradient = NodeDef::new(GRADIENT, "Gradient Ramp", "Color", |cx| {
            let t = ramp_factor(cx.float("position"), cx.float("start"), cx.float("end"));
            Ok(vec![V::Colour(mix(
                cx.vec3("a"),
                cx.vec3("b"),
                MixMode::Normal,
                t,
            ))])
        })
        .input("a", T::Colour, V::Colour([0.0; 3]))
        .input("b", T::Colour, V::Colour([1.0; 3]))
        .input("start", T::Float, V::Float(0.0))
        .input("end", T::Float, V::Float(1.0))
        .input("position", T::Float, V::Float(0.5))
        .output("color", T::Colour);
        let luminance = NodeDef::new(LUMINANCE, "Luminance", "Convert", |cx| {
            Ok(vec![V::Float(
                cx.vec3("color")
                    .into_iter()
                    .zip(LUMA)
                    .map(|(a, b)| a * b)
                    .sum(),
            )])
        })
        .input("color", T::Colour, V::Colour([0.0; 3]))
        .output("value", T::Float);
        let vector = NodeDef::new(VECTOR, "Vector Components", "Convert", |cx| {
            Ok(cx.vec3("vector").into_iter().map(V::Float).collect())
        })
        .input("vector", T::Vec3, V::Vec3([0.0; 3]))
        .output("x", T::Float)
        .output("y", T::Float)
        .output("z", T::Float);
        let view = NodeDef::new(FRESNEL, "Fresnel", "Input", |cx| {
            Ok(vec![V::Float(fresnel(cx.float("ior"), [0.0; 3], [0.0; 3]))])
        })
        .input("ior", T::Float, V::Float(1.5))
        .output("factor", T::Float);
        let mut surface = NodeDef::new(SURFACE, "Surface", "Output", |cx| {
            Ok(PORTS.iter().map(|p| cx.value(p).clone()).collect())
        })
        .sink();
        for (id, value) in SurfaceValues::ports() {
            surface = surface
                .input(id, value.ty(), value.clone())
                .output(id, value.ty());
        }
        for def in [
            color, scalar, texture, blend, gradient, luminance, vector, view, surface,
        ] {
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

fn evaluate_surface(graph: &Graph) -> Result<(GraphNodeId, Evaluation), Error> {
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
    Ok((root.id, graph.evaluate_all(r, &EvalEnv::headless(0.0))?))
}

/// Evaluate the typed surface and convert its inputs into the common source material.
pub fn compile_graph(graph: &Graph) -> Result<ShadingGraph, Error> {
    let (surface, evaluated) = evaluate_surface(graph)?;
    let s = SurfaceValues::from_evaluation(surface, &evaluated);
    let mut material = Material::new();
    for (id, value) in [
        ("color.color", Value::Color(s.base_color)),
        ("color.roughness", Value::Float(s.roughness.clamp(0.0, 1.0))),
    ] {
        material.set(id, value);
    }
    if s.metallic >= 0.5 {
        material.set("reflectance.spec.attenuation", Value::Enum("metal".into()));
    }
    if !s.base_texture.is_empty() {
        material.set("color.texture", Value::String(s.base_texture.clone()));
        material.set("color.mix_mode", Value::Enum("multiply".into()));
    }
    if !s.normal_texture.is_empty() {
        material.set("enable.normal", Value::Bool(true));
        material.set("normal.texture", Value::String(s.normal_texture));
        material.set(
            "normal.strength",
            Value::Float(s.normal_strength.clamp(0.0, 1000.0)),
        );
    }
    if s.emission != [0.0; 3] {
        material.set("enable.luminance", Value::Bool(true));
        material.set("luminance.color", Value::Color(s.emission));
        if !s.emissive_texture.is_empty() {
            material.set(
                "luminance.texture",
                Value::String(s.emissive_texture.clone()),
            );
            material.set("luminance.mix_mode", Value::Enum("multiply".into()));
        }
    }
    let mut ir = compile(&material);
    if graph.nodes().any(|node| node.type_id == FRESNEL) {
        let mut lower = Lower {
            graph,
            evaluated: &evaluated,
            memo: BTreeMap::new(),
            ir: &mut ir,
        };
        let base = lower.dynamic(surface, "base_color");
        let emission = lower.dynamic(surface, "emission");
        let roughness = lower.dynamic(surface, "roughness");
        let metallic = lower.dynamic(surface, "metallic");
        if let Some(base) = base {
            let color = multiply_texture(&mut ir, base, &s.base_texture);
            if let Some(diffuse) = &mut ir.outputs.diffuse {
                diffuse.color = color;
            }
        }
        if let Some(emission) = emission {
            ir.outputs.emission = Some(multiply_texture(&mut ir, emission, &s.emissive_texture));
        }
        ir.outputs.roughness_input = roughness;
        ir.outputs.metallic_input = metallic;
    }
    Ok(ir)
}

fn multiply_texture(ir: &mut ShadingGraph, base: usize, texture: &str) -> usize {
    if texture.is_empty() {
        return base;
    }
    let top = ir.push(IrNode::Texture(texture.to_owned()));
    ir.push(IrNode::Mix {
        base,
        top,
        mode: MixMode::Multiply,
        strength: 1.0,
    })
}

fn numeric(value: &PortValue) -> [f64; 3] {
    match value {
        PortValue::Float(v) => [*v; 3],
        PortValue::Colour(v) | PortValue::Vec3(v) => *v,
        _ => unreachable!("the material node's supported input is numeric"),
    }
}

/// Lower only view-dependent branches. The other branches still use graph evaluation.
struct Lower<'a> {
    graph: &'a Graph,
    evaluated: &'a Evaluation,
    memo: BTreeMap<PortRef, Option<usize>>,
    ir: &'a mut ShadingGraph,
}

impl Lower<'_> {
    fn dynamic(&mut self, node: GraphNodeId, port: &str) -> Option<usize> {
        let from = self
            .graph
            .link_into(&PortRef::new(node, port))?
            .from
            .clone();
        self.output(&from)
    }

    fn input(&mut self, node: GraphNodeId, port: &str) -> usize {
        if let Some(index) = self.dynamic(node, port) {
            return index;
        }
        let definition = self.graph.node(node).expect("validated graph node");
        let value = self
            .graph
            .link_into(&PortRef::new(node, port))
            .and_then(|link| self.evaluated.output(link.from.node, &link.from.port))
            .or_else(|| definition.values.get(port))
            .unwrap_or_else(|| {
                &registry()
                    .get(&definition.type_id)
                    .and_then(|d| d.find_input(port))
                    .expect("known material input")
                    .default
            });
        self.ir.push(IrNode::Color(numeric(value)))
    }

    fn output(&mut self, from: &PortRef) -> Option<usize> {
        if let Some(cached) = self.memo.get(from) {
            return *cached;
        }
        let id = from.node;
        let node = self.graph.node(id).expect("validated source");
        let ir = match node.type_id.as_str() {
            FRESNEL => Some(IrNode::Fresnel {
                ior: self.input(id, "ior"),
            }),
            COLOR | SCALAR => return self.dynamic(id, "value"),
            GRADIENT | MIX
                if self
                    .graph
                    .links()
                    .iter()
                    .any(|link| link.to.node == id && self.output(&link.from).is_some()) =>
            {
                let (position, start, end) = if node.type_id == MIX {
                    (
                        self.input(id, "factor"),
                        self.ir.push(IrNode::Color([0.0; 3])),
                        self.ir.push(IrNode::Color([1.0; 3])),
                    )
                } else {
                    (
                        self.input(id, "position"),
                        self.input(id, "start"),
                        self.input(id, "end"),
                    )
                };
                Some(IrNode::Ramp {
                    a: self.input(id, "a"),
                    b: self.input(id, "b"),
                    position,
                    start,
                    end,
                })
            }
            LUMINANCE => self.dynamic(id, "color").map(IrNode::Luminance),
            VECTOR => self.dynamic(id, "vector").map(|input| IrNode::Component {
                input,
                index: match from.port.as_str() {
                    "x" => 0,
                    "y" => 1,
                    _ => 2,
                },
            }),
            _ => None,
        };
        let index = ir.map(|ir| self.ir.push(ir));
        self.memo.insert(from.clone(), index);
        index
    }
}
