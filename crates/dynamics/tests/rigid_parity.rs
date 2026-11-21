//! Level-C parity of rigid bodies against O39 (docs/specs/DYNAMICS.md §7).

mod common;

use common::*;
use forge_parity_harness::{Thresholds, compare_trajectory};

/// Cases whose assertion is that they still do NOT match (spec §7), with the quantity.
const KNOWN_MISMATCH: &[(&str, &str)] = &[
    ("drop-cube", "rebound ratio"),
    ("drop-bounce", "rest position"),
    ("drop-friction-high", "slide distance"),
    ("tumble", "rest x"),
];

const REST_TOL: f64 = 0.05;

fn body_tag(case: &str) -> toml::Table {
    let t = case_table("dynamics", case);
    let p = t["params"].as_table().unwrap();
    p["body"].as_table().unwrap().clone()
}

fn num(t: &toml::Table, k: &str) -> Option<f64> {
    t.get(k)
        .and_then(|v| v.as_float().or_else(|| v.as_integer().map(|i| i as f64)))
}

/// A cube with the case's body-tag table applied.
fn cube_from_case(case: &str, name: &'static str, pos: [f64; 3]) -> Obj {
    let tag = body_tag(case);
    let mut o = Obj::cube(name, pos);
    if let Some(b) = num(&tag, "bounce") {
        o.bounce = b.min(1.0);
    } else if let Some(b) = num(&tag, "bounce_percent") {
        o.bounce = (b / 100.0).min(1.0);
    }
    if let Some(f) = num(&tag, "friction") {
        o.friction = f;
    } else if let Some(f) = num(&tag, "friction_percent") {
        o.friction = f / 100.0;
    }
    if let Some(v) = tag
        .get("initial_linear_velocity")
        .and_then(|v| v.as_array())
        && tag
            .get("custom_initial_velocity")
            .is_some_and(|c| c.as_bool() == Some(true) || c.as_integer() == Some(1))
    {
        let f = |i: usize| v[i].as_float().unwrap_or(0.0);
        o.velocity = Some([f(0), f(1), f(2)]);
    }
    o
}

fn max_abs_diff(a: &[f64], b: &[f64], upto: usize) -> f64 {
    (0..=upto).map(|f| (a[f] - b[f]).abs()).fold(0.0, f64::max)
}

fn angle_diff(a: f64, b: f64) -> f64 {
    let d = (a - b).rem_euclid(360.0);
    d.min(360.0 - d)
}

/// Whether `a` is within `tol` degrees of a multiple of 90.
fn flat(a: f64, tol: f64) -> bool {
    let d = a.rem_euclid(90.0);
    d.min(90.0 - d) <= tol
}

fn known(case: &str) -> Option<&'static str> {
    KNOWN_MISMATCH.iter().find(|k| k.0 == case).map(|k| k.1)
}

#[test]
fn drop_cube() {
    let b = build(&[
        Obj::plane(),
        cube_from_case("drop-cube", "cube", [0.0, 300.0, 0.0]),
    ]);
    let sim = simulate(&b, 90);
    let ms = &sim[&b.ids["cube"]];
    let (ey, oy) = (
        track("dynamics", "drop-cube", "cube", "pos_y"),
        column(ms, 1),
    );
    // The free fall (no contact before frame 22), to 1 cm.
    assert!(
        max_abs_diff(&ey, &oy, 22) < 1.0,
        "free fall: {}",
        max_abs_diff(&ey, &oy, 22)
    );
    let th = Thresholds {
        rest_position: Some(REST_TOL),
        first_contact_frames: Some(1.0),
        ..Default::default()
    };
    let rep = compare_trajectory(&ey, &oy, &th);
    assert!(rep.passed(), "{:?}", rep.failures());
    // At rest from frame 35 on.
    for f in 35..=90 {
        assert!((oy[f] - oy[90]).abs() < REST_TOL, "frame {f}: {}", oy[f]);
    }
    for (k, p) in ["rot_h", "rot_p", "rot_b"].iter().enumerate() {
        let e = track("dynamics", "drop-cube", "cube", p)[90];
        assert!(angle_diff(hpb_deg(&ms[90])[k], e) < 1.0, "{p}");
    }
    // Pinned: the rebound.
    assert_eq!(known("drop-cube"), Some("rebound ratio"));
    let bounce = Thresholds {
        bounce_ratio: Some(0.1),
        ..Default::default()
    };
    assert!(
        !compare_trajectory(&ey, &oy, &bounce).passed(),
        "rebound matches now"
    );
}

#[test]
fn drop_bounce() {
    let b = build(&[
        Obj::plane(),
        cube_from_case("drop-bounce", "cube", [0.0, 300.0, 0.0]),
    ]);
    let sim = simulate(&b, 90);
    let (ey, oy) = (
        track("dynamics", "drop-bounce", "cube", "pos_y"),
        column(&sim[&b.ids["cube"]], 1),
    );
    let th = Thresholds {
        first_contact_frames: Some(1.0),
        ..Default::default()
    };
    let rep = compare_trajectory(&ey, &oy, &th);
    assert!(rep.passed(), "{:?}", rep.failures());
    assert_eq!(known("drop-bounce"), Some("rest position"));
    let rest = Thresholds {
        rest_position: Some(REST_TOL),
        ..Default::default()
    };
    assert!(!compare_trajectory(&ey, &oy, &rest).passed());
}

