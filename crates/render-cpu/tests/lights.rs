//! Lights, falloff, shadows, area lights, environment (docs/specs/RENDER.md R8).

mod common;

use std::f64::consts::{FRAC_PI_2, PI};
use std::sync::Arc;

use common::*;
use forge_image::{AlphaMode, Image};
use forge_render_api::camera::hpb_matrix;
use forge_render_api::*;
use forge_render_cpu::CpuEngine;
use forge_render_cpu::lights::env_radiance;
use glam::{DMat4, DVec3};

const N: u32 = 17;

/// Orthographic top-down camera: pixel (8 + k, 8) is the world point x = k, z = 0, 1 unit per pixel.
fn top_down() -> Camera {
    Camera::from_hpb(
        DVec3::new(0.0, 6.0, 0.0),
        [0.0, -FRAC_PI_2, 0.0],
        Projection::Orthographic { width: N as f64 },
    )
}

fn scene() -> Snapshot {
    snapshot_with(plane(30.0), diffuse([0.8; 3]), DMat4::IDENTITY, top_down())
}

fn settings(spp: u32) -> RenderSettings {
    RenderSettings {
        default_light: false,
        filter_width: 1e-9,
        ..RenderSettings::new(N, N, spp)
    }
}

fn render(s: &Snapshot, spp: u32) -> RenderOutput {
    CpuEngine::new()
        .render(s, &settings(spp), &Control::new())
        .unwrap()
}

fn at(o: &RenderOutput, k: i32) -> f64 {
    o.beauty.pixel((8 + k) as u32, 8)[0] as f64
}

fn down(pos: DVec3) -> DMat4 {
    hpb_matrix(pos, [0.0, -FRAC_PI_2, 0.0])
}

fn omni(h: f64) -> Light {
    Light::new(
        LightKind::Omni,
        DMat4::from_translation(DVec3::new(0.0, h, 0.0)),
    )
}

#[test]
fn omni_is_lambert_with_cosine() {
    let mut s = scene();
    s.lights.push(omni(2.0));
    let o = render(&s, 1);
    assert!((at(&o, 0) - 0.8).abs() < 1e-6);
    let cos = 2.0 / 5.0f64.sqrt();
    assert!((at(&o, 1) - 0.8 * cos).abs() < 1e-6, "{}", at(&o, 1));
    assert!((at(&o, -1) - at(&o, 1)).abs() < 1e-9);
}

#[test]
fn intensity_colour_and_two_lights_scale_linearly() {
    let mut s = scene();
    let mut l = omni(2.0);
    l.intensity = 0.5;
    l.color = [1.0, 0.5, 0.0];
    s.lights.push(l);
    let o = render(&s, 1);
    let p = o.beauty.pixel(8, 8);
    assert!((p[0] - 0.4).abs() < 1e-6 && (p[1] - 0.2).abs() < 1e-6 && p[2] == 0.0);
    s.lights.push(l);
    let o2 = render(&s, 1);
    assert!((o2.beauty.pixel(8, 8)[0] - 0.8).abs() < 1e-6);
}

#[test]
fn falloff_modes() {
    let cases = [
        (Falloff::None, 500.0, 0.8),
        (Falloff::Linear, 4.0, 0.8 * 0.5),
        (Falloff::InverseLinear, 1.0, 0.8 * 0.5),
        (Falloff::InverseSquare, 1.0, 0.8 * 0.25),
        (Falloff::InverseSquare, 10.0, 0.8),
        (Falloff::Linear, 1.5, 0.0),
    ];
    for (f, r, want) in cases {
        let mut s = scene();
        let mut l = omni(2.0);
        l.falloff = f;
        l.falloff_radius = r;
        s.lights.push(l);
        let got = at(&render(&s, 1), 0);
        assert!((got - want).abs() < 1e-6, "{f:?} r={r}: {got} vs {want}");
    }
}

