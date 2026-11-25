//! RENDER.md R21.7: the raster engine against the CPU tracer, its AOVs, lights, shadows,
//! environment, determinism and events.
//!
//! Skipped without an adapter unless `FORGE_REQUIRE_GPU=1`. One device per test file, created
//! behind the static lock (implementer playbook §5a).

#[path = "../../render-cpu/tests/common/mod.rs"]
mod common;

use common::golden::{case_names, load};
use common::{diffuse, image_parity, looking_at_origin, material, plane, sphere};
use forge_core::Value;
use forge_image::{AlphaMode, Image};
use forge_parity_harness::{CompareOptions, ImageBuf, compare_image};
use forge_render_api::*;
use forge_render_cpu::CpuEngine;
use forge_render_rt::{RtEngine, request_default_device};
use glam::{DMat4, DVec3};
use std::sync::Mutex;

/// R21.7: direct light only, so the CPU tracer's 16 spp noise and the missing GI are inside
/// this budget on a scene with one delta light and no environment.
const MIN_PSNR: f64 = 30.0;
/// R21.7: the O30 structure check (silhouette and mean light level, not per-pixel colour).
const MIN_IOU: f64 = 0.98;
const MEAN_RATIO: (f64, f64) = (0.6, 1.7);

/// R2 is opaque (R21.3): the CPU's transmissive material loses light the raster keeps, so only
/// the silhouette is compared.
const OPAQUE_ONLY: &[(&str, &str)] = &[(
    "mat-transparent",
    "transmission is not refracted in the raster tier (RENDER.md R21.3)",
)];

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

fn psnr(a: &RenderOutput, b: &RenderOutput) -> f64 {
    let x = ImageBuf::from_rgba(a.width, a.height, &a.beauty.data);
    let y = ImageBuf::from_rgba(b.width, b.height, &b.beauty.data);
    compare_image(&x, &y, &CompareOptions::default())
        .unwrap()
        .psnr
}

fn mean_lit(out: &RenderOutput) -> (f64, f64, usize) {
    let mut sum = 0.0f64;
    let mut n = 0usize;
    for p in out.beauty.data.as_chunks::<4>().0 {
        if p[3] > 0.5 {
            sum += 0.2126 * p[0] as f64 + 0.7152 * p[1] as f64 + 0.0722 * p[2] as f64;
            n += 1;
        }
    }
    (sum, sum / n.max(1) as f64, n)
}

fn iou(a: &RenderOutput, b: &RenderOutput) -> f64 {
    let mut inter = 0.0f64;
    let mut union = 0.0f64;
    for (p, q) in a
        .beauty
        .data
        .as_chunks::<4>()
        .0
        .iter()
        .zip(b.beauty.data.as_chunks::<4>().0.iter())
    {
        let (x, y) = (p[3] > 0.5, q[3] > 0.5);
        if x && y {
            inter += 1.0;
        }
        if x || y {
            union += 1.0;
        }
    }
    inter / union.max(1.0)
}

fn light(kind: LightKind, at: DMat4) -> Light {
    Light::new(kind, at)
}

/// A plane at y = 0 and a sphere on it, the camera at -Z: the geometry of most O30 cases
/// without their materials.
fn sphere_on_plane(mat: MaterialData) -> Snapshot {
    let mut snap = Snapshot {
        meshes: vec![plane(400.0), sphere(100.0, 32)],
        instances: vec![
            Instance::new(0, DMat4::IDENTITY, 1, 0),
            Instance::new(
                1,
                DMat4::from_translation(DVec3::new(0.0, 100.0, 0.0)),
                0,
                1,
            ),
        ],
        materials: vec![mat, diffuse([0.8, 0.8, 0.8])],
        camera: looking_at_origin(600.0),
        ..Snapshot::default()
    };
    snap.camera = Camera::from_hpb(
        DVec3::new(0.0, 150.0, -600.0),
        [10f64.to_radians(), 0.0, 0.0],
        Projection::Perspective {
            focal_mm: 36.0,
            sensor_mm: 36.0,
            film_offset: [0.0, 0.0],
        },
    );
    snap
}

