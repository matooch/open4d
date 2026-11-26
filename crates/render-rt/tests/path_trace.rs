//! R25.3 Tier 3 dispatches and R26 consistency across all render tiers.

#[path = "../../render-cpu/tests/common/mod.rs"]
mod common;

use common::{
    golden::{case_names, load},
    image_parity,
};
use forge_parity_harness::{CompareOptions, ImageBuf, compare_image};
use forge_render_api::*;
use forge_render_cpu::CpuEngine;
use forge_render_rt::{
    PathTraceEngine, RtEngine, ray::BackendKind, request_default_device, request_rt_device,
};
use glam::{DMat4, DQuat, DVec3};
use std::sync::Mutex;

type Device = (wgpu::Device, wgpu::Queue);
static DEVICES: Mutex<Option<Vec<Device>>> = Mutex::new(None);

fn device_backend(device: &wgpu::Device) -> BackendKind {
    if !device
        .features()
        .contains(wgpu::Features::EXPERIMENTAL_RAY_QUERY)
    {
        return BackendKind::Compute;
    }
    match device.adapter_info().backend {
        wgpu::Backend::Metal => BackendKind::Metal,
        wgpu::Backend::Vulkan => BackendKind::Vulkan,
        _ => BackendKind::Compute,
    }
}

fn with_devices<T>(run: impl FnMut(&Device) -> T) -> Vec<T> {
    let mut slot = DEVICES.lock().unwrap_or_else(|e| e.into_inner());
    let devices = slot.get_or_insert_with(|| match request_rt_device() {
        Ok(selected)
            if selected
                .0
                .features()
                .contains(wgpu::Features::EXPERIMENTAL_RAY_QUERY) =>
        {
            vec![
                request_default_device().expect("explicit compute device"),
                selected,
            ]
        }
        Ok(selected) => vec![selected],
        Err(e) if std::env::var("FORGE_REQUIRE_GPU").as_deref() == Ok("1") => {
            panic!("FORGE_REQUIRE_GPU=1 but no GPU: {e}")
        }
        Err(e) => {
            eprintln!("skipping GPU test: {e}");
            Vec::new()
        }
    });
    devices.iter().map(run).collect()
}

fn compare(snapshot: &Snapshot, settings: &RenderSettings, aovs: &[Aov]) -> Vec<RenderOutput> {
    // Don't build the reference on machines where the device check skips the test.
    let cpu = std::cell::OnceCell::new();
    let compare_options = CompareOptions::default();
    with_devices(|(device, queue)| {
        let cpu = cpu.get_or_init(|| {
            CpuEngine::new()
                .render(snapshot, settings, &Control::new())
                .unwrap()
        });
        let engine = PathTraceEngine::with_device(device.clone(), queue.clone());
        let expected = device_backend(device);
        let (gpu, backend) = engine
            .render_with_backend(snapshot, settings, &Control::new())
            .unwrap();
        assert_eq!(backend, expected);
        let check = |label, a: &[f32], b: &[f32]| {
            let image = |rgba| ImageBuf::from_rgba(settings.width, settings.height, rgba);
            let report = compare_image(&image(a), &image(b), &compare_options).unwrap();
            eprintln!("{backend:?} {label}: {:.2} dB", report.psnr);
            assert!(report.psnr >= 35.0, "{backend:?} {label}: {report:?}");
        };
        check("beauty", &cpu.beauty.data, &gpu.beauty.data);
        for aov in aovs {
            check(aov.name(), &cpu.aovs[aov].data, &gpu.aovs[aov].data);
        }
        gpu
    })
}

fn same_raw(a: &RenderOutput, b: &RenderOutput) {
    let same_bits = |a: &[f32], b: &[f32]| {
        a.iter()
            .map(|v| v.to_bits())
            .eq(b.iter().map(|v| v.to_bits()))
    };
    assert!(same_bits(&a.beauty.data, &b.beauty.data), "beauty");
    assert_eq!(a.aovs.len(), b.aovs.len());
    for (kind, image) in &a.aovs {
        assert!(same_bits(&image.data, &b.aovs[kind].data), "{kind:?}");
    }
}

