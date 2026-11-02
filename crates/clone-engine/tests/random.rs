//! Unit tests of the Random effector beyond the golden cases (clone-toolset spec E11-E17):
//! moments of each mode, determinism, Indexed, Synchronized, ranges.

use forge_clone_engine::*;
use forge_core::EvalCtx;
use forge_fields::FieldList;
use glam::{DMat4, DVec3};

/// `n` clones on a lattice with 50 cm spacing (x fastest, 20 per row, 20 per plane).
fn lattice(n: usize) -> CloneData {
    let mut d = CloneData::default();
    for i in 0..n {
        let p = DVec3::new(
            (i % 20) as f64 * 50.0,
            ((i / 20) % 20) as f64 * 50.0,
            (i / 400) as f64 * 50.0,
        );
        d.push(CloneItem::at(DMat4::from_translation(p), DVec3::ZERO));
    }
    d
}

fn run_at(r: &Random, mut d: CloneData, time: f64) -> Result<CloneData, CloneError> {
    let eval = EvalCtx {
        time,
        ..EvalCtx::default()
    };
    let ctx = EffectorCtx {
        eval: &eval,
        cloner_world: DMat4::IDENTITY,
    };
    r.apply(&mut d, &FieldList::new(), &ctx)?;
    Ok(d)
}

fn run(r: &Random, d: CloneData) -> CloneData {
    run_at(r, d, 0.0).unwrap()
}

/// The position offsets divided by P (50): the values `v` of the position channel.
fn offsets(before: &CloneData, after: &CloneData) -> Vec<[f64; 3]> {
    (0..before.len())
        .map(|i| {
            let d = (after.matrices[i].w_axis - before.matrices[i].w_axis).truncate() / 50.0;
            d.to_array()
        })
        .collect()
}

fn moments(v: &[f64]) -> (f64, f64) {
    let n = v.len() as f64;
    let mean = v.iter().sum::<f64>() / n;
    (mean, v.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / n)
}

fn flat(v: &[[f64; 3]]) -> Vec<f64> {
    v.iter().flatten().copied().collect()
}

fn with_mode(mode: RandomMode) -> Random {
    Random {
        mode,
        ..Random::default()
    }
}

#[test]
fn the_random_type_is_registered_with_its_defaults() {
    let ty = random_type();
    assert_eq!(ty.category, forge_core::Category::Effector);
    let default = |id: &str| ty.find_param(id).unwrap().default.clone();
    assert_eq!(default("seed"), forge_core::Value::Int(12345));
    assert_eq!(default("minimum"), forge_core::Value::Float(-1.0));
    assert_eq!(default("maximum"), forge_core::Value::Float(1.0));
    assert_eq!(default("p"), forge_core::Value::Vec3([50.0; 3]));
    assert_eq!(default("position"), forge_core::Value::Bool(true));
    assert_eq!(default("color_mode"), forge_core::Value::Enum("off".into()));
    assert_eq!(
        default("random_mode"),
        forge_core::Value::Enum("random".into())
    );
    let mut reg = forge_core::Registry::with_builtins();
    reg.install(&CloneToolset).unwrap();
    assert!(reg.get(RANDOM).is_some());
    // The Plain and Random defaults stay different.
    let plain = plain_type();
    assert_eq!(
        plain.find_param("p").unwrap().default,
        forge_core::Value::Vec3([0.0, 100.0, 0.0])
    );
}

#[test]
fn random_mode_is_uniform_in_the_range() {
    let before = lattice(4000);
    let after = run(&Random::default(), before.clone());
    let v = flat(&offsets(&before, &after));
    let (mean, var) = moments(&v);
    assert!(v.iter().all(|x| (-1.0..1.0).contains(x)));
    assert!(mean.abs() < 0.03, "{mean}");
    assert!((var - 1.0 / 3.0).abs() < 0.02, "{var}");
}

#[test]
fn gaussian_mode_has_a_standard_deviation_of_one_sixth() {
    let before = lattice(4000);
    let after = run(&with_mode(RandomMode::Gaussian), before.clone());
    let v = flat(&offsets(&before, &after));
    let (mean, var) = moments(&v);
    assert!(v.iter().all(|x| (-1.0..=1.0).contains(x)));
    assert!(mean.abs() < 0.01, "{mean}");
    assert!((var - 1.0 / 36.0).abs() < 0.003, "{var}");
}