#[test]
fn direct_light_matches_the_cpu_tracer() {
    let _serial = serial();
    let Some((device, queue)) = gpu() else {
        return;
    };
    let engine = RtEngine::with_device(device, queue);
    let settings = RenderSettings {
        samples: 64,
        max_depth: 0,
        ..RenderSettings::new(160, 120, 64)
    };
    let mut scenes: Vec<(&str, Snapshot)> = Vec::new();
    // (a) one directional light, diffuse sphere on a plane
    let mut a = sphere_on_plane(diffuse([0.8, 0.6, 0.4]));
    a.lights = vec![light(LightKind::Infinite { angle: 0.0 }, DMat4::IDENTITY)];
    scenes.push(("directional", a));
    // (b) an omni light with inverse-square falloff
    let mut b = sphere_on_plane(diffuse([0.9, 0.9, 0.9]));
    let mut l = light(
        LightKind::Omni,
        DMat4::from_translation(DVec3::new(200.0, 350.0, -250.0)),
    );
    l.intensity = 1.0;
    l.falloff = Falloff::Linear;
    l.falloff_radius = 2500.0;
    b.lights = vec![l];
    scenes.push(("omni-falloff", b));
    // (c) glossy reflection
    let mut c = sphere_on_plane(material(&[
        ("enable.reflectance", Value::Bool(true)),
        ("specular.width", Value::Float(20.0)),
    ]));
    c.lights = vec![
        light(LightKind::Infinite { angle: 0.0 }, DMat4::IDENTITY),
        light(
            LightKind::Spot {
                inner: 20f64.to_radians(),
                outer: 45f64.to_radians(),
            },
            DMat4::from_translation(DVec3::new(-300.0, 500.0, -400.0)),
        ),
    ];
    scenes.push(("glossy-two-lights", c));
    // (d) two materials, two lights, no plane
    let mut d = Snapshot {
        meshes: vec![sphere(100.0, 32), sphere(80.0, 32)],
        instances: vec![
            Instance::new(
                0,
                DMat4::from_translation(DVec3::new(-120.0, 0.0, 0.0)),
                0,
                0,
            ),
            Instance::new(
                1,
                DMat4::from_translation(DVec3::new(120.0, 0.0, 0.0)),
                1,
                1,
            ),
        ],
        materials: vec![diffuse([0.9, 0.2, 0.2]), diffuse([0.2, 0.4, 0.9])],
        camera: looking_at_origin(500.0),
        ..Snapshot::default()
    };
    d.lights = vec![
        light(LightKind::Infinite { angle: 0.0 }, DMat4::IDENTITY),
        light(
            LightKind::Omni,
            DMat4::from_translation(DVec3::new(300.0, 200.0, -200.0)),
        ),
    ];
    scenes.push(("two-materials", d));
    for (name, snap) in scenes {
        let cpu = CpuEngine::new()
            .render(&snap, &settings, &Control::new())
            .unwrap_or_else(|e| panic!("{name}: CPU: {e}"));
        let rt = engine
            .render(&snap, &settings, &Control::new())
            .unwrap_or_else(|e| panic!("{name}: RT: {e}"));
        let p = psnr(&cpu, &rt);
        eprintln!("{name}: {p:.2} dB");
        assert!(p >= MIN_PSNR, "{name}: CPU against RT {p:.2} dB");
    }
}

