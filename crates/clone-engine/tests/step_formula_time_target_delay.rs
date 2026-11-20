//! Unit tests of the Step, Formula, Time, Target and Delay effectors beyond the golden cases
//! (clone-toolset spec E18-E23).

use forge_clone_engine::*;
use forge_core::{EvalCtx, Value};
use forge_fields::FieldList;
use glam::{DMat4, DVec3};

/// `n` clones along y at 0, 50, 100, ...
fn column(n: usize) -> CloneData {
    let mut d = CloneData::default();
    for i in 0..n {
        d.push(CloneItem::at(
            DMat4::from_translation(DVec3::new(0.0, 50.0 * i as f64, 0.0)),
            DVec3::ZERO,
        ));
    }
    d
}

fn run(e: &dyn Effector, mut d: CloneData, time: f64) -> Result<CloneData, CloneError> {
    let eval = EvalCtx {
        time,
        fps: 30.0,
        seed: 0,
    };
    let ctx = EffectorCtx {
        eval: &eval,
        cloner_world: DMat4::IDENTITY,
    };
    e.apply(&mut d, &FieldList::new(), &ctx)?;
    Ok(d)
}

fn scale_of(d: &CloneData, i: usize) -> f64 {
    d.matrices[i].x_axis.truncate().length()
}

fn pos(d: &CloneData, i: usize) -> DVec3 {
    d.matrices[i].w_axis.truncate()
}

#[test]
fn the_new_types_are_registered_with_their_defaults() {
    let mut reg = forge_core::Registry::with_builtins();
    reg.install(&CloneToolset).unwrap();
    for id in [STEP, FORMULA, TIME, TARGET, DELAY] {
        let ty = reg.get(id).unwrap();
        assert_eq!(ty.category, forge_core::Category::Effector, "{id}");
    }
    let default = |ty: &forge_core::NodeType, id: &str| ty.find_param(id).unwrap().default.clone();
    let step = step_type();
    assert_eq!(default(&step, "position"), Value::Bool(false));
    assert_eq!(default(&step, "scale"), Value::Bool(true));
    assert_eq!(default(&step, "scale_value"), Value::Float(1.0));
    assert_eq!(default(&step, "color_mode"), Value::Enum("off".into()));
    let time = time_type();
    assert_eq!(default(&time, "rotation"), Value::Bool(true));
    assert_eq!(
        default(&time, "r"),
        Value::Vec3([std::f64::consts::FRAC_PI_2, 0.0, 0.0])
    );
    let formula = formula_type();
    assert_eq!(default(&formula, "p"), Value::Vec3([50.0, 0.0, 0.0]));
    assert_eq!(
        default(&formula, "formula"),
        Value::String(DEFAULT_FORMULA.into())
    );
    assert_eq!(default(&delay_type(), "strength"), Value::Float(0.5));
    assert_eq!(
        default(&target_type(), "target_mode"),
        Value::Enum("object_target".into())
    );
    // Plain keeps its own defaults.
    assert_eq!(default(&plain_type(), "position"), Value::Bool(true));
    assert_eq!(default(&plain_type(), "scale"), Value::Bool(false));
}

#[test]
fn step_curves_end_at_0_and_1_and_ease_differently() {
    for c in [
        StepCurve::Linear,
        StepCurve::EaseIn,
        StepCurve::EaseOut,
        StepCurve::EaseInOut,
    ] {
        assert!(c.at(0.0).abs() < 1e-12 && (c.at(1.0) - 1.0).abs() < 1e-12);
    }
    assert!(StepCurve::EaseIn.at(0.5) < 0.5);
    assert!(StepCurve::EaseOut.at(0.5) > 0.5);
    assert!((StepCurve::EaseInOut.at(0.5) - 0.5).abs() < 1e-12);
    let eased = Step {
        curve: StepCurve::EaseIn,
        ..Step::default()
    };
    let d = run(&eased, column(5), 0.0).unwrap();
    assert!((scale_of(&d, 2) - 1.25).abs() < 1e-9);
}

#[test]
fn step_with_one_clone_gets_weight_zero_and_an_unsupported_flag_errors() {
    let d = run(&Step::default(), column(1), 0.0).unwrap();
    assert!((scale_of(&d, 0) - 1.0).abs() < 1e-12);
    let mut gap = Step::default();
    gap.base.unsupported = Some("Step Gap other than 0");
    assert!(matches!(
        run(&gap, column(3), 0.0),
        Err(CloneError::Unsupported(_))
    ));
}

