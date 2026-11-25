//! RENDER.md R20.3: the GPU compute tracer against the CPU tracer on the O30 scenes.
//!
//! Skipped without an adapter unless `FORGE_REQUIRE_GPU=1`. One device per test file,
//! created behind the static lock (§5a of the implementer playbook).

#[path = "../../render-cpu/tests/common/mod.rs"]
mod common;

use common::golden::{case_names, load};
use common::image_parity;
use forge_parity_harness::{CompareOptions, ImageBuf, compare_image};
use forge_render_api::*;
use forge_render_cpu::CpuEngine;
use forge_render_cpu::sampling::{Rng, splitmix};
use forge_render_gpu::{GpuEngine, RNG_WGSL, request_default_device};
use std::sync::Mutex;

/// R20.3: CPU against GPU beauty, no exposure normalisation.
const MIN_PSNR: f64 = 35.0;

/// Cases the GPU engine rejects (R20.4); none of the O30 cases is pinned any more.
const UNSUPPORTED: &[(&str, &str)] = &[];

static GPU: Mutex<Option<Option<(wgpu::Device, wgpu::Queue)>>> = Mutex::new(None);

fn gpu() -> Option<(wgpu::Device, wgpu::Queue)> {
    let mut slot = GPU.lock().unwrap();
    slot.get_or_insert_with(|| match request_default_device() {
        Ok(d) => Some(d),
        Err(e) if std::env::var("FORGE_REQUIRE_GPU").as_deref() == Ok("1") => {
            panic!("FORGE_REQUIRE_GPU=1 but no GPU: {e}")
        }
        Err(e) => {
            eprintln!("skipping GPU test: {e}");
            None
        }
    })
    .clone()
}

fn unsupported_reason(name: &str) -> Option<&'static str> {
    UNSUPPORTED
        .iter()
        .find(|(n, _)| *n == name)
        .map(|(_, r)| *r)
}

#[test]
fn beauty_matches_the_cpu_on_the_o30_cases() {
    let Some((device, queue)) = gpu() else {
        return;
    };
    let engine = GpuEngine::with_device(device, queue);
    let settings = image_parity::settings((320, 240), Vec::new());
    let mut checked = 0;
    let mut pinned = 0;
    for name in case_names() {
        let case = load(&name);
        let cpu = CpuEngine::new()
            .render(&case.snapshot, &settings, &Control::new())
            .unwrap_or_else(|e| panic!("{name}: CPU render failed: {e}"));
        let gpu = engine.render(&case.snapshot, &settings, &Control::new());
        if let Some(reason) = unsupported_reason(&name) {
            match gpu {
                Err(RenderError::Unsupported(_)) => pinned += 1,
                Err(e) => panic!("{name}: expected Unsupported ({reason}), got {e}"),
                Ok(_) => panic!("{name}: expected Unsupported ({reason}), rendered"),
            }
            continue;
        }
        let gpu = gpu.unwrap_or_else(|e| panic!("{name}: GPU render failed: {e}"));
        assert_eq!(
            (gpu.width, gpu.height),
            (cpu.width, cpu.height),
            "{name}: image size"
        );
        let a = ImageBuf::from_rgba(cpu.width, cpu.height, &cpu.beauty.data);
        let b = ImageBuf::from_rgba(gpu.width, gpu.height, &gpu.beauty.data);
        let report = compare_image(&a, &b, &CompareOptions::default()).unwrap();
        assert!(
            report.psnr >= MIN_PSNR,
            "{name}: CPU against GPU PSNR {:.2} dB is below {MIN_PSNR} dB",
            report.psnr
        );
        eprintln!("{name}: {:.2} dB", report.psnr);
        checked += 1;
    }
    assert_eq!(
        (checked, pinned),
        (case_names().len() - UNSUPPORTED.len(), UNSUPPORTED.len()),
        "every O30 case must be either checked or pinned"
    );
}

