//! R24 acceptance fixtures. Use O43 material inputs, not the reference PNGs:
//! each isolated lighting signal has a higher-sample Forge CPU reference.

#[path = "../../render-cpu/tests/common/mod.rs"]
mod common;

use common::{centre_mean, cube, diffuse, golden, material, plane};
use forge_core::Value;
use forge_render_api::*;
use forge_render_cpu::CpuEngine;
use forge_render_rt::{RtEngine, request_default_device, request_rt_device};
use glam::{DMat4, DVec3};
use std::sync::Mutex;

const SIZE: u32 = 40;
const AO_RADIUS: f64 = 40.0;
// Fixed peak in linear radiance; no per-image exposure or normalization.
const PSNR_PEAK: f64 = 1.0;
static GPU: Mutex<Option<Result<(wgpu::Device, wgpu::Queue), String>>> = Mutex::new(None);

#[derive(Clone, Copy, Debug)]
enum Effect {
    Shadows,
    Reflections,
    Ao,
    Gi,
}

impl Effect {
    const ALL: [Self; 4] = [Self::Shadows, Self::Reflections, Self::Ao, Self::Gi];

    fn occludes(self) -> bool {
        matches!(self, Self::Shadows | Self::Ao)
    }

    fn raster_settings(self) -> RenderSettings {
        RenderSettings {
            default_light: false,
            post: PostSettings {
                ssao: matches!(self, Self::Ao),
                ssao_radius: AO_RADIUS,
                ssr: matches!(self, Self::Reflections),
                ..PostSettings::default()
            },
            ..RenderSettings::new(SIZE, SIZE, 1)
        }
    }
}

fn o43_material(case: &str) -> MaterialData {
    let input = std::fs::read_to_string(golden::root().join(case).join("case.toml")).unwrap();
    let manifest: toml::Table = input.parse().unwrap();
    let params = manifest["params"].as_table().unwrap();
    golden::material_of(Some(params["material"].as_table().unwrap()), case)
}

fn down_from(height: f64) -> DMat4 {
    DMat4::from_cols(
        DVec3::X.extend(0.0),
        DVec3::Z.extend(0.0),
        (-DVec3::Y).extend(0.0),
        (DVec3::Y * height).extend(1.0),
    )
}

fn fixture(effect: Effect) -> Snapshot {
    let (receiver, ceiling_half_width, ceiling_height) = match effect {
        Effect::Shadows => ("mat-red-matte", 11.0, 24.0),
        Effect::Reflections => ("mat-glossy", 14.0, 32.0),
        Effect::Ao => ("mat-red-matte", 14.0, AO_RADIUS / 4.0),
        Effect::Gi => ("mat-red-matte", 16.0, 22.0),
    };
    // The camera is below the ceiling looking straight at a floor that covers
    // the frame. The ceiling is invisible to the primary and screen-space rays,
    // but it intersects secondary rays and is present in world-space shadow maps.
    let ceiling = DMat4::from_translation(DVec3::Y * ceiling_height)
        * DMat4::from_rotation_x(std::f64::consts::PI);
    let mut scene = Snapshot {
        camera: Camera::new(down_from(5.0), Projection::Orthographic { width: 84.0 }),
        materials: vec![
            o43_material(receiver),
            if effect.occludes() {
                diffuse([0.0; 3])
            } else {
                o43_material("mat-luminance")
            },
        ],
        meshes: vec![plane(80.0), plane(ceiling_half_width)],
        instances: vec![
            Instance::new(0, DMat4::IDENTITY, 0, 1),
            Instance::new(1, ceiling, 1, 2),
        ],
        ..Snapshot::default()
    };
    match effect {
        Effect::Shadows => {
            let mut light = Light::new(
                LightKind::Area {
                    shape: AreaShape::Rectangle {
                        width: 56.0,
                        height: 56.0,
                    },
                    two_sided: false,
                },
                down_from(72.0),
            );
            light.shadow = ShadowMode::Soft;
            light.specular = false;
            scene.lights.push(light);
        }
        Effect::Ao => {
            scene.environment.lighting = Lighting::Constant {
                color: [1.0; 3],
                intensity: 1.0,
            };
        }
        // No direct light or environment. The out-of-view emitter reaches the
        // floor through either the glossy or the diffuse lobe, respectively.
        Effect::Reflections | Effect::Gi => {}
    }
    scene
}