#[test]
fn the_o30_cases_keep_the_cpu_silhouette_and_light_level() {
    let _serial = serial();
    let Some((device, queue)) = gpu() else {
        return;
    };
    let engine = RtEngine::with_device(device, queue);
    let settings = image_parity::settings((320, 240), Vec::new());
    let mut checked = 0;
    for name in case_names() {
        let case = load(&name);
        let rt = engine.render(&case.snapshot, &settings, &Control::new());
        let cpu = CpuEngine::new()
            .render(&case.snapshot, &settings, &Control::new())
            .unwrap_or_else(|e| panic!("{name}: CPU: {e}"));
        let rt = rt.unwrap_or_else(|e| panic!("{name}: RT: {e}"));
        let i = iou(&cpu, &rt);
        let (_, cm, cn) = mean_lit(&cpu);
        let (_, rm, rn) = mean_lit(&rt);
        assert!(cn > 0 && rn > 0, "{name}: no lit pixels");
        let ratio = rm / cm;
        eprintln!("{name}: IoU {i:.4}, mean ratio {ratio:.3}");
        assert!(i >= MIN_IOU, "{name}: silhouette IoU {i:.4}");
        if OPAQUE_ONLY.iter().any(|(n, _)| *n == name) {
            checked += 1;
            continue;
        }
        assert!(
            (MEAN_RATIO.0..=MEAN_RATIO.1).contains(&ratio),
            "{name}: mean luminance ratio {ratio:.3} (CPU {cm:.4}, RT {rm:.4})"
        );
        checked += 1;
    }
    assert_eq!(
        checked,
        case_names().len(),
        "every O30 case must be checked"
    );
}

/// R21.3: the shader-node colour reaches the raster image (it is not the fallback grey), so the
/// checker's two squares differ by about as much as they do in the CPU image.
#[test]
fn shader_node_colour_varies_across_the_surface() {
    let _serial = serial();
    let Some((device, queue)) = gpu() else {
        return;
    };
    let engine = RtEngine::with_device(device, queue);
    let settings = image_parity::settings((320, 240), Vec::new());
    for name in ["checker-plane", "mat-checker-uv", "mat-checker-cubic"] {
        let case = load(name);
        let spread = |out: &RenderOutput| {
            let mut lo = f64::MAX;
            let mut hi = 0.0f64;
            for p in out.beauty.data.as_chunks::<4>().0 {
                if p[3] > 0.5 {
                    let l = 0.2126 * p[0] as f64 + 0.7152 * p[1] as f64 + 0.0722 * p[2] as f64;
                    lo = lo.min(l);
                    hi = hi.max(l);
                }
            }
            hi / lo.max(1e-6)
        };
        let cpu = CpuEngine::new()
            .render(&case.snapshot, &settings, &Control::new())
            .unwrap();
        let rt = engine
            .render(&case.snapshot, &settings, &Control::new())
            .unwrap();
        let (c, r) = (spread(&cpu), spread(&rt));
        eprintln!("{name}: brightest/darkest CPU {c:.2}, RT {r:.2}");
        assert!(
            r > 1.5,
            "{name}: the raster image has no checker contrast ({r:.2})"
        );
    }
}

#[test]
fn a_light_intensity_scales_the_image() {
    let _serial = serial();
    let Some((device, queue)) = gpu() else {
        return;
    };
    let engine = RtEngine::with_device(device, queue);
    let mut snap = sphere_on_plane(diffuse([0.8, 0.8, 0.8]));
    let mut l = light(LightKind::Infinite { angle: 0.0 }, DMat4::IDENTITY);
    l.intensity = 1.0;
    snap.lights = vec![l];
    let settings = RenderSettings::new(80, 60, 1);
    let one = engine.render(&snap, &settings, &Control::new()).unwrap();
    snap.lights[0].intensity = 0.5;
    let half = engine.render(&snap, &settings, &Control::new()).unwrap();
    let (a, _, _) = mean_lit(&one);
    let (b, _, _) = mean_lit(&half);
    let ratio = a / b;
    assert!(
        (ratio - 2.0).abs() < 0.02,
        "intensity 1 against 0.5 is {ratio:.4}, not 2"
    );
    assert!(a > 0.0);
}

