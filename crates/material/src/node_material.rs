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
pub const FROM_STANDARD: &str = "from_standard";
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
        let mut gradient = NodeDef::new(GRADIENT, "Gradient Ramp", "Color", |cx| {
            let t = ramp_factor(
                cx.float("position"),
                cx.float("start"),
                cx.float("end"),
                cx.float("midpoint"),
            );
            Ok(vec![
                V::Colour(mix(cx.vec3("a"), cx.vec3("b"), MixMode::Normal, t)),
                V::Float(t),
            ])
        })
        .input("a", T::Colour, V::Colour([0.0; 3]))
        .input("b", T::Colour, V::Colour([1.0; 3]))
        .input("start", T::Float, V::Float(0.0))
        .input("end", T::Float, V::Float(1.0))
        .input("midpoint", T::Float, V::Float(0.5))
        .input("position", T::Float, V::Float(0.5))
        .output("color", T::Colour)
        .output("factor", T::Float);
        for port in &mut gradient.inputs {
            port.label = match port.id.as_str() {
                "a" => "Black Color",
                "b" => "White Color",
                "start" => "Black Point",
                "end" => "White Point",
                "midpoint" => "Gray Point",
                "position" => "Input",
                _ => continue,
            }
            .into();
        }
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
        let mut view = NodeDef::new(FRESNEL, "Fresnel", "Input", |cx| {
            Ok(vec![V::Float(fresnel(
                cx.float("ior"),
                cx.float("falloff"),
                cx.float("center_darkening"),
                cx.float("center_hold"),
                [0.0; 3],
                [0.0; 3],
            ))])
        })
        .input("ior", T::Float, V::Float(1.5))
        .input("falloff", T::Float, V::Float(5.0))
        .input("center_darkening", T::Float, V::Float(0.0))
        .input("center_hold", T::Float, V::Float(0.0))
        .output("factor", T::Float);
        view.inputs[2].label = "Darken Center".into();
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
    let mut ty = NodeType::new(NODE_MATERIAL, "Node Material", Category::Material)
        .not_in_create_menu()
        .param(ParamDesc::bool(FROM_STANDARD, "Converted from Standard", false).hidden())
        .param(ParamDesc::string(GRAPH, "Graph", &default_graph().to_json()).hidden());
    ty.params.extend(crate::params::standard_type().params);
    ty.params
        .iter_mut()
        .find(|p| p.id == "icon_color")
        .unwrap()
        .default = Value::Color([0.65, 0.52, 0.95]);
    ty
}

