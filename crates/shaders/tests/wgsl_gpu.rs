//! docs/specs/REALTIME.md §2.9: every shader family through WGSL on the GPU against the CPU
//! `Shader::sample`, on a grid of contexts. Skipped without an adapter unless
//! FORGE_REQUIRE_GPU=1. One device for the whole file, used one test at a time.

use std::collections::BTreeMap;
use std::sync::{Mutex, MutexGuard, OnceLock};

mod common;

use common::*;
use forge_material::ir::{IrNode, Outputs, ShadeCtx, ShadingGraph};
use forge_shaders::shader::*;
use forge_shaders::wgsl::compile;

const POINTS: usize = 192;

struct Gpu {
    device: wgpu::Device,
    queue: wgpu::Queue,
}

fn gpu() -> Option<(MutexGuard<'static, ()>, &'static Gpu)> {
    static GPU: OnceLock<Option<Gpu>> = OnceLock::new();
    static LOCK: Mutex<()> = Mutex::new(());
    let gpu = GPU.get_or_init(|| {
        let instance =
            wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle_from_env());
        match pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
        {
            Ok(adapter) => {
                let (device, queue) =
                    pollster::block_on(adapter.request_device(&Default::default())).unwrap();
                Some(Gpu { device, queue })
            }
            Err(e) if std::env::var("FORGE_REQUIRE_GPU").as_deref() == Ok("1") => {
                panic!("FORGE_REQUIRE_GPU=1 but no GPU: {e}")
            }
            Err(e) => {
                eprintln!("skipping GPU test: {e}");
                None
            }
        }
    });
    let guard = LOCK.lock().unwrap_or_else(|p| p.into_inner());
    gpu.as_ref().map(|g| (guard, g))
}

/// Deterministic pseudo-random numbers in [0, 1).
struct Lcg(u64);
impl Lcg {
    fn next(&mut self) -> f64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((self.0 >> 40) as f64) / (1u64 << 24) as f64
    }
}

/// The grid: values are rounded to f32 first so CPU and GPU see the same inputs. Every
/// tenth point has no geometry (zero normal), every seventh no vertex weight, every
/// eleventh no occlusion value.
fn contexts() -> Vec<ShadeCtx> {
    let mut r = Lcg(0x5eed);
    let f = |v: f64| f64::from(v as f32);
    (0..POINTS)
        .map(|i| {
            let mut v = |lo: f64, hi: f64| f(lo + (hi - lo) * r.next());
            let geometry = i % 10 != 0;
            ShadeCtx {
                uvw: [v(-0.25, 1.25), v(-0.25, 1.25), v(0.0, 1.0)],
                position: [v(-2.0, 2.0), v(-2.0, 2.0), v(-2.0, 2.0)],
                normal: if geometry {
                    [v(-1.0, 1.0), v(-1.0, 1.0), v(0.1, 1.0)]
                } else {
                    [0.0; 3]
                },
                view: if geometry {
                    [v(-1.0, 1.0), v(-1.0, 1.0), v(-1.0, 1.0)]
                } else {
                    [0.0; 3]
                },
                time: v(0.0, 6.0),
                vertex_weight: (i % 7 != 0).then(|| v(0.0, 1.0)),
                ao: (i % 11 != 0).then(|| v(0.0, 1.0)),
            }
        })
        .collect()
}

