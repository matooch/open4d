//! Environment images: importance sampling and the furnace (docs/specs/RENDER.md R16, brief 133).

mod common;

use std::sync::Arc;

use common::{diffuse, snapshot_with, sphere};
use forge_image::{AlphaMode, Image};
use forge_parity_harness::{CompareOptions, ImageBuf, compare_image};
use forge_render_api::*;
use forge_render_cpu::CpuEngine;
use forge_render_cpu::lights::EnvDist;
use forge_render_cpu::sampling::Rng;
use glam::{DMat4, DVec3};

/// A `w x h` lat-long image: `base` everywhere and `sun` in the box `x0..x1`, `y0..y1`.
fn image(w: u32, h: u32, base: f32, sun: f32, b: (u32, u32, u32, u32)) -> Image {
    let mut data = Vec::new();
    for y in 0..h {
        for x in 0..w {
            let v = if (b.0..b.2).contains(&x) && (b.1..b.3).contains(&y) {
                sun
            } else {
                base
            };
            data.extend_from_slice(&[v, v, v, 1.0]);
        }
    }
    Image {
        width: w,
        height: h,
        data,
        alpha: AlphaMode::Straight,
    }
}

fn lighting(img: Image, intensity: f64) -> Lighting {
    Lighting::Image {
        image: Arc::new(img),
        rotation: 0.0,
        intensity,
    }
}

#[test]
fn the_density_integrates_to_one_and_matches_the_samples() {
    let l = lighting(image(64, 32, 0.2, 500.0, (10, 6, 13, 9)), 1.0);
    let dist = EnvDist::build(&l).expect("an image with light");
    // integral of the density over the sphere (midpoint rule on the image grid)
    let (n, m) = (512, 256);
    let mut total = 0.0;
    for j in 0..m {
        let th = std::f64::consts::PI * (j as f64 + 0.5) / m as f64;
        for i in 0..n {
            let az = std::f64::consts::TAU * (i as f64 + 0.5) / n as f64;
            let d = DVec3::new(th.sin() * az.sin(), th.cos(), th.sin() * az.cos());
            total += dist.pdf(d)
                * th.sin()
                * (std::f64::consts::PI / m as f64)
                * (std::f64::consts::TAU / n as f64);
        }
    }
    assert!((total - 1.0).abs() < 0.01, "integral {total}");
    // every sample reports the density of its own direction, and most land in the sun
    let mut rng = Rng::new(7, 1);
    let mut in_sun = 0;
    let samples = 4000;
    for _ in 0..samples {
        let (d, pdf) = dist.sample(rng.f2());
        assert!((pdf - dist.pdf(d)).abs() < 1e-9 * pdf.max(1.0));
        let u = (d.x.atan2(d.z) / std::f64::consts::TAU).rem_euclid(1.0);
        let v = d.y.acos() / std::f64::consts::PI;
        let (x, y) = ((u * 64.0) as u32, (v * 32.0) as u32);
        if (10..13).contains(&x) && (6..9).contains(&y) {
            in_sun += 1;
        }
    }
    // the sun's share of the weight: 9 pixels of 500 against 2039 pixels of 0.2 (sin-weighted)
    assert!(
        in_sun as f64 / samples as f64 > 0.9,
        "{in_sun} of {samples}"
    );
    // a black image cannot be sampled
    assert!(EnvDist::build(&lighting(image(8, 4, 0.0, 0.0, (0, 0, 0, 0)), 1.0)).is_none());
    assert!(
        EnvDist::build(&Lighting::Constant {
            color: [1.0; 3],
            intensity: 1.0
        })
        .is_none()
    );
}

fn settings(w: u32, h: u32, spp: u32) -> RenderSettings {
    RenderSettings::new(w, h, spp)
}

#[test]
fn a_white_furnace_of_an_image_gives_one() {
    // a convex white diffuse sphere in a constant environment image of radiance 1 shows 1, with
    // the importance sampled estimator (every pixel has the same luminance)
    let mut snap = snapshot_with(
        sphere(1.0, 24),
        diffuse([1.0; 3]),
        DMat4::IDENTITY,
        common::looking_at_origin(4.0),
    );
    snap.environment.lighting = lighting(image(32, 16, 1.0, 1.0, (0, 0, 0, 0)), 1.0);
    snap.environment.background = Background::Environment;
    let out = CpuEngine::new()
        .render(
            &snap,
            &RenderSettings {
                max_depth: 8,
                ..settings(24, 24, 128)
            },
            &Control::new(),
        )
        .unwrap();
    let m = common::centre_mean(&out, 6);
    for c in m {
        assert!((c - 1.0).abs() < 0.03, "furnace {m:?}");
    }
}

#[test]
fn a_scene_lit_by_a_sun_image_has_no_fireflies_at_64_spp() {
    // A sphere under a dim sky with a small, very bright sun: the cheap uniform sampling of the
    // sphere of directions misses the sun most of the time and hits it with a huge weight.
    let img = image(64, 32, 0.3, 100.0, (40, 7, 42, 9));
    let mut snap = snapshot_with(
        sphere(50.0, 32),
        diffuse([0.8; 3]),
        DMat4::IDENTITY,
        Camera::from_hpb(
            DVec3::new(0.0, 0.0, -200.0),
            [0.0; 3],
            Projection::Perspective {
                focal_mm: 36.0,
                sensor_mm: 36.0,
                film_offset: [0.0; 2],
            },
        ),
    );
    snap.environment.lighting = lighting(img, 1.0);
    let render = |spp: u32, seed: u64| {
        let st = RenderSettings {
            seed,
            ..settings(24, 24, spp)
        };
        let out = CpuEngine::new()
            .render(&snap, &st, &Control::new())
            .unwrap();
        ImageBuf::from_rgba(24, 24, &out.beauty.data)
    };
    let reference = render(4096, 99);
    let test = render(64, 1);
    let r = compare_image(&reference, &test, &CompareOptions::default()).unwrap();
    println!("psnr {:.1} dB, gain {:.3}", r.psnr, r.exposure_gain);
    assert!(
        r.psnr >= 30.0,
        "PSNR {:.1} dB at 64 spp against 4096 spp",
        r.psnr
    );
    // and the picture is not black: the sun lights the sphere
    assert!(reference.rgb.iter().cloned().fold(0.0f32, f32::max) > 0.2);
}
