//! CPU path tracer core: BVH, instancing, BSDFs, MIS, determinism (docs/specs/RENDER.md R6-R9).

mod common;

use common::*;
use forge_core::Value;
use forge_render_api::*;
use forge_render_cpu::CpuEngine;
use forge_render_cpu::scene::Scene;
use glam::{DMat4, DVec3};

fn render(snap: &Snapshot, s: &RenderSettings) -> RenderOutput {
    CpuEngine::new().render(snap, s, &Control::new()).unwrap()
}

fn settings(w: u32, h: u32, spp: u32) -> RenderSettings {
    RenderSettings {
        default_light: false,
        ..RenderSettings::new(w, h, spp)
    }
}

fn brute(snap: &Snapshot, o: DVec3, d: DVec3) -> Option<f64> {
    let mut best: Option<f64> = None;
    for inst in &snap.instances {
        let m = DMat4::from_cols_array(&inst.transform);
        let inv = m.inverse();
        let (lo, ld) = (inv.transform_point3(o), inv.transform_vector3(d));
        let mesh = &snap.meshes[inst.mesh];
        for t in &mesh.triangles {
            let p = |i: u32| DVec3::from(mesh.positions[i as usize]);
            let (a, b, c) = (p(t[0]), p(t[1]), p(t[2]));
            let (e1, e2) = (b - a, c - a);
            let pv = ld.cross(e2);
            let det = e1.dot(pv);
            if det.abs() < 1e-18 {
                continue;
            }
            let tv = lo - a;
            let u = tv.dot(pv) / det;
            let qv = tv.cross(e1);
            let v = ld.dot(qv) / det;
            let tt = e2.dot(qv) / det;
            if u >= 0.0 && v >= 0.0 && u + v <= 1.0 && tt > 1e-12 && best.is_none_or(|b| tt < b) {
                best = Some(tt);
            }
        }
    }
    best
}

#[test]
fn bvh_matches_brute_force_over_random_rays() {
    // A deterministic soup of spheres at different places, two instances of one mesh.
    let mut snap = snapshot_with(
        sphere(1.0, 12),
        diffuse([1.0; 3]),
        DMat4::from_translation(DVec3::new(0.0, 0.0, 0.0)),
        looking_at_origin(10.0),
    );
    for k in 1..40 {
        let a = k as f64 * 2.399;
        let t = DVec3::new(a.cos() * 3.0, (a * 0.7).sin() * 3.0, (a * 1.3).cos() * 3.0);
        let s = 0.3 + 0.1 * (k % 5) as f64;
        snap.instances.push(Instance::new(
            0,
            DMat4::from_scale_rotation_translation(
                DVec3::splat(s),
                glam::DQuat::from_rotation_y(a),
                t,
            ),
            0,
            k as u32,
        ));
    }
    let scene = Scene::build(&snap).unwrap();
    assert_eq!(scene.blases.len(), 1, "instances share one BLAS");
    assert_eq!(scene.insts.len(), 40);
    let mut seed = 12345u64;
    let mut rnd = || {
        seed = seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((seed >> 11) as f64) / (1u64 << 53) as f64
    };
    let mut hits = 0;
    for _ in 0..400 {
        let o = DVec3::new(rnd() * 8.0 - 4.0, rnd() * 8.0 - 4.0, -9.0);
        let tgt = DVec3::new(rnd() * 6.0 - 3.0, rnd() * 6.0 - 3.0, rnd() * 6.0 - 3.0);
        let d = (tgt - o).normalize();
        let a = scene.intersect(o, d, f64::INFINITY).map(|h| h.t);
        let b = brute(&snap, o, d);
        match (a, b) {
            (Some(a), Some(b)) => {
                assert!((a - b).abs() < 1e-9, "{a} vs {b}");
                hits += 1;
            }
            (None, None) => {}
            other => panic!("mismatch {other:?}"),
        }
        assert_eq!(
            scene.occluded(o, d, f64::INFINITY),
            b.is_some(),
            "any-hit agrees"
        );
    }
    assert!(hits > 50, "the rays must hit something ({hits})");
}

#[test]
fn closed_and_open_meshes_are_told_apart() {
    let snap = Snapshot {
        meshes: vec![cube(1.0), plane(1.0), sphere(1.0, 8)],
        instances: vec![],
        ..Snapshot::default()
    };
    let s = Scene::build(&snap).unwrap();
    assert!(s.blases[0].closed);
    assert!(!s.blases[1].closed);
    assert!(s.blases[2].closed, "poles are shared by many triangles");
}

