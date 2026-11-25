//! The raster module parses and validates for empty, material and shader-node scenes
//! (RENDER.md R21).

use forge_core::Value;
use forge_material::{IrNode, Material, ShadingGraph, TextureSampler, compile};
use forge_shaders::shader::ShaderSet;
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
    let source = forge_render_rt::shader::assemble(&[]).unwrap().source;
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
    let sampler = forge_material::ir::White;
    let source = forge_render_rt::shader::assemble(&[(&graph, &sampler as &dyn TextureSampler)])
        .unwrap()
        .source;
    assert!(source.contains("fn forge_surface_0"));
    assert!(source.contains("forge_surface_0(ctx)"));
    validate(&source);
}

#[test]
fn a_texture_node_needs_a_sampler_codegen() {
    struct Bare;
    impl TextureSampler for Bare {
        fn sample(&self, _: &str, _: &forge_material::ir::ShadeCtx) -> [f64; 3] {
            [1.0; 3]
        }
    }
    let graph = ShadingGraph {
        nodes: vec![IrNode::Texture("checker".to_string())],
        outputs: Default::default(),
    };
    let err = forge_render_rt::shader::assemble(&[(&graph, &Bare as &dyn TextureSampler)])
        .err()
        .expect("no codegen is unsupported");
    assert!(matches!(err, forge_render_api::RenderError::Unsupported(_)));
}

#[test]
fn shader_node_materials_bind_their_bitmaps_in_group_2() {
    use forge_shaders::shader::{Bitmap, Shader};
    use std::sync::Arc;
    let mut set = ShaderSet::default();
    for (name, id) in [
        ("n", "forge.shader.noise"),
        ("c", "forge.shader.checkerboard"),
        ("f", "forge.shader.fresnel"),
    ] {
        let sh = forge_shaders::build(
            id,
            &forge_shaders::Values::new(),
            &forge_shaders::Inputs::default(),
        )
        .unwrap();
        set.shaders.insert(name.into(), sh);
    }
    set.shaders.insert(
        "b".into(),
        Shader::Bitmap(Bitmap {
            image: Arc::new(forge_image::Image::new(
                4,
                4,
                forge_image::AlphaMode::Straight,
            )),
            repeat: true,
            interpolate: true,
        }),
    );
    let graphs: Vec<ShadingGraph> = [("n", "c"), ("b", "f")]
        .iter()
        .map(|(col, bump)| {
            let mut o = BTreeMap::new();
            o.insert("color.texture".to_string(), Value::String((*col).into()));
            o.insert("bump.texture".to_string(), Value::String((*bump).into()));
            compile(&Material::from_values(&o))
        })
        .collect();
    let materials: Vec<(&ShadingGraph, &dyn TextureSampler)> = graphs
        .iter()
        .map(|g| (g, &set as &dyn TextureSampler))
        .collect();
    let assembled = forge_render_rt::shader::assemble(&materials).unwrap();
    assert_eq!(assembled.resources.len(), 1);
    assert!(assembled.source.contains("@group(2) @binding(0)"));
    assert!(
        !assembled
            .source
            .contains("@group(1) @binding(0) var forge_tex")
    );
    validate(&assembled.source);
}
