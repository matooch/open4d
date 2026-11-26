//! R25.3: real dispatches through the Tier 3 scene bindings and shared integrator.

#[path = "../../render-cpu/tests/common/mod.rs"]
mod common;

use common::{golden::load, image_parity};
use forge_parity_harness::{CompareOptions, ImageBuf, compare_image};
use forge_render_api::*;
use forge_render_cpu::CpuEngine;
use forge_render_rt::{
    PathTraceEngine, ray::BackendKind, request_default_device, request_rt_device,
};
use glam::{DMat4, DQuat, DVec3};
use std::sync::Mutex;

type Device = (wgpu::Device, wgpu::Queue);
static DEVICES: Mutex<Option<Vec<Device>>> = Mutex::new(None);

fn compare(snapshot: &Snapshot, settings: &RenderSettings, aovs: &[Aov]) -> Vec<RenderOutput> {
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
    if devices.is_empty() {
        return Vec::new();
    }
    let cpu = CpuEngine::new()
        .render(snapshot, settings, &Control::new())
        .unwrap();
    let compare_options = CompareOptions::default();
    devices
        .iter()
        .map(|(device, queue)| {
            let engine = PathTraceEngine::with_device(device.clone(), queue.clone());
            let expected = if device
                .features()
                .contains(wgpu::Features::EXPERIMENTAL_RAY_QUERY)
            {
                BackendKind::Metal
            } else {
                BackendKind::Compute
            };
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
        .collect()
}

#[test]
fn area_light_paths_and_shadow_aov_match_cpu() {
    let case = load("area-soft");
    let aovs = [Aov::Diffuse, Aov::Shadow];
    let settings = image_parity::settings((48, 36), aovs.to_vec());
    compare(&case.snapshot, &settings, &aovs);
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
