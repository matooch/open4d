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
fn hardware_ray_queries_translate_to_all_backends() {
    translate("hardware ray queries", forge_render_rt::ray::HARDWARE_WGSL);
}

#[test]
fn instanced_ray_compute_translates_to_all_backends() {
    translate("instanced ray queries", forge_render_rt::ray::COMPUTE_WGSL);
}

#[test]
fn both_traversals_can_be_called_inline_without_ray_batch_bindings() {
    use forge_render_rt::ray::{COMPUTE_TRAVERSAL_WGSL, HARDWARE_TRAVERSAL_WGSL, RAY_HIT_WGSL};
    let inline = "
@group(1) @binding(0) var<storage, read_write> result: array<RtHit>;
@compute @workgroup_size(1)
fn path(@builtin(global_invocation_id) id: vec3<u32>) {
    let o = vec3<f32>(f32(id.x), 0.0, -1.0);
    let d = vec3<f32>(0.0, 0.0, 2.0);
    let primary = rt_closest(o, d, 100.0);
    result[id.x] = rt_closest(o + d * primary.tuv.x, -d, 50.0);
}
";
    let compute = format!(
        "{RAY_HIT_WGSL}{COMPUTE_TRAVERSAL_WGSL}
@group(2) @binding(0) var<storage, read> tlas: array<Node>;
@group(2) @binding(1) var<storage, read> instances: array<Instance>;
@group(2) @binding(2) var<storage, read> blas: array<Node>;
@group(2) @binding(3) var<storage, read> triangles: array<Triangle>;
{inline}"
    );
    translate("inline compute traversal", &compute);
    let hardware = format!(
        "enable wgpu_ray_query;\n{RAY_HIT_WGSL}{HARDWARE_TRAVERSAL_WGSL}
@group(2) @binding(0) var scene: acceleration_structure;
{inline}"
    );
    translate("inline hardware traversal", &hardware);
}

#[test]
fn ambient_visibility_transport_translates_with_each_traversal() {
    use forge_render_rt::{ray::BackendKind, shader::ambient_visibility_wgsl};
    for backend in [
        BackendKind::Compute,
        BackendKind::Metal,
        BackendKind::Vulkan,
    ] {
        translate(
            &format!("{backend:?} ambient visibility"),
            &ambient_visibility_wgsl(backend),
        );
    }
}

#[test]
fn raw_raster_receiver_translates_without_path_transport() {
    let entry = "
@group(0) @binding(1) var<storage, read_write> out: array<vec4<f32>>;
@compute @workgroup_size(1)
fn receiver_probe(@builtin(global_invocation_id) id: vec3<u32>) {
    let ids = receiver_triangle(id.x + 1u, arrayLength(&out));
    if ids.x < arrayLength(&out) {
        let g = receiver_geometry(out[id.x].xyz, ids);
        let view = receiver_view(vec2<f32>(id.xy), vec2<f32>(0.125), g.uvw,
            mat3x3<f32>(g.corners[0], g.corners[1], g.corners[2]));
        out[id.x] = vec4<f32>(g.ns + g.ng + view, f32(g.closed));
    }
}";
    translate(
        "raw raster receiver",
        &format!(
            "{}{entry}",
            forge_render_rt::shader::receiver_geometry_wgsl()
        ),
    );
}

#[test]
fn direct_shadow_light_sampling_has_no_path_transport_dependency() {
    use forge_render_gpu::{LIGHT_SAMPLING_WGSL, RNG_WGSL};
    let entry = "
@group(0) @binding(0) var<storage, read> lights: array<GpuLight>;
@group(0) @binding(1) var<storage, read_write> samples: array<vec4<f32>>;
@compute @workgroup_size(1)
fn sample_shadow_light(@builtin(global_invocation_id) id: vec3<u32>) {
    var rng = rng_new(vec2<u32>(0u), vec2<u32>(id.x, 0u));
    let s = light_sample(lights[id.x], vec3<f32>(1.0, 2.0, 3.0), &rng);
    samples[id.x] = select(vec4<f32>(0.0), vec4<f32>(s.wi, s.dist), s.ok);
}
";
    translate(
        "standalone direct-shadow light sampling",
        &format!("{RNG_WGSL}\n{LIGHT_SAMPLING_WGSL}\n{entry}"),
    );
}

#[test]
fn raw_direct_shadow_transport_translates_with_each_traversal() {
    use forge_render_rt::{ray::BackendKind, shader::direct_shadow_wgsl};
    let case = load("mat-bump-noise");
    let materials: Vec<(&ShadingGraph, &dyn TextureSampler)> = case
        .snapshot
        .materials
        .iter()
        .map(|m| (&m.graph, &*m.sampler as &dyn TextureSampler))
        .collect();
    for backend in [
        BackendKind::Compute,
        BackendKind::Metal,
        BackendKind::Vulkan,
    ] {
        let module = direct_shadow_wgsl(backend, &materials).unwrap();
        let compiled = naga::front::wgsl::parse_str(&module.source)
            .unwrap_or_else(|e| panic!("{backend:?}: {}", e.emit_to_string(&module.source)));
        assert_eq!(compiled.entry_points.len(), 1);
        assert_eq!(compiled.entry_points[0].name, "direct_shadow");
        assert_eq!(compiled.entry_points[0].workgroup_size, [8, 8, 1]);
        translate(&format!("{backend:?} direct shadows"), &module.source);
    }
}

#[test]
fn shared_path_transport_translates_with_every_backend() {
    use forge_render_gpu::shader::assemble_with_scene;
    use forge_render_rt::{path_scene_wgsl, ray::BackendKind};
    let case = load("mat-bump-noise");
    let materials: Vec<(&ShadingGraph, &dyn TextureSampler)> = case
        .snapshot
        .materials
        .iter()
        .map(|m| (&m.graph, &*m.sampler as &dyn TextureSampler))
        .collect();
    for backend in [
        BackendKind::Compute,
        BackendKind::Metal,
        BackendKind::Vulkan,
    ] {
        let (directives, scene) = path_scene_wgsl(backend);
        let assembled = assemble_with_scene(&materials, directives, &scene).unwrap();
        translate(&format!("{backend:?} path transport"), &assembled.source);
    }
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