#[test]
fn a_shadow_map_darkens_the_occluded_plane() {
    let _serial = serial();
    let Some((device, queue)) = gpu() else {
        return;
    };
    let engine = RtEngine::with_device(device, queue);
    let mut snap = sphere_on_plane(diffuse([0.8, 0.8, 0.8]));
    // A spot above the sphere, looking down: its cone reaches the plane behind the sphere.
    let at = DMat4::from_translation(DVec3::new(0.0, 600.0, 0.0))
        * DMat4::from_rotation_x(std::f64::consts::FRAC_PI_2);
    let mut l = light(
        LightKind::Spot {
            inner: 0.0,
            outer: 60f64.to_radians(),
        },
        at,
    );
    l.intensity = 3.0;
    snap.lights = vec![l];
    let settings = RenderSettings::new(160, 120, 1);
    let unshadowed = engine.render(&snap, &settings, &Control::new()).unwrap();
    snap.lights[0].shadow = ShadowMode::Hard;
    let shadowed = engine.render(&snap, &settings, &Control::new()).unwrap();
    let mut darker = 0;
    let mut same = 0;
    for (p, q) in unshadowed
        .beauty
        .data
        .as_chunks::<4>()
        .0
        .iter()
        .zip(shadowed.beauty.data.as_chunks::<4>().0.iter())
    {
        if p[3] < 0.5 || q[3] < 0.5 {
            continue;
        }
        let a = 0.2126 * p[0] + 0.7152 * p[1] + 0.0722 * p[2];
        let b = 0.2126 * q[0] + 0.7152 * q[1] + 0.0722 * q[2];
        if a > 0.01 && b < 0.5 * a {
            darker += 1;
        }
        if (a - b).abs() < 1e-6 {
            same += 1;
        }
    }
    assert!(darker > 100, "only {darker} pixels are in shadow");
    assert!(same > 100, "only {same} pixels are unaffected");
}

/// R21.4: every shadow-casting light draws its own map with its own matrix. Two spots, one
/// above the sphere and one from the side, each cast a shadow on the plane in a different
/// place; both shadows must show in one render (a backend that applied the last light's matrix
/// to every shadow pass, or shared one tile, would lose one of them).
#[test]
fn two_shadowed_lights_each_cast_their_own_shadow() {
    let _serial = serial();
    let Some((device, queue)) = gpu() else {
        return;
    };
    let engine = RtEngine::with_device(device, queue);
    let mut snap = sphere_on_plane(diffuse([0.8, 0.8, 0.8]));
    let spot = |at: DMat4| {
        let mut l = light(
            LightKind::Spot {
                inner: 0.0,
                outer: 60f64.to_radians(),
            },
            at,
        );
        l.intensity = 3.0;
        l
    };
    let above = spot(
        DMat4::from_translation(DVec3::new(0.0, 600.0, 0.0))
            * DMat4::from_rotation_x(std::f64::consts::FRAC_PI_2),
    );
    // From +X, 400 up, aimed at the origin: the light's +Z axis runs along (-500, -400, 0).
    let side = spot(
        DMat4::from_translation(DVec3::new(500.0, 400.0, 0.0))
            * DMat4::from_rotation_y(-std::f64::consts::FRAC_PI_2)
            * DMat4::from_rotation_x(400f64.atan2(500.0)),
    );
    let settings = RenderSettings::new(160, 120, 1);
    let render = |snap: &mut Snapshot, a: bool, b: bool| {
        let mut lights = vec![above, side];
        lights[0].shadow = if a {
            ShadowMode::Hard
        } else {
            ShadowMode::None
        };
        lights[1].shadow = if b {
            ShadowMode::Hard
        } else {
            ShadowMode::None
        };
        snap.lights = lights;
        engine.render(snap, &settings, &Control::new()).unwrap()
    };
    let lum = |o: &RenderOutput| -> Vec<f64> {
        o.beauty
            .data
            .as_chunks::<4>()
            .0
            .iter()
            .map(|p| {
                if p[3] < 0.5 {
                    f64::NAN
                } else {
                    0.2126 * p[0] as f64 + 0.7152 * p[1] as f64 + 0.0722 * p[2] as f64
                }
            })
            .collect()
    };
    let none = lum(&render(&mut snap, false, false));
    let only_a = lum(&render(&mut snap, true, false));
    let only_b = lum(&render(&mut snap, false, true));
    let both = lum(&render(&mut snap, true, true));
    let (mut a_region, mut b_region, mut a_kept, mut b_kept) = (0, 0, 0, 0);
    for i in 0..none.len() {
        if none[i].is_nan() {
            continue;
        }
        // pixels the above light's shadow darkens, and whether the two-light render keeps it
        if only_a[i] < 0.9 * none[i] {
            a_region += 1;
            if both[i] < 0.9 * only_b[i] {
                a_kept += 1;
            }
        }
        if only_b[i] < 0.9 * none[i] {
            b_region += 1;
            if both[i] < 0.9 * only_a[i] {
                b_kept += 1;
            }
        }
    }
    assert!(
        a_region > 100,
        "the light above casts no shadow ({a_region})"
    );
    assert!(
        b_region > 100,
        "the side light casts no shadow ({b_region})"
    );
    assert!(
        a_kept * 10 >= a_region * 9,
        "the shadow of the light above is missing from the two-light render: {a_kept} of {a_region}"
    );
    assert!(
        b_kept * 10 >= b_region * 9,
        "the shadow of the side light is missing from the two-light render: {b_kept} of {b_region}"
    );
}

