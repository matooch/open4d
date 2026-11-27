//! RENDER.md R22: the post chain of the raster tier — SSAO, SSR, bloom, TAA and filmic tone
//! mapping, each toggle held against the image it changes.
//!
//! Skipped without an adapter unless `FORGE_REQUIRE_GPU=1`. One device per test file, created
//! behind the static lock (implementer playbook §5a).

#[path = "../../render-cpu/tests/common/mod.rs"]
mod common;

use common::{cube, diffuse, material, plane, sphere};
use forge_core::Value;
use forge_render_api::*;
use forge_render_rt::{RtEngine, post, request_default_device};
use glam::{DMat4, DVec3};
use std::sync::Mutex;

const SIZE: (u32, u32) = (96, 72);

static GPU: Mutex<Option<Option<(wgpu::Device, wgpu::Queue)>>> = Mutex::new(None);

/// Held for a whole test: GPU tests in this file run one at a time, so a slow device (CI's
/// software adapters) never makes one test's wait include another test's work (§5a).
static SERIAL: Mutex<()> = Mutex::new(());

fn serial() -> std::sync::MutexGuard<'static, ()> {
    SERIAL.lock().unwrap_or_else(|e| e.into_inner())
}

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

fn engine() -> Option<(std::sync::MutexGuard<'static, ()>, RtEngine)> {
    let serial = serial();
    let (device, queue) = gpu()?;
    Some((serial, RtEngine::with_device(device, queue)))
}

/// A camera at `eye` looking at `target`, +Y up (the R21.2 camera basis).
fn look_at(eye: DVec3, target: DVec3) -> Camera {
    let f = (target - eye).normalize();
    let r = DVec3::Y.cross(f).normalize();
    let u = f.cross(r);
    Camera::new(
        DMat4::from_cols(r.extend(0.0), u.extend(0.0), f.extend(0.0), eye.extend(1.0)),
        Projection::Perspective {
            focal_mm: 36.0,
            sensor_mm: 36.0,
            film_offset: [0.0; 2],
        },
    )
}

fn settings(post: PostSettings) -> RenderSettings {
    RenderSettings {
        samples: 1,
        default_light: false,
        post,
        ..RenderSettings::new(SIZE.0, SIZE.1, 1)
    }
}

/// A uniform environment and a black background: the ambient term is what AO and SSR act on.
fn constant_env(snap: &mut Snapshot, colour: [f64; 3]) {
    snap.environment = Environment {
        background: Background::Color([0.0; 3]),
        lighting: Lighting::Constant {
            color: colour,
            intensity: 1.0,
        },
    };
}

fn emissive(colour: [f64; 3], brightness: f64) -> MaterialData {
    material(&[
        ("color.color", Value::Color([0.0, 0.0, 0.0])),
        ("enable.reflectance", Value::Bool(false)),
        ("enable.luminance", Value::Bool(true)),
        ("luminance.color", Value::Color(colour)),
        ("luminance.brightness", Value::Float(brightness)),
    ])
}

/// A glossy floor material (a narrow specular width is a low roughness, R21.3).
fn glossy_floor() -> MaterialData {
    material(&[
        ("color.color", Value::Color([0.15, 0.15, 0.15])),
        ("enable.reflectance", Value::Bool(true)),
        ("reflectance.spec.width", Value::Float(0.2)),
    ])
}

fn lum(p: [f32; 4]) -> f64 {
    post::luminance([p[0], p[1], p[2]]) as f64
}

fn render(engine: &RtEngine, snap: &Snapshot, settings: &RenderSettings) -> RenderOutput {
    engine
        .render(snap, settings, &Control::new())
        .unwrap_or_else(|e| panic!("render failed: {e}"))
}

fn covered(out: &RenderOutput) -> Vec<bool> {
    out.beauty
        .data
        .as_chunks::<4>()
        .0
        .iter()
        .map(|p| p[3] > 0.5)
        .collect()
}

fn pixels(out: &RenderOutput) -> Vec<[f32; 4]> {
    out.beauty.data.as_chunks::<4>().0.to_vec()
}

