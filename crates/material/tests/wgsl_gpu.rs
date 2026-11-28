//! docs/specs/REALTIME.md §1.5: the IR channels through WGSL on the GPU against the CPU
//! interpreter. Skipped without an adapter unless FORGE_REQUIRE_GPU=1.

use forge_material::ir::{
    Alpha, Bump, Diffuse, DiffuseModel, Diffusion, Displacement, Fog, IrNode, MixMode, NormalMap,
    Outputs, ShadeCtx, ShadingGraph, Specular, TextureSampler, Transmission,
};
use forge_material::wgsl::{FALLBACK_COLOR, compile};

const SURFACE_FIELDS: usize = 24;
const POINTS: usize = 64;

struct Fallback;
impl TextureSampler for Fallback {
    fn sample(&self, _: &str, _: &ShadeCtx) -> [f64; 3] {
        FALLBACK_COLOR
    }
}

fn graph() -> ShadingGraph {
    let d = |color, roughness| Diffuse {
        color,
        level: 1.0,
        falloff: 0.5,
        model: DiffuseModel::OrenNayar,
        roughness,
    };
    let nodes = vec![
        IrNode::Color([0.25, 0.5, 0.75]),
        IrNode::Texture("noise".into()),
        IrNode::Scale {
            input: 0,
            factor: 1.5,
        },
        IrNode::Mix {
            base: 0,
            top: 1,
            mode: MixMode::Multiply,
            strength: 0.25,
        },
        IrNode::Color([1.5; 3]),
        IrNode::Color([0.8; 3]),
        IrNode::Color([1.0; 3]),
        IrNode::Color([0.1; 3]),
        IrNode::Fresnel {
            ior: 4,
            falloff: 5,
            center_darkening: 6,
            center_hold: 7,
        },
        IrNode::Ramp {
            a: 0,
            b: 6,
            position: 8,
            start: 7,
            end: 6,
            midpoint: 0,
        },
    ];
    let outputs = Outputs {
        diffuse: Some(d(3, 0.2)),
        specular: Some(Specular {
            kind: "ggx".into(),
            width: 0.3,
            falloff: 0.1,
            inner_width: 0.2,
            strength: 1.0,
            color: 0,
            metal: true,
            colored: false,
        }),
        emission: Some(9),
        transmission: Some(Transmission {
            color: 0,
            ior: 1.5,
            absorption: [0.1, 0.2, 0.3],
            absorption_distance: 10.0,
            fresnel_reflectivity: 1.0,
            total_internal_reflection: true,
            exit_reflections: true,
            additive: false,
        }),
        environment: Some(1),
        fog: Some(Fog {
            color: 2,
            distance: 100.0,
        }),
        alpha: Some(Alpha {
            texture: Some(0),
            color: [0.9, 0.8, 0.7],
            delta: [0.1, 0.2, 0.3],
            invert: false,
            soft: true,
            image_alpha: false,
            premultiplied: false,
        }),
        bump: Some(Bump {
            texture: 0,
            strength: 1.0,
        }),
        normal: Some(NormalMap {
            texture: 1,
            strength: 0.5,
            method: "object".into(),
            flip: [true, false, true],
            swap_yz: false,
        }),
        displacement: Some(Displacement {
            texture: 0,
            strength: 2.0,
            height: 1.0,
            kind: "rgb_object".into(),
        }),
        diffusion: Some(Diffusion {
            brightness: 1.0,
            affect_luminance: true,
            affect_specular: false,
            affect_reflection: true,
        }),
        ..Default::default()
    };
    ShadingGraph { nodes, outputs }
}

fn ctx(i: usize) -> ShadeCtx {
    let t = i as f64 / POINTS as f64;
    ShadeCtx {
        uvw: [t, 1.0 - t, t * 0.5],
        position: [t * 200.0, t * 40.0, -t * 100.0],
        normal: [0.0, 0.0, 1.0],
        view: [(1.0 - t * t).sqrt(), 0.0, -t],
        time: t * 10.0,
        vertex_weight: None,
        ao: None,
    }
}

