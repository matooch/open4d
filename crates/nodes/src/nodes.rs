//! The built-in node types (docs/specs/NODES.md section 4).

use forge_core::{ParamKind, Value};
use forge_noise::{Noise, NoiseKind};
use glam::{DMat4, DVec3};

use crate::ports::{IDENTITY, PortType as T, PortValue as V, convert_explicit};
use crate::registry::{EvalError, NodeCtx, NodeDef, NodePlugin, NodeRegistry, RegistryError};

/// Registers every built-in node type.
pub struct BuiltinNodes;

type Out = Result<Vec<V>, EvalError>;

fn f(x: f64) -> V {
    V::Float(x)
}

/// A node with float inputs `ids` (default 0) and one float output `out`.
fn float_fn(
    id: &str,
    label: &str,
    category: &str,
    inputs: &[(&str, f64)],
    eval: fn(&mut NodeCtx<'_>) -> Out,
) -> NodeDef {
    let mut def = NodeDef::new(id, label, category, eval);
    for (name, default) in inputs {
        def = def.input(name, T::Float, f(*default));
    }
    def.output("out", T::Float)
}

fn binary(id: &str, label: &str, eval: fn(&mut NodeCtx<'_>) -> Out) -> NodeDef {
    float_fn(id, label, "math", &[("a", 0.0), ("b", 0.0)], eval)
}

fn constant(id: &str, label: &str, ty: T, default: V) -> NodeDef {
    NodeDef::new(id, label, "constant", |c| {
        Ok(vec![c.value("value").clone()])
    })
    .input("value", ty, default)
    .output("out", ty)
}

/// The noise kind of a snake-case name (NODES.md section 4, `math.noise`).
pub fn noise_kind(name: &str) -> Option<NoiseKind> {
    Some(match name {
        "perlin" => NoiseKind::Perlin,
        "value" => NoiseKind::Value,
        "simplex" => NoiseKind::Simplex,
        "box" => NoiseKind::Box,
        "cell" => NoiseKind::Cell,
        "voronoi1" => NoiseKind::Voronoi1,
        "voronoi2" => NoiseKind::Voronoi2,
        "voronoi3" => NoiseKind::Voronoi3,
        "fbm" => NoiseKind::Fbm,
        "turbulence" => NoiseKind::Turbulence,
        "ridged_multifractal" => NoiseKind::RidgedMultifractal,
        "wavy_turbulence" => NoiseKind::WavyTurbulence,
        "electric" => NoiseKind::Electric,
        "gaseous" => NoiseKind::Gaseous,
        "blistered_turbulence" => NoiseKind::BlisteredTurbulence,
        "displaced_turbulence" => NoiseKind::DisplacedTurbulence,
        "displaced_voronoi" => NoiseKind::DisplacedVoronoi,
        _ => return None,
    })
}

fn noise(c: &mut NodeCtx<'_>) -> Out {
    let kind = noise_kind(c.string("kind"))
        .ok_or_else(|| EvalError(format!("unknown noise kind {:?}", c.string("kind"))))?;
    let scale = c.float("scale");
    if scale <= 0.0 {
        return Err("scale must be greater than 0".into());
    }
    let n = Noise {
        kind,
        seed: c.int("seed").rem_euclid(1 << 32) as u32,
        scale,
        octaves: c.int("octaves").clamp(1, 16) as u32,
        ..Noise::default()
    };
    let v = n.sample4(DVec3::from_array(c.vec3("position")), c.float("time"));
    Ok(vec![f(v), f(0.5 + 0.5 * v)])
}

fn compare(c: &mut NodeCtx<'_>) -> Out {
    let (a, b) = (c.float("a"), c.float("b"));
    let r = match c.string("op") {
        "lt" => a < b,
        "le" => a <= b,
        "gt" => a > b,
        "ge" => a >= b,
        "eq" => a == b,
        "ne" => a != b,
        other => return Err(format!("unknown comparison {other:?} (lt le gt ge eq ne)").into()),
    };
    Ok(vec![V::Bool(r)])
}

fn remap(c: &mut NodeCtx<'_>) -> Out {
    let (lo, hi) = (c.float("in_min"), c.float("in_max"));
    let mut t = if hi == lo {
        0.0
    } else {
        (c.float("value") - lo) / (hi - lo)
    };
    if c.bool("clamp") {
        t = t.clamp(0.0, 1.0);
    }
    let (a, b) = (c.float("out_min"), c.float("out_max"));
    Ok(vec![f(a + (b - a) * t)])
}

fn clamp(c: &mut NodeCtx<'_>) -> Out {
    let (a, b) = (c.float("min"), c.float("max"));
    Ok(vec![f(c.float("value").max(a.min(b)).min(a.max(b)))])
}

fn convert_to(c: &mut NodeCtx<'_>, to: T) -> Out {
    Ok(vec![convert_explicit(c.value("value"), to)?])
}

/// A scene parameter value as a port value.
pub(crate) fn value_to_port(v: &Value) -> Result<V, EvalError> {
    Ok(match v {
        Value::Bool(b) => V::Bool(*b),
        Value::Int(i) => V::Int(*i),
        Value::Float(x) => V::Float(*x),
        Value::Vec3(x) => V::Vec3(*x),
        Value::Color(x) => V::Colour(*x),
        Value::Enum(s) | Value::String(s) => V::String(s.clone()),
        Value::Link(n) => V::Node(*n),
        _ => return Err("this parameter kind cannot be read by a node".into()),
    })
}

/// A port value as a value for a parameter of `kind` (implicit conversions only).
pub(crate) fn port_to_value(kind: &ParamKind, v: &V) -> Result<Value, EvalError> {
    use crate::ports::convert;
    Ok(match kind {
        ParamKind::Bool => match convert(v, T::Bool)? {
            V::Bool(b) => Value::Bool(b),
            _ => unreachable!("converted to bool"),
        },
        ParamKind::Int { .. } => match convert(v, T::Int)? {
            V::Int(i) => Value::Int(i),
            _ => unreachable!("converted to int"),
        },
        ParamKind::Float { .. } => match convert(v, T::Float)? {
            V::Float(x) => Value::Float(x),
            _ => unreachable!("converted to float"),
        },
        ParamKind::Vec3 { .. } => match convert(v, T::Vec3)? {
            V::Vec3(x) => Value::Vec3(x),
            _ => unreachable!("converted to vec3"),
        },
        ParamKind::Color => match convert(v, T::Colour)? {
            V::Colour(x) => Value::Color(x),
            _ => unreachable!("converted to colour"),
        },
        ParamKind::Enum { .. } => match convert(v, T::String)? {
            V::String(s) => Value::Enum(s),
            _ => unreachable!("converted to string"),
        },
        ParamKind::String => match convert(v, T::String)? {
            V::String(s) => Value::String(s),
            _ => unreachable!("converted to string"),
        },
        ParamKind::Link { .. } => match convert(v, T::Node)? {
            V::Node(n) => Value::Link(n),
            _ => unreachable!("converted to node"),
        },
        _ => return Err("this parameter kind cannot be written by a node".into()),
    })
}

fn get_param(c: &mut NodeCtx<'_>) -> Out {
    let scene = c.scene()?;
    let id = c.object_or_host("object")?;
    let name = c.string("param");
    if scene.node(id).is_none() {
        return Err(format!("object {id} does not exist").into());
    }
    let v = scene
        .param(id, name)
        .ok_or_else(|| EvalError(format!("object {id} has no parameter {name:?}")))?;
    Ok(vec![value_to_port(v)?])
}

fn set_param(c: &mut NodeCtx<'_>) -> Out {
    if !c.bool("enabled") {
        return Ok(Vec::new());
    }
    let scene = c.scene()?;
    let id = c.object_or_host("object")?;
    let name = c.string("param").to_string();
    let ty = scene
        .node_type(id)
        .ok_or_else(|| EvalError(format!("object {id} does not exist")))?;
    let desc = ty
        .find_param(&name)
        .ok_or_else(|| EvalError(format!("object {id} has no parameter {name:?}")))?;
    let value = port_to_value(&desc.kind, c.value("value"))
        .map_err(|e| EvalError(format!("{name}: {}", e.0)))?;
    c.write_param(id, &name, value);
    Ok(Vec::new())
}

fn world_matrix(c: &mut NodeCtx<'_>) -> Out {
    let scene = c.scene()?;
    let id = c.object_or_host("object")?;
    let m = scene
        .world_matrix(id)
        .ok_or_else(|| EvalError(format!("object {id} does not exist")))?;
    Ok(vec![V::Matrix(m.to_cols_array())])
}

impl NodePlugin for BuiltinNodes {
    fn id(&self) -> &str {
        "forge.nodes.builtin"
    }

    fn register(&self, r: &mut NodeRegistry) -> Result<(), RegistryError> {
        let defs = [
            constant("const.float", "Float", T::Float, f(0.0)),
            constant("const.int", "Integer", T::Int, V::Int(0)),
            constant("const.bool", "Bool", T::Bool, V::Bool(false)),
            constant("const.vec3", "Vector", T::Vec3, V::Vec3([0.0; 3])),
            constant("const.colour", "Colour", T::Colour, V::Colour([0.0; 3])),
            constant(
                "const.string",
                "String",
                T::String,
                V::String(String::new()),
            ),
            NodeDef::new("time.now", "Time", "time", |c| {
                Ok(vec![f(c.env.seconds), f(c.env.frame()), f(c.env.fps)])
            })
            .output("seconds", T::Float)
            .output("frame", T::Float)
            .output("fps", T::Float),
            binary("math.add", "Add", |c| {
                Ok(vec![f(c.float("a") + c.float("b"))])
            }),
            binary("math.sub", "Subtract", |c| {
                Ok(vec![f(c.float("a") - c.float("b"))])
            }),
            binary("math.mul", "Multiply", |c| {
                Ok(vec![f(c.float("a") * c.float("b"))])
            }),
            binary("math.div", "Divide", |c| {
                let b = c.float("b");
                Ok(vec![f(if b == 0.0 { 0.0 } else { c.float("a") / b })])
            }),
            float_fn(
                "math.mix",
                "Mix",
                "math",
                &[("a", 0.0), ("b", 1.0), ("t", 0.0)],
                |c| {
                    let (a, b) = (c.float("a"), c.float("b"));
                    Ok(vec![f(a + (b - a) * c.float("t"))])
                },
            ),
            float_fn(
                "math.clamp",
                "Clamp",
                "math",
                &[("value", 0.0), ("min", 0.0), ("max", 1.0)],
                clamp,
            ),
            float_fn(
                "math.remap",
                "Range Mapper",
                "math",
                &[
                    ("value", 0.0),
                    ("in_min", 0.0),
                    ("in_max", 1.0),
                    ("out_min", 0.0),
                    ("out_max", 1.0),
                ],
                remap,
            )
            .input("clamp", T::Bool, V::Bool(false)),
            float_fn("math.sin", "Sine", "math", &[("x", 0.0)], |c| {
                Ok(vec![f(c.float("x").sin())])
            }),
            float_fn("math.cos", "Cosine", "math", &[("x", 0.0)], |c| {
                Ok(vec![f(c.float("x").cos())])
            }),
            NodeDef::new("math.noise", "Noise", "math", noise)
                .input("position", T::Vec3, V::Vec3([0.0; 3]))
                .input("time", T::Float, f(0.0))
                .input("seed", T::Int, V::Int(0))
                .input("scale", T::Float, f(1.0))
                .input("kind", T::String, V::String("perlin".into()))
                .input("octaves", T::Int, V::Int(4))
                .output("value", T::Float)
                .output("unit", T::Float),
            NodeDef::new("vector.compose", "Compose Vector", "vector", |c| {
                Ok(vec![V::Vec3([c.float("x"), c.float("y"), c.float("z")])])
            })
            .input("x", T::Float, f(0.0))
            .input("y", T::Float, f(0.0))
            .input("z", T::Float, f(0.0))
            .output("out", T::Vec3),
            NodeDef::new("vector.decompose", "Decompose Vector", "vector", |c| {
                let v = c.vec3("v");
                Ok(vec![f(v[0]), f(v[1]), f(v[2])])
            })
            .input("v", T::Vec3, V::Vec3([0.0; 3]))
            .output("x", T::Float)
            .output("y", T::Float)
            .output("z", T::Float),
            NodeDef::new("vector.add", "Add Vectors", "vector", |c| {
                let (a, b) = (c.vec3("a"), c.vec3("b"));
                Ok(vec![V::Vec3([a[0] + b[0], a[1] + b[1], a[2] + b[2]])])
            })
            .input("a", T::Vec3, V::Vec3([0.0; 3]))
            .input("b", T::Vec3, V::Vec3([0.0; 3]))
            .output("out", T::Vec3),
            NodeDef::new("vector.scale", "Scale Vector", "vector", |c| {
                let (v, s) = (c.vec3("v"), c.float("s"));
                Ok(vec![V::Vec3([v[0] * s, v[1] * s, v[2] * s])])
            })
            .input("v", T::Vec3, V::Vec3([0.0; 3]))
            .input("s", T::Float, f(1.0))
            .output("out", T::Vec3),
            NodeDef::new("vector.length", "Vector Length", "vector", |c| {
                let v = c.vec3("v");
                Ok(vec![f((v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt())])
            })
            .input("v", T::Vec3, V::Vec3([0.0; 3]))
            .output("out", T::Float),
            NodeDef::new("matrix.translation", "Matrix Translation", "matrix", |c| {
                let m = c.matrix("m");
                Ok(vec![V::Vec3([m[12], m[13], m[14]])])
            })
            .input("m", T::Matrix, V::Matrix(IDENTITY))
            .output("out", T::Vec3),
            NodeDef::new(
                "matrix.from_translation",
                "Translation Matrix",
                "matrix",
                |c| {
                    let v = c.vec3("v");
                    Ok(vec![V::Matrix(
                        DMat4::from_translation(DVec3::from_array(v)).to_cols_array(),
                    )])
                },
            )
            .input("v", T::Vec3, V::Vec3([0.0; 3]))
            .output("out", T::Matrix),
            NodeDef::new("compare.compare", "Compare", "logic", compare)
                .input("a", T::Float, f(0.0))
                .input("b", T::Float, f(0.0))
                .input("op", T::String, V::String("lt".into()))
                .output("out", T::Bool),
            NodeDef::new("logic.and", "And", "logic", |c| {
                Ok(vec![V::Bool(c.bool("a") && c.bool("b"))])
            })
            .input("a", T::Bool, V::Bool(false))
            .input("b", T::Bool, V::Bool(false))
            .output("out", T::Bool),
            NodeDef::new("logic.or", "Or", "logic", |c| {
                Ok(vec![V::Bool(c.bool("a") || c.bool("b"))])
            })
            .input("a", T::Bool, V::Bool(false))
            .input("b", T::Bool, V::Bool(false))
            .output("out", T::Bool),
            NodeDef::new("logic.not", "Not", "logic", |c| {
                Ok(vec![V::Bool(!c.bool("a"))])
            })
            .input("a", T::Bool, V::Bool(false))
            .output("out", T::Bool),
            NodeDef::new("logic.condition", "Condition", "logic", |c| {
                let pick = if c.bool("condition") {
                    "if_true"
                } else {
                    "if_false"
                };
                Ok(vec![c.value(pick).clone()])
            })
            .input("condition", T::Bool, V::Bool(false))
            .input("if_true", T::Any, f(1.0))
            .input("if_false", T::Any, f(0.0))
            .output("out", T::Any),
            convert_def("convert.to_float", "To Float", T::Float),
            convert_def("convert.to_int", "To Integer", T::Int),
            convert_def("convert.to_bool", "To Bool", T::Bool),
            convert_def("convert.to_string", "To String", T::String),
            NodeDef::new("object.self", "This Object", "object", |c| {
                let host = c.env.host.ok_or("no host object")?;
                Ok(vec![V::Node(Some(host))])
            })
            .output("object", T::Node),
            NodeDef::new("object.get_param", "Get Parameter", "object", get_param)
                .input("object", T::Node, V::Node(None))
                .input("param", T::String, V::String("transform.pos".into()))
                .output("value", T::Any),
            NodeDef::new(
                "object.world_matrix",
                "World Matrix",
                "object",
                world_matrix,
            )
            .input("object", T::Node, V::Node(None))
            .output("out", T::Matrix),
            NodeDef::new("object.set_param", "Set Parameter", "object", set_param)
                .input("object", T::Node, V::Node(None))
                .input("param", T::String, V::String("transform.pos".into()))
                .input("value", T::Any, f(0.0))
                .input("enabled", T::Bool, V::Bool(true))
                .sink(),
            NodeDef::new(
                "script.rhai",
                "Script",
                "script",
                crate::script::eval_script,
            )
            .input("source", T::String, V::String("a + b".into()))
            .input("a", T::Float, f(0.0))
            .input("b", T::Float, f(0.0))
            .input("c", T::Float, f(0.0))
            .input("d", T::Float, f(0.0))
            .input("v", T::Vec3, V::Vec3([0.0; 3]))
            .input("flag", T::Bool, V::Bool(false))
            .output("result", T::Float)
            .output("vec", T::Vec3)
            .output("flag", T::Bool),
        ];
        for d in defs {
            r.register(d)?;
        }
        Ok(())
    }
}

fn convert_def(id: &str, label: &str, to: T) -> NodeDef {
    // The target type is fixed by the output port; the eval function reads it from there.
    let eval: fn(&mut NodeCtx<'_>) -> Out = match to {
        T::Float => |c| convert_to(c, T::Float),
        T::Int => |c| convert_to(c, T::Int),
        T::Bool => |c| convert_to(c, T::Bool),
        _ => |c| convert_to(c, T::String),
    };
    NodeDef::new(id, label, "convert", eval)
        .input("value", T::Any, f(0.0))
        .output("out", to)
}
