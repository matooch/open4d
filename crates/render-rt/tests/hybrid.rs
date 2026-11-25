//! R24 acceptance fixtures. Use O43 material inputs, not the reference PNGs:
//! each isolated lighting signal has a higher-sample Forge CPU reference.

#[path = "../../render-cpu/tests/common/mod.rs"]
mod common;

use common::{centre_mean, diffuse, golden, plane};
use forge_render_api::*;
use forge_render_cpu::CpuEngine;
use forge_render_rt::{RtEngine, request_default_device};
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

fn with_raster_engine(test: impl FnOnce(RtEngine)) {
    // Keep the per-file device locked for the full test, including all submits
    // and readbacks (software Vulkan in CI can take longer than Metal).
    let mut gpu = GPU.lock().unwrap_or_else(|e| e.into_inner());
    match gpu.get_or_insert_with(request_default_device) {
        Ok((device, queue)) => test(RtEngine::with_device(device.clone(), queue.clone())),
        Err(e) if std::env::var("FORGE_REQUIRE_GPU").as_deref() == Ok("1") => {
            panic!("FORGE_REQUIRE_GPU=1 but no GPU: {e}")
        }
        Err(e) => eprintln!("skipping GPU test: {e}"),
    }
}

#[test]
fn tier_one_has_a_measurable_error_against_each_cpu_reference() {
    with_raster_engine(|engine| {
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