#[test]
fn noise_and_turbulence_stay_in_range_and_have_their_bias() {
    let before = lattice(4000);
    let noise = flat(&offsets(
        &before,
        &run(&with_mode(RandomMode::Noise), before.clone()),
    ));
    let (mean, var) = moments(&noise);
    assert!(noise.iter().all(|x| (-1.0..=1.0).contains(x)));
    assert!(mean.abs() < 0.05, "{mean}");
    assert!((0.03..0.15).contains(&var), "{var}");
    let turb = flat(&offsets(
        &before,
        &run(&with_mode(RandomMode::Turbulence), before.clone()),
    ));
    let (mean, _) = moments(&turb);
    assert!(turb.iter().all(|x| (-1.0..=1.0).contains(x)));
    assert!(
        mean < -0.3,
        "turbulence is biased low (NOISE.md N4): {mean}"
    );
}

/// Spec E13: clones 50 cm apart along Y; the observed root-mean-square step per channel is
/// 0.119, the feature size 300 cm gives about 0.115 (0.168 at 200, 0.087 at 400).
#[test]
fn the_feature_size_reproduces_the_observed_smoothness() {
    let mut sum = 0.0;
    let mut n = 0.0;
    for seed in 0..40 {
        let r = Random {
            seed,
            ..with_mode(RandomMode::Noise)
        };
        let mut d = CloneData::default();
        for i in 0..40 {
            let p = DVec3::new(
                13.3 * seed as f64,
                0.5 + 50.0 * i as f64,
                11.0 * seed as f64,
            );
            d.push(CloneItem::at(DMat4::from_translation(p), DVec3::ZERO));
        }
        let v = offsets(&d, &run(&r, d.clone()));
        for w in v.windows(2) {
            for (a, b) in w[0].iter().zip(&w[1]) {
                sum += (b - a).powi(2);
                n += 1.0;
            }
        }
    }
    let rms = (sum / n).sqrt();
    assert!((0.08..0.16).contains(&rms), "{rms}");
}

#[test]
fn the_same_seed_gives_the_same_values_and_another_seed_other_ones() {
    let before = lattice(100);
    let a = run(&Random::default(), before.clone());
    let b = run(&Random::default(), before.clone());
    assert_eq!(a, b);
    let c = run(
        &Random {
            seed: 7,
            ..Random::default()
        },
        before.clone(),
    );
    assert_ne!(a.matrices, c.matrices);
}

#[test]
fn indexed_values_follow_the_clone_index_not_the_list_position() {
    let before = lattice(10);
    let mut reversed = CloneData::default();
    for i in (0..10).rev() {
        reversed.push(CloneItem {
            matrix: before.matrices[i],
            ..CloneItem::at(before.matrices[i], DVec3::ZERO)
        });
    }
    // Give the reversed list the original indices.
    for (slot, index) in reversed.index.iter_mut().zip((0..10u32).rev()) {
        *slot = index;
    }
    let indexed = Random {
        indexed: true,
        ..Random::default()
    };
    let a = offsets(&before, &run(&indexed, before.clone()));
    let b = offsets(&reversed, &run(&indexed, reversed.clone()));
    for i in 0..10 {
        assert_eq!(a[i], b[9 - i], "clone {i}");
    }
    // Without Indexed the value follows the list position.
    let plain = Random::default();
    let a = offsets(&before, &run(&plain, before.clone()));
    let b = offsets(&reversed, &run(&plain, reversed.clone()));
    assert_ne!(a[0], b[9]);
    // A clone keeps its Indexed value when clones are added.
    let more = run(&indexed, lattice(20));
    let few = run(&indexed, lattice(10));
    assert_eq!(more.matrices[..10], few.matrices[..10]);
}

#[test]
fn synchronized_uses_one_value_for_all_channels() {
    let before = lattice(50);
    let r = Random {
        synchronized: true,
        ..Random::default()
    };
    for o in offsets(&before, &run(&r, before.clone())) {
        assert!(
            (o[0] - o[1]).abs() < 1e-12 && (o[1] - o[2]).abs() < 1e-12,
            "{o:?}"
        );
    }
    // Not synchronized: the axes differ.
    let o = offsets(&before, &run(&Random::default(), before.clone()));
    assert!(o.iter().any(|v| (v[0] - v[1]).abs() > 0.01));
}