#[test]
fn ssao_darkens_a_crease_and_leaves_open_faces() {
    let Some((_serial, engine)) = engine() else {
        return;
    };
    let mut snap = Snapshot {
        meshes: vec![plane(400.0), cube(40.0)],
        instances: vec![
            Instance::new(0, DMat4::IDENTITY, 0, 0),
            Instance::new(1, DMat4::from_translation(DVec3::new(0.0, 40.0, 0.0)), 1, 1),
        ],
        materials: vec![diffuse([0.8, 0.8, 0.8]), diffuse([0.8, 0.8, 0.8])],
        camera: look_at(
            DVec3::new(-190.0, 130.0, -260.0),
            DVec3::new(0.0, 25.0, 0.0),
        ),
        ..Snapshot::default()
    };
    constant_env(&mut snap, [1.0; 3]);
    let off = settings(PostSettings::default());
    let on = settings(PostSettings {
        ssao: true,
        ssao_radius: 30.0,
        ssao_strength: 1.0,
        ..PostSettings::default()
    });
    let a = render(&engine, &snap, &off);
    let b = render(&engine, &snap, &on);
    assert_eq!(covered(&a), covered(&b), "AO must not move the silhouette");
    let (pa, pb) = (pixels(&a), pixels(&b));
    let mask = covered(&a);
    let mut dark = 0usize;
    let mut open = 0usize;
    let mut n = 0usize;
    let mut max_ratio = 0.0f64;
    for i in 0..pa.len() {
        if !mask[i] {
            continue;
        }
        n += 1;
        let ratio = if lum(pa[i]) > 1e-9 {
            lum(pb[i]) / lum(pa[i])
        } else {
            1.0
        };
        max_ratio = max_ratio.max(ratio);
        if ratio < 0.9 {
            dark += 1;
        }
        if ratio > 0.98 {
            open += 1;
        }
    }
    assert!(n > 100, "the scene covers {n} pixels");
    assert!(dark >= 5, "the crease darkens ({dark} pixels below 0.9)");
    assert!(
        open * 2 >= n,
        "open faces stay lit ({open} of {n} pixels above 0.98)"
    );
    assert!(max_ratio > 0.999, "some face is untouched ({max_ratio})");
    // strength 0 is exactly no AO
    let zero = settings(PostSettings {
        ssao: true,
        ssao_strength: 0.0,
        ..PostSettings::default()
    });
    assert_eq!(render(&engine, &snap, &zero).beauty.data, a.beauty.data);
}

#[test]
fn ssr_replaces_the_environment_reflection_with_the_screen_hit() {
    let Some((_serial, engine)) = engine() else {
        return;
    };
    let mut snap = Snapshot {
        meshes: vec![plane(400.0), cube(40.0)],
        instances: vec![
            Instance::new(0, DMat4::IDENTITY, 0, 0),
            Instance::new(
                1,
                DMat4::from_translation(DVec3::new(0.0, 120.0, 0.0)),
                1,
                1,
            ),
        ],
        materials: vec![glossy_floor(), emissive([1.0, 0.1, 0.1], 400.0)],
        camera: look_at(DVec3::new(0.0, 240.0, -420.0), DVec3::new(0.0, 40.0, 0.0)),
        ..Snapshot::default()
    };
    constant_env(&mut snap, [0.02; 3]);
    let off = settings(PostSettings::default());
    let on = settings(PostSettings {
        ssr: true,
        ssr_steps: 32,
        ..PostSettings::default()
    });
    let a = render(&engine, &snap, &off);
    let b = render(&engine, &snap, &on);
    assert_eq!(covered(&a), covered(&b), "SSR must not move the silhouette");
    let (pa, pb) = (pixels(&a), pixels(&b));
    let mut redder = 0usize;
    let mut changed = 0usize;
    for i in 0..pa.len() {
        if pa[i][3] <= 0.5 {
            continue;
        }
        // a floor pixel that takes the red cube's reflection: more red, still more red than green
        if pb[i][0] > 1.5 * pa[i][0] && pb[i][0] > pb[i][1] + 0.02 {
            redder += 1;
        }
        if (lum(pb[i]) - lum(pa[i])).abs() > 0.02 * lum(pa[i]).max(1e-3) {
            changed += 1;
        }
    }
    assert!(
        redder >= 5,
        "the glossy floor reflects the bright cube ({redder} pixels)"
    );
    assert!(changed >= 5, "SSR changes the image ({changed} pixels)");
}

