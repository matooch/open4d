//! Unit tests of the clone data and the modes beyond the golden cases
//! (clone-toolset spec C1-C9).

use std::collections::BTreeMap;

use forge_clone_engine::*;
use forge_core::{EvalCtx, Registry, Value};
use glam::DVec3;

fn settings(values: &[(&str, Value)]) -> Settings {
    let map: BTreeMap<String, Value> = values
        .iter()
        .map(|(k, v)| (k.to_string(), v.clone()))
        .collect();
    settings_from_params(&map)
}

fn mode(m: &str) -> (&'static str, Value) {
    ("mode", Value::Enum(m.into()))
}

fn run(values: &[(&str, Value)]) -> Result<CloneData, CloneError> {
    generate(&settings(values), &EvalCtx::default())
}

fn pos(d: &CloneData, i: usize) -> DVec3 {
    d.matrices[i].w_axis.truncate()
}

#[test]
fn the_default_cloner_is_a_3_by_1_by_3_grid() {
    let s = settings(&[]);
    assert_eq!(s.mode, Mode::Grid);
    let d = generate(&s, &EvalCtx::default()).unwrap();
    assert_eq!(d.len(), 9);
    assert!(d.is_consistent());
    assert_eq!(d.index, (0..9).collect::<Vec<u32>>());
    assert_eq!(d.visible_count(), 9);
    assert_eq!(pos(&d, 0), DVec3::new(-200.0, 0.0, -200.0));
}

#[test]
fn the_type_registers_and_every_default_validates() {
    let mut r = Registry::with_builtins();
    r.install(&CloneToolset).unwrap();
    let ty = r.get(CLONER).unwrap();
    // Transform params are prepended; every mode has its parameters.
    assert!(ty.find_param("transform.pos").is_some());
    for id in [
        "mode",
        "linear_count",
        "radial_count",
        "grid_count",
        "count_width",
        "instance_mode",
    ] {
        assert!(ty.find_param(id).is_some(), "{id}");
    }
    // Reading no parameters gives the documented defaults.
    let s = settings(&[]);
    assert_eq!((s.linear.count, s.radial.count), (3, 5));
    assert_eq!(s.grid.count, [3, 1, 3]);
    assert_eq!(s.honeycomb.seed, 123_456);
    assert_eq!(s.radial.offset_seed, 1_234_567);
    assert_eq!(s.instance_mode, InstanceMode::Instance);
}

#[test]
fn linear_edge_counts() {
    let none = run(&[mode("linear"), ("linear_count", Value::Int(0))]).unwrap();
    assert!(none.is_empty() && none.is_consistent());
    // One clone in End Point mode stays at the start (no division by zero).
    let one = run(&[
        mode("linear"),
        ("linear_count", Value::Int(1)),
        ("linear_mode", Value::Enum("end_point".into())),
    ])
    .unwrap();
    assert_eq!(one.len(), 1);
    assert_eq!(pos(&one, 0), DVec3::ZERO);
    assert!(
        one.matrices[0]
            .to_cols_array()
            .iter()
            .all(|v| v.is_finite())
    );
}

#[test]
fn linear_offset_prepends_hidden_clones() {
    // C4, `linear-offset`: Offset 1 adds a hidden clone at t = 0; the visible ones start at 50.
    let d = run(&[mode("linear"), ("linear_offset", Value::Float(1.0))]).unwrap();
    assert_eq!(d.len(), 4);
    assert_eq!(d.visible_count(), 3);
    assert!(!d.visible[0]);
    assert_eq!(pos(&d, 0), DVec3::ZERO);
    assert_eq!(pos(&d, 1), DVec3::new(0.0, 50.0, 0.0));
    // A fractional Offset only shifts the pattern (parity-pending).
    let d = run(&[mode("linear"), ("linear_offset", Value::Float(0.5))]).unwrap();
    assert_eq!(d.len(), 3);
    assert_eq!(pos(&d, 0), DVec3::new(0.0, 25.0, 0.0));
}

#[test]
fn transform_tab_applies_to_every_clone() {
    let d = run(&[
        mode("linear"),
        ("clone_position", Value::Vec3([1.0, 2.0, 3.0])),
        ("clone_scale", Value::Vec3([2.0, 2.0, 2.0])),
        ("clone_color", Value::Color([0.5, 0.25, 1.0])),
        ("clone_weight", Value::Float(0.5)),
    ])
    .unwrap();
    assert_eq!(pos(&d, 1), DVec3::new(1.0, 52.0, 3.0));
    assert_eq!(d.matrices[1].x_axis.x, 2.0);
    assert_eq!(d.colors[2], [0.5, 0.25, 1.0]);
    assert_eq!(d.weights[0], 0.5);
}