/// Step Gap (spec E19): runs of `gap + 1` clones share the weight of the run's first clone,
/// the weight still measured against the last clone (`step-gap-1`, `step-gap-2`).
#[test]
fn step_gap_holds_the_weight_over_runs() {
    let weights = |gap: usize| {
        let step = Step {
            gap,
            ..Step::default()
        };
        let d = run(&step, column(5), 0.0).unwrap();
        (0..5).map(|i| scale_of(&d, i) - 1.0).collect::<Vec<_>>()
    };
    let close = |a: Vec<f64>, b: [f64; 5]| a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1e-9);
    assert!(close(weights(0), [0.0, 0.25, 0.5, 0.75, 1.0]));
    assert!(close(weights(1), [0.0, 0.0, 0.5, 0.5, 1.0]));
    assert!(close(weights(2), [0.0, 0.0, 0.0, 0.75, 0.75]));
}

#[test]
fn formula_reads_its_variables_and_treats_bad_values_as_zero() {
    let f = Formula {
        formula: "id + count * 10".into(),
        ..Formula::default()
    };
    let d = run(&f, column(3), 0.0).unwrap();
    // weight = 30, 31, 32: scale 1 + w, x = 50 w.
    assert!((scale_of(&d, 1) - 32.0).abs() < 1e-9);
    assert!((pos(&d, 2).x - 50.0 * 32.0).abs() < 1e-9);
    // py is the clone height; t and f scale the sine (degrees).
    let f = Formula {
        formula: "py / 50 + t * f".into(),
        t: 2.0,
        f: 3.0,
        ..Formula::default()
    };
    let d = run(&f, column(3), 0.5).unwrap();
    assert!((scale_of(&d, 2) - (1.0 + 2.0 + 3.0)).abs() < 1e-9);
    // 1 / 0 is not finite: no change.
    let f = Formula {
        formula: "1 / 0".into(),
        ..Formula::default()
    };
    let d = run(&f, column(2), 0.0).unwrap();
    assert!((scale_of(&d, 1) - 1.0).abs() < 1e-12 && pos(&d, 1).x == 0.0);
    let bad = Formula {
        formula: "sin(".into(),
        ..Formula::default()
    };
    assert!(matches!(
        run(&bad, column(2), 0.0),
        Err(CloneError::Unsupported(_))
    ));
}

#[test]
fn time_scales_the_effect_with_the_seconds() {
    let t = Time::default();
    let quarter = run(&t, column(1), 0.5).unwrap();
    let full = run(&t, column(1), 1.0).unwrap();
    let still = run(&t, column(1), 0.0).unwrap();
    // R = (90, 0, 0) degrees per second about heading.
    let x = |d: &CloneData| d.matrices[0].x_axis.truncate();
    assert!((x(&still) - DVec3::X).length() < 1e-12);
    assert!((x(&full) - DVec3::Z).length() < 1e-9);
    assert!((x(&quarter) - DVec3::new(0.5f64.sqrt(), 0.0, 0.5f64.sqrt())).length() < 1e-9);
}

fn target_at(p: DVec3) -> Target {
    Target {
        target: Some(p),
        ..Target::default()
    }
}

#[test]
fn target_aims_the_z_axis_and_keeps_x_horizontal() {
    let d = run(&target_at(DVec3::new(300.0, 0.0, 0.0)), column(2), 0.0).unwrap();
    // From the origin towards +x: z = +x, x = Y × z = (0, 0, -1).
    let m = d.matrices[0];
    assert!((m.z_axis.truncate() - DVec3::X).length() < 1e-9);
    assert!((m.x_axis.truncate() - DVec3::NEG_Z).length() < 1e-9);
    assert!((m.y_axis.truncate() - DVec3::Y).length() < 1e-9);
    // Positions and scale are kept.
    assert!((pos(&d, 1) - DVec3::new(0.0, 50.0, 0.0)).length() < 1e-12);
}

#[test]
fn target_without_pitch_stays_level_and_strength_blends() {
    let level = Target {
        use_pitch: false,
        ..target_at(DVec3::new(0.0, -300.0, 300.0))
    };
    let d = run(&level, column(2), 0.0).unwrap();
    assert!((d.matrices[1].z_axis.truncate() - DVec3::Z).length() < 1e-9);
    let half = Target {
        strength: 0.5,
        ..target_at(DVec3::new(300.0, 0.0, 0.0))
    };
    let d = run(&half, column(1), 0.0).unwrap();
    let want = DVec3::new(1.0, 0.0, 1.0).normalize();
    assert!((d.matrices[0].z_axis.truncate() - want).length() < 1e-9);
}