#[test]
fn minimum_and_maximum_are_the_range_in_every_transform_mode() {
    let before = lattice(500);
    for mode in [
        TransformMode::Relative,
        TransformMode::Absolute,
        TransformMode::Remap,
    ] {
        let r = Random {
            base: Plain {
                minimum: 0.0,
                maximum: 0.5,
                mode,
                ..Random::default().base
            },
            ..Random::default()
        };
        let v = flat(&offsets(&before, &run(&r, before.clone())));
        assert!(v.iter().all(|x| (0.0..0.5).contains(x)), "{mode:?}");
        let (mean, _) = moments(&v);
        assert!((mean - 0.25).abs() < 0.02, "{mode:?}: {mean}");
    }
}

#[test]
fn switching_a_channel_does_not_change_the_others() {
    let before = lattice(50);
    let only_pos = run(&Random::default(), before.clone());
    let all = Random {
        base: Plain {
            rotation: Some(DVec3::splat(0.5)),
            scale: Some(DVec3::ONE),
            uniform_scale: true,
            weight: 1.0,
            ..Random::default().base
        },
        ..Random::default()
    };
    let more = run(&all, before.clone());
    for i in 0..50 {
        assert_eq!(only_pos.matrices[i].w_axis, more.matrices[i].w_axis);
    }
    // The rotation and the scale do change.
    assert!(
        more.matrices
            .iter()
            .any(|m| (m.x_axis.x - 1.0).abs() > 1e-3)
    );
    assert!(more.weights.iter().any(|w| w.abs() > 1e-3));
}

#[test]
fn uniform_scale_draws_once_and_the_axes_otherwise_differ() {
    let before = lattice(20);
    let scale_only = |uniform: bool| Random {
        base: Plain {
            position: None,
            scale: Some(DVec3::ONE),
            uniform_scale: uniform,
            ..Random::default().base
        },
        ..Random::default()
    };
    for m in &run(&scale_only(true), before.clone()).matrices {
        let (a, b, c) = (m.x_axis.length(), m.y_axis.length(), m.z_axis.length());
        assert!((a - b).abs() < 1e-12 && (b - c).abs() < 1e-12);
        assert!((0.0..2.0).contains(&a));
    }
    let m = &run(&scale_only(false), before).matrices;
    assert!(
        m.iter()
            .any(|m| (m.x_axis.length() - m.y_axis.length()).abs() > 0.01)
    );
}

#[test]
fn noise_moves_with_time_unless_the_speed_is_zero() {
    let before = lattice(30);
    let r = with_mode(RandomMode::Noise);
    assert_ne!(
        run_at(&r, before.clone(), 0.0).unwrap().matrices,
        run_at(&r, before.clone(), 1.0).unwrap().matrices
    );
    let still = Random {
        animation_speed: 0.0,
        ..r
    };
    assert_eq!(
        run_at(&still, before.clone(), 0.0).unwrap().matrices,
        run_at(&still, before, 5.0).unwrap().matrices
    );
    // The random mode does not animate.
    let b = lattice(30);
    assert_eq!(
        run_at(&Random::default(), b.clone(), 0.0).unwrap().matrices,
        run_at(&Random::default(), b, 2.0).unwrap().matrices
    );
}

#[test]
fn sorted_is_unsupported() {
    let eval = EvalCtx::default();
    let ctx = EffectorCtx {
        eval: &eval,
        cloner_world: DMat4::IDENTITY,
    };
    assert_eq!(
        with_mode(RandomMode::Sorted).apply(&mut lattice(3), &FieldList::new(), &ctx),
        Err(CloneError::Unsupported("Random Mode Sorted"))
    );
}

#[test]
fn strength_scales_the_values_and_zero_strength_does_nothing() {
    let before = lattice(100);
    let half = Random {
        base: Plain {
            strength: 0.5,
            ..Random::default().base
        },
        ..Random::default()
    };
    let full = offsets(&before, &run(&Random::default(), before.clone()));
    let h = offsets(&before, &run(&half, before.clone()));
    for (a, b) in full.iter().zip(&h) {
        for k in 0..3 {
            assert!((a[k] * 0.5 - b[k]).abs() < 1e-9);
        }
    }
    let zero = Random {
        base: Plain {
            strength: 0.0,
            ..Random::default().base
        },
        ..Random::default()
    };
    assert_eq!(run(&zero, before.clone()).matrices, before.matrices);
}