/// Every AOV the CPU writes has the same values on the GPU (R3, R20.2).
#[test]
fn aovs_match_the_cpu() {
    let Some((device, queue)) = gpu() else {
        return;
    };
    let engine = GpuEngine::with_device(device, queue);
    let all = [
        Aov::Depth,
        Aov::Normal,
        Aov::Albedo,
        Aov::Position,
        Aov::ObjectId,
        Aov::MaterialId,
        Aov::Shadow,
        Aov::Diffuse,
        Aov::Specular,
        Aov::Emission,
    ];
    let settings = image_parity::settings((320, 240), all.to_vec());
    let case = load("cube-plane");
    let cpu = CpuEngine::new()
        .render(&case.snapshot, &settings, &Control::new())
        .unwrap();
    let gpu = engine
        .render(&case.snapshot, &settings, &Control::new())
        .unwrap();
    for a in all {
        let x = cpu.aovs.get(&a).unwrap_or_else(|| panic!("{a:?} missing"));
        let y = gpu.aovs.get(&a).unwrap_or_else(|| panic!("{a:?} missing"));
        let a1 = ImageBuf::from_rgba(320, 240, &x.data);
        let b1 = ImageBuf::from_rgba(320, 240, &y.data);
        let r = compare_image(&a1, &b1, &CompareOptions::default()).unwrap();
        assert!(r.psnr >= MIN_PSNR, "{a:?}: PSNR {:.2} dB", r.psnr);
        eprintln!("{a:?}: {:.2} dB", r.psnr);
    }
}

/// R4 with the GPU engine: one `Tile` per tile, the tiles rebuild the image, cancel stops.
#[test]
fn events_and_cancel() {
    let Some((device, queue)) = gpu() else {
        return;
    };
    let engine = GpuEngine::with_device(device, queue);
    let case = load("cube-plane");
    let settings = RenderSettings {
        tile: 32,
        ..RenderSettings::new(64, 48, 1)
    };
    let events = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let sink = events.clone();
    let control = Control::with_events(move |e| sink.lock().unwrap().push(e.clone()));
    let out = engine.render(&case.snapshot, &settings, &control).unwrap();
    let events = events.lock().unwrap();
    let tiles: Vec<&RenderEvent> = events
        .iter()
        .filter(|e| matches!(e, RenderEvent::Tile { .. }))
        .collect();
    assert_eq!(tiles.len(), 4, "64x48 at tile 32 is four tiles");
    assert!(matches!(
        events.first(),
        Some(RenderEvent::Started { tiles: 4, .. })
    ));
    assert!(matches!(events.last(), Some(RenderEvent::Finished)));
    let progress: Vec<usize> = events
        .iter()
        .filter_map(|e| match e {
            RenderEvent::Progress { done, .. } => Some(*done),
            _ => None,
        })
        .collect();
    assert_eq!(progress, vec![1, 2, 3, 4]);
    for e in &tiles {
        if let RenderEvent::Tile {
            x,
            y,
            width,
            height,
            rgba,
        } = e
        {
            for j in 0..*height {
                for i in 0..*width {
                    let k = ((j * width + i) * 4) as usize;
                    assert_eq!(
                        out.beauty.pixel(x + i, y + j),
                        [rgba[k], rgba[k + 1], rgba[k + 2], rgba[k + 3]],
                        "tile pixel ({}, {})",
                        x + i,
                        y + j
                    );
                }
            }
        }
    }
    let cancel = Control::new();
    cancel.cancel();
    assert!(matches!(
        engine.render(&case.snapshot, &settings, &cancel),
        Err(RenderError::Cancelled)
    ));
}

#[test]
fn unsupported_pins_are_exact() {
    // The list above is a contract: a case that starts working must leave it.
    let names: Vec<String> = case_names();
    for (name, _) in UNSUPPORTED {
        assert!(
            names.iter().any(|n| n == name),
            "{name} is pinned but has no golden case"
        );
    }
}

#[test]
fn image_environment_is_unsupported() {
    let Some((device, queue)) = gpu() else {
        return;
    };
    let engine = GpuEngine::with_device(device, queue);
    let mut snap = Snapshot::default();
    let img = forge_image::Image::new(4, 2, forge_image::AlphaMode::Straight);
    snap.environment.lighting = Lighting::Image {
        image: std::sync::Arc::new(img),
        rotation: 0.0,
        intensity: 1.0,
    };
    snap.materials.push(MaterialData::new(
        "m",
        forge_material::compile(&forge_material::Material::new()),
    ));
    let e = engine
        .render(&snap, &RenderSettings::new(8, 8, 1), &Control::new())
        .unwrap_err();
    assert!(matches!(e, RenderError::Unsupported(_)), "{e}");
}

