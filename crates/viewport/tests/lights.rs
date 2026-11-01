//! Lights, rig and shadow cameras (docs/specs/VIEWPORT.md §8.1-§8.2). Pure, no GPU.

use forge_viewport::camera::Camera;
use forge_viewport::lights::{
    AMBIENT, HEADLIGHT_INTENSITY, Light, LightKind, Lighting, MAX_LIGHTS, SceneLights,
    bounds_sphere, default_rig, headlight, resolve, shadow_matrix,
};
use glam::{DMat4, DVec3};

const BOUNDS: (DVec3, DVec3) = (DVec3::new(-3.0, -1.0, -2.0), DVec3::new(5.0, 4.0, 2.5));

fn corners((lo, hi): (DVec3, DVec3)) -> Vec<DVec3> {
    (0..8)
        .map(|i| {
            DVec3::new(
                if i & 1 == 1 { hi.x } else { lo.x },
                if i & 2 == 2 { hi.y } else { lo.y },
                if i & 4 == 4 { hi.z } else { lo.z },
            )
        })
        .collect()
}

fn ndc(m: &DMat4, p: DVec3) -> DVec3 {
    let c = *m * p.extend(1.0);
    c.truncate() / c.w
}

#[test]
fn constructors_and_travel_direction() {
    let d = Light::directional(DVec3::new(0.0, -3.0, 0.0), 2.0);
    assert_eq!(d.kind, LightKind::Directional);
    assert!((d.travel() - DVec3::NEG_Y).length() < 1e-12);
    assert_eq!(d.intensity, 2.0);
    assert!(!d.cast_shadow);
    // a zero direction falls back to straight down
    assert_eq!(Light::directional(DVec3::ZERO, 1.0).travel(), DVec3::NEG_Y);
    let p = Light::point(DVec3::new(1.0, 2.0, 3.0), 1.0).with_range(5.0);
    assert_eq!((p.kind, p.range), (LightKind::Point, 5.0));
    // inner is never wider than outer
    let s = Light::spot(DVec3::ZERO, DVec3::X, 0.9, 0.4, 1.0)
        .with_shadow(true)
        .with_color([1.0, 0.5, 0.0]);
    assert_eq!(s.kind, LightKind::Spot);
    assert!(s.inner <= s.outer);
    assert!(s.cast_shadow);
    assert_eq!(s.color, [1.0, 0.5, 0.0]);
}

#[test]
fn headlight_is_the_legacy_light() {
    let cam = Camera {
        yaw: 0.7,
        pitch: 0.3,
        ..Camera::default()
    };
    let h = headlight(&cam);
    assert!((h.travel() - cam.forward_world()).length() < 1e-12);
    assert_eq!(h.intensity, HEADLIGHT_INTENSITY);
    assert!(!h.cast_shadow);
    // ambient + headlight = 1 facing the light: the old `0.25 + 0.75 |n.v|`
    assert_eq!(AMBIENT + HEADLIGHT_INTENSITY, 1.0);
    let lights = resolve(&Lighting::Headlight, &cam);
    assert_eq!(lights.len(), 1);
    assert_eq!(lights[0], h);
}

#[test]
fn the_rig_is_three_world_anchored_directionals_and_the_key_casts() {
    let rig = default_rig();
    assert_eq!(rig.len(), 3);
    for l in &rig {
        assert_eq!(l.kind, LightKind::Directional);
        assert!((l.direction.length() - 1.0).abs() < 1e-12);
    }
    assert!(rig[0].cast_shadow && !rig[1].cast_shadow && !rig[2].cast_shadow);
    // the key shines from above (+Y side), i.e. it travels downwards
    assert!(rig[0].direction.y < 0.0);
    // world anchored: the same for two cameras
    let a = resolve(&Lighting::Rig, &Camera::default());
    let b = resolve(
        &Lighting::Rig,
        &Camera {
            yaw: 2.0,
            pitch: -0.4,
            ..Camera::default()
        },
    );
    assert_eq!(a, b);
    // total intensity stays below the headroom left by the ambient term
    let total: f32 = rig.iter().map(|l| l.intensity).sum();
    assert!(AMBIENT + total <= 1.1);
}

#[test]
fn scene_lights_are_capped_at_eight() {
    let many: Vec<Light> = (0..12)
        .map(|i| Light::point(DVec3::new(i as f64, 0.0, 0.0), 1.0))
        .collect();
    let s = SceneLights::new(&many);
    assert_eq!(s.count, MAX_LIGHTS);
    assert_eq!(s.as_slice(), &many[..8]);
    let r = resolve(&Lighting::Scene(s), &Camera::default());
    assert_eq!(r.len(), 8);
    assert_eq!(SceneLights::default().count, 0);
    assert!(resolve(&Lighting::Scene(SceneLights::default()), &Camera::default()).is_empty());
}

#[test]
fn directional_shadow_camera_covers_the_bounds() {
    let (_, r) = bounds_sphere(BOUNDS);
    assert!((r - (BOUNDS.1 - BOUNDS.0).length() / 2.0).abs() < 1e-12);
    for dir in [
        DVec3::new(0.3, -1.0, 0.2),
        DVec3::new(-1.0, -0.2, 0.5),
        DVec3::NEG_Y,
        DVec3::Y,
    ] {
        let m = shadow_matrix(&Light::directional(dir, 1.0).with_shadow(true), BOUNDS)
            .expect("directional casts");
        for c in corners(BOUNDS) {
            let n = ndc(&m, c);
            assert!(
                n.x.abs() <= 1.0 && n.y.abs() <= 1.0 && (0.0..=1.0).contains(&n.z),
                "{dir:?} {c:?} -> {n:?}"
            );
        }
    }
}

#[test]
fn spot_shadow_camera_looks_down_the_axis() {
    let pos = DVec3::new(1.0, 10.0, 0.0);
    let centre = (BOUNDS.0 + BOUNDS.1) * 0.5;
    let axis = centre - pos;
    let l = Light::spot(pos, axis, 0.2, 0.8, 1.0).with_shadow(true);
    let m = shadow_matrix(&l, BOUNDS).expect("spot casts");
    let n = ndc(&m, centre);
    assert!(n.x.abs() < 1e-9 && n.y.abs() < 1e-9, "{n:?}");
    assert!((0.0..1.0).contains(&n.z));
    // nearer points have smaller depth
    let near = ndc(&m, pos + axis.normalize() * 2.0);
    assert!(near.z < n.z);
}

#[test]
fn point_and_non_casting_lights_have_no_shadow_camera() {
    assert!(shadow_matrix(&Light::point(DVec3::Y, 1.0).with_shadow(true), BOUNDS).is_none());
    assert!(shadow_matrix(&Light::directional(DVec3::NEG_Y, 1.0), BOUNDS).is_none());
    assert!(
        shadow_matrix(
            &Light::spot(DVec3::Y * 9.0, DVec3::NEG_Y, 0.1, 0.5, 1.0),
            BOUNDS
        )
        .is_none()
    );
}