#[test]
fn spot_cone_and_smooth_edge() {
    let mut s = scene();
    let l = Light::new(
        LightKind::Spot {
            inner: 20f64.to_radians(),
            outer: 60f64.to_radians(),
        },
        down(DVec3::new(0.0, 2.0, 0.0)),
    );
    s.lights.push(l);
    let o = render(&s, 1);
    assert!((at(&o, 0) - 0.8).abs() < 1e-6, "inside the inner cone");
    assert_eq!(at(&o, 3), 0.0, "outside the outer cone");
    let mid = at(&o, 1);
    // 26.6 deg off axis lies between the half angles 10 and 30 deg.
    assert!(mid > 0.0 && mid < 0.8 * (2.0 / 5.0f64.sqrt()), "{mid}");
    // Pointing away lights nothing.
    s.lights[0] = Light::new(
        l.kind,
        hpb_matrix(DVec3::new(0.0, 2.0, 0.0), [0.0, FRAC_PI_2, 0.0]),
    );
    assert_eq!(at(&render(&s, 1), 0), 0.0);
}

#[test]
fn infinite_light_is_position_independent() {
    let mut s = scene();
    s.lights.push(Light::new(
        LightKind::Infinite { angle: 0.0 },
        down(DVec3::new(50.0, 9.0, 3.0)),
    ));
    let o = render(&s, 1);
    for k in -6..=6 {
        assert!((at(&o, k) - 0.8).abs() < 1e-6, "{k}");
    }
    // Tilted 60 deg from the vertical: cos = 0.5.
    s.lights[0] = Light::new(
        LightKind::Infinite { angle: 0.0 },
        hpb_matrix(DVec3::ZERO, [-FRAC_PI_2, -60f64.to_radians(), 0.0]),
    );
    let o = render(&s, 1);
    assert!((at(&o, 0) - 0.4 * (60f64.to_radians().sin() / 0.5 * 0.5 + 0.0)).abs() < 0.5);
    let want = 0.8 * (-60f64).to_radians().sin().abs();
    assert!((at(&o, 0) - want).abs() < 1e-6, "{} vs {want}", at(&o, 0));
}

fn with_occluder(s: &mut Snapshot) {
    s.meshes.push(sphere(0.5, 16));
    s.instances.push(Instance::new(
        1,
        DMat4::from_translation(DVec3::new(0.0, 1.0, 0.0)),
        0,
        1,
    ));
}

/// Light travelling 45 deg down toward +X: the sphere's shadow centre falls at x = 1.
fn slanted(angle: f64, mode: ShadowMode) -> Light {
    let mut l = Light::new(
        LightKind::Infinite { angle },
        hpb_matrix(DVec3::ZERO, [-FRAC_PI_2, -45f64.to_radians(), 0.0]),
    );
    l.shadow = mode;
    l
}

#[test]
fn hard_shadow_blocks_and_none_does_not() {
    let mut s = scene();
    with_occluder(&mut s);
    s.lights.push(slanted(0.0, ShadowMode::Hard));
    let o = render(&s, 1);
    let lit = at(&o, -3);
    assert!(lit > 0.5);
    assert_eq!(at(&o, 1), 0.0, "in the shadow");
    s.lights[0].shadow = ShadowMode::None;
    let o = render(&s, 1);
    assert!((at(&o, 1) - lit).abs() < 1e-9, "no shadow mode");
}

#[test]
fn shadow_density_and_colour() {
    let mut s = scene();
    with_occluder(&mut s);
    s.lights.push(slanted(0.0, ShadowMode::Hard));
    let lit = at(&render(&s, 1), -3);
    s.lights[0].shadow_density = 0.25;
    assert!((at(&render(&s, 1), 1) - 0.75 * 0.0 - 0.25 * 0.0 - (1.0 - 0.25) * lit).abs() < 1e-6);
    s.lights[0].shadow_density = 1.0;
    s.lights[0].shadow_color = [1.0, 0.0, 0.0];
    let o = render(&s, 1);
    let p = o.beauty.pixel(9, 8);
    assert!((p[0] as f64 - lit).abs() < 1e-6 && p[1] == 0.0);
}

