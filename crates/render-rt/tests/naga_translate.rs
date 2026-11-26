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
fn raw_reflection_transport_translates_with_each_traversal() {
    check_raw_bounce_translation(
        forge_render_rt::shader::reflection_wgsl,
        "reflection_signal",
        true,
    );
}

#[test]
fn raw_diffuse_transport_translates_without_environment_binding() {
    check_raw_bounce_translation(
        forge_render_rt::shader::diffuse_wgsl,
        "diffuse_signal",
        false,
    );
}

fn check_raw_bounce_translation(
    assemble: impl Fn(
        forge_render_rt::ray::BackendKind,
        &[(&ShadingGraph, &dyn TextureSampler)],
    ) -> Result<forge_render_gpu::shader::Assembled, forge_render_api::RenderError>,
    entry: &str,
    uses_environment: bool,
) {
    use forge_render_rt::ray::BackendKind;
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
        let module = assemble(backend, &materials).unwrap();
        let compiled = naga::front::wgsl::parse_str(&module.source)
            .unwrap_or_else(|e| panic!("{backend:?}: {}", e.emit_to_string(&module.source)));
        assert_eq!(compiled.entry_points.len(), 1);
        assert_eq!(compiled.entry_points[0].name, entry);
        assert_eq!(compiled.entry_points[0].workgroup_size, [8, 8, 1]);
        assert_eq!(
            compiled.global_variables.iter().any(|(_, global)| global
                .binding
                .is_some_and(|binding| binding.group == 0 && binding.binding == 3)),
            uses_environment
        );
        translate(&format!("{backend:?} {entry}"), &module.source);
    }
}

#[test]
fn reflection_sampler_translates_to_all_backends() {
    translate("reflection sampler", &reflection_probe_source());
}

fn reflection_probe_source() -> String {
    let graph = compile(&Material::default());
    let mat =
        forge_render_gpu::shader::materials(&[(&graph, &forge_material::ir::White)], 1).unwrap();
    let fns = forge_render_rt::shader::REFLECTION_SAMPLING_WGSL;
    let rng = forge_render_gpu::RNG_WGSL;
    let geometry = forge_render_gpu::GEOMETRY_WGSL;
    let surface = forge_render_gpu::SURFACE_WGSL;
    format!(
        "{}\n{rng}\n{geometry}\n{surface}\n{fns}\n{}",
        mat.source,
        r"
@group(0) @binding(0) var<storage, read> inputs: array<vec4<u32>>;
@group(0) @binding(1) var<storage, read_write> output: array<vec4<u32>>;

@compute @workgroup_size(1)
fn probe(@builtin(global_invocation_id) id: vec3<u32>) {
    let b = id.x * 6u;
    let key = inputs[b + 5u];
    let n = bitcast<vec4<f32>>(inputs[b]).xyz;
    let ng = bitcast<vec4<f32>>(inputs[b + 1u]).xyz;
    let wo = bitcast<vec4<f32>>(inputs[b + 2u]).xyz;
    let spec_alpha = bitcast<vec4<f32>>(inputs[b + 3u]);
    var rng = rng_new(key.xy ^ vec2<u32>(key.w, 0u), vec2<u32>(key.z, 0u));
    var m: Mat;
    m.spec = spec_alpha.xyz;
    m.alpha = spec_alpha.w;
    m.trans = bitcast<vec4<f32>>(inputs[b + 4u]).xyz;
    // A nonzero diffuse lobe must not enter the reflection sampler.
    m.albedo = vec3<f32>(0.6);
    for (var i = 0u; i < REFLECTION_SAMPLES; i += 1u) {
        let s = reflection_sample(m, n, ng, wo, &rng);
        let dst = id.x * 17u + i * 2u;
        output[dst] = vec4<u32>(bitcast<vec3<u32>>(s.direction), u32(s.accepted));
        output[dst + 1u] = bitcast<vec4<u32>>(vec4<f32>(s.weight, 0.0));
    }
    output[id.x * 17u + 16u] = vec4<u32>(rng_next(&rng), 0u, 0u, 0u);
}"
    )
}