/// The WGSL 64-bit emulation and PCG32 against `forge_render_cpu::sampling`.
#[test]
fn random_sequence_matches_the_cpu() {
    let Some((device, queue)) = gpu() else {
        return;
    };
    const N: usize = 256;
    let source = format!(
        "{RNG_WGSL}
@group(0) @binding(0) var<storage, read_write> out: array<u32>;
@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {{
    if (id.x >= {N}u) {{ return; }}
    let seed = splitmix(vec2<u32>(id.x * 7919u + 13u, 0x12345678u));
    var r = rng_new(vec2<u32>(id.x * 104729u + 7u, 0x9E3779B9u), vec2<u32>(id.x, 3u));
    let n0 = rng_next(&r);
    let n1 = rng_next(&r);
    let f = rng_f(&r);
    let base = id.x * 6u;
    out[base] = seed.x;
    out[base + 1u] = seed.y;
    out[base + 2u] = n0;
    out[base + 3u] = n1;
    out[base + 4u] = bitcast<u32>(f);
    out[base + 5u] = u64_shr(seed, 13u).x;
}}
"
    );
    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("rng"),
        source: wgpu::ShaderSource::Wgsl(source.into()),
    });
    let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: None,
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::COMPUTE,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Storage { read_only: false },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }],
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
    let bytes = (N * 6 * 4) as u64;
    let buf = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: bytes,
        usage: wgpu::BufferUsages::STORAGE
            | wgpu::BufferUsages::COPY_SRC
            | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let staging = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: bytes,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: buf.as_entire_binding(),
        }],
    });
    let mut enc = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
    {
        let mut pass = enc.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: None,
            timestamp_writes: None,
        });
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &bind, &[]);
        pass.dispatch_workgroups((N as u32).div_ceil(64), 1, 1);
    }
    enc.copy_buffer_to_buffer(&buf, 0, &staging, 0, bytes);
    queue.submit([enc.finish()]);
    let slice = staging.slice(..);
    slice.map_async(wgpu::MapMode::Read, |r| r.expect("map"));
    device
        .poll(wgpu::PollType::wait_indefinitely())
        .expect("poll");
    let data = slice.get_mapped_range().unwrap();
    let got: Vec<u32> = data
        .as_chunks::<4>()
        .0
        .iter()
        .map(|b| u32::from_le_bytes(*b))
        .collect();
    drop(data);
    staging.unmap();

    for i in 0..N {
        // The shader's vec2 is (low word, high word).
        let seed = splitmix((i as u64 * 7_919 + 13) | (0x1234_5678u64 << 32));
        let mut r = Rng::new(
            (i as u64 * 104_729 + 7) | (0x9E37_79B9u64 << 32),
            (i as u64) | (3u64 << 32),
        );
        let n0 = r.next_u32();
        let n1 = r.next_u32();
        let f = r.f();
        let base = i * 6;
        assert_eq!(got[base], seed as u32, "point {i}: splitmix low");
        assert_eq!(
            got[base + 1],
            (seed >> 32) as u32,
            "point {i}: splitmix high"
        );
        assert_eq!(got[base + 2], n0, "point {i}: next_u32");
        assert_eq!(got[base + 3], n1, "point {i}: next_u32 (2)");
        let gf = f32::from_bits(got[base + 4]) as f64;
        assert!((gf - f).abs() < 1e-6, "point {i}: f {gf} vs {f}");
        assert_eq!(got[base + 5], (seed >> 13) as u32, "point {i}: shift");
    }
}