#[test]
fn a_constant_environment_lights_a_sphere() {
    let _serial = serial();
    let Some((device, queue)) = gpu() else {
        return;
    };
    let engine = RtEngine::with_device(device, queue);
    let mut snap = Snapshot {
        meshes: vec![sphere(100.0, 32)],
        instances: vec![Instance::new(0, DMat4::IDENTITY, 0, 0)],
        materials: vec![diffuse([1.0, 1.0, 1.0])],
        camera: looking_at_origin(400.0),
        environment: Environment {
            background: Background::Color([0.0; 3]),
            lighting: Lighting::Constant {
                color: [1.0, 1.0, 1.0],
                intensity: 1.0,
            },
        },
        ..Snapshot::default()
    };
    snap.lights = vec![];
    let settings = RenderSettings {
        default_light: false,
        ..RenderSettings::new(80, 60, 1)
    };
    let lit = engine.render(&snap, &settings, &Control::new()).unwrap();
    let (_, m, n) = mean_lit(&lit);
    assert!(n > 0 && m > 0.1, "mean {m} over {n} pixels");
    // a white constant environment on white diffuse gives albedo * radiance ~ 1
    assert!((m - 1.0).abs() < 0.1, "mean {m}, expected about 1");
}

#[test]
fn an_image_environment_matches_a_constant_one() {
    let _serial = serial();
    let Some((device, queue)) = gpu() else {
        return;
    };
    let engine = RtEngine::with_device(device, queue);
    let mut snap = Snapshot {
        meshes: vec![sphere(100.0, 32)],
        instances: vec![Instance::new(0, DMat4::IDENTITY, 0, 0)],
        materials: vec![diffuse([0.8, 0.8, 0.8])],
        camera: looking_at_origin(400.0),
        environment: Environment {
            background: Background::Color([0.0; 3]),
            lighting: Lighting::Constant {
                color: [0.4, 0.6, 0.8],
                intensity: 2.0,
            },
        },
        ..Snapshot::default()
    };
    snap.lights = vec![];
    let settings = RenderSettings {
        default_light: false,
        ..RenderSettings::new(80, 60, 1)
    };
    let constant = engine.render(&snap, &settings, &Control::new()).unwrap();
    let mut image = Image::new(8, 4, AlphaMode::Straight);
    for y in 0..4 {
        for x in 0..8 {
            image.set_pixel(x, y, [0.4, 0.6, 0.8, 1.0]);
        }
    }
    snap.environment.lighting = Lighting::Image {
        image: std::sync::Arc::new(image),
        rotation: 0.0,
        intensity: 2.0,
    };
    let sampled = engine.render(&snap, &settings, &Control::new()).unwrap();
    let p = psnr(&constant, &sampled);
    assert!(p >= 50.0, "uniform image against constant: {p:.2} dB");
    // a dark upper half dims the top of the sphere
    let mut image = Image::new(8, 4, AlphaMode::Straight);
    for y in 0..4 {
        for x in 0..8 {
            let c = if y < 2 { 0.05 } else { 1.0 };
            image.set_pixel(x, y, [c, c, c, 1.0]);
        }
    }
    snap.environment.lighting = Lighting::Image {
        image: std::sync::Arc::new(image),
        rotation: 0.0,
        intensity: 1.0,
    };
    let split = engine.render(&snap, &settings, &Control::new()).unwrap();
    let (cx, cy) = (settings.width / 2, settings.height / 2);
    let top = split.beauty.pixel(cx, cy - 15)[0];
    let bottom = split.beauty.pixel(cx, cy + 15)[0];
    assert!(top < bottom, "top {top} is not darker than bottom {bottom}");
}