#[test]
fn target_next_node_and_the_unsupported_options() {
    let next = Target {
        mode: TargetMode::NextNode,
        ..Target::default()
    };
    let d = run(&next, column(3), 0.0).unwrap();
    // The clones are stacked along y and the aim is straight up: the up axis is parallel,
    // so x falls back to the clone's own x axis and z = +y.
    for i in 0..3 {
        assert!(
            (d.matrices[i].z_axis.truncate() - DVec3::Y).length() < 1e-9,
            "{i}"
        );
    }
    for unsupported in [
        Target {
            mode: TargetMode::LookAtCamera,
            unsupported: Some("Target Mode Look At Camera"),
            ..Target::default()
        },
        Target {
            unsupported: Some("Target Repel"),
            ..Target::default()
        },
    ] {
        assert!(matches!(
            run(&unsupported, column(2), 0.0),
            Err(CloneError::Unsupported(_))
        ));
    }
    // No Target Object: nothing happens.
    let d = run(&Target::default(), column(2), 0.0).unwrap();
    assert_eq!(d, column(2));
}

fn moving(x: f64) -> CloneData {
    let mut d = CloneData::default();
    d.push(CloneItem::at(
        DMat4::from_translation(DVec3::new(x, 0.0, 0.0)),
        DVec3::ZERO,
    ));
    d
}

fn smooth(delay: &Delay, inputs: &[f64]) -> Vec<f64> {
    let eval = EvalCtx::default();
    let ctx = EffectorCtx {
        eval: &eval,
        cloner_world: DMat4::IDENTITY,
    };
    let mut out = Vec::new();
    for k in 0..inputs.len() {
        let history: Vec<CloneData> = inputs[..k].iter().map(|x| moving(*x)).collect();
        let mut d = moving(inputs[k]);
        delay
            .smooth(&history, &mut d, &FieldList::new(), &ctx)
            .unwrap();
        out.push(pos(&d, 0).x);
    }
    out
}

#[test]
fn delay_is_the_identity_without_history_and_at_strength_zero() {
    let d = run(&Delay::default(), moving(7.0), 0.0).unwrap();
    assert_eq!(d, moving(7.0));
    let none = Delay {
        strength: 0.0,
        ..Delay::default()
    };
    assert_eq!(smooth(&none, &[0.0, 10.0, 20.0]), vec![0.0, 10.0, 20.0]);
}

#[test]
fn delay_modes_follow_a_ramp_differently() {
    let ramp = [0.0, 10.0, 20.0, 30.0];
    // The persistence is 0.999 * strength (`cloner/delay-blend-f1`).
    let p = 0.5 * 0.999;
    let even = smooth(&Delay::default(), &ramp);
    assert_eq!(even[0], 0.0);
    let mut y = 0.0;
    for (k, x) in ramp.iter().enumerate().skip(1) {
        y += (x - y) * (1.0 - p);
        assert!((even[k] - y).abs() < 1e-9, "even {k}");
    }
    // Blend is the observed exponential smoothing: the same numbers as Even.
    let blend = smooth(
        &Delay {
            mode: DelayMode::Blend,
            ..Delay::default()
        },
        &ramp,
    );
    assert_eq!(blend, even);
    let spring = smooth(
        &Delay {
            mode: DelayMode::Spring,
            ..Delay::default()
        },
        &ramp,
    );
    let (mut y, mut v) = (0.0, 0.0);
    for (k, x) in ramp.iter().enumerate().skip(1) {
        v = v * p + (x - y) * (1.0 - p);
        y += v;
        assert!((spring[k] - y).abs() < 1e-9, "spring {k}");
    }
    assert!(spring[3] > even[3]);
}

#[test]
fn delay_channels_can_be_switched_off_and_history_of_another_size_is_ignored() {
    let eval = EvalCtx::default();
    let ctx = EffectorCtx {
        eval: &eval,
        cloner_world: DMat4::IDENTITY,
    };
    let off = Delay {
        position: false,
        ..Delay::default()
    };
    let mut d = moving(10.0);
    off.smooth(&[moving(0.0)], &mut d, &FieldList::new(), &ctx)
        .unwrap();
    assert_eq!(pos(&d, 0).x, 10.0);
    // A history with two clones does not match the one clone: the input is the output.
    let mut d = moving(10.0);
    Delay::default()
        .smooth(&[column(2)], &mut d, &FieldList::new(), &ctx)
        .unwrap();
    assert_eq!(pos(&d, 0).x, 10.0);
    // Rotation and scale follow too.
    let mut start = moving(0.0);
    start.matrices[0] = DMat4::from_scale(DVec3::splat(1.0));
    let mut end = moving(0.0);
    end.matrices[0] = DMat4::from_scale(DVec3::splat(3.0));
    Delay::default()
        .smooth(&[start], &mut end, &FieldList::new(), &ctx)
        .unwrap();
    assert!((scale_of(&end, 0) - (1.0 + 2.0 * (1.0 - 0.5 * 0.999))).abs() < 1e-9);
}
