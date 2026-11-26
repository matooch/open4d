//! The force objects (DYNAMICS.md section 10). Own design (D20): analytic checks where the
//! scheme has a closed form, statistics otherwise. Fixed seeds throughout.

mod common;

use common::*;
use forge_core::{LinkRow, NodeId, Value};
use forge_particles::{
    ATTRACTOR, Collision, DEFLECTOR, FRICTION, Force, GRAVITY, ROTATION, TURBULENCE, WIND,
};
use glam::DVec3;

const FPS: f64 = 30.0;
/// The emitter default: 4 substeps per frame.
const DT: f64 = 1.0 / (FPS * 4.0);

/// A burst of `n` particles in frame 0 that live forever, with the extra `params`.
fn burst_world(n: usize, params: &[(&str, Value)]) -> World {
    let mut all = vec![
        ("birthrate_editor", f(n as f64)),
        ("start_frame", i(0)),
        ("stop_frame", i(1)),
        ("lifetime", f(100000.0)),
        ("seed", i(4242)),
    ];
    all.extend(params.iter().cloned());
    world(&all)
}

/// Frame-0 and frame-`frame` states by particle id.
fn states(w: &World, frame: i64) -> (forge_particles::Particles, forge_particles::Particles) {
    let mut c = cache(w);
    let a = c.at(0).clone();
    let b = c.at(frame).clone();
    (a, b)
}

#[test]
fn gravity_follows_the_analytic_trajectory_within_the_scheme_error() {
    // Semi-implicit Euler with constant a after n = 4 f steps: v = v0 + a t exactly and
    // p = p0 + v0 t + a t^2 / 2 + a dt t / 2: the last term is the scheme's error. The test
    // checks the exact discrete law (1e-6 for float accumulation over <= 240 steps of
    // magnitude <= 1e4) and that the distance to the continuous parabola is the error term.
    let mut w = burst_world(
        2000,
        &[
            ("angle_h", f(1.0)),
            ("angle_v", f(1.0)),
            ("speed_variation_percent", f(0.5)),
        ],
    );
    add(&mut w, GRAVITY, &[], [0.0; 3]);
    let g = DVec3::new(0.0, -981.0, 0.0);
    for frame in [5i64, 10, 20, 60] {
        let (p0, p) = states(&w, frame);
        assert_eq!(p.len(), 2000);
        let t = frame as f64 / FPS;
        for k in 0..p.len() {
            let a = p0.id.iter().position(|x| *x == p.id[k]).unwrap();
            let continuous = p0.position[a] + p0.velocity[a] * t + g * (0.5 * t * t);
            let discrete = continuous + g * (0.5 * DT * t);
            assert!(
                (p.position[k] - discrete).length() < 1e-6,
                "frame {frame}: discrete law"
            );
            let err = (p.position[k] - continuous).length();
            assert!(
                (err - 0.5 * 981.0 * DT * t).abs() < 1e-6,
                "frame {frame}: error term {err}"
            );
            assert!((p.velocity[k] - (p0.velocity[a] + g * t)).length() < 1e-6);
        }
    }
}

#[test]
fn gravity_follows_the_force_objects_rotation() {
    // A 180 degree bank (B) turns the force's local -Y axis to point up in world space.
    let mut w = burst_world(10, &[("speed", f(0.0))]);
    let gid = add(&mut w, GRAVITY, &[], [0.0; 3]);
    set(
        &mut w,
        gid,
        "transform.rot",
        Value::Vec3([0.0, 0.0, std::f64::consts::PI]),
    );
    let (_, p) = states(&w, 10);
    assert!(
        p.velocity.iter().all(|v| v.y > 0.0 && v.x.abs() < 1e-9),
        "banked gravity points up"
    );
}

#[test]
fn friction_decays_the_speed_exponentially() {
    // v_n = v0 (1 - k dt)^n exactly (v += -k v dt). Against the continuous e^(-k t): with
    // x = k dt, |ln (1 - x)^n + n x| = n |ln(1 - x) + x| <= n x^2 / (2 (1 - x)) =: B, so the ratio
    // to e^(-k t) lies within e^B - 1 of 1 (the test uses that bound, relative).
    let k = 2.0;
    let mut w = burst_world(
        200,
        &[
            ("speed", f(300.0)),
            ("angle_h", f(1.0)),
            ("angle_v", f(1.0)),
        ],
    );
    add(&mut w, FRICTION, &[("friction", f(k))], [0.0; 3]);
    let x = k * DT;
    for frame in [10i64, 30, 60] {
        let (p0, p) = states(&w, frame);
        let n = (4 * frame) as f64;
        let b = n * x * x / (2.0 * (1.0 - x));
        for idx in 0..p.len() {
            let a = p0.id.iter().position(|q| *q == p.id[idx]).unwrap();
            let ratio = p.velocity[idx].length() / p0.velocity[a].length();
            assert!(
                (ratio - (1.0 - x).powf(n)).abs() < 1e-9,
                "frame {frame}: exact scheme"
            );
            let continuous = (-k * frame as f64 / FPS).exp();
            assert!(
                (ratio / continuous - 1.0).abs() <= b.exp() - 1.0 + 1e-12,
                "frame {frame}: bound"
            );
            // direction untouched
            assert!((p.velocity[idx].normalize() - p0.velocity[a].normalize()).length() < 1e-9);
        }
    }
}