#[test]
fn preview_accumulates_ordered_samples_and_filters_the_first_frame() {
    with_devices(|(device, queue)| {
        let engine = PathTraceEngine::with_device(device.clone(), queue.clone());
        let snapshot = load("area-soft").snapshot;
        let mut settings = image_parity::settings(
            (39, 31),
            vec![
                Aov::Diffuse,
                Aov::Shadow,
                Aov::Depth,
                Aov::Normal,
                Aov::ObjectId,
            ],
        );
        settings.region = Some([3, 4, 36, 29]);
        settings.tile = 12;
        let final_frame = engine
            .render(&snapshot, &settings, &Control::new())
            .unwrap();
        let mut view = engine.preview_view();
        let first = view
            .render_batch(&snapshot, &settings, 1, &Control::new())
            .unwrap();
        assert_eq!(first.samples, 1);
        assert_ne!(first.raw.beauty.data, first.denoised.data);
        assert_eq!(first.denoised.pixel(0, 0), [0.0; 4]);
        assert!(
            first.raw.beauty.data.iter().skip(3).step_by(4).eq(first
                .denoised
                .data
                .iter()
                .skip(3)
                .step_by(4))
        );

        let independent = engine
            .preview_view()
            .render_batch(&snapshot, &settings, 1, &Control::new())
            .unwrap();
        same_raw(&first.raw, &independent.raw);
        assert_eq!(first.denoised.data, independent.denoised.data);
        same_raw(
            &final_frame,
            &engine
                .render(&snapshot, &settings, &Control::new())
                .unwrap(),
        );

        settings.samples = 7; // The preview uses its explicit batch size, not the final budget.
        let second = view
            .render_batch(&snapshot, &settings, 3, &Control::new())
            .unwrap();
        assert_eq!(second.samples, 4);
        assert_ne!(first.raw.beauty.data, second.raw.beauty.data);
        let whole = engine
            .preview_view()
            .render_batch(&snapshot, &settings, 4, &Control::new())
            .unwrap();
        same_raw(&second.raw, &whole.raw);
        assert_eq!(second.denoised.data, whole.denoised.data);
        settings.samples = 4;
        same_raw(
            &second.raw,
            &engine
                .render(&snapshot, &settings, &Control::new())
                .unwrap(),
        );
    });
}