fn slide(case: &str) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
    let b = build(&[Obj::plane(), cube_from_case(case, "cube", [0.0, 25.0, 0.0])]);
    let sim = simulate(&b, 90);
    let ms = &sim[&b.ids["cube"]];
    (
        track("dynamics", case, "cube", "pos_x"),
        column(ms, 0),
        column(ms, 1),
    )
}

#[test]
fn drop_friction_slide() {
    let (ex, ox, oy) = slide("drop-friction-slide");
    let th = Thresholds {
        slide_distance: Some(0.10),
        ..Default::default()
    };
    let rep = compare_trajectory(&ex, &ox, &th);
    assert!(rep.passed(), "{:?}", rep.failures());
    let ey = track("dynamics", "drop-friction-slide", "cube", "pos_y");
    assert!((oy[90] - ey[90]).abs() < REST_TOL, "rest y {}", oy[90]);
}

#[test]
fn drop_friction_high_stays_pinned() {
    let (ex, ox, _) = slide("drop-friction-high");
    assert_eq!(known("drop-friction-high"), Some("slide distance"));
    let th = Thresholds {
        slide_distance: Some(0.10),
        ..Default::default()
    };
    assert!(!compare_trajectory(&ex, &ox, &th).passed(), "matches now");
}

#[test]
fn stack_stays_up() {
    let b = build(&[
        Obj::plane(),
        cube_from_case("stack", "cube1", [0.0, 25.0, 0.0]),
        cube_from_case("stack", "cube2", [0.0, 75.0, 0.0]),
        cube_from_case("stack", "cube3", [0.0, 125.0, 0.0]),
    ]);
    let sim = simulate(&b, 90);
    for (n, y0) in [("cube1", 25.0), ("cube2", 75.0), ("cube3", 125.0)] {
        let m = &sim[&b.ids[n]][90];
        assert!((m.w_axis.y - y0).abs() < 0.25, "{n} y {}", m.w_axis.y);
        assert!(
            m.w_axis.x.abs() < 0.3 && m.w_axis.z.abs() < 0.3,
            "{n} drift"
        );
        for a in hpb_deg(m) {
            assert!(angle_diff(a, 0.0) < 1.0, "{n} tilted");
        }
        // The reference stays within 0.045 cm: the same claim at our tolerance.
        let ey = track("dynamics", "stack", n, "pos_y")[90];
        assert!((ey - y0).abs() < 0.05, "{n} reference");
    }
}

#[test]
fn slope_sphere_rolls_off_the_plane() {
    let mut s = cube_from_case("slope-sphere", "sphere", [0.0, 25.0, 0.0]);
    s.kind = "forge.sphere";
    s.size = 25.0;
    let mut p = Obj::plane();
    p.rot_deg = [0.0, 0.0, 20.0];
    let b = build(&[p, s]);
    let sim = simulate(&b, 90);
    let ms = &sim[&b.ids["sphere"]];
    let (ex, ey) = (
        track("dynamics", "slope-sphere", "sphere", "pos_x"),
        track("dynamics", "slope-sphere", "sphere", "pos_y"),
    );
    let (ox, oy) = (column(ms, 0), column(ms, 1));
    let rel = |e: f64, o: f64| (o / e - 1.0).abs();
    assert!(rel(ex[30], ox[30]) < 0.10, "x30 {} vs {}", ox[30], ex[30]);
    assert!(rel(ex[90], ox[90]) < 0.10, "x90 {} vs {}", ox[90], ex[90]);
    assert!(rel(ey[90], oy[90]) < 0.10, "y90 {} vs {}", oy[90], ey[90]);
}

#[test]
fn tumble_lands_flat() {
    let mut c = cube_from_case("tumble", "cube", [0.0, 300.0, 0.0]);
    c.rot_deg = [30.0, 20.0, 10.0];
    let b = build(&[Obj::plane(), c]);
    let sim = simulate(&b, 90);
    let ms = &sim[&b.ids["cube"]];
    let (ey, oy) = (track("dynamics", "tumble", "cube", "pos_y"), column(ms, 1));
    let th = Thresholds {
        rest_position: Some(REST_TOL),
        first_contact_frames: Some(1.0),
        ..Default::default()
    };
    let rep = compare_trajectory(&ey, &oy, &th);
    assert!(rep.passed(), "{:?}", rep.failures());
    let hpb = hpb_deg(&ms[90]);
    assert!(flat(hpb[1], 1.0) && flat(hpb[2], 1.0), "{hpb:?}");
    // The reference also rests flat.
    for p in ["rot_p", "rot_b"] {
        assert!(flat(track("dynamics", "tumble", "cube", p)[90], 1.0), "{p}");
    }
    // Pinned: where it lands.
    assert_eq!(known("tumble"), Some("rest x"));
    let ex = track("dynamics", "tumble", "cube", "pos_x")[90];
    assert!((ms[90].w_axis.x - ex).abs() > 1.0, "x matches now");
}