#[test]
fn every_aov_has_its_r3_value() {
    let _serial = serial();
    let Some((device, queue)) = gpu() else {
        return;
    };
    let engine = RtEngine::with_device(device, queue);
    let mut snap = sphere_on_plane(material(&[("enable.reflectance", Value::Bool(true))]));
    snap.lights = vec![light(LightKind::Infinite { angle: 0.0 }, DMat4::IDENTITY)];
    let settings = RenderSettings {
        aovs: Aov::ALL.to_vec(),
        ..RenderSettings::new(120, 90, 1)
    };
    let out = engine.render(&snap, &settings, &Control::new()).unwrap();
    for a in Aov::ALL {
        assert!(out.aovs.contains_key(&a), "{a:?} missing");
    }
    // a pixel on the sphere: object 1, material 1, a unit normal, and B = D + S + E
    let ids = &out.aovs[&Aov::ObjectId];
    let mut found = None;
    for y in 0..settings.height {
        for x in 0..settings.width {
            // the sphere is instance 1, so its object id is 2
            if ids.pixel(x, y)[0] == 2.0 {
                found = Some((x, y));
                break;
            }
        }
        if found.is_some() {
            break;
        }
    }
    let (cx, cy) = found.expect("a pixel on object 2");
    assert!(
        out.aovs[&Aov::MaterialId].pixel(cx, cy)[0] == 1.0,
        "material id"
    );
    let n = out.aovs[&Aov::Normal].pixel(cx, cy);
    let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
    assert!((len - 1.0).abs() < 1e-4, "normal length {len}");
    let p = out.aovs[&Aov::Position].pixel(cx, cy);
    let d = (p[0] as f64)
        .hypot(p[1] as f64 - 150.0)
        .hypot(p[2] as f64 + 600.0);
    let depth = out.aovs[&Aov::Depth].pixel(cx, cy)[0] as f64;
    assert!((depth - d).abs() < 1e-2, "depth {depth} against {d}");
    // the sphere's surface is 100 units from its centre at (0, 100, 0)
    let r = (p[0] as f64).hypot(p[1] as f64 - 100.0).hypot(p[2] as f64);
    assert!((r - 100.0).abs() < 0.2, "sphere radius {r}");
    let b = out.beauty.pixel(cx, cy);
    let dsum: Vec<f32> = (0..3)
        .map(|k| {
            out.aovs[&Aov::Diffuse].pixel(cx, cy)[k]
                + out.aovs[&Aov::Specular].pixel(cx, cy)[k]
                + out.aovs[&Aov::Emission].pixel(cx, cy)[k]
        })
        .collect();
    for k in 0..3 {
        assert!((b[k] - dsum[k]).abs() < 1e-4, "beauty is not D + S + E");
    }
    let s = out.aovs[&Aov::Shadow].pixel(cx, cy)[0];
    assert!((s - 1.0).abs() < 1e-6, "unshadowed pixel shadow {s}");
    // background
    let corner = out.aovs[&Aov::ObjectId].pixel(1, 1);
    assert_eq!(corner[0], 0.0);
    assert_eq!(out.aovs[&Aov::Depth].pixel(1, 1)[0], 0.0);
    assert_eq!(out.aovs[&Aov::Albedo].pixel(1, 1)[3], 0.0);
    assert_eq!(out.beauty.pixel(1, 1), [0.0, 0.0, 0.0, 0.0]);
}

