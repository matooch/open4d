use super::direct_shadow::{DirectShadowBindings, DirectShadowFrame, DirectShadowProgram};
use super::*;
use forge_render_api::{AreaShape, Camera, Instance, Light, LightKind, MaterialData, MeshData};
use forge_render_api::{Projection, ShadowMode};
use glam::{DMat4, DQuat};

#[test]
fn gpu_raw_direct_shadows_match_batched_reference() {
    let _serial = crate::gpu_test_serial();
    let rotation = DQuat::from_rotation_y(std::f64::consts::PI);
    let emitter = |kind, z| {
        let mut light = Light::new(
            kind,
            DMat4::from_rotation_translation(rotation, DVec3::new(0., 0., z)),
        );
        light.shadow = ShadowMode::Soft;
        light.shadow_density = 0.75;
        light.shadow_color = [0.2, 0.6, 0.4];
        light
    };
    let near = emitter(LightKind::Omni, 1.);
    let mut far = emitter(LightKind::Omni, 4.);
    far.specular = false;
    let area = emitter(
        LightKind::Area {
            shape: AreaShape::Rectangle {
                width: 1.,
                height: 0.75,
            },
            two_sided: false,
        },
        4.,
    );
    let mut infinite = emitter(LightKind::Infinite { angle: 0.12 }, 0.);
    infinite.diffuse = false;
    let mut no_shadow = near;
    no_shadow.shadow = ShadowMode::None;
    let mut mesh = MeshData::new(
        vec![[-8., -8., 0.], [8., -8., 0.], [0., 8., 0.]],
        vec![[0, 1, 2]],
    );
    mesh.normals = vec![[0.12, -0.07, 1.]; 3];
    let mut snapshot = Snapshot {
        camera: Camera::from_hpb(
            DVec3::new(0., 0., 5.),
            [std::f64::consts::PI, 0., 0.],
            Projection::Orthographic { width: 4. },
        ),
        meshes: vec![mesh],
        instances: vec![
            Instance::new(0, DMat4::IDENTITY, 0, 1),
            Instance::new(0, DMat4::from_translation(DVec3::Z * 2.), 0, 2),
        ],
        materials: vec![MaterialData::new(
            "receiver",
            forge_material::ir::compile(&forge_material::Material::default()),
        )],
        ..Snapshot::default()
    };
    let settings = RenderSettings {
        width: 11,
        height: 9,
        region: Some([8, 2, 11, 9]),
        seed: 0x1234_abcd_f00d_5678,
        default_light: false,
        ..Default::default()
    };
    // Absolute indices, including a receiver beyond both first workgroup axes.
    let receivers = [(30, [0.1, 0.25, 0., 1.]), (98, [-0.3, -0.2, 0., 1.])];
    let mut positions = [[0.; 4]; 99];
    let mut normals = [[0.; 4]; 99];
    for (i, position) in receivers {
        positions[i] = position;
        normals[i] = [0., 0., 1., 1.];
    }
    normals[31] = [0., 0., 1., 0.];
    let input = PrimaryInput {
        positions: &positions,
        normals: &normals,
        epsilon: 1e-5,
        jitter: [-0.375, 0.25],
    };
    // Covered, compatible guide with an invalid receiver key: raw black.
    // The batched raw reference expects valid keys for all covered pixels.
    let mut spatial_positions = positions;
    spatial_positions[31] = [0.11, 0.25, 0., 1.];
    let guide = PrimaryInput {
        positions: &spatial_positions,
        ..input
    };
    for request in [crate::request_default_device, crate::request_rt_device] {
        let (device, queue) = match request() {
            Ok(gpu) => gpu,
            Err(err) if std::env::var("FORGE_REQUIRE_GPU").as_deref() == Ok("1") => {
                panic!("FORGE_REQUIRE_GPU=1 but no GPU: {err}");
            }
            Err(err) => {
                eprintln!("skipping GPU test: {err}");
                return;
            }
        };
        let (kind, mut backend) = ray::on_device(device.clone(), queue.clone()).unwrap();
        assert_eq!(kind, ray::enabled_backend(&device));
        backend.update_scene(&snapshot).unwrap();
        let geometry = PrimaryGeometry::new(&snapshot).unwrap();
        let attrs = crate::path_trace::attributes(&device, &snapshot, &geometry.scene).unwrap();
        let views = [spatial_positions, normals]
            .map(|data| texture_view(&device, &queue, &settings, &data));
        let materials: Vec<_> = snapshot
            .materials
            .iter()
            .map(|m| (&m.graph, &*m.sampler as &dyn forge_material::TextureSampler))
            .collect();
        let assembled = crate::shader::direct_shadow_wgsl(kind, &materials).unwrap();
        assert!(assembled.resources.is_empty());
        let program = DirectShadowProgram::new(&device, kind, assembled.source, 0);
        let spatial = BounceSpatialProgram::new(&device);
        let scene = backend.inline_scene().unwrap();
        let control = Control::new();
        for (name, input_lights) in [
            ("none", vec![no_shadow]),
            ("near", vec![near]),
            ("far", vec![far]),
            ("area", vec![area]),
            ("infinite", vec![infinite]),
            ("mixed", vec![near, no_shadow, area, infinite, far]),
        ] {
            snapshot.lights = input_lights;
            let lights: Vec<_> = lights::prepare(&snapshot, false)
                .unwrap()
                .into_iter()
                .filter(lights::PLight::casts_shadow)
                .collect();
            let mut previous = None;
            for history in [0, 3] {
                let expected = raw_direct_shadows(
                    (&geometry, &*backend),
                    &snapshot,
                    &settings,
                    input,
                    &lights,
                    history,
                    &control,
                )
                .unwrap();
                let actual = program
                    .read(
                        (&device, &queue),
                        DirectShadowBindings {
                            scene,
                            primary: [&views[0], &views[1]],
                            attributes: &attrs,
                            bitmaps: &[],
                            lights: &lights,
                        },
                        &settings,
                        DirectShadowFrame::new(
                            &snapshot,
                            &settings,
                            (input.jitter, input.epsilon),
                            geometry.scene.scale,
                            history,
                            lights.len(),
                        ),
                    )
                    .unwrap();
                assert_eq!(actual.len(), expected.len());
                for (i, (actual, expected)) in actual.iter().zip(expected).enumerate() {
                    if !receivers.iter().any(|(j, _)| *j == i) || name == "none" {
                        assert_eq!(*actual, [0.; 8], "{kind:?} {name} pixel {i}");
                        continue;
                    }
                    for (j, (&a, b)) in actual.iter().zip(expected).enumerate() {
                        assert!(
                            (a - b).abs() <= 2e-6 + 2e-4 * b.abs(),
                            "{kind:?} {name} history {history} pixel {i} channel {j}: {a} vs {b}"
                        );
                    }
                    assert!(actual[3] > 1e-4, "{name} needs a nonzero signal");
                    match name {
                        "near" => assert_eq!(actual[3], actual[7], "blocker is beyond the light"),
                        "far" => {
                            assert!(actual[7] < actual[3] && actual[7] > 0.);
                            assert!(actual[1] > actual[0] && actual[2] > actual[0]);
                            assert_eq!(&actual[4..7], &[0.; 3]);
                        }
                        "infinite" => assert_eq!(&actual[..3], &[0.; 3]),
                        _ => {}
                    }
                }
                if name == "mixed" {
                    let width = settings.width as usize;
                    let limit = geometry.filter_limit(&snapshot, width);
                    let once = spatial_filter(width, &guide, &actual, limit);
                    let twice = spatial_filter(width, &guide, &once, limit);
                    let filtered = program
                        .read_spatial(
                            (&device, &queue),
                            DirectShadowBindings {
                                scene,
                                primary: [&views[0], &views[1]],
                                attributes: &attrs,
                                bitmaps: &[],
                                lights: &lights,
                            },
                            &settings,
                            DirectShadowFrame::new(
                                &snapshot,
                                &settings,
                                (input.jitter, input.epsilon),
                                geometry.scene.scale,
                                history,
                                lights.len(),
                            ),
                            spatial.shadow_pass(limit),
                        )
                        .unwrap();
                    for alpha in [3, 7] {
                        assert!(once[31][alpha] > 1e-4);
                        assert!((once[31][alpha] - twice[31][alpha]).abs() > 2e-6);
                    }
                    assert_ne!(once[31][3], once[31][7]);
                    assert_eq!(filtered[32], [0.; 8]);
                    for (i, (out, expected)) in filtered.iter().zip(once).enumerate() {
                        for (j, (&a, b)) in out.iter().zip(expected).enumerate() {
                            assert!(
                                (a - b).abs() <= 2e-6 * b.abs().max(1.),
                                "{kind:?} history {history} pixel {i} channel {j}: {a} vs {b}"
                            );
                        }
                    }
                }
                if let Some(previous) = previous
                    && matches!(name, "area" | "mixed")
                {
                    assert_ne!(actual[98], previous, "{name} must exercise history RNG");
                }
                previous = Some(actual[98]);
            }
        }
        let views =
            [&positions, &normals].map(|data| texture_view(&device, &queue, &settings, data));
        let source = forge_render_gpu::shader::materials(&materials, 1).unwrap();
        let gpu = MaterialGpu {
            primary: AmbientGpu {
                device: &device,
                queue: &queue,
                textures: [&views[0], &views[1]],
            },
            materials: &source.source,
            bitmaps: &[],
        };
        let mut live = HybridView::default();
        live.prepare(&snapshot, &settings, &device, &queue).unwrap();
        let prepared: Vec<_> = lights::prepare(&snapshot, false)
            .unwrap()
            .into_iter()
            .filter(lights::PLight::casts_shadow)
            .collect();
        let mut reference: [History<[f32; 8]>; 2] = Default::default();
        let cancelled = Control::new();
        cancelled.cancel();
        for (i, use_gpu, resident, stop) in [
            (0, true, true, false),
            (1, true, true, false),
            (0, true, true, true),
            (0, true, true, false),
            (0, false, false, false),
            (1, true, true, false),
            (0, true, false, false),
        ] {
            let input = PrimaryInput {
                jitter: if i == 0 { input.jitter } else { [0.25, -0.375] },
                ..input
            };
            let actual = live.direct_shadows(
                &snapshot,
                &settings,
                input,
                if stop { &cancelled } else { &control },
                use_gpu.then_some(gpu),
            );
            if stop {
                assert!(matches!(actual, Err(RenderError::Cancelled)));
                continue;
            }
            let history = &mut reference[i];
            let raw = raw_direct_shadows(
                (&geometry, &*backend),
                &snapshot,
                &settings,
                input,
                &prepared,
                history.frames,
                &control,
            )
            .unwrap();
            let width = settings.width as usize;
            history.accumulate(spatial_filter(
                width,
                &input,
                &raw,
                geometry.filter_limit(&snapshot, width),
            ));
            let actual = match actual.unwrap() {
                ShadowSignal::Gpu(mean) => {
                    assert!(resident);
                    assert_eq!(mean.frames(), history.frames);
                    mean.read_shadow_mean(&device, &queue).unwrap()
                }
                ShadowSignal::Cpu(mean) => {
                    assert!(!resident, "{kind:?} unexpectedly staged a resident shadow");
                    mean
                }
            };
            assert_eq!(actual.len(), history.mean.len());
            assert_eq!(actual[32], [0.; 8]);
            assert!(actual[98][3] > actual[98][7] && actual[98][7] > 0.);
            for (actual, expected) in actual.iter().zip(&history.mean) {
                for (&a, &b) in actual.iter().zip(expected) {
                    assert!(
                        (a - b).abs() <= 2e-6 + 2e-4 * b.abs(),
                        "{kind:?} jitter {i} frame {}: {a} vs {b}",
                        history.frames
                    );
                }
            }
        }
        live.reset();
        assert!(matches!(
            live.direct_shadows(&snapshot, &settings, input, &control, Some(gpu))
                .unwrap(),
            ShadowSignal::Gpu(mean) if mean.frames() == 1
        ));
        snapshot.lights = vec![no_shadow];
        live.prepare(&snapshot, &settings, &device, &queue).unwrap();
        assert!(
            live.direct_shadows(&snapshot, &settings, input, &control, Some(gpu))
                .unwrap()
                .is_empty()
        );
        assert!(live.shadows.is_empty());
    }
}

pub(super) fn texture_view(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    settings: &RenderSettings,
    data: &[[f32; 4]],
) -> wgpu::TextureView {
    let size = wgpu::Extent3d {
        width: settings.width,
        height: settings.height,
        depth_or_array_layers: 1,
    };
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("raw signal probe primary"),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba32Float,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    queue.write_texture(
        texture.as_image_copy(),
        bytemuck::cast_slice(data),
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(settings.width * 16),
            rows_per_image: Some(settings.height),
        },
        size,
    );
    texture.create_view(&Default::default())
}