#[test]
fn wind_pulls_the_velocity_toward_the_wind_and_drifts_the_cloud() {
    // v_{n+1} = v_n + dt k (w - v_n) = (1 - x) v_n + x w, x = k dt: v_n = w + (v0 - w)(1 - x)^n.
    // The wind force is turned so it blows along -X (H = +90 degrees sends +Z to -X); the
    // particles start at rest. The continuous law w (1 - e^(-k t)) is within the same relative
    // bound e^B - 1 as the friction test, applied to |v0 - w|.
    let k = 1.5;
    let mut w = burst_world(100, &[("speed", f(0.0))]);
    let wid = add(
        &mut w,
        WIND,
        &[("speed", f(100.0)), ("air_resistance", f(k))],
        [0.0; 3],
    );
    set(
        &mut w,
        wid,
        "transform.rot",
        Value::Vec3([std::f64::consts::FRAC_PI_2, 0.0, 0.0]),
    );
    let x = k * DT;
    let wind = DVec3::new(-100.0, 0.0, 0.0);
    let mut last_x = 0.0;
    for frame in [10i64, 30, 90] {
        let (_, p) = states(&w, frame);
        let n = (4 * frame) as f64;
        let b = n * x * x / (2.0 * (1.0 - x));
        for v in &p.velocity {
            let expect = wind * (1.0 - (1.0 - x).powf(n));
            assert!((*v - expect).length() < 1e-9, "frame {frame}: exact scheme");
            let continuous = wind * (1.0 - (-k * frame as f64 / FPS).exp());
            assert!((*v - continuous).length() <= 100.0 * (b.exp() - 1.0) + 1e-9);
        }
        let mean_x = mean(&p.position.iter().map(|q| q.x).collect::<Vec<_>>());
        assert!(
            mean_x < last_x - 1.0,
            "the cloud drifts along the wind (-X)"
        );
        last_x = mean_x;
    }
}

#[test]
fn the_attractor_pulls_in_and_a_negative_strength_pushes_out() {
    let mean_distance = |p: &forge_particles::Particles, c: DVec3| {
        mean(
            &p.position
                .iter()
                .map(|q| (*q - c).length())
                .collect::<Vec<_>>(),
        )
    };
    let centre = DVec3::new(0.0, 0.0, 300.0);
    for (strength, shrinks) in [(500.0, true), (-500.0, false)] {
        let mut w = burst_world(
            500,
            &[
                ("speed", f(0.0)),
                ("size_x", f(100.0)),
                ("size_y", f(100.0)),
            ],
        );
        add(
            &mut w,
            ATTRACTOR,
            &[("strength", f(strength))],
            [0.0, 0.0, 300.0],
        );
        let mut c = cache(&w);
        let mut previous = mean_distance(c.at(0), centre);
        for frame in 1..=15 {
            let now = mean_distance(c.at(frame), centre);
            assert_eq!(
                now < previous,
                shrinks,
                "strength {strength} frame {frame}: {now} vs {previous}"
            );
            previous = now;
        }
    }
}

#[test]
fn the_attractor_law_is_constant_inside_the_radius_and_inverse_square_outside() {
    let f = Force::Attractor {
        center: DVec3::ZERO,
        strength: 500.0,
        radius: 100.0,
    };
    let a = |p: DVec3| f.acceleration(p, DVec3::ZERO, 0.0);
    assert!((a(DVec3::new(50.0, 0.0, 0.0)) - DVec3::new(-500.0, 0.0, 0.0)).length() < 1e-9);
    assert!((a(DVec3::new(0.0, 100.0, 0.0)) - DVec3::new(0.0, -500.0, 0.0)).length() < 1e-9);
    assert!((a(DVec3::new(0.0, 0.0, 200.0)) - DVec3::new(0.0, 0.0, -125.0)).length() < 1e-9);
    assert_eq!(a(DVec3::ZERO), DVec3::ZERO);
}