/// Runs the cases on the GPU; returns the colour of case `c` at point `i` at `[c][i]`.
fn run_gpu(gpu: &Gpu, cases: &[Case], ctxs: &[ShadeCtx]) -> Vec<Vec<[f32; 3]>> {
    let set = ShaderSet {
        shaders: cases
            .iter()
            .map(|c| (c.name.clone(), c.shader.clone()))
            .collect::<BTreeMap<_, _>>(),
    };
    let graph = ShadingGraph {
        nodes: cases
            .iter()
            .map(|c| IrNode::Texture(c.name.clone()))
            .collect(),
        outputs: Outputs::default(),
    };
    let m = compile(&graph, &set);
    let n = cases.len();
    let mut kernel = String::from(
        r#"
@group(0) @binding(0) var<storage, read> forge_in: array<vec4<f32>>;
@group(0) @binding(1) var<storage, read_write> forge_out: array<vec4<f32>>;
@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    if (id.x >= arrayLength(&forge_in) / 5u) { return; }
    let base = id.x * 5u;
    var ctx: ShadeCtx;
    ctx.uvw = forge_in[base].xyz;
    ctx.time = forge_in[base].w;
    ctx.position = forge_in[base + 1u].xyz;
    ctx.normal = forge_in[base + 2u].xyz;
    ctx.view = forge_in[base + 3u].xyz;
    ctx.vertex_weight = forge_in[base + 4u].x;
    ctx.ao = forge_in[base + 4u].y;
"#,
    );
    for i in 0..n {
        kernel.push_str(&format!(
            "    forge_out[id.x * {n}u + {i}u] = vec4<f32>(ir{i}(ctx), 1.0);\n"
        ));
    }
    kernel.push_str("}\n");
    let source = format!("{}{}", m.source, kernel);
    let (device, queue) = (&gpu.device, &gpu.queue);
    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("shaders"),
        source: wgpu::ShaderSource::Wgsl(source.into()),
    });
    let storage = |binding, read_only| wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Storage { read_only },
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    };
    let layout0 = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: None,
        entries: &[storage(0, true), storage(1, false)],
    });
    let layout1 = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: None,
        entries: &(0..m.resources.len() as u32)
            .map(|binding| wgpu::BindGroupLayoutEntry {
                binding,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: false },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            })
            .collect::<Vec<_>>(),
    });
    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: None,
        layout: Some(
            &device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &[Some(&layout0), Some(&layout1)],
                immediate_size: 0,
            }),
        ),
        module: &module,
        entry_point: Some("main"),
        compilation_options: Default::default(),
        cache: None,
    });

    let mut input: Vec<f32> = Vec::with_capacity(ctxs.len() * 20);
    for c in ctxs {
        input.extend(c.uvw.map(|v| v as f32));
        input.push(c.time as f32);
        input.extend(c.position.map(|v| v as f32));
        input.push(0.0);
        input.extend(c.normal.map(|v| v as f32));
        input.push(0.0);
        input.extend(c.view.map(|v| v as f32));
        input.push(0.0);
        input.push(c.vertex_weight.unwrap_or(0.0) as f32);
        input.push(c.ao.unwrap_or(1.0) as f32);
        input.extend([0.0, 0.0]);
    }
    let bytes: Vec<u8> = input.iter().flat_map(|v| v.to_le_bytes()).collect();
    let out_size = (ctxs.len() * n * 16) as u64;
    let buffer = |size, usage| {
        device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size,
            usage,
            mapped_at_creation: false,
        })
    };
    let in_buf = buffer(
        bytes.len() as u64,
        wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
    );
    queue.write_buffer(&in_buf, 0, &bytes);
    let out_buf = buffer(
        out_size,
        wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
    );
    let staging = buffer(
        out_size,
        wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
    );
    let bind0 = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &layout0,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: in_buf.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: out_buf.as_entire_binding(),
            },
        ],
    });
    let views: Vec<wgpu::TextureView> = m
        .resources
        .iter()
        .map(|r| {
            let img = r
                .downcast_ref::<forge_image::Image>()
                .expect("bitmap resource");
            let tex = device.create_texture(&wgpu::TextureDescriptor {
                label: None,
                size: wgpu::Extent3d {
                    width: img.width,
                    height: img.height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba32Float,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            });
            let data: Vec<u8> = img.data.iter().flat_map(|v| v.to_le_bytes()).collect();
            queue.write_texture(
                tex.as_image_copy(),
                &data,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(img.width * 16),
                    rows_per_image: Some(img.height),
                },
                wgpu::Extent3d {
                    width: img.width,
                    height: img.height,
                    depth_or_array_layers: 1,
                },
            );
            tex.create_view(&Default::default())
        })
        .collect();
    let bind1 = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &layout1,
        entries: &views
            .iter()
            .enumerate()
            .map(|(i, v)| wgpu::BindGroupEntry {
                binding: i as u32,
                resource: wgpu::BindingResource::TextureView(v),
            })
            .collect::<Vec<_>>(),
    });
    let mut encoder =
        device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
    {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: None,
            timestamp_writes: None,
        });
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &bind0, &[]);
        pass.set_bind_group(1, &bind1, &[]);
        pass.dispatch_workgroups((ctxs.len() as u32).div_ceil(64), 1, 1);
    }
    encoder.copy_buffer_to_buffer(&out_buf, 0, &staging, 0, out_size);
    queue.submit([encoder.finish()]);
    let slice = staging.slice(..);
    slice.map_async(wgpu::MapMode::Read, |r| r.expect("readback map"));
    device
        .poll(wgpu::PollType::wait_indefinitely())
        .expect("poll");
    let data = slice.get_mapped_range().unwrap();
    let values: Vec<f32> = data
        .as_chunks::<4>()
        .0
        .iter()
        .map(|b| f32::from_le_bytes(*b))
        .collect();
    drop(data);
    staging.unmap();
    (0..n)
        .map(|c| {
            (0..ctxs.len())
                .map(|i| {
                    let o = (i * n + c) * 4;
                    [values[o], values[o + 1], values[o + 2]]
                })
                .collect()
        })
        .collect()
}

/// Compares every case on the grid; prints the measured maximum per case (the numbers
/// of REALTIME.md §2.9) and asserts each against its bound.
fn check(family: &str, cases: Vec<Case>) {
    let Some((_guard, gpu)) = gpu() else { return };
    let ctxs = contexts();
    let got = run_gpu(gpu, &cases, &ctxs);
    let mut failures = Vec::new();
    for (c, rows) in cases.iter().zip(&got) {
        let (mut worst, mut at) = (0.0f64, 0usize);
        for (i, (ctx, g)) in ctxs.iter().zip(rows).enumerate() {
            let want = c.shader.sample(ctx);
            for k in 0..3 {
                let e = (f64::from(g[k]) - want[k]).abs();
                if !e.is_finite() || e > worst {
                    worst = if e.is_finite() { e } else { f64::INFINITY };
                    at = i;
                }
            }
        }
        eprintln!(
            "{family}/{}: max error {worst:.3e} at point {at} (bound {:.0e})",
            c.name, c.bound
        );
        if worst > c.bound {
            failures.push(format!(
                "{family}/{}: {worst:.3e} > {:.0e}",
                c.name, c.bound
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn noise_matches_the_cpu() {
    check("noise", noise_cases());
}

#[test]
fn gradient_matches_the_cpu() {
    check("gradient", gradient_cases());
}

#[test]
fn colour_checkerboard_fresnel_falloff_match_the_cpu() {
    check("surface", surface_cases());
}

#[test]
fn tiles_match_the_cpu() {
    check("tiles", tiles_cases());
}
