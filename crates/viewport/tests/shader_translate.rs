//! The viewport shader must translate to SPIR-V (Vulkan), MSL (Metal) and HLSL (DX12) through
//! naga without a GPU (implementer playbook §5a, docs/specs/VIEWPORT.md §16.5).

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
        &hlsl::Options {
            shader_model: hlsl::ShaderModel::V6_5,
            ..Default::default()
        },
        &hlsl::PipelineOptions::default(),
    )
    .write(&module, &info, None)
    .unwrap_or_else(|e| panic!("{name}: HLSL: {e:?}"));
    msl::write_string(
        &module,
        &info,
        &msl::Options {
            lang_version: (2, 4),
            ..Default::default()
        },
        &msl::PipelineOptions::default(),
    )
    .unwrap_or_else(|e| panic!("{name}: MSL: {e:?}"));
}

#[test]
fn the_viewport_shader_translates_to_all_backends() {
    let source = include_str!("../src/render.wgsl");
    translate("viewport shaders", source);
    for entry in ["fs_mask", "fs_none"] {
        assert!(
            source.contains(&format!("fn {entry}(")),
            "{entry} is missing"
        );
    }
}

#[test]
fn the_outline_shader_translates_to_all_backends() {
    let source = include_str!("../src/outline.wgsl");
    translate("outline shaders", source);
    for entry in ["vs_full", "fs_dilate"] {
        assert!(
            source.contains(&format!("fn {entry}(")),
            "{entry} is missing"
        );
    }
}