/// The CPU values of every Surface row, in the codegen order.
fn expected(g: &ShadingGraph, c: &ShadeCtx) -> [[f64; 4]; SURFACE_FIELDS] {
    let sampler = Fallback;
    let colour = |ix: Option<usize>| ix.map_or([0.0; 3], |ix| g.eval(ix, &sampler, c));
    let mut out = [[0.0; 4]; SURFACE_FIELDS];
    let o = &g.outputs;
    if let Some(d) = &o.diffuse {
        out[0] = [
            colour(Some(d.color))[0],
            colour(Some(d.color))[1],
            colour(Some(d.color))[2],
            d.level,
        ];
        out[1] = [
            d.falloff,
            d.roughness,
            match d.model {
                DiffuseModel::Lambertian => 0.0,
                DiffuseModel::OrenNayar => 1.0,
            },
            1.0,
        ];
    }
    if let Some(sp) = &o.specular {
        let cc = colour(Some(sp.color));
        out[2] = [cc[0], cc[1], cc[2], sp.strength];
        out[3] = [sp.width, sp.falloff, sp.inner_width, f64::from(sp.metal)];
        let kind = match sp.kind.as_str() {
            "beckmann" => 0.0,
            "ggx" => 1.0,
            "phong" => 2.0,
            "ward" => 3.0,
            "anisotropic" => 4.0,
            "lambertian" => 5.0,
            "oren_nayar" => 6.0,
            "irawan" => 7.0,
            "reflection_legacy" => 8.0,
            "blinn_legacy" => 9.0,
            "phong_legacy" => 10.0,
            _ => 0.0,
        };
        out[4] = [1.0, kind, f64::from(sp.colored), 0.0];
    }
    if let Some(e) = o.emission {
        let cc = colour(Some(e));
        out[5] = [cc[0], cc[1], cc[2], 1.0];
    }
    if let Some(t) = &o.transmission {
        let cc = colour(Some(t.color));
        out[6] = [cc[0], cc[1], cc[2], t.ior];
        out[7] = [
            t.absorption[0],
            t.absorption[1],
            t.absorption[2],
            t.absorption_distance,
        ];
        out[8] = [
            t.fresnel_reflectivity,
            f64::from(t.total_internal_reflection),
            f64::from(t.exit_reflections),
            f64::from(t.additive),
        ];
        out[9] = [1.0, 0.0, 0.0, 0.0];
    }
    if let Some(e) = o.environment {
        let cc = colour(Some(e));
        out[10] = [cc[0], cc[1], cc[2], 1.0];
    }
    if let Some(fo) = &o.fog {
        let cc = colour(Some(fo.color));
        out[11] = [cc[0], cc[1], cc[2], fo.distance];
        out[12] = [1.0, 0.0, 0.0, 0.0];
    }
    out[13] = [1.0, 1.0, 1.0, 0.0];
    if let Some(a) = &o.alpha {
        out[13] = [a.color[0], a.color[1], a.color[2], f64::from(a.soft)];
        out[14] = [a.delta[0], a.delta[1], a.delta[2], f64::from(a.invert)];
        out[15] = [
            1.0,
            f64::from(a.image_alpha),
            f64::from(a.premultiplied),
            0.0,
        ];
    }
    if let Some(b) = &o.bump {
        let cc = colour(Some(b.texture));
        out[16] = [cc[0], cc[1], cc[2], b.strength];
        out[17] = [1.0, 0.0, 0.0, 0.0];
    }
    if let Some(n) = &o.normal {
        let cc = colour(Some(n.texture));
        out[18] = [cc[0], cc[1], cc[2], n.strength];
        let flips =
            u32::from(n.flip[0]) | (u32::from(n.flip[1]) << 1) | (u32::from(n.flip[2]) << 2);
        let method = match n.method.as_str() {
            "tangent" => 0.0,
            "object" => 1.0,
            "world" => 2.0,
            _ => 0.0,
        };
        out[19] = [1.0, method, flips as f64, f64::from(n.swap_yz)];
    }
    if let Some(d) = &o.displacement {
        let cc = colour(Some(d.texture));
        out[20] = [cc[0], cc[1], cc[2], d.strength];
        let kind = match d.kind.as_str() {
            "intensity" => 0.0,
            "intensity_centered" => 1.0,
            "red_green" => 2.0,
            "rgb_tangent" => 3.0,
            "rgb_object" => 4.0,
            "rgb_world" => 5.0,
            _ => 0.0,
        };
        out[21] = [1.0, d.height, kind, 0.0];
    }
    if let Some(d) = &o.diffusion {
        out[22] = [
            d.brightness,
            f64::from(d.affect_luminance),
            f64::from(d.affect_specular),
            f64::from(d.affect_reflection),
        ];
        out[23] = [1.0, 0.0, 0.0, 0.0];
    }
    out
}