pub fn is_converted(node: &Node) -> bool {
    node.type_id == NODE_MATERIAL && node.params.get(FROM_STANDARD) == Some(&Value::Bool(true))
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

/// Supplies inherited values for presenting a converted graph. Edits must still be applied
/// to the stored graph so unedited channels continue to follow Attributes.
pub fn graph_with_inherited_inputs(node: &Node, include_shader_refs: bool) -> Result<Graph, Error> {
    let mut graph = load_graph(node)?;
    if !is_converted(node) {
        return Ok(graph);
    }
    let (surface, _) = evaluate_surface(&graph)?;
    let standard = Material::from_node(node);
    let texture = |param| {
        PortValue::String(if include_shader_refs {
            standard.e(param).to_owned()
        } else {
            String::new()
        })
    };
    for (port, value) in [
        ("base_color", PortValue::Colour(standard.c("color.color"))),
        ("base_texture", texture("color.texture")),
        (
            "roughness",
            PortValue::Float(standard.f("reflectance.spec.width")),
        ),
        (
            "metallic",
            PortValue::Float(if standard.e("reflectance.spec.attenuation") == "metal" {
                1.0
            } else {
                0.0
            }),
        ),
        ("normal_texture", texture("normal.texture")),
        (
            "normal_strength",
            PortValue::Float(standard.f("normal.strength")),
        ),
        (
            "emission",
            PortValue::Colour(if standard.enabled("luminance") {
                standard.c("luminance.color")
            } else {
                [0.0; 3]
            }),
        ),
        ("emissive_texture", texture("luminance.texture")),
        ("occlusion_texture", texture("color.occlusion_texture")),
    ] {
        if !surface_has_input(&graph, surface, port) {
            graph.set_value(registry(), surface, port, value)?;
        }
    }
    Ok(graph)
}

pub fn compile_hosted(node: &Node) -> Result<ShadingGraph, Error> {
    match node.type_id.as_str() {
        NODE_MATERIAL => compile_surface(
            &load_graph(node)?,
            is_converted(node).then(|| Material::from_node(node)),
        ),
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
    compile_surface(graph, None)
}

fn surface_has_input(graph: &Graph, surface: GraphNodeId, port: &str) -> bool {
    graph
        .node(surface)
        .is_some_and(|n| n.values.contains_key(port))
        || graph
            .links()
            .iter()
            .any(|link| link.to.node == surface && link.to.port == port)
}

fn compile_surface(graph: &Graph, standard: Option<Material>) -> Result<ShadingGraph, Error> {
    let (surface, evaluated) = evaluate_surface(graph)?;
    let s = SurfaceValues::from_evaluation(surface, &evaluated);
    let converted = standard.is_some();
    let has_input = |port| surface_has_input(graph, surface, port);
    let mut material = standard.unwrap_or_default();
    if converted {
        for (port, param, value) in [
            ("base_color", "color.color", Value::Color(s.base_color)),
            (
                "base_texture",
                "color.texture",
                Value::String(s.base_texture.clone()),
            ),
            (
                "roughness",
                "color.roughness",
                Value::Float(s.roughness.clamp(0.0, 1.0)),
            ),
            (
                "roughness",
                "reflectance.spec.width",
                Value::Float(s.roughness.clamp(0.0, 1.0)),
            ),
            (
                "metallic",
                "reflectance.spec.attenuation",
                Value::Enum(
                    if s.metallic >= 0.5 {
                        "metal"
                    } else {
                        "additive"
                    }
                    .into(),
                ),
            ),
            (
                "normal_texture",
                "normal.texture",
                Value::String(s.normal_texture.clone()),
            ),
            (
                "normal_strength",
                "normal.strength",
                Value::Float(s.normal_strength.clamp(0.0, 1000.0)),
            ),
            ("emission", "luminance.color", Value::Color(s.emission)),
            (
                "emissive_texture",
                "luminance.texture",
                Value::String(s.emissive_texture.clone()),
            ),
            (
                "occlusion_texture",
                "color.occlusion_texture",
                Value::String(s.occlusion_texture.clone()),
            ),
        ] {
            if has_input(port) {
                material.set(param, value);
            }
        }
        if has_input("base_color") || has_input("base_texture") {
            material.set("enable.color", Value::Bool(true));
        }
        if has_input("normal_texture") {
            material.set("enable.normal", Value::Bool(!s.normal_texture.is_empty()));
        }
        if has_input("emission") || has_input("emissive_texture") {
            material.set("enable.luminance", Value::Bool(true));
        }
    } else {
        for (id, value) in [
            ("color.color", Value::Color(s.base_color)),
            ("color.roughness", Value::Float(s.roughness.clamp(0.0, 1.0))),
            (
                "reflectance.spec.width",
                Value::Float(s.roughness.clamp(0.0, 1.0)),
            ),
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
            let color = multiply_texture(&mut ir, base, material.e("color.texture"));
            if let Some(diffuse) = &mut ir.outputs.diffuse {
                diffuse.color = color;
            }
        }
        if let Some(emission) = emission {
            ir.outputs.emission = Some(multiply_texture(
                &mut ir,
                emission,
                material.e("luminance.texture"),
            ));
        }
        if roughness.is_some() {
            ir.outputs.roughness_input = roughness;
        }
        if metallic.is_some() {
            ir.outputs.metallic_input = metallic;
        }
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
                falloff: self.input(id, "falloff"),
                center_darkening: self.input(id, "center_darkening"),
                center_hold: self.input(id, "center_hold"),
            }),
            COLOR | SCALAR => return self.dynamic(id, "value"),
            GRADIENT | MIX
                if self
                    .graph
                    .links()
                    .iter()
                    .any(|link| link.to.node == id && self.output(&link.from).is_some()) =>
            {
                let (position, start, end, midpoint) = if node.type_id == MIX {
                    (
                        self.input(id, "factor"),
                        self.ir.push(IrNode::Color([0.0; 3])),
                        self.ir.push(IrNode::Color([1.0; 3])),
                        self.ir.push(IrNode::Color([0.5; 3])),
                    )
                } else {
                    (
                        self.input(id, "position"),
                        self.input(id, "start"),
                        self.input(id, "end"),
                        self.input(id, "midpoint"),
                    )
                };
                let (a, b) = if node.type_id == GRADIENT && from.port == "factor" {
                    (
                        self.ir.push(IrNode::Color([0.0; 3])),
                        self.ir.push(IrNode::Color([1.0; 3])),
                    )
                } else {
                    (self.input(id, "a"), self.input(id, "b"))
                };
                Some(IrNode::Ramp {
                    a,
                    b,
                    position,
                    start,
                    end,
                    midpoint,
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