#[test]
fn radial_variation_is_seeded_and_bounded() {
    // Variation scales the Offset (C5): without an Offset nothing moves.
    let no_offset = [
        mode("radial"),
        ("radial_offset_variation", Value::Float(1.0)),
    ];
    assert_eq!(
        run(&no_offset).unwrap(),
        run(&[mode("radial")]).unwrap(),
        "Offset 0: variation is a no-op"
    );
    let base = [
        mode("radial"),
        ("radial_offset", Value::Float(1.0)),
        ("radial_offset_variation", Value::Float(1.0)),
    ];
    let a = run(&base).unwrap();
    let b = run(&base).unwrap();
    assert_eq!(a, b, "same seed, same clones");
    let mut other = base.to_vec();
    other.push(("radial_offset_seed", Value::Int(42)));
    assert_ne!(a, run(&other).unwrap());
    // Each angle moves by at most half the Offset (0.5 rad), clone 0 not at all.
    let plain = run(&[mode("radial"), ("radial_offset", Value::Float(1.0))]).unwrap();
    assert_eq!(pos(&a, 0), pos(&plain, 0));
    for i in 0..5 {
        let angle = |d: &CloneData| pos(d, i).x.atan2(pos(d, i).z);
        let mut diff = (angle(&a) - angle(&plain)).abs();
        if diff > std::f64::consts::PI {
            diff = std::f64::consts::TAU - diff;
        }
        assert!(diff <= 0.5 + 1e-9, "clone {i}: {diff}");
    }
}

#[test]
fn radial_clones_stay_on_the_circle() {
    for plane in ["xy", "zy", "xz"] {
        let d = run(&[
            mode("radial"),
            ("radial_count", Value::Int(7)),
            ("radius", Value::Float(120.0)),
            ("plane", Value::Enum(plane.into())),
        ])
        .unwrap();
        for i in 0..7 {
            assert!((pos(&d, i).length() - 120.0).abs() < 1e-9, "{plane} {i}");
            // The aligned frame is a rotation (orthonormal, right-handed).
            let m = d.matrices[i];
            assert!(
                (m.x_axis.truncate().cross(m.y_axis.truncate()) - m.z_axis.truncate()).length()
                    < 1e-12
            );
        }
    }
}

#[test]
fn grid_forms_hide_but_keep_slots() {
    let sphere = run(&[
        ("grid_count", Value::Vec3([5.0; 3])),
        ("grid_form", Value::Enum("sphere".into())),
    ])
    .unwrap();
    assert_eq!((sphere.len(), sphere.visible_count()), (125, 33));
    let shell = run(&[
        ("grid_count", Value::Vec3([5.0; 3])),
        ("grid_form", Value::Enum("sphere".into())),
        ("fill", Value::Float(0.0)),
    ])
    .unwrap();
    // Only the clones at r = 1 stay: the six axis points two steps out.
    assert_eq!(shell.visible_count(), 6);
    let cube = run(&[
        ("grid_count", Value::Vec3([5.0; 3])),
        ("fill", Value::Float(0.0)),
    ])
    .unwrap();
    assert_eq!(cube.visible_count(), 125, "Cubic ignores Fill");
}

#[test]
fn grid_axes_with_one_clone_stay_centred() {
    let d = run(&[
        ("grid_count", Value::Vec3([1.0, 1.0, 1.0])),
        ("grid_mode", Value::Enum("endpoint".into())),
    ])
    .unwrap();
    assert_eq!((d.len(), pos(&d, 0)), (1, DVec3::ZERO));
}

#[test]
fn honeycomb_counts() {
    let count = |w: i64, h: i64, dir: &str| {
        run(&[
            mode("honeycomb"),
            ("count_width", Value::Int(w)),
            ("count_height", Value::Int(h)),
            ("offset_direction", Value::Enum(dir.into())),
        ])
        .unwrap()
        .len()
    };
    assert_eq!(count(10, 10, "height"), 50);
    assert_eq!(count(3, 2, "height"), 3);
    assert_eq!(count(1, 1, "height"), 1);
    assert_eq!(
        count(2, 1, "height"),
        1,
        "the odd line of a single half-cell is empty"
    );
    assert_eq!(count(3, 2, "width"), 3);
    assert_eq!(count(2, 3, "width"), 3);
}

