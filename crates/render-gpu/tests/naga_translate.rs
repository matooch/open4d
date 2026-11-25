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
        let graphs: Vec<&forge_material::ShadingGraph> =
            case.snapshot.materials.iter().map(|m| &m.graph).collect();
        let Ok(source) = assemble(&graphs) else {
            continue;
        };
        translate(&name, &source);
    }
    // No materials at all still builds a module.
    translate("empty", &assemble(&[]).unwrap());
}
