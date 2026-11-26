//! The emitter (DYNAMICS.md section 9): exact counts, lifetimes, seeds, statistics, transform,
//! cache and file round trip. Own design (D20): statistics, not per-particle golden data.

mod common;

use common::*;
use forge_core::Value;
use forge_particles::{ParticleMode, births_in_frame, particles_at};

/// Integer oracle for a rate `num / den`: particles born in the first `m` emitting frames.
fn born_by(num: u64, den: u64, m: u64) -> u64 {
    num * m / den
}

#[test]
fn counts_per_frame_are_exact_for_several_rates() {
    // (rate, numerator, denominator): the oracle uses integers, so 0.1 and 33.3 carry no
    // binary rounding of their own.
    for (rate, num, den) in [
        (10.0, 10, 1),
        (2.5, 5, 2),
        (0.1, 1, 10),
        (0.7, 7, 10),
        (33.3, 333, 10),
        (1.0, 1, 1),
    ] {
        let w = world(&[
            ("birthrate_editor", f(rate)),
            ("lifetime", f(1000.0)),
            ("start_frame", i(0)),
            ("stop_frame", i(40)),
        ]);
        let mut c = cache(&w);
        let mut previous = 0usize;
        for frame in 0..60i64 {
            let alive = c.at(frame).len();
            let m = (frame + 1).min(40) as u64;
            assert_eq!(
                alive as u64,
                born_by(num, den, m),
                "rate {rate} frame {frame}: total"
            );
            let born_now = alive - previous;
            let expect = if frame < 40 {
                born_by(num, den, frame as u64 + 1) - born_by(num, den, frame as u64)
            } else {
                0
            };
            assert_eq!(born_now as u64, expect, "rate {rate} frame {frame}: births");
            previous = alive;
        }
    }
}

#[test]
fn the_birth_function_follows_the_written_rule() {
    // Rate 2.5 gives 2, 3, 2, 3 and the ids of one frame continue the previous one.
    let seq: Vec<(u64, u64)> = (0..4).map(|f| births_in_frame(2.5, 0, 100, f)).collect();
    assert_eq!(seq, vec![(0, 2), (2, 3), (5, 2), (7, 3)]);
    // Before the start and from the stop frame on: nothing; rate <= 0 never emits.
    assert_eq!(births_in_frame(10.0, 5, 8, 4).1, 0);
    assert_eq!(births_in_frame(10.0, 5, 8, 7).1, 10);
    assert_eq!(births_in_frame(10.0, 5, 8, 8).1, 0);
    assert_eq!(births_in_frame(0.0, 0, 8, 3).1, 0);
    assert_eq!(births_in_frame(-3.0, 0, 8, 3).1, 0);
}

#[test]
fn start_and_stop_frames_bound_the_emission() {
    let w = world(&[
        ("birthrate_editor", f(4.0)),
        ("lifetime", f(1000.0)),
        ("start_frame", i(5)),
        ("stop_frame", i(8)),
    ]);
    let mut c = cache(&w);
    assert_eq!(c.at(4).len(), 0, "nothing before the start");
    assert_eq!(c.at(5).len(), 4);
    assert_eq!(c.at(7).len(), 12);
    assert_eq!(c.at(8).len(), 12, "the stop frame does not emit");
    assert_eq!(c.at(50).len(), 12);
}

#[test]
fn the_editor_and_the_renderer_rate_apply_by_mode() {
    let w = world(&[
        ("birthrate_editor", f(3.0)),
        ("birthrate_renderer", f(7.0)),
        ("lifetime", f(1000.0)),
    ]);
    let ed = particles_at(w.doc.scene(), w.emitter, 9, ParticleMode::Editor).unwrap();
    let re = particles_at(w.doc.scene(), w.emitter, 9, ParticleMode::Render).unwrap();
    assert_eq!(ed.len(), 30);
    assert_eq!(re.len(), 70);
}