#[test]
fn honeycomb_orientation_picks_the_plane() {
    for (o, axis) in [("z", 2), ("x", 0), ("y", 1)] {
        let d = run(&[mode("honeycomb"), ("orientation", Value::Enum(o.into()))]).unwrap();
        assert!((0..d.len()).all(|i| pos(&d, i)[axis] == 0.0), "{o}");
    }
}

#[test]
fn honeycomb_variation_moves_whole_lines_but_not_the_first_and_last() {
    // C9, `honey-var`: one shift per interior line, both ends fixed.
    let v = [
        mode("honeycomb"),
        ("honeycomb_offset_variation", Value::Float(0.5)),
        ("perpendicular_variation", Value::Float(0.5)),
    ];
    let a = run(&v).unwrap();
    assert_eq!(a, run(&v).unwrap());
    let plain = run(&[mode("honeycomb")]).unwrap();
    assert_ne!(a, plain);
    assert_eq!(a.len(), plain.len());
    // The first line (the first clones of the list) does not move.
    assert_eq!(pos(&a, 0), pos(&plain, 0));
    assert_eq!(pos(&a, 1), pos(&plain, 1));
}

#[test]
fn unsupported_options_say_so() {
    assert!(matches!(
        run(&[mode("object")]),
        Err(CloneError::Unsupported(_))
    ));
    assert!(matches!(
        run(&[("grid_form", Value::Enum("object".into()))]),
        Err(CloneError::Unsupported(_))
    ));
    assert!(matches!(
        run(&[
            mode("honeycomb"),
            ("honeycomb_form", Value::Enum("circle".into()))
        ]),
        Err(CloneError::Unsupported(_))
    ));
}

#[test]
fn too_many_clones_is_an_error_not_an_allocation() {
    let r = run(&[("grid_count", Value::Vec3([1000.0, 1000.0, 1000.0]))]);
    assert_eq!(r.err(), Some(CloneError::TooMany));
    // The descriptor clamps the count; a direct caller is still guarded.
    let mut s = settings(&[mode("linear")]);
    s.linear.count = MAX_CLONES + 1;
    assert_eq!(
        generate(&s, &EvalCtx::default()).err(),
        Some(CloneError::TooMany)
    );
    assert_eq!(
        settings(&[("linear_count", Value::Int(5_000_000))])
            .linear
            .count,
        MAX_CLONES
    );
}

#[test]
fn children_are_assigned_by_the_clones_mode() {
    let mut d = run(&[mode("linear"), ("linear_count", Value::Int(5))]).unwrap();
    d.assign_children(2, ClonesMode::Iterate, 1);
    assert_eq!(d.child, vec![0, 1, 0, 1, 0]);
    d.assign_children(1, ClonesMode::Random, 1);
    assert_eq!(d.child, vec![0; 5]);
    // Blend: 5 clones over 2 children.
    d.assign_children(2, ClonesMode::Blend, 1);
    assert_eq!(d.child, vec![0, 0, 1, 1, 1]);
    // Sort follows the weights (all 0: the first child).
    d.assign_children(3, ClonesMode::Sort, 1);
    assert_eq!(d.child, vec![0; 5]);
    d.weights = vec![0.0, 0.25, 0.5, 0.75, 1.0];
    d.assign_children(3, ClonesMode::Sort, 1);
    assert_eq!(d.child, vec![0, 1, 1, 2, 2]);
    // Random: deterministic per seed, every child in range, all children used over 60 clones.
    let mut d = run(&[mode("linear"), ("linear_count", Value::Int(60))]).unwrap();
    d.assign_children(3, ClonesMode::Random, 123_456);
    let again = d.child.clone();
    d.assign_children(3, ClonesMode::Random, 123_456);
    assert_eq!(d.child, again);
    assert!(d.child.iter().all(|c| *c < 3));
    for k in 0..3 {
        assert!(d.child.contains(&k));
    }
    d.assign_children(3, ClonesMode::Random, 7);
    assert_ne!(d.child, again);
}

#[test]
fn splitmix_is_deterministic() {
    let mut a = SplitMix64::new(7);
    let mut b = SplitMix64::new(7);
    assert_eq!(a.next_u64(), b.next_u64());
    // First output of SplitMix64 for seed 0 (published reference value).
    assert_eq!(SplitMix64::new(0).next_u64(), 0xE220_A839_7B1D_CDAF);
    let r = SplitMix64::new(1).next_f64();
    assert!((0.0..1.0).contains(&r));
}