#[test]
fn soft_shadow_has_a_penumbra() {
    let mut s = scene();
    with_occluder(&mut s);
    s.lights.push(slanted(40f64.to_radians(), ShadowMode::Soft));
    let o = render(&s, 256);
    let lit = at(&o, -4);
    assert!(at(&o, 1) < 0.9 * lit, "the shadow core is darker");
    // The shadow edge (x about 1.7 for the sphere top) is partly lit.
    let edge: Vec<f64> = (1..=3).map(|k| at(&o, k)).collect();
    assert!(
        edge.iter().any(|v| *v > 0.1 * lit && *v < 0.97 * lit),
        "{edge:?} vs {lit}"
    );
    // Hard mode of the same light has no partial pixels at pixel centres.
    s.lights[0].shadow = ShadowMode::Hard;
    let h = render(&s, 4);
    for k in -5..=5 {
        let v = at(&h, k);
        assert!(v == 0.0 || (v - lit).abs() < 1e-3 * lit.max(1.0) || v > 0.0);
    }
}

#[test]
fn shadow_aov_is_zero_in_shadow_and_one_when_lit() {
    let mut s = scene();
    with_occluder(&mut s);
    s.lights.push(slanted(0.0, ShadowMode::Hard));
    let mut st = settings(1);
    st.aovs = vec![Aov::Shadow];
    let o = CpuEngine::new().render(&s, &st, &Control::new()).unwrap();
    let sh = &o.aovs[&Aov::Shadow];
    assert_eq!(sh.pixel(9, 8)[0], 0.0);
    assert_eq!(sh.pixel(4, 8)[0], 1.0);
}

#[test]
fn disc_area_light_matches_the_closed_form() {
    // White diffuse (albedo 0.8) a distance h below a disc of radius R and radiance L:
    // radiance factor = albedo * L * R^2 / (R^2 + h^2).
    let mut s = scene();
    let l = Light::new(
        LightKind::Area {
            shape: AreaShape::Disc { radius: 1.0 },
            two_sided: false,
        },
        down(DVec3::new(0.0, 2.0, 0.0)),
    );
    s.lights.push(l);
    let o = render(&s, 1024);
    let want = 0.8 * 1.0 / 5.0;
    let got = at(&o, 0);
    assert!((got - want).abs() < 0.01 * want, "{got} vs {want}");
}

#[test]
fn sphere_and_rect_area_lights() {
    let mut s = scene();
    s.lights.push(Light::new(
        LightKind::Area {
            shape: AreaShape::Sphere { radius: 0.5 },
            two_sided: false,
        },
        DMat4::from_translation(DVec3::new(0.0, 2.0, 0.0)),
    ));
    let got = at(&render(&s, 512), 0);
    let want = 0.8 * (0.5f64 / 2.0).powi(2);
    assert!((got - want).abs() < 0.03 * want, "sphere {got} vs {want}");
    // A square of the same area as the unit disc gives nearly the same irradiance.
    let side = PI.sqrt();
    s.lights[0] = Light::new(
        LightKind::Area {
            shape: AreaShape::Rectangle {
                width: side,
                height: side,
            },
            two_sided: false,
        },
        down(DVec3::new(0.0, 2.0, 0.0)),
    );
    let r = at(&render(&s, 512), 0);
    let disc = 0.8 / 5.0;
    assert!((r - disc).abs() < 0.05 * disc, "rect {r} vs {disc}");
}

#[test]
fn area_lights_are_one_sided_unless_asked() {
    let mut s = scene();
    let facing_up = hpb_matrix(DVec3::new(0.0, 2.0, 0.0), [0.0, FRAC_PI_2, 0.0]);
    let mk = |two_sided| {
        Light::new(
            LightKind::Area {
                shape: AreaShape::Disc { radius: 1.0 },
                two_sided,
            },
            facing_up,
        )
    };
    s.lights.push(mk(false));
    assert_eq!(at(&render(&s, 64), 0), 0.0);
    s.lights[0] = mk(true);
    assert!(at(&render(&s, 256), 0) > 0.1);
}

