//! MATERIAL.md §M8: Standard and graph surfaces share an IR compiler.

use forge_core::Value;
use forge_material::node_material::{
    COLOR, SCALAR, SURFACE, TEXTURE, compile_graph, default_graph, registry,
};
use forge_material::{Material, compile};
use forge_nodes::{PortRef, PortValue};

#[test]
fn equivalent_standard_and_node_materials_compile_to_equal_ir() {
    let mut graph = default_graph();
    let r = registry();
    let surface = graph.nodes().find(|n| n.type_id == SURFACE).unwrap().id;
    let color = graph.add_node(r, COLOR).unwrap();
    let roughness = graph.add_node(r, SCALAR).unwrap();
    let texture = graph.add_node(r, TEXTURE).unwrap();
    let rgb = [0.22, 0.61, 0.8];
    graph
        .set_value(r, color, "value", PortValue::Colour(rgb))
        .unwrap();
    graph
        .set_value(r, roughness, "value", PortValue::Float(0.34))
        .unwrap();
    graph
        .set_value(r, texture, "path", PortValue::String("base.png".into()))
        .unwrap();
    for (node, src, dst) in [
        (color, "value", "base_color"),
        (roughness, "value", "roughness"),
        (texture, "path", "base_texture"),
    ] {
        graph
            .connect(r, PortRef::new(node, src), PortRef::new(surface, dst))
            .unwrap();
    }
    graph
        .set_value(r, surface, "metallic", PortValue::Float(1.0))
        .unwrap();
    graph
        .set_value(
            r,
            surface,
            "normal_texture",
            PortValue::String("normal.png".into()),
        )
        .unwrap();
    graph
        .set_value(r, surface, "normal_strength", PortValue::Float(0.42))
        .unwrap();
    graph
        .set_value(r, surface, "emission", PortValue::Colour([0.05, 0.1, 0.3]))
        .unwrap();
    let mut standard = Material::new();
    for (param, value) in [
        ("color.color", Value::Color(rgb)),
        ("color.texture", Value::String("base.png".into())),
        ("color.mix_mode", Value::Enum("multiply".into())),
        ("color.roughness", Value::Float(0.34)),
        ("reflectance.spec.width", Value::Float(0.34)),
        ("reflectance.spec.attenuation", Value::Enum("metal".into())),
        ("enable.normal", Value::Bool(true)),
        ("normal.texture", Value::String("normal.png".into())),
        ("normal.strength", Value::Float(0.42)),
        ("enable.luminance", Value::Bool(true)),
        ("luminance.color", Value::Color([0.05, 0.1, 0.3])),
    ] {
        standard.set(param, value);
    }
    assert_eq!(compile_graph(&graph).unwrap(), compile(&standard));
    graph.add_node(r, SURFACE).unwrap();
    assert!(compile_graph(&graph).is_err());
}

#[test]
fn levels_clip_endpoints_and_map_the_gray_point_to_half() {
    use forge_material::node_material::GRADIENT;
    use forge_nodes::EvalEnv;

    let mut graph = forge_nodes::Graph::new();
    let r = registry();
    let ramp = graph.add_node(r, GRADIENT).unwrap();
    for (port, value) in [("start", 0.2), ("end", 0.8), ("midpoint", 0.25)] {
        graph
            .set_value(r, ramp, port, PortValue::Float(value))
            .unwrap();
    }
    for (input, expected) in [(0.1, 0.0), (0.35, 0.5), (0.5, 0.5_f64.sqrt()), (0.9, 1.0)] {
        graph
            .set_value(r, ramp, "position", PortValue::Float(input))
            .unwrap();
        let out = graph.evaluate_all(r, &EvalEnv::headless(0.0)).unwrap();
        assert!((out.float(ramp, "factor").unwrap() - expected).abs() < 1e-12);
        let Some(PortValue::Colour(rgb)) = out.output(ramp, "color") else {
            panic!("missing ramp color");
        };
        assert!(
            rgb.iter()
                .all(|component| (component - expected).abs() < 1e-12)
        );
    }
}