#[test]
fn bloom_spreads_a_bright_object_and_ignores_a_dim_one() {
    let Some((_serial, engine)) = engine() else {
        return;
    };
    let cube_at = |mat: MaterialData| Snapshot {
        meshes: vec![cube(20.0)],
        instances: vec![Instance::new(0, DMat4::IDENTITY, 0, 0)],
        materials: vec![mat],
        camera: look_at(DVec3::new(0.0, 0.0, -200.0), DVec3::ZERO),
        ..Snapshot::default()
    };
    let off = settings(PostSettings::default());
    let on = settings(PostSettings {
        bloom: true,
        bloom_threshold: 1.0,
        bloom_strength: 0.25,
        ..PostSettings::default()
    });
    let bright = cube_at(emissive([1.0, 1.0, 1.0], 400.0));
    let a = render(&engine, &bright, &off);
    let b = render(&engine, &bright, &on);
    // the last covered column of the middle row, then eight pixels into the background
    let row = SIZE.1 / 2;
    let mut edge = 0;
    for x in 0..SIZE.0 {
        if a.beauty.pixel(x, row)[3] > 0.5 {
            edge = x;
        }
    }
    assert!(edge > 2 && edge + 5 < SIZE.0, "bright object at x={edge}");
    let away = (edge + 3).min(SIZE.0 - 1);
    assert_eq!(a.beauty.pixel(away, row), [0.0; 4], "no glow without bloom");
    let glow = b.beauty.pixel(away, row);
    assert!(lum(glow) > 1e-3, "bloom lifts the background ({glow:?})");
    assert!(b.beauty.pixel(edge, row)[0] > a.beauty.pixel(edge, row)[0]);
    // a dim object (0.5, below the threshold) does not bloom at all
    let dim = cube_at(emissive([1.0, 1.0, 1.0], 0.5));
    let d0 = render(&engine, &dim, &off);
    let d1 = render(&engine, &dim, &on);
    assert_eq!(
        d0.beauty.data, d1.beauty.data,
        "a dim object does not bloom"
    );
}

#[test]
fn taa_grades_the_silhouette() {
    let Some((_serial, engine)) = engine() else {
        return;
    };
    let mut snap = Snapshot {
        meshes: vec![sphere(100.0, 32)],
        instances: vec![Instance::new(0, DMat4::IDENTITY, 0, 0)],
        materials: vec![diffuse([0.8, 0.8, 0.8])],
        camera: look_at(DVec3::new(0.0, 0.0, -400.0), DVec3::ZERO),
        ..Snapshot::default()
    };
    constant_env(&mut snap, [1.0; 3]);
    let off = settings(PostSettings::default());
    let on = settings(PostSettings {
        taa: true,
        taa_samples: 4,
        ..PostSettings::default()
    });
    let a = render(&engine, &snap, &off);
    let b = render(&engine, &snap, &on);
    let fractional = |out: &RenderOutput| {
        out.beauty
            .data
            .as_chunks::<4>()
            .0
            .iter()
            .filter(|p| p[3] > 0.05 && p[3] < 0.95)
            .count()
    };
    let covered_a = covered(&a).iter().filter(|c| **c).count();
    let covered_b = covered(&b).iter().filter(|c| **c).count();
    assert_eq!(fractional(&a), 0, "one raster sample has a hard edge");
    assert!(
        fractional(&b) >= 3,
        "four jittered subframes grade the silhouette ({} pixels)",
        fractional(&b)
    );
    let (a, b) = (covered_a as f64, covered_b as f64);
    assert!(
        (a - b).abs() <= 0.02 * a.max(1.0),
        "the silhouette keeps its size ({a} vs {b})"
    );
}

#[test]
fn tonemap_maps_the_beauty_through_the_curve() {
    let Some((_serial, engine)) = engine() else {
        return;
    };
    let snap = Snapshot {
        meshes: vec![cube(20.0)],
        instances: vec![Instance::new(0, DMat4::IDENTITY, 0, 0)],
        materials: vec![emissive([1.0, 0.5, 0.25], 400.0)],
        camera: look_at(DVec3::new(0.0, 0.0, -200.0), DVec3::ZERO),
        ..Snapshot::default()
    };
    let off = settings(PostSettings::default());
    let on = settings(PostSettings {
        tonemap: true,
        exposure: 1.0,
        ..PostSettings::default()
    });
    let a = render(&engine, &snap, &off);
    let b = render(&engine, &snap, &on);
    let (pa, pb) = (pixels(&a), pixels(&b));
    assert!(
        pa.iter().any(|p| p[0] > 2.0),
        "the linear beauty has values above the display range"
    );
    for i in 0..pa.len() {
        assert_eq!(pa[i][3], pb[i][3], "the coverage is untouched");
        for c in 0..3 {
            let want = post::tonemap(pa[i][c], 1.0);
            assert!(
                (pb[i][c] - want).abs() < 1e-4,
                "pixel {i} channel {c}: {} vs {want}",
                pb[i][c]
            );
            assert!((0.0..=1.0).contains(&pb[i][c]), "pixel {i} stays in range");
        }
    }
}