#[test]
fn two_renders_are_byte_identical() {
    let _serial = serial();
    let Some((device, queue)) = gpu() else {
        return;
    };
    let engine = RtEngine::with_device(device, queue);
    let mut snap = sphere_on_plane(material(&[("enable.reflectance", Value::Bool(true))]));
    let mut l = light(
        LightKind::Spot {
            inner: 10f64.to_radians(),
            outer: 40f64.to_radians(),
        },
        DMat4::from_translation(DVec3::new(200.0, 500.0, -300.0)),
    );
    l.shadow = ShadowMode::Soft;
    snap.lights = vec![l];
    let settings = RenderSettings {
        aovs: Aov::ALL.to_vec(),
        ..RenderSettings::new(64, 48, 1)
    };
    let a = engine.render(&snap, &settings, &Control::new()).unwrap();
    let b = engine.render(&snap, &settings, &Control::new()).unwrap();
    assert_eq!(a.beauty.data, b.beauty.data);
    for (k, x) in &a.aovs {
        assert_eq!(x.data, b.aovs[k].data, "{k:?} differs");
    }
}

#[test]
fn a_region_is_rendered_and_events_are_reported() {
    let _serial = serial();
    let Some((device, queue)) = gpu() else {
        return;
    };
    let engine = RtEngine::with_device(device, queue);
    let mut snap = sphere_on_plane(diffuse([0.8, 0.8, 0.8]));
    snap.lights = vec![light(LightKind::Infinite { angle: 0.0 }, DMat4::IDENTITY)];
    let settings = RenderSettings {
        region: Some([8, 6, 40, 30]),
        ..RenderSettings::new(64, 48, 1)
    };
    let events = std::sync::Arc::new(Mutex::new(Vec::new()));
    let sink = events.clone();
    let control = Control::with_events(move |e| sink.lock().unwrap().push(e.clone()));
    let out = engine.render(&snap, &settings, &control).unwrap();
    let events = events.lock().unwrap();
    assert!(matches!(
        events.first(),
        Some(RenderEvent::Started { tiles: 1, .. })
    ));
    let tiles: Vec<&RenderEvent> = events
        .iter()
        .filter(|e| matches!(e, RenderEvent::Tile { .. }))
        .collect();
    assert_eq!(tiles.len(), 1);
    if let RenderEvent::Tile {
        x,
        y,
        width,
        height,
        rgba,
    } = tiles[0]
    {
        assert_eq!((*x, *y, *width, *height), (8, 6, 32, 24));
        assert_eq!(rgba.len(), (32 * 24 * 4) as usize);
        for j in 0..*height {
            for i in 0..*width {
                let k = ((j * width + i) * 4) as usize;
                assert_eq!(
                    out.beauty.pixel(x + i, y + j),
                    [rgba[k], rgba[k + 1], rgba[k + 2], rgba[k + 3]]
                );
            }
        }
    }
    assert!(matches!(events.last(), Some(RenderEvent::Finished)));
    assert_eq!(out.beauty.pixel(0, 0), [0.0, 0.0, 0.0, 0.0]);
    assert_eq!(out.stats.tiles, 1);
    drop(events);
    let cancel = Control::new();
    cancel.cancel();
    assert!(matches!(
        engine.render(&snap, &settings, &cancel),
        Err(RenderError::Cancelled)
    ));
}

#[test]
fn depth_of_field_is_pinned_unsupported() {
    let _serial = serial();
    let Some((device, queue)) = gpu() else {
        return;
    };
    let engine = RtEngine::with_device(device, queue);
    let mut snap = sphere_on_plane(diffuse([0.8, 0.8, 0.8]));
    snap.lights = vec![light(LightKind::Infinite { angle: 0.0 }, DMat4::IDENTITY)];
    let settings = RenderSettings {
        depth_of_field: true,
        ..RenderSettings::new(32, 24, 1)
    };
    assert!(matches!(
        engine.render(&snap, &settings, &Control::new()),
        Err(RenderError::Unsupported(_))
    ));
}