#[test]
fn alive_counts_follow_the_lifetime() {
    // 5 per frame, 1 s at 30 fps: a particle lives frames 0..=29 after its birth (age < 1 s),
    // so the steady state is 5 * 30 = 150 and after the stop frame 150 the cloud empties.
    let w = world(&[
        ("birthrate_editor", f(5.0)),
        ("lifetime", f(1.0)),
        ("start_frame", i(0)),
        ("stop_frame", i(150)),
    ]);
    let mut c = cache(&w);
    assert_eq!(c.at(0).len(), 5);
    assert_eq!(c.at(28).len(), 5 * 29);
    assert_eq!(c.at(29).len(), 150);
    assert_eq!(
        c.at(30).len(),
        150,
        "the first batch died as the next was born"
    );
    assert_eq!(c.at(100).len(), 150);
    assert_eq!(c.at(149).len(), 150);
    assert_eq!(c.at(150).len(), 145, "no birth in frame 150");
    assert_eq!(c.at(178).len(), 5);
    assert_eq!(c.at(179).len(), 0);
    // every live particle has age < lifetime and the ages are whole frames
    let p = c.at(100).clone();
    assert!(p.alive.iter().all(|a| *a));
    for k in 0..p.len() {
        assert!(p.age[k] < p.lifetime[k]);
        let frames = p.age[k] * 30.0;
        assert!((frames - frames.round()).abs() < 1e-9);
    }
}

#[test]
fn the_same_seed_is_identical_and_another_differs() {
    let params = |seed: i64| {
        [
            ("seed", i(seed)),
            ("speed_variation_percent", f(0.5)),
            ("lifetime_variation_percent", f(0.5)),
            ("angle_h", f(1.0)),
            ("angle_v", f(1.0)),
            ("birthrate_editor", f(20.0)),
        ]
    };
    let wa = world(&params(7));
    let wb = world(&params(7));
    let wc = world(&params(8));
    let pa = particles_at(wa.doc.scene(), wa.emitter, 25, ParticleMode::Editor).unwrap();
    let pb = particles_at(wb.doc.scene(), wb.emitter, 25, ParticleMode::Editor).unwrap();
    let pc = particles_at(wc.doc.scene(), wc.emitter, 25, ParticleMode::Editor).unwrap();
    assert_eq!(pa, pb, "same seed: bit-identical");
    assert_eq!(pa.len(), pc.len(), "the seed does not change counts");
    assert_ne!(pa.position, pc.position, "another seed: other particles");
    assert_ne!(pa.velocity, pc.velocity);
}

#[test]
fn a_particle_does_not_depend_on_the_birthrate() {
    // Particle n has its own stream (section 9.4): particle 3 is the same at rate 1 and rate 5.
    let base = [
        ("speed_variation_percent", f(0.5)),
        ("angle_h", f(1.0)),
        ("angle_v", f(1.0)),
    ];
    let mut a = base.to_vec();
    a.push(("birthrate_editor", f(1.0)));
    let mut b = base.to_vec();
    b.push(("birthrate_editor", f(5.0)));
    let (wa, wb) = (world(&a), world(&b));
    // particle 3 is born in frame 3 at rate 1 and in frame 0 at rate 5: compare birth velocities
    let pa = particles_at(wa.doc.scene(), wa.emitter, 3, ParticleMode::Editor).unwrap();
    let pb = particles_at(wb.doc.scene(), wb.emitter, 0, ParticleMode::Editor).unwrap();
    let ia = pa.id.iter().position(|x| *x == 3).unwrap();
    let ib = pb.id.iter().position(|x| *x == 3).unwrap();
    assert_eq!(pa.velocity[ia], pb.velocity[ib]);
    assert_eq!(pa.position[ia], pb.position[ib]);
}

/// Born-this-frame velocities of a 3000-particle burst.
fn burst(params: &[(&str, Value)]) -> forge_particles::Particles {
    let mut all = vec![
        ("birthrate_editor", f(3000.0)),
        ("start_frame", i(0)),
        ("stop_frame", i(1)),
        ("lifetime", f(1000.0)),
        ("seed", i(12345)),
    ];
    all.extend(params.iter().cloned());
    let w = world(&all);
    particles_at(w.doc.scene(), w.emitter, 0, ParticleMode::Editor).unwrap()
}

const N: f64 = 3000.0;