#[test]
fn the_deflector_response_is_the_written_reflection() {
    let d = |bounce, friction, kill| Force::Deflector {
        point: DVec3::ZERO,
        normal: DVec3::Y,
        bounce,
        friction,
        kill,
    };
    let (old, new, v) = (
        DVec3::new(0.0, 1.0, 0.0),
        DVec3::new(0.0, -1.0, 0.0),
        DVec3::new(3.0, -10.0, 0.0),
    );
    match d(0.5, 0.0, false).collide(old, new, v) {
        Some(Collision::Bounce { position, velocity }) => {
            assert!((position - DVec3::new(0.0, 0.5, 0.0)).length() < 1e-12);
            assert!((velocity - DVec3::new(3.0, 5.0, 0.0)).length() < 1e-12);
        }
        other => panic!("{other:?}"),
    }
    match d(0.5, 0.5, false).collide(old, new, v) {
        Some(Collision::Bounce { velocity, .. }) => {
            assert!((velocity - DVec3::new(1.5, 5.0, 0.0)).length() < 1e-12);
        }
        other => panic!("{other:?}"),
    }
    assert_eq!(
        d(0.5, 0.0, true).collide(old, new, v),
        Some(Collision::Kill)
    );
    // no crossing, or already below: nothing
    assert_eq!(
        d(0.5, 0.0, false).collide(old, DVec3::new(0.0, 0.5, 0.0), v),
        None
    );
    assert_eq!(d(0.5, 0.0, false).collide(new, new - DVec3::Y, v), None);
}

/// Particles born above y = 0 with an upward-ish cone, falling onto a deflector at y = 0.
fn rain(bounce: f64, kill: bool) -> World {
    let mut w = world(&[
        ("birthrate_editor", f(200.0)),
        ("start_frame", i(0)),
        ("stop_frame", i(10)),
        ("lifetime", f(100000.0)),
        ("seed", i(77)),
        ("speed", f(150.0)),
        ("speed_variation_percent", f(0.5)),
        ("angle_h", f(2.0)),
        ("angle_v", f(2.0)),
    ]);
    let e = w.emitter;
    set(&mut w, e, "transform.pos", Value::Vec3([0.0, 100.0, 0.0]));
    add(&mut w, GRAVITY, &[], [0.0; 3]);
    add(
        &mut w,
        DEFLECTOR,
        &[("bounce", f(bounce)), ("kill", Value::Bool(kill))],
        [0.0; 3],
    );
    w
}

#[test]
fn no_particle_crosses_the_deflector_plane() {
    for bounce in [0.0, 0.5, 1.0] {
        let w = rain(bounce, false);
        let mut c = cache(&w);
        let mut lowest = f64::MAX;
        for frame in 0..=90 {
            let p = c.at(frame);
            for q in &p.position {
                assert!(q.y >= -1e-9, "bounce {bounce} frame {frame}: y {}", q.y);
                lowest = lowest.min(q.y);
            }
        }
        assert!(c.at(90).len() == 2000);
        // the plane really was reached (so the test is not vacuous)
        assert!(lowest < 5.0, "bounce {bounce}: lowest {lowest}");
    }
}

#[test]
fn a_bouncing_particle_rebounds_and_a_dead_stick_one_does_not() {
    // After landing, the mean vertical speed of bounce 0.5 is positive for some particles;
    // with bounce 0 no particle ever moves upward after its first contact: it rests (v.y = 0
    // right after the collision, then gravity pulls it into the plane again every step).
    let up = |bounce: f64| {
        let w = rain(bounce, false);
        let mut c = cache(&w);
        let mut ups = 0;
        for frame in 30..=60 {
            let p = c.at(frame);
            ups += p
                .position
                .iter()
                .zip(&p.velocity)
                .filter(|(q, v)| q.y < 1.0 && v.y > 1.0)
                .count();
        }
        ups
    };
    assert!(up(0.5) > 0, "bounce 0.5: particles rebound off the plane");
    assert_eq!(up(0.0), 0, "bounce 0: nothing leaves the plane");
}

#[test]
fn a_killing_deflector_removes_the_particles_that_reach_it() {
    let w = rain(0.5, true);
    let mut c = cache(&w);
    let early = c.at(9).len();
    let late = c.at(90).len();
    assert!(
        late < early,
        "particles died on the plane: {early} -> {late}"
    );
    assert!(late < 2000);
    assert!(c.at(90).position.iter().all(|q| q.y >= 0.0));
    assert!(c.at(90).alive.iter().all(|a| *a));
}

