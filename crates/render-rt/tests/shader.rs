//! The raster module parses and validates for empty and material scenes (RENDER.md R21).

use forge_core::Value;
use forge_material::{IrNode, Material, ShadingGraph, compile};
use std::collections::BTreeMap;

fn validate(source: &str) {
    let module = naga::front::wgsl::parse_str(source).expect("WGSL parses");
    let mut validator = naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::all(),
    );
    validator.validate(&module).expect("naga validates");
}

#[test]
fn an_empty_material_list_validates() {
    let source = forge_render_rt::shader::assemble(&[]).unwrap();
    assert!(source.contains("fn vs_gbuffer"));
    assert!(source.contains("fn fs_light"));
    assert!(source.contains("const F_SHADOW_MAT"));
    validate(&source);
}

#[test]
fn every_material_channel_validates() {
    let values: BTreeMap<String, Value> = [
        ("color.color", Value::Color([0.2, 0.4, 0.6])),
        ("enable.reflectance", Value::Bool(true)),
        ("enable.luminance", Value::Bool(true)),
        ("enable_transparency", Value::Bool(true)),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_string(), v))
    .collect();
    let graph = compile(&Material::from_values(&values));
    let source = forge_render_rt::shader::assemble(&[&graph]).unwrap();
    assert!(source.contains("fn forge_surface_0"));
    assert!(source.contains("forge_surface_0(ctx)"));
    validate(&source);
}

#[test]
fn a_texture_node_is_pinned_unsupported() {
    let graph = ShadingGraph {
        nodes: vec![IrNode::Texture("checker".to_string())],
        outputs: Default::default(),
    };
    let err = forge_render_rt::shader::assemble(&[&graph]).unwrap_err();
    assert!(matches!(err, forge_render_api::RenderError::Unsupported(_)));
}