#[test]
fn preview_resets_on_edits_and_cancellation_does_not_commit() {
    use std::sync::{Arc, OnceLock};

    with_devices(|(device, queue)| {
        let engine = PathTraceEngine::with_device(device.clone(), queue.clone());
        let mut snapshot = load("area-soft").snapshot;
        let mut settings = image_parity::settings((40, 32), vec![Aov::ObjectId]);
        settings.tile = 16;
        let mut view = engine.preview_view();
        view.render_batch(&snapshot, &settings, 2, &Control::new())
            .unwrap();

        let shared = Arc::new(OnceLock::<Control>::new());
        let from_event = Arc::downgrade(&shared);
        let cancelled = Control::with_events(move |event| {
            if matches!(event, RenderEvent::Tile { .. }) {
                from_event.upgrade().unwrap().get().unwrap().cancel();
            }
        });
        assert!(shared.set(cancelled.clone()).is_ok());
        assert!(matches!(
            view.render_batch(&snapshot, &settings, 8, &cancelled),
            Err(RenderError::Cancelled)
        ));
        let mut broken = snapshot.clone();
        broken.instances[0].mesh = broken.meshes.len();
        assert!(matches!(
            view.render_batch(&broken, &settings, 8, &Control::new()),
            Err(RenderError::Snapshot(_))
        ));
        let retained = view
            .render_batch(&snapshot, &settings, 1, &Control::new())
            .unwrap();
        assert_eq!(retained.samples, 3);
        let fresh = |snapshot: &Snapshot, settings: &RenderSettings, n| {
            engine
                .preview_view()
                .render_batch(snapshot, settings, n, &Control::new())
                .unwrap()
        };
        same_raw(&retained.raw, &fresh(&snapshot, &settings, 3).raw);

        let check_reset = |view: &mut _, snapshot: &Snapshot, settings: &RenderSettings| {
            // An edit is immediately visible and does not retain the previous raw or filtered image.
            let view: &mut forge_render_rt::PathTraceView<'_> = view;
            let reset = view
                .render_batch(snapshot, settings, 1, &Control::new())
                .unwrap();
            let expected = fresh(snapshot, settings, 1);
            assert_eq!(reset.samples, 1);
            same_raw(&reset.raw, &expected.raw);
            assert_eq!(reset.denoised.data, expected.denoised.data);
        };
        snapshot.camera.transform[12] += 0.25;
        check_reset(&mut view, &snapshot, &settings);
        snapshot.lights.clear();
        check_reset(&mut view, &snapshot, &settings);
        settings.region = Some([2, 3, 25, 27]);
        check_reset(&mut view, &snapshot, &settings);
        settings.seed += 1;
        check_reset(&mut view, &snapshot, &settings);
    });
}

#[test]
fn area_light_paths_and_shadow_aov_match_cpu() {
    let case = load("area-soft");
    let aovs = [Aov::Diffuse, Aov::Shadow];
    let settings = image_parity::settings((48, 36), aovs.to_vec());
    compare(&case.snapshot, &settings, &aovs);
}

