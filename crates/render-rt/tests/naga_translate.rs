//! Every module the raster engine builds must translate to SPIR-V (Vulkan), MSL (Metal) and
//! HLSL (DX12) through naga, without a GPU (RENDER.md R21.7, implementer playbook §5a).

#[path = "../../render-cpu/tests/common/mod.rs"]
mod common;

use common::golden::{case_names, load};
use forge_core::Value;
use forge_material::{Material, ShadingGraph, TextureSampler, compile};
use forge_render_rt::shader::assemble;
use naga::back::{hlsl, msl, spv};
use naga::valid::{Capabilities, ValidationFlags, Validator};
use std::collections::BTreeMap;
use std::sync::Arc;

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
fn instanced_ray_compute_translates_to_all_backends() {
    translate("instanced ray queries", forge_render_rt::ray::COMPUTE_WGSL);
}

#[test]
fn every_raster_module_translates_to_all_backends() {
    for name in case_names() {
        let case = load(&name);
        let materials: Vec<(&ShadingGraph, &dyn TextureSampler)> = case
            .snapshot
            .materials
            .iter()
            .map(|m| (&m.graph, &*m.sampler as &dyn TextureSampler))
            .collect();
        let assembled = assemble(&materials).unwrap_or_else(|e| panic!("{name}: {e}"));
        translate(&name, &assembled.source);
    }
    translate("empty", &assemble(&[]).unwrap().source);
}

#[test]
fn shader_node_modules_translate_to_all_backends() {
    use forge_shaders::shader::{Bitmap, Shader, ShaderSet};
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
    let assembled = assemble(&materials).unwrap();
    assert_eq!(assembled.resources.len(), 1);
    translate("shader nodes", &assembled.source);
}
