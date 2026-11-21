//! Thin-lens depth of field (docs/specs/RENDER.md R15): geometry of the lens rays and the blur
//! size against the circle-of-confusion formula. Own design (the reference's Physical renderer
//! was not observed: docs/parity/camera.md `dof`), so there is no golden data.

mod common;

use common::{diffuse, snapshot_with, sphere};
use forge_render_api::*;
use forge_render_cpu::CpuEngine;
use glam::{DMat4, DVec3};

fn camera(dist: f64, aperture: Option<Aperture>) -> Camera {
    let c = Camera::from_hpb(
        DVec3::new(0.0, 0.0, -dist),
        [0.0; 3],
        Projection::Perspective {
            focal_mm: 36.0,
            sensor_mm: 36.0,
            film_offset: [0.0; 2],
        },
    );
    match aperture {
        Some(a) => c.with_aperture(a),
        None => c,
    }
}

#[test]
fn lens_rays_of_one_pixel_meet_on_the_focus_plane() {
    let ap = Aperture {
        radius: 5.0,
        focus_distance: 400.0,
    };
    let cam = camera(600.0, Some(ap));
    let pin = cam.ray(200.0, 90.0, 320, 240);
    // the point where the pinhole ray crosses the focus plane (z = -600 + 400)
    let t = 400.0 / pin.1.z;
    let target = pin.0 + pin.1 * t;
    for u in [(0.1, 0.2), (0.9, 0.3), (0.5, 0.95), (0.3, 0.7)] {
        let (o, d) = cam.ray_lens(200.0, 90.0, 320, 240, u);
        assert!(
            (o - cam.position()).length() <= 5.0 + 1e-9,
            "origin on the lens disc"
        );
        assert!((d.length() - 1.0).abs() < 1e-12);
        let s = (target.z - o.z) / d.z;
        assert!((o + d * s - target).length() < 1e-9, "{u:?}");
    }
    // the centre of the lens is the pinhole ray
    let (o, d) = cam.ray_lens(200.0, 90.0, 320, 240, (0.5, 0.5));
    assert!((o - pin.0).length() < 1e-12 && (d - pin.1).length() < 1e-12);
    // no aperture, or an orthographic camera: the pinhole ray for any sample
    let plain = camera(600.0, None);
    assert_eq!(
        plain.ray_lens(10.0, 20.0, 64, 48, (0.2, 0.9)),
        plain.ray(10.0, 20.0, 64, 48)
    );
    let mut ortho = Camera::new(DMat4::IDENTITY, Projection::Orthographic { width: 100.0 });
    ortho.aperture = Some(ap);
    assert_eq!(
        ortho.ray_lens(10.0, 20.0, 64, 48, (0.2, 0.9)),
        ortho.ray(10.0, 20.0, 64, 48)
    );
}

/// Standard deviation along x of the coverage of a small sphere at distance `z`, in pixels.
fn spread(z: f64, focus: f64, dof: bool) -> f64 {
    let (w, h) = (64u32, 64u32);
    let ap = Aperture {
        radius: 20.0,
        focus_distance: focus,
    };
    let snap = snapshot_with(
        sphere(10.0, 48),
        diffuse([1.0; 3]),
        DMat4::IDENTITY,
        camera(z, Some(ap)),
    );
    let st = RenderSettings {
        depth_of_field: dof,
        ..RenderSettings::new(w, h, 256)
    };
    let out = CpuEngine::new()
        .render(&snap, &st, &Control::new())
        .unwrap();
    let (mut s0, mut s1, mut s2) = (0.0, 0.0, 0.0);
    for y in 0..h {
        for x in 0..w {
            let a = out.beauty.pixel(x, y)[3] as f64;
            let px = x as f64 + 0.5;
            s0 += a;
            s1 += a * px;
            s2 += a * px * px;
        }
    }
    (s2 / s0 - (s1 / s0).powi(2)).sqrt()
}

#[test]
fn blur_follows_the_circle_of_confusion() {
    // f_px = 64 (36 mm lens on a 36 mm gate over 64 px). A sphere of radius r0 at distance z,
    // focus F, lens radius a: its image is the sphere's disc convolved with a uniform disc of
    // radius c = a |F - z| / F (in the object plane), so per axis sigma^2 = (r0^2 + c^2) / 4.
    let (r0, a, f_px) = (10.0f64, 20.0f64, 64.0f64);
    for (z, focus) in [
        (300.0f64, 600.0f64),
        (600.0, 600.0),
        (1200.0, 600.0),
        (900.0, 600.0),
    ] {
        let c = a * (focus - z).abs() / focus;
        let want = ((r0 * r0 + c * c) / 4.0).sqrt() * f_px / z;
        let got = spread(z, focus, true);
        assert!(
            (got - want).abs() < 0.06 * want + 0.05,
            "z {z} focus {focus}: sigma {got:.3} px, predicted {want:.3} px"
        );
    }
    // without the setting the camera is a pinhole whatever its aperture
    let pin = spread(300.0, 600.0, false);
    let sharp = r0 / 2.0 * f_px / 300.0;
    assert!(
        (pin - sharp).abs() < 0.06 * sharp + 0.05,
        "{pin} vs {sharp}"
    );
}

#[test]
fn depth_of_field_renders_are_deterministic() {
    let a = spread(300.0, 600.0, true);
    assert_eq!(a, spread(300.0, 600.0, true));
}