#[test]
fn o30_and_o43_cross_tier_consistency() {
    let settings = image_parity::settings((320, 240), Vec::new());
    let hybrid = RenderSettings {
        hybrid: HybridSettings {
            enabled: true,
            shadows: true,
            reflections: true,
            ao: true,
            gi: true,
            ..HybridSettings::default()
        },
        ..settings.clone()
    };
    assert_eq!(settings.samples, 16);
    assert!(settings.post.is_off() && hybrid.post.is_off());
    const BUDGETS: [f64; 3] = [24.0, 28.0, 35.0];
    // Only these CPU comparisons are pinned (RENDER.md R26). New backends
    // and GPU-to-GPU comparisons have no exemptions.
    const KNOWN_MISMATCH: [(&str, usize); 9] = [
        ("cube-plane", 1),
        ("cube-plane", 2),
        ("mat-bump-noise", 1),
        ("mat-glossy", 1),
        ("mat-glossy", 2),
        ("omni", 1),
        ("omni", 2),
        ("omni-shadow", 1),
        ("omni-shadow", 2),
    ];
    let required = std::env::var("FORGE_R26_HARDWARE")
        .ok()
        .map(|name| match name.as_str() {
            "Metal" => BackendKind::Metal,
            "Vulkan" => BackendKind::Vulkan,
            _ => panic!("FORGE_R26_HARDWARE must be Metal or Vulkan"),
        });
    let captures = std::env::var_os("FORGE_R26_METAL_IMAGES").map(std::path::PathBuf::from);
    if let Some(dir) = &captures {
        match required {
            Some(BackendKind::Metal) => std::fs::create_dir_all(dir).unwrap(),
            Some(BackendKind::Vulkan) => assert!(dir.is_dir(), "missing Metal captures: {dir:?}"),
            _ => panic!("FORGE_R26_METAL_IMAGES requires FORGE_R26_HARDWARE"),
        }
    }
    let mut exercised = Vec::new();
    let mut failures = Vec::new();
    let cases = case_names();
    assert!(!cases.is_empty(), "no O30/O43 render fixtures");
    for (name, _) in KNOWN_MISMATCH {
        assert!(
            cases.iter().any(|case| case == name),
            "missing pinned case {name}"
        );
    }
    for name in cases {
        let case = load(&name);
        let reference = std::cell::OnceCell::new();
        let mut compute = None;
        with_devices(|(device, queue)| {
            let cpu = reference.get_or_init(|| {
                let output = CpuEngine::new()
                    .render(&case.snapshot, &settings, &Control::new())
                    .unwrap_or_else(|e| panic!("{name}: CPU render failed: {e}"));
                beauty(&output)
            });
            let engine = RtEngine::with_device(device.clone(), queue.clone());
            let (raster, raster_backend) = engine
                .render_with_backend(&case.snapshot, &settings, &Control::new())
                .unwrap_or_else(|e| panic!("{name}: Tier 1 render failed: {e}"));
            let (hybrid, hybrid_backend) = engine
                .render_with_backend(&case.snapshot, &hybrid, &Control::new())
                .unwrap_or_else(|e| panic!("{name}: Tier 2 render failed: {e}"));
            let path = PathTraceEngine::with_device(device.clone(), queue.clone());
            let (path, backend) = path
                .render_with_backend(&case.snapshot, &settings, &Control::new())
                .unwrap_or_else(|e| panic!("{name}: Tier 3 render failed: {e}"));
            assert_eq!(raster_backend, None, "{name}: raster used RT");
            assert_eq!(hybrid_backend, Some(backend), "{name}: hybrid backend");
            assert_eq!(
                backend,
                device_backend(device),
                "{name}: unexpected fallback or ray-query device"
            );
            if !exercised.contains(&backend) {
                exercised.push(backend);
            }
            let images = [beauty(&raster), beauty(&hybrid), beauty(&path)];
            for (i, (image, budget)) in images.iter().zip(BUDGETS).enumerate() {
                let tier = i + 1;
                let label = format!("{name} Tier {tier} {backend:?}");
                record_score(
                    &mut failures,
                    &format!("{label} vs CPU"),
                    cpu,
                    image,
                    budget,
                    matches!(backend, BackendKind::Compute | BackendKind::Metal)
                        && KNOWN_MISMATCH.contains(&(name.as_str(), tier)),
                );
                if let Some(baseline) = &compute {
                    let baseline: &[ImageBuf; 3] = baseline;
                    record_score(
                        &mut failures,
                        &format!("{label} vs Compute"),
                        &baseline[i],
                        image,
                        budget,
                        false,
                    );
                }
                if let Some(dir) = &captures {
                    let file = dir.join(format!("{name}-tier{tier}.rgb-f32le"));
                    match backend {
                        BackendKind::Metal if required == Some(backend) => {
                            std::fs::write(&file, encode_rgb(image))
                                .unwrap_or_else(|e| panic!("{file:?}: {e}"));
                        }
                        BackendKind::Vulkan if required == Some(backend) => {
                            let bytes =
                                std::fs::read(&file).unwrap_or_else(|e| panic!("{file:?}: {e}"));
                            let metal = decode_rgb(image.width, image.height, &bytes);
                            record_score(
                                &mut failures,
                                &format!("{label} vs Metal"),
                                &metal,
                                image,
                                budget,
                                false,
                            );
                        }
                        _ => {}
                    }
                }
            }
            if backend == BackendKind::Compute {
                assert!(
                    compute.replace(images).is_none(),
                    "expected one compute device"
                );
            }
        });
    }
    if let Some(required) = required {
        assert!(
            exercised.contains(&required),
            "missing {required:?} hardware run"
        );
    }
    assert!(
        failures.is_empty(),
        "R26 cross-tier consistency:\n{}",
        failures.join("\n")
    );
}

fn encode_rgb(image: &ImageBuf) -> Vec<u8> {
    image.rgb.iter().flat_map(|x| x.to_le_bytes()).collect()
}