#[test]
fn rotation_builds_angular_momentum_about_its_axis() {
    // Particles at rest on a 100 x 100 patch (x, y, 0). The force swirls about its Y axis
    // through the origin: the axial velocity v.Y stays exactly 0 (the acceleration has no axial
    // part), and L = mean((r_perp x v) . Y) rises every frame from 0.
    let mut w = burst_world(
        1000,
        &[
            ("speed", f(0.0)),
            ("size_x", f(100.0)),
            ("size_y", f(100.0)),
        ],
    );
    add(&mut w, ROTATION, &[("strength", f(200.0))], [0.0; 3]);
    let mut c = cache(&w);
    let l = |p: &forge_particles::Particles| {
        mean(
            &p.position
                .iter()
                .zip(&p.velocity)
                .map(|(r, v)| {
                    let perp = DVec3::new(r.x, 0.0, r.z);
                    perp.cross(*v).y
                })
                .collect::<Vec<_>>(),
        )
    };
    assert_eq!(l(c.at(0)), 0.0);
    let mut previous = 0.0;
    for frame in 1..=20 {
        let p = c.at(frame).clone();
        assert!(
            p.velocity.iter().all(|v| v.y.abs() < 1e-9),
            "axial velocity untouched"
        );
        let now = l(&p);
        assert!(
            now > previous,
            "frame {frame}: angular momentum {now} after {previous}"
        );
        previous = now;
    }
    // a negative strength turns the other way
    let mut w = burst_world(
        1000,
        &[
            ("speed", f(0.0)),
            ("size_x", f(100.0)),
            ("size_y", f(100.0)),
        ],
    );
    add(&mut w, ROTATION, &[("strength", f(-200.0))], [0.0; 3]);
    assert!(l(cache(&w).at(10)) < 0.0);
}

fn turbulence_world(seed: i64) -> World {
    // 2000 particles spread over 2000 x 2000 cm with noise scale 10 cm: a few per noise cell
    // among 40000 cells, so the samples are close to independent.
    let mut w = burst_world(
        2000,
        &[
            ("speed", f(0.0)),
            ("size_x", f(2000.0)),
            ("size_y", f(2000.0)),
        ],
    );
    add(
        &mut w,
        TURBULENCE,
        &[
            ("strength", f(200.0)),
            ("scale", f(10.0)),
            ("seed", i(seed)),
        ],
        [0.0; 3],
    );
    w
}

#[test]
fn turbulence_has_zero_mean_and_growing_variance() {
    let w = turbulence_world(5);
    let mut c = cache(&w);
    let mut previous = 0.0;
    for frame in [3i64, 6, 12, 24] {
        let p = c.at(frame).clone();
        let n = p.len() as f64;
        assert_eq!(p.len(), 2000);
        let mut var_sum = 0.0;
        for axis in 0..3 {
            let vs: Vec<f64> = p.velocity.iter().map(|v| v[axis]).collect();
            // mean unchanged (0): within 4 standard errors, the standard error taken from the
            // sample's own spread for independent samples: std / sqrt N
            let se = std_dev(&vs) / n.sqrt();
            assert!(
                mean(&vs).abs() < 4.0 * se,
                "frame {frame} axis {axis}: mean {} se {se}",
                mean(&vs)
            );
            var_sum += std_dev(&vs).powi(2);
        }
        assert!(
            var_sum > previous,
            "frame {frame}: variance {var_sum} after {previous}"
        );
        previous = var_sum;
    }
}

#[test]
fn turbulence_is_deterministic_for_a_seed() {
    let a = cache(&turbulence_world(5)).at(10).clone();
    let b = cache(&turbulence_world(5)).at(10).clone();
    let c = cache(&turbulence_world(6)).at(10).clone();
    assert_eq!(a, b);
    assert_ne!(a.velocity, c.velocity);
}

/// A row for a field list.
fn row(id: NodeId) -> Value {
    Value::LinkRows(vec![LinkRow::new(id, "normal")])
}

