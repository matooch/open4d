use super::direct_shadow::{DirectShadowBindings, DirectShadowFrame, DirectShadowProgram};
use super::*;
use forge_render_api::{AreaShape, Camera, Instance, Light, LightKind, MaterialData, MeshData};
use forge_render_api::{Projection, ShadowMode};
use glam::{DMat4, DQuat};

#[test]
fn gpu_raw_direct_shadows_match_batched_reference() {
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
    let input = PrimaryInput {
        positions: &positions,
        normals: &normals,
        epsilon: 1e-5,
        jitter: [-0.375, 0.25],
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
        let views =
            [positions, normals].map(|data| texture_view(&device, &queue, &settings, &data));
        let materials: Vec<_> = snapshot
            .materials
            .iter()
            .map(|m| (&m.graph, &*m.sampler as &dyn forge_material::TextureSampler))
            .collect();
        let assembled = crate::shader::direct_shadow_wgsl(kind, &materials).unwrap();
        assert!(assembled.resources.is_empty());
        let program = DirectShadowProgram::new(&device, kind, assembled.source, 0);
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
                            input,
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
                if let Some(previous) = previous
                    && matches!(name, "area" | "mixed")
                {
                    assert_ne!(actual[98], previous, "{name} must exercise history RNG");
                }
                previous = Some(actual[98]);
            }
        }
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
