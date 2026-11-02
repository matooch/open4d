//! The O17 spline primitives beyond the golden extrudes (docs/specs/GENERATORS2.md §6.3):
//! registered defaults, option behaviour and what stays unsupported.

use std::collections::BTreeMap;
use std::sync::Arc;

use forge_core::{Document, EvalCtx, Registry, Value};
use forge_generators::splines::build_curve;
use forge_generators::{BuildError, Primitives, evaluate};
use glam::DVec3;

fn registry() -> Arc<Registry> {
    let mut reg = Registry::with_builtins();
    reg.install(&Primitives).unwrap();
    Arc::new(reg)
}

fn values(pairs: &[(&str, Value)]) -> BTreeMap<String, Value> {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.clone()))
        .collect()
}

fn e(id: &str) -> Value {
    Value::Enum(id.into())
}

fn f(x: f64) -> Value {
    Value::Float(x)
}

fn points(ty: &str, extra: &[(&str, Value)]) -> Vec<DVec3> {
    build_curve(&format!("forge.{ty}_spline"), &values(extra))
        .unwrap()
        .points()
}

const NEW: [&str; 10] = [
    "arc", "nside", "star", "flower", "fourside", "cogwheel", "cycloid", "profile", "cissoid",
    "formula",
];

#[test]
fn every_new_spline_is_registered_with_the_observed_defaults() {
    let reg = registry();
    for name in NEW {
        let ty = reg.get(&format!("forge.{name}_spline")).unwrap();
        for id in ["plane", "reverse", "intermediate"] {
            assert!(ty.find_param(id).is_some(), "{name}.{id}");
        }
        let mode = &ty.find_param("intermediate").unwrap().default;
        // Arc defaults to Uniform, every other new spline to Adaptive
        let want = if name == "arc" { "uniform" } else { "adaptive" };
        assert_eq!(mode, &e(want), "{name}");
        assert_eq!(
            ty.find_param("intermediate_number").unwrap().default,
            Value::Int(8)
        );
    }
    let d = |ty: &str, id: &str| {
        reg.get(&format!("forge.{ty}_spline"))
            .unwrap()
            .find_param(id)
            .unwrap()
            .default
            .clone()
    };
    assert_eq!(d("arc", "radius"), f(200.0));
    assert_eq!(d("arc", "inner_radius"), f(100.0));
    assert_eq!(d("arc", "end_angle"), f(90f64.to_radians()));
    assert_eq!(d("nside", "sides"), Value::Int(6));
    assert_eq!(d("nside", "rounding_radius"), f(50.0));
    assert_eq!(d("star", "points"), Value::Int(8));
    assert_eq!(d("flower", "petals"), Value::Int(8));
    assert_eq!(d("fourside", "fourside_type"), e("diamond"));
    assert_eq!(d("fourside", "angle"), f(30f64.to_radians()));
    assert_eq!(d("cogwheel", "teeth"), Value::Int(20));
    assert_eq!(d("cogwheel", "pressure_angle"), f(20f64.to_radians()));
    assert_eq!(d("cogwheel", "witness_ratio"), f(10.0));
    assert_eq!(d("cogwheel", "center_hole"), Value::Bool(true));
    assert_eq!(d("cycloid", "a"), f(75.0));
    assert_eq!(d("profile", "profile_type"), e("h_shape"));
    assert_eq!(d("cissoid", "tension"), f(2.0));
    assert_eq!(d("formula", "y_t"), Value::String("100.0*Sin(t*PI)".into()));
    assert_eq!(d("formula", "samples"), Value::Int(20));
    // the Circle and Rectangle options exist with their observed defaults
    assert_eq!(
        reg.get("forge.circle_spline")
            .unwrap()
            .find_param("ring")
            .unwrap()
            .default,
        Value::Bool(false)
    );
}

#[test]
fn objects_can_be_created_headless_and_the_unbuilt_ones_say_why() {
    let reg = registry();
    for name in NEW {
        let ty = format!("forge.{name}_spline");
        let mut doc = Document::new(reg.clone());
        let ex = doc
            .transact("t", |tx| {
                let ex = tx.create("forge.extrude", None, None)?;
                tx.create(&ty, Some(ex), None)?;
                Ok(ex)
            })
            .unwrap();
        let r = evaluate(doc.scene(), ex, &EvalCtx::default());
        match name {
            "cogwheel" | "formula" => {
                assert!(matches!(r, Err(BuildError::Unsupported(_))), "{name}")
            }
            _ => assert!(r.unwrap().unwrap().points.len() >= 8, "{name}"),
        }
    }
}