fn decode_rgb(width: u32, height: u32, bytes: &[u8]) -> ImageBuf {
    assert_eq!(bytes.len(), width as usize * height as usize * 3 * 4);
    let rgb: Vec<_> = bytes
        .as_chunks::<4>()
        .0
        .iter()
        .copied()
        .map(f32::from_le_bytes)
        .collect();
    assert!(rgb.iter().all(|value| value.is_finite()));
    ImageBuf {
        width,
        height,
        rgb,
        alpha: None,
    }
}

#[test]
fn r26_capture_preserves_linear_hdr_values() {
    let image = ImageBuf::from_fn(2, 1, |x, _| [x as f32, 1.25, -0.125]);
    assert_eq!(&encode_rgb(&image)[4..8], &[0, 0, 160, 63]);
    assert_eq!(decode_rgb(2, 1, &encode_rgb(&image)), image);
}

fn beauty(output: &RenderOutput) -> ImageBuf {
    assert!(output.beauty.data.iter().all(|value| value.is_finite()));
    ImageBuf::from_rgba(output.width, output.height, &output.beauty.data)
}

fn record_score(
    failures: &mut Vec<String>,
    label: &str,
    expected: &ImageBuf,
    actual: &ImageBuf,
    budget: f64,
    known_mismatch: bool,
) {
    let report = compare_image(expected, actual, &CompareOptions::default()).unwrap();
    let result = format!(
        "{label}: {:.2} dB (budget {budget:.0} dB, gain {:.6})",
        report.psnr, report.exposure_gain
    );
    let disposition = if known_mismatch {
        " [KNOWN_MISMATCH]"
    } else {
        ""
    };
    eprintln!("{result}{disposition}");
    let missed = report.psnr < budget;
    if known_mismatch && !missed {
        failures.push(format!("stale mismatch pin: {result}"));
    } else if (!known_mismatch && missed) || report.psnr.is_nan() {
        failures.push(result);
    }
}

#[test]
fn mirrored_and_nonuniform_instances_keep_authored_normals_and_ids() {
    let mut snapshot = load("cube-plane").snapshot;
    let mut mesh = MeshData::new(
        vec![[-0.8, -0.8, 0.0], [0.8, -0.8, 0.0], [0.0, 0.8, 0.0]],
        vec![[0, 1, 2]],
    );
    mesh.normals = vec![[-0.5, 0.0, 1.0], [1.0, 0.7, 1.0], [0.1, 1.0, 1.0]];
    snapshot.meshes = vec![mesh];
    snapshot.instances = [(1.0, 1.0, 123), (-1.0, -1.0, 456)]
        .into_iter()
        .enumerate()
        .map(|(material, (x, mirror, id))| {
            Instance::new(
                0,
                DMat4::from_scale_rotation_translation(
                    DVec3::new(mirror * 0.8, 1.25, 0.6),
                    DQuat::from_rotation_y(0.35),
                    DVec3::new(x, 0.0, 0.0),
                ),
                material,
                id,
            )
        })
        .collect();
    snapshot.camera = Camera::new(
        DMat4::from_translation(DVec3::new(0.0, 0.0, -4.0)),
        Projection::Orthographic { width: 4.0 },
    );
    snapshot.lights.clear();
    let aovs = [Aov::Normal, Aov::Depth, Aov::ObjectId, Aov::MaterialId];
    let settings = image_parity::settings((64, 48), aovs.to_vec());
    for out in compare(&snapshot, &settings, &aovs) {
        let ids = &out.aovs[&Aov::ObjectId];
        // R9 ID AOVs reserve zero for misses and encode hits as ID + 1.
        assert_eq!(ids.pixel(16, 24)[0], 457.0);
        assert_eq!(ids.pixel(48, 24)[0], 124.0);
        assert_eq!(out.aovs[&Aov::MaterialId].pixel(16, 24)[0], 2.0);
        assert_eq!(out.aovs[&Aov::MaterialId].pixel(48, 24)[0], 1.0);
    }
}