fn gpu() -> Option<(wgpu::Device, wgpu::Queue)> {
    let instance =
        wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle_from_env());
    let adapter =
        match pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
        {
            Ok(a) => a,
            Err(e) if std::env::var("FORGE_REQUIRE_GPU").as_deref() == Ok("1") => {
                panic!("FORGE_REQUIRE_GPU=1 but no GPU: {e}")
            }
            Err(e) => {
                eprintln!("skipping GPU test: {e}");
                return None;
            }
        };
    let (device, queue) = pollster::block_on(adapter.request_device(&Default::default())).unwrap();
    Some((device, queue))
}

#[test]
fn ir_channels_match_the_cpu_on_the_gpu() {
    let Some((device, queue)) = gpu() else { return };
    let g = graph();
    let m = compile(&g);
    let source = format!(
        "{}{}",
        m.source,
        r#"
@group(0) @binding(0) var<storage, read> forge_in: array<vec4<f32>>;
@group(0) @binding(1) var<storage, read_write> forge_out: array<Surface>;
@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    if (id.x >= arrayLength(&forge_out)) { return; }
    let base = id.x * 5u;
    var ctx: ShadeCtx;
    ctx.uvw = forge_in[base].xyz;
    ctx.time = forge_in[base].w;
    ctx.position = forge_in[base + 1u].xyz;
    ctx.normal = forge_in[base + 2u].xyz;
    ctx.view = forge_in[base + 3u].xyz;
    ctx.vertex_weight = forge_in[base + 4u].x;
    ctx.ao = forge_in[base + 4u].y;
    forge_out[id.x] = forge_surface(ctx);
}
"#
    );
    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("ir"),
        source: wgpu::ShaderSource::Wgsl(source.into()),
    });
    let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: None,
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
        ],
    });
    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: None,
        layout: Some(
            &device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &[Some(&layout)],
                immediate_size: 0,
            }),
        ),
        module: &module,
        entry_point: Some("main"),
        compilation_options: Default::default(),
        cache: None,
    });

    let mut input = Vec::with_capacity(POINTS * 5 * 4);
    for i in 0..POINTS {
        let c = ctx(i);
        input.extend_from_slice(&c.uvw.map(|v| v as f32));
        input.push(c.time as f32);
        input.extend_from_slice(&c.position.map(|v| v as f32));
        input.push(0.0);
        input.extend_from_slice(&c.normal.map(|v| v as f32));
        input.push(0.0);
        input.extend_from_slice(&c.view.map(|v| v as f32));
        input.push(0.0);
        input.push(c.vertex_weight.unwrap_or(0.0) as f32);
        input.push(c.ao.unwrap_or(1.0) as f32);
        input.extend_from_slice(&[0.0, 0.0]);
    }
    let input_bytes: Vec<u8> = input.iter().flat_map(|v| v.to_le_bytes()).collect();
    let output_size = (POINTS * SURFACE_FIELDS * 4 * 4) as u64;
    let in_buf = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: input_bytes.len() as u64,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    queue.write_buffer(&in_buf, 0, &input_bytes);
    let out_buf = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: output_size,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    let staging = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: output_size,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &layout,
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
    let mut encoder =
        device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
    {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: None,
            timestamp_writes: None,
        });
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &bind, &[]);
        pass.dispatch_workgroups((POINTS as u32).div_ceil(64), 1, 1);
    }
    encoder.copy_buffer_to_buffer(&out_buf, 0, &staging, 0, output_size);
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

    for i in 0..POINTS {
        let expect = expected(&g, &ctx(i));
        for field in 0..SURFACE_FIELDS {
            for k in 0..4 {
                let got = values[(i * SURFACE_FIELDS + field) * 4 + k];
                let want = expect[field][k] as f32;
                assert!(
                    (got - want).abs() <= 1e-4,
                    "point {i} field {field} component {k}: {got} vs {want}"
                );
            }
        }
    }
}