#[test]
fn arc_is_four_pieces_and_the_sector_closes_with_the_chord() {
    // Uniform 8 on 4 pieces: 4 * 9 + 1 points; the sector is closed: 5 * 9
    assert_eq!(points("arc", &[]).len(), 37);
    let sector = points("arc", &[("arc_type", e("sector"))]);
    assert_eq!(sector.len(), 45);
    // the chord points lie on x + y = r
    assert!(
        sector[30..]
            .iter()
            .all(|p| (p.x + p.y - 200.0).abs() < 1e-6)
    );
    // equal steps of arc length over the whole outline, chord included
    // (the step across the arc/chord corner is a chord of the bend, so it is shorter)
    let steps: Vec<f64> = (0..45)
        .filter(|i| *i != 23)
        .map(|i| (sector[(i + 1) % 45] - sector[i]).length())
        .collect();
    let mean = steps.iter().sum::<f64>() / 44.0;
    assert!(
        steps.iter().all(|s| (s - mean).abs() < 0.05 * mean),
        "{steps:?}"
    );
    // the not yet observed types stay unsupported
    for t in ["segment", "ring"] {
        let r = build_curve("forge.arc_spline", &values(&[("arc_type", e(t))]));
        assert!(matches!(r, Err(BuildError::Unsupported(_))), "{t}");
    }
}

#[test]
fn nside_star_and_fourside_shapes() {
    let hex = points("nside", &[]);
    assert_eq!(hex.len(), 6);
    assert!((hex[1] - DVec3::new(100.0, 173.20508075688772, 0.0)).length() < 1e-9);
    // rounding reaches r * tan(pi / n) towards the neighbours, and is clamped to half an edge
    let big = points(
        "nside",
        &[
            ("rounding", Value::Bool(true)),
            ("rounding_radius", f(1.0e6)),
        ],
    );
    assert!(big.iter().all(|p| p.length() <= 200.0 + 1e-9));
    let star = points("star", &[("points", Value::Int(5)), ("twist", f(30.0))]);
    assert_eq!(star.len(), 10);
    // the inner points turn by twist percent of the 72 degree step
    let a = star[1].y.atan2(star[1].x).to_degrees();
    assert!((a - 57.6).abs() < 1e-9, "{a}");
    let kite = points("fourside", &[("fourside_type", e("kite"))]);
    assert!((kite[0].x - 200.0 * 100.0 / 50000f64.sqrt()).abs() < 1e-9);
    for t in ["parallelogram", "trapezium"] {
        let r = build_curve("forge.fourside_spline", &values(&[("fourside_type", e(t))]));
        assert!(matches!(r, Err(BuildError::Unsupported(_))), "{t}");
    }
}

#[test]
fn profile_cissoid_and_cycloid() {
    let h = points("profile", &[]);
    assert_eq!(h.len(), 12);
    assert_eq!(h[4], DVec3::new(60.0, 180.0, 0.0));
    let c = points("cissoid", &[]);
    assert_eq!(c.len(), 101);
    assert!((c[0] - DVec3::new(-200.0, 200.0, 0.0)).length() < 1e-9);
    assert!(c[50].length() < 1e-9);
    // an open curve through 63 points, one extra point in the first fifteen pieces
    let cy = points("cycloid", &[("intermediate", e("none"))]);
    assert_eq!(cy.len(), 63);
    assert!((cy[62] - DVec3::new(157.07963267948966, -50.0, 0.0)).length() < 1e-9);
    for (ty, key) in [("profile", "profile_type"), ("cissoid", "cissoid_type")] {
        for t in ["l_shape", "lemniscate"] {
            let r = build_curve(&format!("forge.{ty}_spline"), &values(&[(key, e(t))]));
            if t == "l_shape" && ty == "profile" || t == "lemniscate" && ty == "cissoid" {
                assert!(matches!(r, Err(BuildError::Unsupported(_))), "{ty} {t}");
            }
        }
    }
    let hypo = build_curve(
        "forge.cycloid_spline",
        &values(&[("cycloid_type", e("hypocycloid"))]),
    );
    assert!(matches!(hypo, Err(BuildError::Unsupported(_))));
}