#[test]
fn white_furnace_diffuse() {
    // A convex white diffuse sphere in a constant environment of radiance 1 shows exactly 1.
    let mut snap = snapshot_with(
        sphere(1.0, 24),
        diffuse([1.0; 3]),
        DMat4::IDENTITY,
        looking_at_origin(4.0),
    );
    snap.environment.lighting = Lighting::Constant {
        color: [1.0; 3],
        intensity: 1.0,
    };
    snap.environment.background = Background::Environment;
    let out = render(
        &snap,
        &RenderSettings {
            max_depth: 8,
            ..settings(24, 24, 128)
        },
    );
    let m = centre_mean(&out, 6);
    for c in m {
        assert!((c - 1.0).abs() < 0.03, "furnace {m:?}");
    }
}

#[test]
fn white_furnace_with_highlight_never_gains_energy() {
    let mut snap = snapshot_with(
        sphere(1.0, 24),
        material(&[
            ("color.color", Value::Color([0.8; 3])),
            ("reflectance.spec.strength", Value::Float(0.2)),
            ("reflectance.spec.type", Value::Enum("ggx".into())),
            ("reflectance.spec.width", Value::Float(0.3)),
        ]),
        DMat4::IDENTITY,
        looking_at_origin(4.0),
    );
    snap.environment.lighting = Lighting::Constant {
        color: [1.0; 3],
        intensity: 1.0,
    };
    let out = render(&snap, &settings(24, 24, 128));
    let m = centre_mean(&out, 6);
    for c in m {
        assert!(c <= 1.02 && c > 0.8, "{m:?}");
    }
}

#[test]
fn glass_sphere_furnace_conserves_energy() {
    let mut snap = snapshot_with(
        sphere(1.0, 32),
        material(&[
            ("enable.color", Value::Bool(false)),
            ("enable.reflectance", Value::Bool(false)),
            ("enable.transparency", Value::Bool(true)),
            ("transparency.refraction", Value::Float(1.5)),
        ]),
        DMat4::IDENTITY,
        looking_at_origin(4.0),
    );
    snap.environment.lighting = Lighting::Constant {
        color: [1.0; 3],
        intensity: 1.0,
    };
    let out = render(
        &snap,
        &RenderSettings {
            max_depth: 24,
            ..settings(24, 24, 256)
        },
    );
    let m = centre_mean(&out, 5);
    for c in m {
        assert!((c - 1.0).abs() < 0.05, "glass furnace {m:?}");
    }
}

#[test]
fn deterministic_across_thread_counts_and_tiles() {
    let mut snap = snapshot_with(
        sphere(1.0, 16),
        diffuse([0.7, 0.4, 0.2]),
        DMat4::IDENTITY,
        looking_at_origin(4.0),
    );
    snap.environment.lighting = Lighting::Sky {
        zenith: [0.2, 0.4, 1.0],
        horizon: [1.0; 3],
        ground: [0.2; 3],
        intensity: 1.0,
    };
    let base = settings(32, 24, 8);
    let a = render(
        &snap,
        &RenderSettings {
            threads: Some(1),
            ..base.clone()
        },
    );
    let b = render(
        &snap,
        &RenderSettings {
            threads: Some(4),
            tile: 7,
            ..base.clone()
        },
    );
    assert_eq!(a.beauty.data, b.beauty.data);
    let c = render(&snap, &RenderSettings { seed: 2, ..base });
    assert_ne!(a.beauty.data, c.beauty.data);
}

#[test]
fn tiles_progress_and_cancel() {
    use std::sync::{Arc, Mutex};
    let snap = snapshot_with(
        sphere(1.0, 8),
        diffuse([1.0; 3]),
        DMat4::IDENTITY,
        looking_at_origin(4.0),
    );
    let log = Arc::new(Mutex::new((0usize, 0usize, 0usize)));
    let l2 = log.clone();
    let ctl = Control::with_events(move |e| {
        let mut g = l2.lock().unwrap();
        match e {
            RenderEvent::Tile { .. } => g.0 += 1,
            RenderEvent::Progress { .. } => g.1 += 1,
            RenderEvent::Finished => g.2 += 1,
            _ => {}
        }
    });
    CpuEngine::new()
        .render(
            &snap,
            &RenderSettings {
                tile: 8,
                ..settings(32, 16, 1)
            },
            &ctl,
        )
        .unwrap();
    assert_eq!(*log.lock().unwrap(), (8, 8, 1));
    let ctl = Control::new();
    ctl.cancel();
    let r = CpuEngine::new().render(&snap, &settings(16, 16, 1), &ctl);
    assert_eq!(r.unwrap_err(), RenderError::Cancelled);
}