#[test]
fn speed_and_lifetime_distributions_match_the_parameters() {
    // x = base (1 + v u), u uniform in [-1, 1): mean base, std base v / sqrt 3. With N = 3000
    // the mean has standard error std / sqrt N; the sample std has standard error
    // std sqrt((kurtosis - 1) / (4 N)) = std sqrt(0.2 / N) (uniform kurtosis 1.8).
    // The tolerance is 4 standard errors of the estimator.
    let p = burst(&[
        ("speed", f(150.0)),
        ("speed_variation_percent", f(0.5)),
        ("lifetime", f(3.0)),
        ("lifetime_variation_percent", f(0.4)),
    ]);
    assert_eq!(p.len(), 3000);
    let speeds: Vec<f64> = p.velocity.iter().map(|v| v.length()).collect();
    for (name, xs, base, var) in [
        ("speed", speeds, 150.0, 0.5),
        ("lifetime", p.lifetime.clone(), 3.0, 0.4),
    ] {
        let std = base * var / 3f64.sqrt();
        let se_mean = std / N.sqrt();
        let se_std = std * (0.2 / N).sqrt();
        assert!(
            (mean(&xs) - base).abs() < 4.0 * se_mean,
            "{name} mean {}",
            mean(&xs)
        );
        assert!(
            (std_dev(&xs) - std).abs() < 4.0 * se_std,
            "{name} std {}",
            std_dev(&xs)
        );
        let (lo, hi) = (base * (1.0 - var), base * (1.0 + var));
        assert!(
            xs.iter().all(|x| *x >= lo && *x < hi),
            "{name} stays in [lo, hi)"
        );
    }
}

#[test]
fn no_variation_gives_exact_values() {
    let p = burst(&[("speed", f(150.0)), ("lifetime", f(3.0))]);
    assert!(p.velocity.iter().all(|v| (v.length() - 150.0).abs() < 1e-9));
    assert!(p.lifetime.iter().all(|l| *l == 3.0));
    // angles 0: every particle along +Z
    assert!(
        p.velocity
            .iter()
            .all(|v| v.x.abs() < 1e-9 && v.y.abs() < 1e-9)
    );
}

#[test]
fn direction_spread_matches_the_cone_and_the_pyramid() {
    let (ah, av) = (60f64.to_radians(), 40f64.to_radians());
    let (th, tv) = ((ah / 2.0).tan(), (av / 2.0).tan());
    // vx / vz = u tan(ah / 2) exactly. Angular: (u, v) uniform in the unit disc: E[u^2] = 1/4,
    // E[u^4] = 1/8, so the mean of r = vx/vz has standard error th / (2 sqrt N) and the mean
    // of r^2 (expected th^2 / 4) has standard error th^2 sqrt(1/8 - 1/16) / sqrt N = th^2 / (4 sqrt N).
    // Pyramid: u uniform in [-1, 1]: E[u^2] = 1/3, E[u^4] = 1/5: the mean of r has standard
    // error th / sqrt(3 N), the mean of r^2 (expected th^2 / 3) th^2 sqrt(4/45) / sqrt N.
    // Tolerance: 4 standard errors, N = 3000.
    for pyramid in [false, true] {
        let p = burst(&[
            ("angle_h", f(ah)),
            ("angle_v", f(av)),
            (
                "emitter_type",
                e(if pyramid { "pyramid" } else { "angular" }),
            ),
        ]);
        for (axis, t) in [(0usize, th), (1usize, tv)] {
            let r: Vec<f64> = p.velocity.iter().map(|v| v[axis] / v.z).collect();
            let r2: Vec<f64> = r.iter().map(|x| x * x).collect();
            let (sd_r, e2, se2) = if pyramid {
                (t / 3f64.sqrt(), t * t / 3.0, t * t * (4.0f64 / 45.0).sqrt())
            } else {
                (t / 2.0, t * t / 4.0, t * t / 4.0)
            };
            assert!(
                mean(&r).abs() < 4.0 * sd_r / N.sqrt(),
                "pyramid {pyramid} axis {axis} mean"
            );
            assert!(
                (mean(&r2) - e2).abs() < 4.0 * se2 / N.sqrt(),
                "pyramid {pyramid} axis {axis} second moment {} vs {e2}",
                mean(&r2)
            );
            let bound = t * 1.0000001;
            assert!(
                r.iter().all(|x| x.abs() <= bound),
                "inside the cone / pyramid"
            );
        }
    }
}