#[test]
fn a_field_limits_a_force_to_a_region() {
    // Particles at rest spread over x in [-1000, 1000]; gravity limited by a Spherical field of
    // radius 300 at the origin: outside the sphere the field is 0, so those particles keep
    // v = 0 exactly; inside they accelerate downward.
    let mut w = burst_world(
        1000,
        &[
            ("speed", f(0.0)),
            ("size_x", f(2000.0)),
            ("size_y", f(10.0)),
        ],
    );
    let field = add(
        &mut w,
        forge_fields::SPHERICAL,
        &[("radius", f(300.0))],
        [0.0; 3],
    );
    let g = add(&mut w, GRAVITY, &[], [0.0; 3]);
    set(&mut w, g, "fields", row(field));
    let (p0, p) = states(&w, 10);
    let (mut inside, mut outside) = (0, 0);
    for k in 0..p.len() {
        let a = p0.id.iter().position(|x| *x == p.id[k]).unwrap();
        let r = p0.position[a].length();
        if r > 320.0 {
            outside += 1;
            assert_eq!(
                p.velocity[k],
                DVec3::ZERO,
                "outside the field: untouched (r = {r})"
            );
        } else if r < 100.0 {
            inside += 1;
            assert!(
                p.velocity[k].y < -50.0,
                "inside: falling (r = {r}, vy = {})",
                p.velocity[k].y
            );
        }
    }
    assert!(
        inside > 20 && outside > 400,
        "both regions are populated: {inside}, {outside}"
    );
}

#[test]
fn a_force_that_lists_emitters_leaves_the_others_alone() {
    let mut w = burst_world(50, &[("speed", f(0.0))]);
    let a = w.emitter;
    let b = add(
        &mut w,
        forge_particles::EMITTER,
        &[
            ("birthrate_editor", f(50.0)),
            ("stop_frame", i(1)),
            ("speed", f(0.0)),
            ("lifetime", f(100000.0)),
        ],
        [500.0, 0.0, 0.0],
    );
    let listed = add(&mut w, GRAVITY, &[], [0.0; 3]);
    set(&mut w, listed, "emitters", Value::LinkList(vec![a]));
    let scene = w.doc.scene();
    let mut ca =
        forge_particles::ParticleCache::build(scene, a, forge_particles::ParticleMode::Editor)
            .unwrap();
    let mut cb =
        forge_particles::ParticleCache::build(scene, b, forge_particles::ParticleMode::Editor)
            .unwrap();
    assert!(
        ca.at(10).velocity.iter().all(|v| v.y < -100.0),
        "the listed emitter falls"
    );
    assert!(
        cb.at(10).velocity.iter().all(|v| *v == DVec3::ZERO),
        "the other one does not"
    );
    // unlisted (empty list): every emitter
    set(&mut w, listed, "emitters", Value::LinkList(vec![]));
    let scene = w.doc.scene();
    let mut cb =
        forge_particles::ParticleCache::build(scene, b, forge_particles::ParticleMode::Editor)
            .unwrap();
    assert!(
        cb.at(10).velocity.iter().all(|v| v.y < -100.0),
        "scene-wide by default"
    );
}

#[test]
fn a_disabled_force_does_nothing_and_the_forces_add_up() {
    let mut w = burst_world(20, &[("speed", f(0.0))]);
    let g1 = add(&mut w, GRAVITY, &[("acceleration", f(100.0))], [0.0; 3]);
    let g2 = add(&mut w, GRAVITY, &[("acceleration", f(300.0))], [0.0; 3]);
    let (_, p) = states(&w, 8);
    let t = 8.0 / FPS;
    assert!(
        p.velocity.iter().all(|v| (v.y + 400.0 * t).abs() < 1e-9),
        "accelerations add"
    );
    set(&mut w, g2, "object.enabled", Value::Bool(false));
    let (_, p) = states(&w, 8);
    assert!(
        p.velocity.iter().all(|v| (v.y + 100.0 * t).abs() < 1e-9),
        "disabled: ignored"
    );
    let _ = g1;
}

#[test]
fn forces_survive_a_file_round_trip() {
    let mut w = burst_world(100, &[("angle_h", f(1.0)), ("angle_v", f(1.0))]);
    let field = add(
        &mut w,
        forge_fields::SPHERICAL,
        &[("radius", f(400.0))],
        [0.0; 3],
    );
    let g = add(&mut w, GRAVITY, &[], [0.0; 3]);
    set(&mut w, g, "fields", row(field));
    let a = w.emitter;
    set(&mut w, g, "emitters", Value::LinkList(vec![a]));
    add(&mut w, TURBULENCE, &[("seed", i(3))], [0.0; 3]);
    add(
        &mut w,
        DEFLECTOR,
        &[("bounce", f(0.3)), ("kill", Value::Bool(false))],
        [0.0, -50.0, 0.0],
    );
    let text = forge_core::file::save(w.doc.scene());
    let (loaded, _) = forge_core::file::load(registry(), &text).unwrap();
    let run = |scene: &forge_core::Scene| {
        forge_particles::particles_at(scene, a, 20, forge_particles::ParticleMode::Editor).unwrap()
    };
    let before = run(w.doc.scene());
    assert!(!before.is_empty());
    assert_eq!(before, run(&loaded));
}