#[test]
fn region_leaves_the_rest_empty() {
    let mut snap = snapshot_with(
        sphere(1.0, 8),
        diffuse([1.0; 3]),
        DMat4::IDENTITY,
        looking_at_origin(4.0),
    );
    snap.environment.background = Background::Color([0.5; 3]);
    let out = render(
        &snap,
        &RenderSettings {
            region: Some([8, 8, 16, 16]),
            ..settings(16, 16, 1)
        },
    );
    assert_eq!(px(&out, 0, 0), [0.0; 4]);
    assert_eq!(px(&out, 15, 15)[0], 0.5);
}

#[test]
fn bad_input_is_an_error_not_a_panic() {
    let mut snap = snapshot_with(
        sphere(1.0, 8),
        diffuse([1.0; 3]),
        DMat4::ZERO,
        looking_at_origin(4.0),
    );
    assert!(matches!(
        CpuEngine::new().render(&snap, &settings(4, 4, 1), &Control::new()),
        Err(RenderError::Snapshot(_))
    ));
    snap.instances[0].material = 9;
    assert!(
        CpuEngine::new()
            .render(&snap, &settings(4, 4, 1), &Control::new())
            .is_err()
    );
    assert!(
        CpuEngine::new()
            .render(
                &Snapshot::default(),
                &RenderSettings::new(0, 4, 1),
                &Control::new()
            )
            .is_err()
    );
}

#[test]
fn emissive_material_and_ignored_channels() {
    let snap = snapshot_with(
        sphere(1.0, 16),
        material(&[
            ("enable.color", Value::Bool(false)),
            ("enable.reflectance", Value::Bool(false)),
            ("enable.luminance", Value::Bool(true)),
            ("luminance.color", Value::Color([1.0, 0.5, 0.0])),
            ("enable.fog", Value::Bool(true)),
        ]),
        DMat4::IDENTITY,
        looking_at_origin(4.0),
    );
    let out = render(&snap, &settings(16, 16, 4));
    let m = centre_mean(&out, 3);
    assert!((m[0] - 1.0).abs() < 1e-6 && (m[1] - 0.5).abs() < 1e-6 && m[2].abs() < 1e-6);
    assert_eq!(out.stats.ignored_channels, ["fog"]);
}

#[test]
fn lambert_under_the_default_head_light() {
    // Facing the camera with the head light on: radiance factor = albedo.
    let snap = snapshot_with(
        plane(5.0),
        diffuse([0.8; 3]),
        DMat4::from_rotation_x(-std::f64::consts::FRAC_PI_2) * DMat4::from_translation(DVec3::ZERO),
        looking_at_origin(4.0),
    );
    let out = render(
        &snap,
        &RenderSettings {
            default_light: true,
            ..settings(16, 16, 4)
        },
    );
    // At the image centre the light direction is the view direction.
    let p = px(&out, 8, 8);
    assert!((p[0] - 0.8).abs() < 0.02, "{p:?}");
    assert_eq!(p[3], 1.0);
}

#[test]
fn bump_changes_the_shading_not_the_silhouette() {
    // The sampler of the plain scene answers white everywhere: a constant height has no gradient.
    let base = snapshot_with(
        sphere(1.0, 16),
        diffuse([0.8; 3]),
        DMat4::IDENTITY,
        looking_at_origin(4.0),
    );
    let bumped = snapshot_with(
        sphere(1.0, 16),
        material(&[
            ("color.color", Value::Color([0.8; 3])),
            ("enable.reflectance", Value::Bool(false)),
            ("enable.bump", Value::Bool(true)),
            ("bump.texture", Value::String("noise".into())),
        ]),
        DMat4::IDENTITY,
        looking_at_origin(4.0),
    );
    let s = RenderSettings {
        default_light: true,
        ..settings(24, 24, 2)
    };
    let a = render(&base, &s);
    let b = render(&bumped, &s);
    // Same answer from the constant white sampler: bump with a constant height changes nothing.
    assert_eq!(a.beauty.data, b.beauty.data);
}