#[test]
fn birth_positions_fill_the_area() {
    // Rectangle 80 x 40: x uniform in [-40, 40]: E[x^2] = a^2 / 3 = size^2 / 12, E[x^4] = a^4 / 5,
    // standard error of the mean of x^2: a^2 sqrt(4/45) / sqrt N. Ellipse: x = a u with u in the
    // unit disc: E[x^2] = a^2 / 4 = size^2 / 16, E[x^4] = a^4 / 8, standard error a^2 / (4 sqrt N).
    // Tolerance: 4 standard errors of the second moment, and 4 of the mean (std / sqrt N).
    for circle in [false, true] {
        let p = burst(&[
            ("shape", e(if circle { "circle" } else { "rectangle" })),
            ("size_x", f(80.0)),
            ("size_y", f(40.0)),
        ]);
        for (axis, a) in [(0usize, 40.0f64), (1usize, 20.0f64)] {
            let xs: Vec<f64> = p.position.iter().map(|v| v[axis]).collect();
            let (m2, se2, sd) = if circle {
                (a * a / 4.0, a * a / 4.0, a / 2.0)
            } else {
                (a * a / 3.0, a * a * (4.0f64 / 45.0).sqrt(), a / 3f64.sqrt())
            };
            let got = xs.iter().map(|x| x * x).sum::<f64>() / N;
            assert!(
                (got - m2).abs() < 4.0 * se2 / N.sqrt(),
                "circle {circle} axis {axis}: {got} vs {m2}"
            );
            assert!(
                mean(&xs).abs() < 4.0 * sd / N.sqrt(),
                "circle {circle} axis {axis} mean"
            );
        }
        assert!(p.position.iter().all(|v| v.z == 0.0));
        let half = [40.0, 20.0];
        assert!(p.position.iter().all(|v| if circle {
            (v.x / half[0]).powi(2) + (v.y / half[1]).powi(2) <= 1.0 + 1e-12
        } else {
            v.x.abs() <= half[0] && v.y.abs() <= half[1]
        }));
    }
}

#[test]
fn rotation_and_scale_follow_age() {
    // Spin rate: base 90 deg/s, variation 0: the angle is rate * age; the end scale 3 with no
    // variation gives scale 1 + 2 age / lifetime until the end of life.
    let w = world(&[
        ("birthrate_editor", f(1.0)),
        ("lifetime", f(2.0)),
        ("rotation", f(90f64.to_radians())),
        ("rotational_variation_percent", f(0.0)),
        ("end_scale", f(3.0)),
    ]);
    let mut c = cache(&w);
    let p = c.at(30).clone();
    for k in 0..p.len() {
        assert!((p.rotation[k] - 90f64.to_radians() * p.age[k]).abs() < 1e-12);
        assert!((p.scale[k] - (1.0 + 2.0 * p.age[k] / p.lifetime[k])).abs() < 1e-12);
        assert!((p.spin_axis[k].length() - 1.0).abs() < 1e-12);
    }
}

#[test]
fn the_emitter_transform_places_the_births() {
    let mut w = world(&[
        ("birthrate_editor", f(500.0)),
        ("size_x", f(0.0)),
        ("size_y", f(0.0)),
        ("lifetime", f(1000.0)),
    ]);
    let id = w.emitter;
    set(
        &mut w,
        id,
        "transform.pos",
        Value::Vec3([100.0, 200.0, 300.0]),
    );
    // rotate 90 degrees by the H angle: the document's rotation convention sends +Z to -X
    set(
        &mut w,
        id,
        "transform.rot",
        Value::Vec3([std::f64::consts::FRAC_PI_2, 0.0, 0.0]),
    );
    let mut c = cache(&w);
    let p = c.at(0).clone();
    assert_eq!(p.len(), 500);
    for k in 0..p.len() {
        assert!((p.position[k] - glam::DVec3::new(100.0, 200.0, 300.0)).length() < 1e-9);
        let dir = p.velocity[k].normalize();
        assert!(
            (dir - glam::DVec3::NEG_X).length() < 1e-9,
            "direction {dir:?}"
        );
        assert!((p.velocity[k].length() - 150.0).abs() < 1e-9);
    }
    // and the particles keep moving along it: frame 30 is one second on
    let q = c.at(30).clone();
    let k = q.id.iter().position(|x| *x == 0).unwrap();
    assert!((q.position[k].x - (100.0 - 150.0)).abs() < 1e-6);
}