/// R20.1b: bitmaps (two images sharing one module), noise, vertex map and AO shaders across
/// several materials of one module give the CPU image. The geometry is the O30 `checker-plane`
/// scene with its instances spread over the materials.
#[test]
fn shader_materials_share_one_module_and_match_the_cpu() {
    use forge_core::Value;
    use forge_material::{Material, compile};
    use forge_shaders::shader::{Bitmap, Shader, ShaderSet};
    use std::sync::Arc;
    let Some((device, queue)) = gpu() else {
        return;
    };
    let engine = GpuEngine::with_device(device, queue);
    let image = |w: u32, h: u32, seed: u32| {
        let mut img = forge_image::Image::new(w, h, forge_image::AlphaMode::Straight);
        for y in 0..h {
            for x in 0..w {
                let k = (x * 7 + y * 13 + seed * 5) % 11;
                img.set_pixel(
                    x,
                    y,
                    [k as f32 / 10.0, ((x + y) % 3) as f32 / 2.0, 0.25, 1.0],
                );
            }
        }
        Arc::new(img)
    };
    let mut set = ShaderSet::default();
    let mut put = |name: &str, s: Shader| {
        set.shaders.insert(name.to_string(), s);
    };
    put(
        "bm_a",
        Shader::Bitmap(Bitmap {
            image: image(5, 3, 1),
            repeat: true,
            interpolate: true,
        }),
    );
    put(
        "bm_b",
        Shader::Bitmap(Bitmap {
            image: image(8, 8, 2),
            repeat: false,
            interpolate: false,
        }),
    );
    put(
        "vmap",
        Shader::VertexMap {
            low: [0.9, 0.2, 0.1],
            high: [0.1, 0.2, 0.9],
        },
    );
    put(
        "ao",
        Shader::Ao {
            low: [0.1, 0.1, 0.1],
            high: [0.3, 0.9, 0.3],
        },
    );
    let sampler = Arc::new(set);
    let material = |tex: &str, bump: Option<&str>| {
        let mut o = std::collections::BTreeMap::new();
        o.insert("color.texture".to_string(), Value::String(tex.into()));
        if let Some(b) = bump {
            o.insert("bump.texture".to_string(), Value::String(b.into()));
            o.insert("bump.strength".to_string(), Value::Float(0.5));
        }
        MaterialData::new("m", compile(&Material::from_values(&o))).with_sampler(sampler.clone())
    };
    let mut case = load("checker-plane");
    case.snapshot.materials = vec![
        material("bm_a", None),
        material("bm_b", Some("bm_a")),
        material("vmap", None),
        material("ao", Some("bm_b")),
    ];
    let count = case.snapshot.materials.len();
    for (i, inst) in case.snapshot.instances.iter_mut().enumerate() {
        inst.material = i % count;
    }
    let settings = image_parity::settings((160, 120), Vec::new());
    for first in 0..count {
        // Rotate which material each instance uses so every material is exercised alone.
        for (i, inst) in case.snapshot.instances.iter_mut().enumerate() {
            inst.material = (i + first) % count;
        }
        let cpu = CpuEngine::new()
            .render(&case.snapshot, &settings, &Control::new())
            .unwrap();
        let gpu = engine
            .render(&case.snapshot, &settings, &Control::new())
            .unwrap();
        let a = ImageBuf::from_rgba(cpu.width, cpu.height, &cpu.beauty.data);
        let b = ImageBuf::from_rgba(gpu.width, gpu.height, &gpu.beauty.data);
        let report = compare_image(&a, &b, &CompareOptions::default()).unwrap();
        eprintln!("shader materials, rotation {first}: {:.2} dB", report.psnr);
        assert!(
            report.psnr >= MIN_PSNR,
            "rotation {first}: PSNR {:.2} dB below {MIN_PSNR} dB",
            report.psnr
        );
    }
    // A sampler without a codegen is refused, not silently white.
    struct Opaque;
    impl forge_material::TextureSampler for Opaque {
        fn sample(&self, _: &str, _: &forge_material::ShadeCtx) -> [f64; 3] {
            [0.5; 3]
        }
    }
    let mut snap = case.snapshot.clone();
    snap.materials[0].sampler = Arc::new(Opaque);
    for inst in &mut snap.instances {
        inst.material = 0;
    }
    assert!(matches!(
        engine.render(&snap, &settings, &Control::new()),
        Err(RenderError::Unsupported(_))
    ));
}