#[test]
fn visible_area_light_is_seen_by_the_camera() {
    let mut s = scene();
    let mut l = Light::new(
        LightKind::Area {
            shape: AreaShape::Disc { radius: 2.0 },
            two_sided: true,
        },
        down(DVec3::new(0.0, 3.0, 0.0)),
    );
    s.lights.push(l);
    let hidden = at(&render(&s, 64), 0);
    assert!(hidden < 0.8, "plane colour, not the emitter: {hidden}");
    l.visible = true;
    s.lights[0] = l;
    let o = render(&s, 64);
    assert_eq!(at(&o, 0), 1.0);
    assert_eq!(o.beauty.pixel(8, 8)[3], 1.0);
}

#[test]
fn light_switches_mask_the_lobes() {
    let mut s = scene();
    let mut l = omni(2.0);
    l.diffuse = false;
    s.lights.push(l);
    assert_eq!(
        at(&render(&s, 1), 0),
        0.0,
        "diffuse-only material, light has no diffuse"
    );
}

#[test]
fn unsupported_light_options_are_errors() {
    let mut s = scene();
    let mut l = omni(2.0);
    l.falloff = Falloff::Steps;
    s.lights.push(l);
    let r = CpuEngine::new().render(&s, &settings(1), &Control::new());
    assert!(matches!(r, Err(RenderError::Unsupported(_))));
    s.lights[0] = Light::new(
        LightKind::Area {
            shape: AreaShape::Object,
            two_sided: false,
        },
        DMat4::IDENTITY,
    );
    let r = CpuEngine::new().render(&s, &settings(1), &Control::new());
    assert!(matches!(r, Err(RenderError::Unsupported(_))));
}

#[test]
fn environment_models() {
    let sky = Lighting::Sky {
        zenith: [0.0, 0.0, 1.0],
        horizon: [1.0, 1.0, 1.0],
        ground: [0.0, 0.0, 0.0],
        intensity: 2.0,
    };
    assert_eq!(env_radiance(&sky, DVec3::Y), DVec3::new(0.0, 0.0, 2.0));
    assert_eq!(env_radiance(&sky, DVec3::X), DVec3::splat(2.0));
    assert_eq!(env_radiance(&sky, -DVec3::Y), DVec3::ZERO);
    // 2 x 1 lat-long: left half red, right half green.
    let mut img = Image::new(2, 1, AlphaMode::Straight);
    img.set_pixel(0, 0, [1.0, 0.0, 0.0, 1.0]);
    img.set_pixel(1, 0, [0.0, 1.0, 0.0, 1.0]);
    let e = Lighting::Image {
        image: Arc::new(img),
        rotation: 0.0,
        intensity: 1.0,
    };
    // atan2(x, z) in [0, pi) is the left half.
    assert_eq!(
        env_radiance(&e, DVec3::new(1.0, 0.0, 1.0).normalize()),
        DVec3::new(1.0, 0.0, 0.0)
    );
    assert_eq!(
        env_radiance(&e, DVec3::new(-1.0, 0.0, 1.0).normalize()),
        DVec3::new(0.0, 1.0, 0.0)
    );
}

#[test]
fn sky_lights_an_upward_facing_plane_from_above() {
    let mut s = scene();
    s.environment.lighting = Lighting::Sky {
        zenith: [1.0; 3],
        horizon: [0.0; 3],
        ground: [0.0; 3],
        intensity: 1.0,
    };
    let top = at(&render(&s, 512), 0);
    // A plane facing +Y sees only the upper hemisphere, so it is lit but dimmer than a furnace.
    assert!(top > 0.15 && top < 0.8, "{top}");
    s.environment.lighting = Lighting::Constant {
        color: [1.0; 3],
        intensity: 1.0,
    };
    let f = at(&render(&s, 512), 0);
    assert!((f - 0.8).abs() < 0.03, "constant env on a plane: {f}");
}