#[test]
fn gpu_reflection_sampler_matches_cpu_directions_weights_and_stream() {
    use forge_render_cpu::{sampling::Rng, surface::Surface};
    use forge_render_gpu::{await_mapped, map_read};
    use glam::DVec3;
    use wgpu::util::DeviceExt;

    let (device, queue) = match forge_render_rt::request_default_device() {
        Ok(gpu) => gpu,
        Err(err) if std::env::var("FORGE_REQUIRE_GPU").as_deref() == Ok("1") => {
            panic!("FORGE_REQUIRE_GPU=1 but no GPU: {err}");
        }
        Err(err) => {
            eprintln!("skipping GPU test: {err}");
            return;
        }
    };
    let n = DVec3::Z;
    let view = DVec3::new(0.6, 0.0, 0.8);
    let tilted = DVec3::new(0.8, 0.0, 0.6);
    let spec = DVec3::new(0.8, 0.3, 0.1);
    let trans = DVec3::new(0.0, 0.25, 1.0);
    let seed = 0x1234_abcd_8642_ace0_u64;
    let pack = |v: DVec3, w| [v.x as f32, v.y as f32, v.z as f32, w].map(f32::to_bits);
    let mut rows = Vec::new();
    let mut expected = Vec::new();
    let mut rejections = [0; 2];
    for history in [0_u32, 9] {
        for (n, ng, wo, alpha, specular) in [
            (tilted, tilted, view, 0.02_f64.powi(2), spec),
            (n, tilted, view, 0.8, spec),
            (n, n, -view, 0.35, spec),
            (n, n, view, 0.35, DVec3::ZERO),
        ] {
            let pixel = 1920 * 257 + 139 + expected.len() as u32;
            rows.extend([
                pack(n, 0.),
                pack(ng, 0.),
                pack(wo, 0.),
                pack(specular, alpha as f32),
                pack(trans, 0.),
                [seed as u32, (seed >> 32) as u32, pixel, history],
            ]);
            let surface = Surface {
                albedo: DVec3::splat(0.6),
                spec: specular,
                alpha,
                emission: DVec3::ZERO,
                trans,
                ior: 1.5,
                n,
            };
            let mut rng = Rng::new(seed ^ u64::from(history), u64::from(pixel));
            let samples: [_; 8] =
                std::array::from_fn(|_| match surface.sample_specular(n, wo, &mut rng) {
                    Some((wi, weight)) if ng.dot(wi) > 0.0 => Some((wi, weight / 8.0)),
                    Some(_) => {
                        rejections[0] += 1;
                        None
                    }
                    None => {
                        rejections[1] += usize::from(specular != DVec3::ZERO && n.dot(wo) > 0.0);
                        None
                    }
                });
            expected.push((samples, rng.next_u32()));
        }
    }
    assert!(rejections.into_iter().all(|count| count > 0));
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("reflection sampler probe"),
        source: wgpu::ShaderSource::Wgsl(reflection_probe_source().into()),
    });
    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: None,
        layout: None,
        module: &shader,
        entry_point: Some("probe"),
        compilation_options: Default::default(),
        cache: None,
    });
    let input = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: None,
        contents: bytemuck::cast_slice(&rows),
        usage: wgpu::BufferUsages::STORAGE,
    });
    let size = expected.len() as u64 * 17 * 16;
    let buffer = |usage| {
        device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size,
            usage,
            mapped_at_creation: false,
        })
    };
    let output = buffer(wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC);
    let read = buffer(wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST);
    let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &pipeline.get_bind_group_layout(0),
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: input.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: output.as_entire_binding(),
            },
        ],
    });
    let mut encoder = device.create_command_encoder(&Default::default());
    {
        let mut pass = encoder.begin_compute_pass(&Default::default());
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &group, &[]);
        pass.dispatch_workgroups(expected.len() as u32, 1, 1);
    }
    encoder.copy_buffer_to_buffer(&output, 0, &read, 0, size);
    queue.submit([encoder.finish()]);
    let slice = read.slice(..);
    await_mapped(&device, &map_read(&slice)).unwrap();
    let mapped = slice.get_mapped_range().unwrap();
    let actual: &[[u32; 4]] = bytemuck::cast_slice(&mapped);
    for (fixture, (samples, rng_tail)) in expected.into_iter().enumerate() {
        let rows = &actual[fixture * 17..][..17];
        assert_eq!(rows[16][0], rng_tail, "fixture {fixture} RNG");
        for (attempt, (sample, pair)) in
            samples.into_iter().zip(rows.as_chunks::<2>().0).enumerate()
        {
            let Some((wi, weight)) = sample else {
                assert_eq!(pair, &[[0; 4]; 2], "fixture {fixture}, attempt {attempt}");
                continue;
            };
            assert_eq!(pair[0][3], 1, "fixture {fixture}, attempt {attempt}");
            for (gpu, cpu) in pair.iter().zip([wi, weight]) {
                for (gpu, cpu) in gpu.map(f32::from_bits).into_iter().zip(cpu.to_array()) {
                    assert!(
                        (f64::from(gpu) - cpu).abs() <= 2e-6 + 2e-4 * cpu.abs(),
                        "fixture {fixture}, attempt {attempt}: {gpu} != {cpu}"
                    );
                }
            }
        }
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
