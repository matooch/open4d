use super::direct_shadow_tests::texture_view;
use super::reflection_probe::{ReflectionProbe, ReflectionProbeBindings, ReflectionProbeFrame};
use super::*;
use forge_core::Value;
use forge_image::{AlphaMode, Image};
use forge_render_api::{
    AreaShape, Background, Camera, Instance, Light, LightKind, Lighting, MaterialData, MeshData,
    Projection, ShadowMode,
};
use glam::DMat4;

#[test]
fn gpu_raw_reflections_match_batched_reference() {
    let triangle = |radius: f64, z| {
        MeshData::new(
            vec![[-radius, -radius, z], [radius, -radius, z], [0., radius, z]],
            vec![[0, 1, 2]],
        )
    };
    let receiver = forge_material::Material::default();
    let mut emitter = receiver.clone();
    emitter.set("enable.luminance", Value::Bool(true));
    emitter.set("luminance.color", Value::Color([1.5, 0.6, 0.25]));
    let mut snapshot = Snapshot {
        camera: Camera::from_hpb(
            DVec3::Z * 5.,
            [std::f64::consts::PI, 0., 0.],
            Projection::Orthographic { width: 12. },
        ),
        meshes: vec![triangle(16., 0.), triangle(2., 0.125)],
        instances: vec![
            Instance::new(0, DMat4::IDENTITY, 0, 1),
            Instance::new(1, DMat4::from_translation(-DVec3::X * 4.), 1, 2),
            Instance::new(1, DMat4::from_translation(DVec3::X * 4.), 0, 3),
        ],
        materials: [receiver, emitter]
            .iter()
            .map(|m| MaterialData::new("probe", forge_material::ir::compile(m)))
            .collect(),
        ..Snapshot::default()
    };
    snapshot.environment.background = Background::Color([40., 50., 60.]);
    let settings = RenderSettings {
        width: 13,
        height: 11,
        region: Some([2, 1, 13, 11]),
        seed: 0x1234_abcd_f00d_5678,
        default_light: false,
        ..Default::default()
    };
    let [emissive, dark, miss, invalid_zero, invalid_end] = [28, 63, 125, 109, 138];
    let mut positions = [[0.; 4]; 143];
    let mut normals = [[0.; 4]; 143];
    for (i, x, key) in [
        (emissive, -4., 1.),
        (dark, 4., 1.),
        (miss, 0., 1.),
        (invalid_zero, 0., 0.),
        (invalid_end, 0., 4.),
    ] {
        positions[i] = [x, 0., 0., 1.];
        normals[i] = [0., 0., 1., key];
    }
    let input = PrimaryInput {
        positions: &positions,
        normals: &normals,
        epsilon: 1e-5,
        jitter: [-0.375, 0.25],
    };
    let mut valid_positions = positions;
    for i in [invalid_zero, invalid_end] {
        valid_positions[i] = [0.; 4];
    }
    let emitter = |kind, z| {
        let mut light = Light::new(kind, DMat4::from_translation(DVec3::new(4., 0., z)));
        light.shadow = ShadowMode::Soft;
        light.radius = 0.;
        light.shadow_density = 0.75;
        light.shadow_color = [0.2, 0.6, 0.4];
        light
    };
    let mut near = emitter(LightKind::Omni, 0.0625);
    near.specular = false;
    let far = emitter(LightKind::Omni, -1.);
    let mut infinite = emitter(LightKind::Infinite { angle: 0.12 }, 0.);
    infinite.diffuse = false;
    let area = emitter(
        LightKind::Area {
            shape: AreaShape::Rectangle {
                width: 1.,
                height: 0.75,
            },
            two_sided: false,
        },
        0.0625,
    );
    let mut near_open = near;
    near_open.shadow = ShadowMode::None;
    let mut far_open = far;
    far_open.shadow = ShadowMode::None;
    let mut disabled = near;
    disabled.diffuse = false;
    let scenarios = [
        ("black", Lighting::None, vec![]),
        (
            "constant",
            Lighting::Constant {
                color: [0.4, 0.8, 0.15],
                intensity: 1.4,
            },
            vec![],
        ),
        (
            "sky",
            Lighting::Sky {
                zenith: [0.1, 0.6, 1.4],
                horizon: [0.65, 0.25, 0.4],
                ground: [0.6, 0.17, 0.05],
                intensity: 0.8,
            },
            vec![],
        ),
        (
            "image",
            Lighting::Image {
                image: Arc::new(Image::from_fn(7, 5, AlphaMode::Straight, |x, y| {
                    [0.2 + x as f32 * 0.18, 0.1 + y as f32 * 0.35, 0.14, 0.01]
                })),
                rotation: 0.61,
                intensity: 1.7,
            },
            vec![],
        ),
        ("near-open", Lighting::None, vec![near_open]),
        ("near", Lighting::None, vec![near]),
        ("far-open", Lighting::None, vec![far_open]),
        ("far", Lighting::None, vec![far]),
        ("area", Lighting::None, vec![area]),
        ("infinite", Lighting::None, vec![infinite]),
        ("disabled", Lighting::None, vec![disabled]),
        (
            "mixed",
            Lighting::None,
            vec![near, area, infinite, far_open, disabled],
        ),
    ];
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
        eprintln!("raw reflection probe: {kind:?}");
        backend.update_scene(&snapshot).unwrap();
        let geometry = PrimaryGeometry::new(&snapshot).unwrap();
        let scale = geometry.scene.scale;
        let attrs = crate::path_trace::attributes(&device, &snapshot, &geometry.scene).unwrap();
        let views =
            [positions, normals].map(|data| texture_view(&device, &queue, &settings, &data));
        let materials: Vec<_> = snapshot
            .materials
            .iter()
            .map(|m| (&m.graph, &*m.sampler as &dyn forge_material::TextureSampler))
            .collect();
        let assembled = crate::shader::reflection_wgsl(kind, &materials).unwrap();
        assert!(assembled.resources.is_empty());
        let probe = ReflectionProbe::new(&device, kind, assembled.source, 0);
        let mut reference = HybridView {
            geometry: Some(geometry),
            backend: Some((kind, backend)),
            ..HybridView::default()
        };
        let mut near_controls = [[0.; 4]; 2];
        let mut far_controls = [[0.; 4]; 2];
        for (name, lighting, lights) in &scenarios {
            snapshot.environment.lighting = lighting.clone();
            snapshot.lights.clone_from(lights);
            let mut first_emissive = None;
            for (phase, history) in [0, 3].into_iter().enumerate() {
                reference.reflections.clear();
                let expected = reference
                    .indirect(
                        Bounce::Reflection,
                        &snapshot,
                        &RenderSettings {
                            seed: settings.seed ^ u64::from(history),
                            ..settings.clone()
                        },
                        PrimaryInput {
                            positions: &valid_positions,
                            ..input
                        },
                        &Control::new(),
                    )
                    .unwrap();
                if let Some(previous) = first_emissive {
                    assert_ne!(expected[emissive], previous, "history must vary the signal");
                }
                first_emissive = Some(expected[emissive]);
                match *name {
                    "near-open" => near_controls[phase] = expected[dark],
                    "near" => assert_eq!(
                        expected[dark], near_controls[phase],
                        "blocker beyond emitter"
                    ),
                    "far-open" => far_controls[phase] = expected[dark],
                    "far" => {
                        for (channel, (value, open)) in expected[dark][..3]
                            .iter()
                            .zip(&far_controls[phase][..3])
                            .enumerate()
                        {
                            let factor = 0.25 + 0.75 * far.shadow_color[channel] as f32;
                            assert!((value - open * factor).abs() <= 1e-7);
                        }
                    }
                    _ => {}
                }
                let scene = reference
                    .backend
                    .as_ref()
                    .unwrap()
                    .1
                    .inline_scene()
                    .unwrap();
                let actual = probe
                    .read(
                        (&device, &queue),
                        ReflectionProbeBindings {
                            scene,
                            primary: [&views[0], &views[1]],
                            attributes: &attrs,
                            bitmaps: &[],
                        },
                        ReflectionProbeFrame {
                            snapshot: &snapshot,
                            settings: &settings,
                            input,
                            scale,
                            history,
                        },
                    )
                    .unwrap();
                assert_eq!(actual.len(), expected.len());
                for (i, (actual, expected)) in actual.iter().zip(expected).enumerate() {
                    if i != emissive
                        && (i != miss || matches!(lighting, Lighting::None))
                        && (i != dark || lights.is_empty() || *name == "disabled")
                    {
                        assert_eq!(expected, [0.; 4], "{kind:?} mode {name} pixel {i}");
                        assert_eq!(*actual, [0.; 4], "{kind:?} mode {name} pixel {i}");
                    } else {
                        assert!(
                            expected[0] > 2e-6,
                            "{kind:?} mode {name} pixel {i} has no measurable signal: {expected:?}"
                        );
                        for (j, (&a, b)) in actual.iter().zip(expected).enumerate() {
                            assert!(
                                (a - b).abs() <= 2e-6 + 2e-4 * b.abs(),
                                "{kind:?} mode {name} history {history} pixel {i} channel {j}: {a} vs {b}"
                            );
                        }
                    }
                }
            }
        }
    }
}