fn reference(scene: &Snapshot) -> RenderOutput {
    let settings = RenderSettings {
        default_light: false,
        max_depth: 2,
        threads: Some(2),
        ..RenderSettings::new(SIZE, SIZE, 256)
    };
    CpuEngine.render(scene, &settings, &Control::new()).unwrap()
}

fn psnr(actual: &RenderOutput, expected: &RenderOutput) -> f64 {
    let a = &actual.beauty;
    let b = &expected.beauty;
    assert_eq!((a.width, a.height), (b.width, b.height));
    let rgb_error = a
        .data
        .as_chunks::<4>()
        .0
        .iter()
        .zip(b.data.as_chunks::<4>().0)
        .map(|(p, q)| {
            assert_eq!(p[3], q[3], "primary visibility differs");
            (0..3)
                .map(|c| (f64::from(p[c]) - f64::from(q[c])).powi(2))
                .sum::<f64>()
        })
        .sum::<f64>();
    let mse = rgb_error / (3.0 * f64::from(a.width) * f64::from(a.height));
    assert!(mse.is_finite(), "non-finite linear beauty");
    10.0 * (PSNR_PEAK.powi(2) / mse).log10()
}

fn with_gpu(test: impl FnOnce(&wgpu::Device, &wgpu::Queue)) {
    // Keep the per-file device locked for the full test, including all submits
    // and readbacks (software Vulkan in CI can take longer than Metal).
    let mut gpu = GPU.lock().unwrap_or_else(|e| e.into_inner());
    match gpu.get_or_insert_with(|| {
        if std::env::var_os("FORGE_TEST_RT_COMPUTE").is_some() {
            request_default_device()
        } else {
            request_rt_device()
        }
    }) {
        Ok((device, queue)) => test(device, queue),
        Err(e) if std::env::var("FORGE_REQUIRE_GPU").as_deref() == Ok("1") => {
            panic!("FORGE_REQUIRE_GPU=1 but no GPU: {e}")
        }
        Err(e) => eprintln!("skipping GPU test: {e}"),
    }
}

#[test]
fn synchronized_profiling_preserves_render_and_names_executed_passes() {
    with_gpu(|device, queue| {
        let scene = fixture(Effect::Shadows);
        for hybrid in [false, true] {
            let settings = RenderSettings {
                aovs: vec![Aov::Diffuse, Aov::Albedo, Aov::ObjectId],
                hybrid: HybridSettings {
                    enabled: hybrid,
                    ao: true,
                    shadows: true,
                    reflections: true,
                    gi: true,
                    ..HybridSettings::default()
                },
                ..Effect::Shadows.raster_settings()
            };
            let engine = || RtEngine::with_device(device.clone(), queue.clone());
            let (normal, normal_backend) = engine()
                .render_with_backend(&scene, &settings, &Control::new())
                .unwrap();
            let (profiled, backend, passes) = engine()
                .render_profiled(&scene, &settings, &Control::new())
                .unwrap();
            assert_eq!(backend, normal_backend);
            assert_eq!(backend.is_some(), hybrid);
            assert_eq!(normal.beauty.data, profiled.beauty.data);
            assert_eq!(normal.aovs.len(), profiled.aovs.len());
            for (aov, image) in normal.aovs {
                assert_eq!(image.data, profiled.aovs[&aov].data, "{aov:?}");
            }
            let mut expected = vec!["setup", "shadow-atlas", "g-buffer"];
            if hybrid {
                expected.extend([
                    "hybrid-primary",
                    "rt-ao",
                    "rt-shadows",
                    "rt-reflections",
                    "rt-gi",
                    "hybrid-upload",
                ]);
            }
            expected.extend(["lighting", "split", "albedo", "output"]);
            assert_eq!(
                passes.iter().map(|pass| pass.name).collect::<Vec<_>>(),
                expected
            );
        }
    });
}