#[test]
fn flower_has_three_points_per_petal_and_stays_closed() {
    let none = [("intermediate", e("none"))];
    assert_eq!(points("flower", &none).len(), 24);
    let more = points(
        "flower",
        &[("intermediate", e("none")), ("petals", Value::Int(5))],
    );
    assert_eq!(more.len(), 15);
}

#[test]
fn circle_ellipse_and_ring() {
    let ell = points(
        "circle",
        &[("ellipse", Value::Bool(true)), ("radius_y", f(100.0))],
    );
    assert!(ell.iter().all(|p| p.y.abs() <= 100.0 + 1e-6));
    // the ring is a second closed loop: the inner circle
    let curve = build_curve(
        "forge.circle_spline",
        &values(&[("ring", Value::Bool(true))]),
    )
    .unwrap();
    let loops = curve.loops();
    assert_eq!(loops.len(), 2);
    assert!(loops[1].iter().all(|p| (p.length() - 100.0).abs() < 0.2));
    // it follows the plane
    let xz = build_curve(
        "forge.circle_spline",
        &values(&[("ring", Value::Bool(true)), ("plane", e("xz"))]),
    )
    .unwrap();
    assert!(
        xz.loops()
            .iter()
            .flatten()
            .all(|p| p.z.abs() > -1.0 && p.y.abs() < 1e-9)
    );
    let both = build_curve(
        "forge.circle_spline",
        &values(&[("ring", Value::Bool(true)), ("ellipse", Value::Bool(true))]),
    );
    assert!(matches!(both, Err(BuildError::Unsupported(_))));
}

#[test]
fn rounded_rectangle_alternates_arcs_and_lines() {
    let c = build_curve(
        "forge.rectangle_spline",
        &values(&[("rounding", Value::Bool(true))]),
    )
    .unwrap();
    assert_eq!(c.segments.len(), 8);
    assert_eq!(c.points().len(), 72);
    // the radius is clamped to half the shorter side
    let big = build_curve(
        "forge.rectangle_spline",
        &values(&[
            ("rounding", Value::Bool(true)),
            ("rounding_radius", f(1.0e6)),
        ]),
    )
    .unwrap();
    assert!(big.points().iter().all(|p| p.x.abs() <= 200.0 + 1e-9));
}

#[test]
fn a_ring_extrudes_with_a_reversed_inner_wall_and_refuses_caps_and_other_generators() {
    let reg = registry();
    let mut doc = Document::new(reg.clone());
    let ex = doc
        .transact("t", |tx| {
            let ex = tx.create("forge.extrude", None, None)?;
            tx.set_param(ex, "cap_start", e("none"))?;
            tx.set_param(ex, "cap_end", e("none"))?;
            let c = tx.create("forge.circle_spline", Some(ex), None)?;
            tx.set_param(c, "ring", Value::Bool(true))?;
            Ok(ex)
        })
        .unwrap();
    let mesh = evaluate(doc.scene(), ex, &EvalCtx::default())
        .unwrap()
        .unwrap();
    assert_eq!(mesh.polys.len(), 72);
    // outer wall [a, b, b', a'], inner wall [a, a', b', b]
    assert_eq!(mesh.polys[0], vec![0, 1, 73, 72]);
    assert_eq!(mesh.polys[36], vec![36, 108, 109, 37]);
    doc.transact("caps", |tx| tx.set_param(ex, "cap_start", e("cap")))
        .unwrap();
    assert!(matches!(
        evaluate(doc.scene(), ex, &EvalCtx::default()),
        Err(BuildError::Unsupported(_))
    ));
    let lathe = doc
        .transact("t", |tx| {
            let l = tx.create("forge.lathe", None, None)?;
            let c = tx.create("forge.circle_spline", Some(l), None)?;
            tx.set_param(c, "ring", Value::Bool(true))?;
            Ok(l)
        })
        .unwrap();
    assert!(matches!(
        evaluate(doc.scene(), lathe, &EvalCtx::default()),
        Err(BuildError::Unsupported(_))
    ));
}