#[test]
fn post_settings_validate() {
    assert!(PostSettings::default().validate().is_ok());
    assert!(PostSettings::default().is_off());
    let bad = |p: PostSettings| p.validate().is_err();
    assert!(bad(PostSettings {
        ssao_radius: 0.0,
        ..PostSettings::default()
    }));
    assert!(bad(PostSettings {
        ssao_strength: 2.0,
        ..PostSettings::default()
    }));
    assert!(bad(PostSettings {
        ssr_steps: 0,
        ..PostSettings::default()
    }));
    assert!(bad(PostSettings {
        taa_samples: 33,
        ..PostSettings::default()
    }));
    assert!(bad(PostSettings {
        bloom_threshold: -1.0,
        ..PostSettings::default()
    }));
    assert!(bad(PostSettings {
        exposure: 0.0,
        ..PostSettings::default()
    }));
    // the render settings validate the block too
    let mut s = RenderSettings::new(8, 8, 1);
    s.post.taa_samples = 0;
    assert!(s.validate().is_err());
}

#[test]
fn the_default_post_block_changes_nothing_and_stays_deterministic() {
    let Some((_serial, engine)) = engine() else {
        return;
    };
    let mut snap = Snapshot {
        meshes: vec![plane(400.0), sphere(100.0, 32)],
        instances: vec![
            Instance::new(0, DMat4::IDENTITY, 0, 0),
            Instance::new(
                1,
                DMat4::from_translation(DVec3::new(0.0, 100.0, 0.0)),
                0,
                1,
            ),
        ],
        materials: vec![diffuse([0.8, 0.8, 0.8]), diffuse([0.7, 0.6, 0.5])],
        camera: look_at(DVec3::new(0.0, 150.0, -600.0), DVec3::new(0.0, 80.0, 0.0)),
        ..Snapshot::default()
    };
    constant_env(&mut snap, [0.4; 3]);
    let base = settings(PostSettings::default());
    let a = render(&engine, &snap, &base);
    let b = render(&engine, &snap, &base);
    assert_eq!(
        a.beauty.data, b.beauty.data,
        "two renders are byte-identical"
    );
    // every effect on: still deterministic (fixed kernels, fixed jitter, no history)
    let all = settings(PostSettings {
        ssao: true,
        ssr: true,
        bloom: true,
        taa: true,
        taa_samples: 3,
        tonemap: true,
        ..PostSettings::default()
    });
    let c = render(&engine, &snap, &all);
    let d = render(&engine, &snap, &all);
    assert_eq!(c.beauty.data, d.beauty.data, "post on is deterministic too");
    assert_eq!(c.aovs.len(), 0, "no AOV was asked for, so none is produced");
}

#[test]
fn antialiasing_tracks_a_thin_edge_during_a_camera_pan() {
    let Some((_serial, engine)) = engine() else {
        return;
    };
    let mut scene = common::snapshot_with(
        cube(1.0),
        glossy_floor(),
        DMat4::from_rotation_z(0.2) * DMat4::from_scale(DVec3::new(1.5, 32.0, 1.0)),
        common::looking_at_origin(150.0),
    );
    constant_env(&mut scene, [1.0; 3]);
    let capture = |scene: &Snapshot, samples| {
        render(
            &engine,
            scene,
            &settings(PostSettings {
                taa: samples > 1,
                taa_samples: samples,
                ..PostSettings::default()
            }),
        )
    };
    let mut error = [0.0; 2];
    for x in [0.0, 0.4, 0.8, 1.2] {
        scene.camera = look_at(DVec3::new(x, 0.0, -150.0), DVec3::new(x, 0.0, 0.0));
        let reference = capture(&scene, 32);
        for (index, samples) in [1, 4].into_iter().enumerate() {
            let image = capture(&scene, samples);
            error[index] += image
                .beauty
                .data
                .iter()
                .zip(&reference.beauty.data)
                .map(|(a, b)| f64::from((a - b).powi(2)))
                .sum::<f64>();
        }
    }
    assert!(error[1] < error[0] * 0.5, "pan errors {error:?}");
}