#[test]
fn optional_aovs_preserve_cropped_beauty_and_events() {
    const ALL: [Aov; 10] = [
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
    with_gpu(|device, queue| {
        let scene = fixture(Effect::Shadows);
        let crop = [3, 7, SIZE - 2, SIZE - 5];
        let engine = || RtEngine::with_device(device.clone(), queue.clone());
        for hybrid in [false, true] {
            let settings = RenderSettings {
                region: Some(crop),
                aovs: ALL.to_vec(),
                post: PostSettings {
                    bloom: true,
                    tonemap: true,
                    ..PostSettings::default()
                },
                hybrid: HybridSettings {
                    enabled: hybrid,
                    shadows: true,
                    ..HybridSettings::default()
                },
                ..Effect::Shadows.raster_settings()
            };
            let full = engine().render(&scene, &settings, &Control::new()).unwrap();
            assert!(full.aovs[&Aov::ObjectId].data.iter().any(|&v| v > 0.0));
            for aov in std::iter::once(None).chain(ALL.map(Some)) {
                let events = std::sync::Arc::new(Mutex::new(Vec::new()));
                let sink = events.clone();
                let control = Control::with_events(move |e| sink.lock().unwrap().push(e.clone()));
                let actual = engine()
                    .render(
                        &scene,
                        &RenderSettings {
                            aovs: aov.into_iter().collect(),
                            ..settings.clone()
                        },
                        &control,
                    )
                    .unwrap();
                assert_eq!(actual.beauty.data, full.beauty.data);
                assert_eq!(actual.beauty.alpha, full.beauty.alpha);
                assert_eq!(actual.stats.samples, full.stats.samples);
                assert_eq!(actual.aovs.len(), usize::from(aov.is_some()));
                if let Some(aov) = aov {
                    assert_eq!(actual.aovs[&aov].data, full.aovs[&aov].data, "{aov:?}");
                    assert_eq!(actual.aovs[&aov].alpha, full.aovs[&aov].alpha);
                }
                let [x, y, x1, y1] = crop;
                let rgba = (y..y1)
                    .flat_map(|row| {
                        full.beauty.data
                            [((row * SIZE + x) * 4) as usize..((row * SIZE + x1) * 4) as usize]
                            .iter()
                            .copied()
                    })
                    .collect();
                assert_eq!(
                    *events.lock().unwrap(),
                    [
                        RenderEvent::Started {
                            width: SIZE,
                            height: SIZE,
                            tiles: 1
                        },
                        RenderEvent::Tile {
                            x,
                            y,
                            width: x1 - x,
                            height: y1 - y,
                            rgba
                        },
                        RenderEvent::Progress { done: 1, total: 1 },
                        RenderEvent::Finished,
                    ]
                );
            }
        }
    });
}

#[test]
fn tier_one_has_a_measurable_error_against_each_cpu_reference() {
    with_gpu(|device, queue| {
        let engine = RtEngine::with_device(device.clone(), queue.clone());
        for effect in Effect::ALL {
            let scene = fixture(effect);
            let target = reference(&scene);
            let raster = engine
                .render(&scene, &effect.raster_settings(), &Control::new())
                .unwrap();
            let baseline = psnr(&raster, &target);
            // All four references must leave room for a strict improvement.
            assert!(baseline.is_finite(), "{effect:?}: Tier 1 PSNR {baseline}");
            eprintln!("{effect:?}: Tier 1 PSNR {baseline:.3} dB");
        }
    });
}

#[test]
fn ray_traced_area_shadows_replace_raster_shadows_and_improve_psnr() {
    with_gpu(|device, queue| {
        let engine = RtEngine::with_device(device.clone(), queue.clone());
        let mut scene = fixture(Effect::Shadows);
        let target = reference(&scene);
        let mut settings = Effect::Shadows.raster_settings();
        settings.aovs = vec![Aov::Diffuse, Aov::Specular, Aov::Shadow];
        let render = |scene: &Snapshot, settings: &RenderSettings| {
            engine.render(scene, settings, &Control::new()).unwrap()
        };

        let raster = render(&scene, &settings);
        settings.hybrid.enabled = true;
        assert_eq!(render(&scene, &settings).beauty.data, raster.beauty.data);
        settings.hybrid.enabled = false;
        settings.hybrid.shadows = true;
        assert_eq!(render(&scene, &settings).beauty.data, raster.beauty.data);
        settings.hybrid.enabled = true;
        let traced = render(&scene, &settings);
        let baseline = psnr(&raster, &target);
        let improved = psnr(&traced, &target);
        assert!(
            improved > baseline,
            "shadows: {improved:.3} vs {baseline:.3} dB"
        );
        eprintln!("shadows: {baseline:.3} → {improved:.3} dB");

        for (i, rgb) in traced.beauty.data.as_chunks::<4>().0.iter().enumerate() {
            let shadow = traced.aovs[&Aov::Shadow].data[4 * i];
            assert!((0.0..=1.0).contains(&shadow));
            for (c, &value) in rgb[..3].iter().enumerate() {
                assert_eq!(traced.aovs[&Aov::Diffuse].data[4 * i + c], value);
                assert_eq!(traced.aovs[&Aov::Specular].data[4 * i + c], 0.0);
            }
        }
        assert_ne!(
            traced.aovs[&Aov::Shadow].data,
            raster.aovs[&Aov::Shadow].data
        );

        scene.lights[0].shadow = ShadowMode::None;
        let hybrid_unshadowed = render(&scene, &settings);
        settings.hybrid.enabled = false;
        assert_eq!(
            hybrid_unshadowed.beauty.data,
            render(&scene, &settings).beauty.data,
            "a non-shadow-casting light keeps its Tier 1 contribution"
        );
    });
}

#[test]
fn ray_traced_ao_improves_psnr_and_respects_the_tier_one_fallback() {
    with_gpu(|device, queue| {
        let engine = RtEngine::with_device(device.clone(), queue.clone());
        let scene = fixture(Effect::Ao);
        let target = reference(&scene);
        let mut settings = Effect::Ao.raster_settings();
        let render =
            |settings: &RenderSettings| engine.render(&scene, settings, &Control::new()).unwrap();
        let raster = render(&settings);
        settings.hybrid.enabled = true;
        assert_eq!(render(&settings).beauty.data, raster.beauty.data);
        settings.hybrid.enabled = false;
        settings.hybrid.ao = true;
        assert_eq!(render(&settings).beauty.data, raster.beauty.data);

        settings.hybrid.enabled = true;
        settings.hybrid.ao_radius = AO_RADIUS;
        let hybrid = render(&settings);
        let baseline = psnr(&raster, &target);
        let improved = psnr(&hybrid, &target);
        assert!(improved > baseline, "AO: {improved:.3} vs {baseline:.3} dB");
        eprintln!("AO: {baseline:.3} → {improved:.3} dB");

        // RT AO replaces, rather than multiplying, the SSAO approximation.
        settings.post.ssao = false;
        let fresh = RtEngine::with_device(device.clone(), queue.clone());
        let no_ssao = fresh.render(&scene, &settings, &Control::new()).unwrap();
        assert_eq!(no_ssao.beauty.data, hybrid.beauty.data);

        settings.hybrid.ao_radius = 0.0;
        let zero_radius = render(&settings);
        settings.hybrid.enabled = false;
        assert_eq!(zero_radius.beauty.data, render(&settings).beauty.data);
    });
}

#[test]
fn ray_traced_reflections_improve_off_screen_psnr() {
    with_gpu(|device, queue| {
        let engine = RtEngine::with_device(device.clone(), queue.clone());
        let scene = fixture(Effect::Reflections);
        let target = reference(&scene);
        let mut settings = Effect::Reflections.raster_settings();
        settings.aovs = vec![Aov::Diffuse, Aov::Specular];
        let render =
            |settings: &RenderSettings| engine.render(&scene, settings, &Control::new()).unwrap();
        let raster = render(&settings);
        settings.hybrid.reflections = true;
        assert_eq!(render(&settings).beauty.data, raster.beauty.data);
        settings.hybrid.enabled = true;
        let traced = render(&settings);
        let baseline = psnr(&raster, &target);
        let improved = psnr(&traced, &target);
        assert!(
            improved > baseline,
            "reflections: {improved:.3} vs {baseline:.3} dB"
        );
        eprintln!("reflections: {baseline:.3} → {improved:.3} dB");
        assert_eq!(
            traced.aovs[&Aov::Diffuse].data,
            raster.aovs[&Aov::Diffuse].data
        );
        for (beauty, specular) in traced
            .beauty
            .data
            .as_chunks::<4>()
            .0
            .iter()
            .zip(traced.aovs[&Aov::Specular].data.as_chunks::<4>().0)
        {
            assert_eq!(beauty[..3], specular[..3]);
        }
    });
}

#[test]
fn ray_traced_reflections_replace_effective_ssr() {
    with_gpu(|device, queue| {
        // An in-frame emitter, unlike the PSNR fixture, gives SSR a visible hit.
        let eye = DVec3::new(0.0, 240.0, -420.0);
        let forward = (DVec3::Y * 40.0 - eye).normalize();
        let right = DVec3::Y.cross(forward).normalize();
        let scene = Snapshot {
            camera: Camera::new(
                DMat4::from_cols(
                    right.extend(0.0),
                    forward.cross(right).extend(0.0),
                    forward.extend(0.0),
                    eye.extend(1.0),
                ),
                Projection::Perspective {
                    focal_mm: 36.0,
                    sensor_mm: 36.0,
                    film_offset: [0.0; 2],
                },
            ),
            meshes: vec![plane(400.0), cube(40.0)],
            materials: vec![
                material(&[
                    ("color.color", Value::Color([0.15; 3])),
                    ("enable.reflectance", Value::Bool(true)),
                    ("reflectance.spec.width", Value::Float(0.2)),
                ]),
                o43_material("mat-luminance"),
            ],
            instances: vec![
                Instance::new(0, DMat4::IDENTITY, 0, 1),
                Instance::new(1, DMat4::from_translation(DVec3::Y * 120.0), 1, 2),
            ],
            environment: Environment {
                lighting: Lighting::Constant {
                    color: [0.02; 3],
                    intensity: 1.0,
                },
                ..Environment::default()
            },
            ..Snapshot::default()
        };
        let render = |settings: &RenderSettings| {
            RtEngine::with_device(device.clone(), queue.clone())
                .render(&scene, settings, &Control::new())
                .unwrap()
        };
        let mut settings = RenderSettings {
            default_light: false,
            ..RenderSettings::new(96, 72, 1)
        };
        let raster_no_ssr = render(&settings);
        settings.post.ssr = true;
        settings.post.ssr_steps = 32;
        assert_ne!(render(&settings).beauty.data, raster_no_ssr.beauty.data);
        settings.hybrid.enabled = true;
        settings.hybrid.reflections = true;
        let traced = render(&settings);
        settings.post.ssr = false;
        assert_eq!(traced.beauty.data, render(&settings).beauty.data);
    });
}

#[test]
fn ray_traced_gi_improves_psnr_and_does_not_double_the_diffuse_environment() {
    with_gpu(|device, queue| {
        let engine = RtEngine::with_device(device.clone(), queue.clone());
        let mut scene = fixture(Effect::Gi);
        let target = reference(&scene);
        let mut settings = Effect::Gi.raster_settings();
        settings.aovs = vec![Aov::Diffuse, Aov::Specular];
        let render = |scene: &Snapshot, settings: &RenderSettings| {
            engine.render(scene, settings, &Control::new()).unwrap()
        };
        let raster = render(&scene, &settings);
        settings.hybrid.gi = true;
        assert_eq!(render(&scene, &settings).beauty.data, raster.beauty.data);
        settings.hybrid.enabled = true;
        let traced = render(&scene, &settings);
        let baseline = psnr(&raster, &target);
        let improved = psnr(&traced, &target);
        assert!(improved > baseline, "GI: {improved:.3} vs {baseline:.3} dB");
        eprintln!("GI: {baseline:.3} → {improved:.3} dB");
        assert_eq!(
            traced.aovs[&Aov::Specular].data,
            raster.aovs[&Aov::Specular].data
        );
        for (beauty, diffuse) in traced
            .beauty
            .data
            .as_chunks::<4>()
            .0
            .iter()
            .zip(traced.aovs[&Aov::Diffuse].data.as_chunks::<4>().0)
        {
            assert_eq!(beauty[..3], diffuse[..3]);
        }

        scene.instances.pop();
        scene.environment.lighting = Lighting::Constant {
            color: [0.3; 3],
            intensity: 1.0,
        };
        let gi_sky = render(&scene, &settings);
        settings.hybrid.enabled = false;
        let raster_sky = render(&scene, &settings);
        assert!(centre_mean(&raster_sky, 4)[0] > 0.0);
        assert_eq!(gi_sky.beauty.data, raster_sky.beauty.data);
    });
}

#[test]
fn all_hybrid_signals_compose_in_the_same_frame() {
    with_gpu(|device, queue| {
        let mut scene = fixture(Effect::Reflections);
        let mut light = Light::new(
            LightKind::Area {
                shape: AreaShape::Rectangle {
                    width: 56.0,
                    height: 56.0,
                },
                two_sided: false,
            },
            down_from(72.0),
        );
        light.shadow = ShadowMode::Soft;
        scene.lights.push(light);
        let mut settings = Effect::Reflections.raster_settings();
        settings.post.ssr = false;
        settings.aovs = vec![Aov::Diffuse, Aov::Specular, Aov::Shadow];
        settings.hybrid.enabled = true;
        let base = settings.hybrid;
        let render = |settings: &RenderSettings| {
            RtEngine::with_device(device.clone(), queue.clone())
                .render(&scene, settings, &Control::new())
                .unwrap()
        };

        // No environment: AO still occupies a layer, but cannot darken direct
        // lighting. Starting fresh gives the same samples for each effect.
        settings.hybrid.shadows = true;
        let shadows = render(&settings);
        settings.hybrid.reflections = true;
        let reflections = render(&settings);
        settings.hybrid = HybridSettings {
            gi: true,
            shadows: true,
            ..base
        };
        let gi = render(&settings);
        settings.hybrid.ao = true;
        settings.hybrid.reflections = true;
        let all = render(&settings);

        assert_ne!(
            shadows.aovs[&Aov::Specular].data,
            reflections.aovs[&Aov::Specular].data
        );
        assert_ne!(
            shadows.aovs[&Aov::Diffuse].data,
            gi.aovs[&Aov::Diffuse].data
        );
        assert_eq!(all.aovs[&Aov::Shadow].data, shadows.aovs[&Aov::Shadow].data);
        assert_eq!(
            all.aovs[&Aov::Specular].data,
            reflections.aovs[&Aov::Specular].data
        );
        assert_eq!(all.aovs[&Aov::Diffuse].data, gi.aovs[&Aov::Diffuse].data);
    });
}

fn frame_delta(a: &RenderOutput, b: &RenderOutput) -> f64 {
    a.beauty
        .data
        .iter()
        .zip(&b.beauty.data)
        .map(|(a, b)| (f64::from(*a) - f64::from(*b)).abs())
        .sum()
}

#[test]
fn ray_traced_ao_converges_and_discards_history_after_camera_change() {
    converges_and_discards_history_after_camera_change(Effect::Ao);
}

#[test]
fn ray_traced_shadows_converge_and_discard_history_after_camera_change() {
    converges_and_discards_history_after_camera_change(Effect::Shadows);
}

#[test]
fn ray_traced_reflections_converge_and_discard_history_after_camera_change() {
    converges_and_discards_history_after_camera_change(Effect::Reflections);
}

#[test]
fn ray_traced_gi_converges_and_discards_history_after_camera_change() {
    converges_and_discards_history_after_camera_change(Effect::Gi);
}

fn converges_and_discards_history_after_camera_change(effect: Effect) {
    with_gpu(|device, queue| {
        let engine = RtEngine::with_device(device.clone(), queue.clone());
        let mut scene = fixture(effect);
        let mut settings = effect.raster_settings();
        settings.hybrid.enabled = true;
        settings.hybrid.ao = matches!(effect, Effect::Ao);
        settings.hybrid.shadows = matches!(effect, Effect::Shadows);
        settings.hybrid.reflections = matches!(effect, Effect::Reflections);
        settings.hybrid.gi = matches!(effect, Effect::Gi);
        settings.hybrid.ao_radius = AO_RADIUS;

        let mut previous = engine.render(&scene, &settings, &Control::new()).unwrap();
        let mut early = 0.0;
        let mut late = 0.0;
        for frame in 1..16 {
            let next = engine.render(&scene, &settings, &Control::new()).unwrap();
            let delta = frame_delta(&previous, &next);
            if frame == 1 {
                early = delta;
            }
            late = delta;
            previous = next;
        }
        assert!(early > 0.0 && late < early, "early {early}, late {late}");

        scene.camera = Camera::new(
            DMat4::from_translation(DVec3::X * 3.0) * down_from(5.0),
            Projection::Orthographic { width: 84.0 },
        );
        let changed = engine.render(&scene, &settings, &Control::new()).unwrap();
        let fresh = RtEngine::with_device(device.clone(), queue.clone());
        let expected = fresh.render(&scene, &settings, &Control::new()).unwrap();
        assert_eq!(changed.beauty.data, expected.beauty.data);
    });
}

#[test]
fn cpu_references_resolve_each_secondary_signal() {
    for effect in Effect::ALL {
        let scene = fixture(effect);
        let mut without_ceiling = scene.clone();
        without_ceiling.instances.pop();
        let present = reference(&scene);
        let absent = reference(&without_ceiling);

        // Both primary images see exactly the same floor. The centre receives
        // a measurable change even though the ceiling never appears on screen.
        for image in [&present, &absent] {
            assert!(
                image
                    .beauty
                    .data
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .all(|p| p[3] == 1.0)
            );
        }
        let with = centre_mean(&present, 4)[0];
        let without = centre_mean(&absent, 4)[0];
        let signal = if effect.occludes() {
            without - with
        } else {
            with - without
        };
        assert!(signal > 0.01, "{effect:?}: centre red {with} vs {without}");
    }
}

#[test]
fn emissive_panel_lights_the_receiver_through_an_opening() {
    with_gpu(|device, queue| {
        let engine = RtEngine::with_device(device.clone(), queue.clone());
        let mut scene = fixture(Effect::Gi);
        let mut settings = Effect::Gi.raster_settings();
        settings.hybrid.enabled = true;
        settings.hybrid.gi = true;
        let floor_light = |scene: &Snapshot| {
            let out = engine.render(scene, &settings, &Control::new()).unwrap();
            centre_mean(&out, 4)[0]
        };
        scene.materials.push(diffuse([0.0; 3]));
        scene.meshes.push(cube(1.0));
        let screen = |x, width| {
            DMat4::from_translation(DVec3::new(x, 12.0, 0.0))
                * DMat4::from_scale(DVec3::new(width, 0.5, 80.0))
        };
        for (id, x) in [(3, -43.0), (4, 43.0)] {
            scene
                .instances
                .push(Instance::new(2, screen(x, 37.0), 2, id));
        }
        let open = floor_light(&scene);
        scene
            .instances
            .push(Instance::new(2, screen(0.0, 6.0), 2, 5));
        let closed = floor_light(&scene);
        assert!(open > 0.001 && closed < open * 0.05, "{open} vs {closed}");
    });
}