#[test]
fn a_scaled_emitter_scales_the_area_but_not_the_speed() {
    let mut w = world(&[
        ("birthrate_editor", f(300.0)),
        ("size_x", f(10.0)),
        ("size_y", f(10.0)),
        ("lifetime", f(1000.0)),
    ]);
    let id = w.emitter;
    set(&mut w, id, "transform.scale", Value::Vec3([4.0, 4.0, 4.0]));
    let p = cache(&w).at(0).clone();
    assert!(
        p.position.iter().any(|v| v.x.abs() > 10.0),
        "area scaled by 4: up to +-20"
    );
    assert!(p.position.iter().all(|v| v.x.abs() <= 20.0 + 1e-9));
    assert!(p.velocity.iter().all(|v| (v.length() - 150.0).abs() < 1e-9));
}

#[test]
fn scrubbing_gives_the_same_frames() {
    let w = world(&[
        ("birthrate_editor", f(7.5)),
        ("lifetime", f(0.5)),
        ("speed_variation_percent", f(0.3)),
        ("angle_h", f(1.0)),
        ("angle_v", f(1.0)),
    ]);
    let mut c = cache(&w);
    let f50 = c.at(50).clone();
    assert_eq!(c.cached_frames(), 51);
    let f10 = c.at(10).clone();
    assert_eq!(c.cached_frames(), 51, "going back simulates nothing");
    assert_eq!(c.at(50), &f50);
    let mut straight = cache(&w);
    assert_eq!(straight.at(10), &f10);
    assert_eq!(straight.at(50), &f50);
    let pure = particles_at(w.doc.scene(), w.emitter, 10, ParticleMode::Editor).unwrap();
    assert_eq!(pure, f10);
    assert!(c.at(-3).is_empty(), "before the start: no particles");
}

#[test]
fn an_animated_emitter_leaves_a_trail() {
    use forge_core::{Interp, Key};
    // The emitter moves 300 cm along x in one second (30 frames), size 0: the particle born in
    // frame f is born at x = 10 f (linear key), then moves with speed 150 along +Z only.
    let mut w = world(&[
        ("birthrate_editor", f(1.0)),
        ("size_x", f(0.0)),
        ("size_y", f(0.0)),
        ("lifetime", f(1000.0)),
    ]);
    let id = w.emitter;
    w.doc
        .transact("key", |tx| {
            tx.set_key(
                id,
                "transform.pos.x",
                Key::new(0.0, 0.0).with_interp(Interp::Linear),
            )?;
            tx.set_key(
                id,
                "transform.pos.x",
                Key::new(1.0, 300.0).with_interp(Interp::Linear),
            )
        })
        .unwrap();
    let mut c = cache(&w);
    let p = c.at(20).clone();
    assert_eq!(p.len(), 21);
    for k in 0..p.len() {
        let born = p.birth_frame[k] as f64;
        // scene_at rounds the time to a millisecond: at most 0.5 ms * 300 cm/s = 0.15 cm off.
        assert!(
            (p.position[k].x - 10.0 * born).abs() < 0.2,
            "x {} born {born}",
            p.position[k].x
        );
    }
}

#[test]
fn the_emitter_survives_a_file_round_trip() {
    let mut w = world(&[
        ("birthrate_editor", f(6.5)),
        ("seed", i(99)),
        ("lifetime", f(2.0)),
        ("shape", e("circle")),
        ("emitter_type", e("pyramid")),
        ("angle_h", f(0.7)),
        ("speed_variation_percent", f(0.25)),
    ]);
    let id = w.emitter;
    set(&mut w, id, "transform.pos", Value::Vec3([1.0, 2.0, 3.0]));
    let text = forge_core::file::save(w.doc.scene());
    let (loaded, _) = forge_core::file::load(registry(), &text).unwrap();
    let a = particles_at(w.doc.scene(), id, 20, ParticleMode::Editor).unwrap();
    let b = particles_at(&loaded, id, 20, ParticleMode::Editor).unwrap();
    assert!(!a.is_empty());
    assert_eq!(a, b);
}
