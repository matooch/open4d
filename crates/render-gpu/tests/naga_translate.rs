//! Every module the engine builds must translate to SPIR-V (Vulkan), MSL (Metal) and HLSL
//! (DX12) through naga, without a GPU: the naga 30 SPIR-V writer panicked on a pointer to a
//! sub-field passed as an argument, which only Vulkan devices hit (RENDER.md R20).

#[path = "../../render-cpu/tests/common/mod.rs"]
mod common;

use common::golden::{case_names, load};
use forge_render_gpu::shader::assemble;
use naga::back::{hlsl, msl, spv};
use naga::valid::{Capabilities, ValidationFlags, Validator};

fn translate(name: &str, source: &str) {
    let module = naga::front::wgsl::parse_str(source)
        .unwrap_or_else(|e| panic!("{name}: WGSL parse: {}", e.emit_to_string(source)));
    let info = Validator::new(ValidationFlags::all(), Capabilities::all())
        .validate(&module)
        .unwrap_or_else(|e| panic!("{name}: validation: {e:?}"));
    spv::write_vec(&module, &info, &spv::Options::default(), None)
        .unwrap_or_else(|e| panic!("{name}: SPIR-V: {e:?}"));
    let mut out = String::new();
    hlsl::Writer::new(
        &mut out,
        &hlsl::Options::default(),
        &hlsl::PipelineOptions::default(),
    )
    .write(&module, &info, None)
    .unwrap_or_else(|e| panic!("{name}: HLSL: {e:?}"));
    msl::write_string(
        &module,
        &info,
        &msl::Options::default(),
        &msl::PipelineOptions::default(),
    )
    .unwrap_or_else(|e| panic!("{name}: MSL: {e:?}"));
}

#[test]
fn every_module_the_engine_builds_translates_to_all_backends() {
    for name in case_names() {
        let case = load(&name);
        let materials: Vec<(
            &forge_material::ShadingGraph,
            &dyn forge_material::TextureSampler,
        )> = case
            .snapshot
            .materials
            .iter()
            .map(|m| (&m.graph, &*m.sampler as &dyn forge_material::TextureSampler))
            .collect();
        let assembled = assemble(&materials).unwrap_or_else(|e| panic!("{name}: {e}"));
        translate(&name, &assembled.source);
    }
    // No materials at all still builds a module.
    translate("empty", &assemble(&[]).unwrap().source);
}

#[test]
fn transport_accepts_a_scene_without_flattened_bindings() {
    // Empty-scene implementation with no triangle, BVH or primitive buffers.
    let scene = r#"
fn scene_intersect(o: vec3<f32>, d: vec3<f32>, tmax: f32) -> Hit {
    var h = empty_hit();
    h.t = tmax;
    return h;
}
fn scene_occluded(o: vec3<f32>, d: vec3<f32>, tmax: f32) -> bool {
    return false;
}
fn scene_geometry(o: vec3<f32>, d: vec3<f32>, h: Hit) -> Geom {
    var g: Geom;
    return g;
}
fn scene_ids(h: Hit) -> vec2<f32> {
    return vec2<f32>(0.0);
}
"#;
    let compiled = forge_render_gpu::shader::assemble_with_scene(&[], "", scene).unwrap();
    assert!(!compiled.source.contains("forge_tris"));
    assert!(!compiled.source.contains("forge_nodes"));
    assert!(!compiled.source.contains("forge_prims"));
    translate("transport with a replacement scene", &compiled.source);
}

#[test]
fn shader_node_modules_translate_to_all_backends() {
    use forge_core::Value;
    use forge_material::{Material, compile};
    use forge_shaders::shader::{Bitmap, Shader, ShaderSet};
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
    let set = Arc::new(set);
    let graphs: Vec<_> = [("n", "c"), ("b", "f")]
        .iter()
        .map(|(col, bump)| {
            let mut o = std::collections::BTreeMap::new();
            o.insert("color.texture".to_string(), Value::String((*col).into()));
            o.insert("bump.texture".to_string(), Value::String((*bump).into()));
            compile(&Material::from_values(&o))
        })
        .collect();
    let materials: Vec<(
        &forge_material::ShadingGraph,
        &dyn forge_material::TextureSampler,
    )> = graphs
        .iter()
        .map(|g| (g, &*set as &dyn forge_material::TextureSampler))
        .collect();
    let assembled = assemble(&materials).unwrap();
    assert_eq!(assembled.resources.len(), 1);
    translate("shader nodes", &assembled.source);
}
