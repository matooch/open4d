//! docs/specs/PRIMITIVES3.md: registration, defaults, parameters and the Figure.

use std::collections::BTreeMap;
use std::sync::Arc;

use forge_core::{Category, Document, EvalCtx, Registry, Value};
use forge_generators::{BuildError, Primitives, build, evaluate};
use forge_geometry::PolyMesh;
use glam::DVec3;

fn p(pairs: &[(&str, Value)]) -> BTreeMap<String, Value> {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.clone()))
        .collect()
}

fn e(s: &str) -> Value {
    Value::Enum(s.into())
}

const IDS: [&str; 4] = [
    "forge.platonic",
    "forge.oil_tank",
    "forge.polygon_prim",
    "forge.figure",
];

fn bbox(m: &PolyMesh) -> (DVec3, DVec3) {
    let mut lo = DVec3::splat(f64::MAX);
    let mut hi = DVec3::splat(f64::MIN);
    for q in &m.points {
        lo = lo.min(*q);
        hi = hi.max(*q);
    }
    (lo, hi)
}

#[test]
fn the_four_register_with_the_r25_defaults_and_evaluate_headless() {
    let mut r = Registry::with_builtins();
    r.install(&Primitives).unwrap();
    let default = |id: &str, key: &str| r.get(id).unwrap().find_param(key).unwrap().default.clone();
    for id in IDS {
        assert_eq!(r.get(id).unwrap().category, Category::Generator);
    }
    assert_eq!(default("forge.platonic", "type"), e("icosa"));
    assert_eq!(default("forge.platonic", "radius"), Value::Float(100.0));
    assert_eq!(default("forge.oil_tank", "height_segments"), Value::Int(4));
    assert_eq!(default("forge.oil_tank", "cap_segments"), Value::Int(4));
    assert_eq!(default("forge.oil_tank", "cap_height"), Value::Float(25.0));
    assert_eq!(
        default("forge.oil_tank", "rotation_segments"),
        Value::Int(16)
    );
    assert_eq!(default("forge.polygon_prim", "width"), Value::Float(100.0));
    assert_eq!(default("forge.figure", "height"), Value::Float(180.0));
    assert_eq!(default("forge.figure", "segments"), Value::Int(14));

    // what `object.create` does: create the node with its registry defaults, evaluate it
    let mut doc = Document::new(Arc::new(r));
    for (id, counts) in [
        ("forge.platonic", (12, 20)),
        ("forge.oil_tank", (178, 192)),
        ("forge.polygon_prim", (4, 1)),
    ] {
        let node = doc
            .transact("create", |tx| tx.create(id, None, None))
            .unwrap();
        let m = evaluate(doc.scene(), node, &EvalCtx::default())
            .unwrap()
            .unwrap();
        assert_eq!((m.points.len(), m.polys.len()), counts, "{id}");
    }
    let node = doc
        .transact("create", |tx| tx.create("forge.figure", None, None))
        .unwrap();
    let m = evaluate(doc.scene(), node, &EvalCtx::default())
        .unwrap()
        .unwrap();
    m.validate().unwrap();
    assert!(!m.polys.is_empty());
}

#[test]
fn platonic_types_have_the_exported_counts_and_radius() {
    for (kind, counts) in [
        ("tetra", (4, 4)),
        ("hexa", (8, 6)),
        ("octa", (6, 8)),
        ("dodeca", (20, 24)),
        ("icosa", (12, 20)),
        ("bucky", (60, 64)),
    ] {
        let m = build("forge.platonic", &p(&[("type", e(kind))])).unwrap();
        m.validate().unwrap();
        assert_eq!((m.points.len(), m.polys.len()), counts, "{kind}");
        // radius scales every point
        let big = build(
            "forge.platonic",
            &p(&[("type", e(kind)), ("radius", Value::Float(250.0))]),
        )
        .unwrap();
        for (a, b) in m.points.iter().zip(&big.points) {
            assert!((*a * 2.5 - *b).length() < 1e-9, "{kind}");
        }
    }
    // the tetra is not centred: its box runs from -radius/3 to radius in y
    let t = build("forge.platonic", &p(&[("type", e("tetra"))])).unwrap();
    let (lo, hi) = bbox(&t);
    assert!((lo.y + 100.0 / 3.0).abs() < 1e-9 && (hi.y - 100.0).abs() < 1e-9);
}

#[test]
fn platonic_segments_cut_the_faces_flat() {
    for (kind, faces) in [("tetra", 4), ("hexa", 6), ("octa", 8), ("icosa", 20)] {
        let m = build(
            "forge.platonic",
            &p(&[("type", e(kind)), ("segments", Value::Int(3))]),
        )
        .unwrap();
        m.validate().unwrap();
        assert_eq!(m.polys.len(), faces * 9, "{kind}");
        // flat: no point is further from the origin than the corners
        let corner = m.points[0].length();
        assert!(m.points.iter().all(|q| q.length() <= corner + 1e-9));
    }
}

#[test]
fn oil_tank_parameters() {
    let default = build("forge.oil_tank", &p(&[])).unwrap();
    let (lo, hi) = bbox(&default);
    assert!((hi.y - 100.0).abs() < 1e-9 && (lo.y + 100.0).abs() < 1e-9);
    // 4 + 3 + 4 profile points per column plus the poles
    let tall = build(
        "forge.oil_tank",
        &p(&[
            ("height", Value::Float(400.0)),
            ("cap_height", Value::Float(50.0)),
        ]),
    )
    .unwrap();
    tall.validate().unwrap();
    assert!((bbox(&tall).1.y - 200.0).abs() < 1e-9);
    assert_eq!(tall.points.len(), default.points.len());
    // flat ends, and a cap higher than half the height is clamped
    for cap in [0.0, 1000.0] {
        let m = build("forge.oil_tank", &p(&[("cap_height", Value::Float(cap))])).unwrap();
        m.validate().unwrap();
        assert!((bbox(&m).1.y - 100.0).abs() < 1e-9);
        assert!(m.points.iter().all(|q| q.is_finite()));
    }
    // orientation +X puts the axis along X
    let x = build("forge.oil_tank", &p(&[("orientation", e("pos_x"))])).unwrap();
    let (lo, hi) = bbox(&x);
    assert!((hi.x - lo.x - 200.0).abs() < 1e-9 && (hi.y - lo.y - 200.0).abs() < 1e-9);
    // regular grid is not derived
    let grid = p(&[
        ("slice", Value::Bool(true)),
        ("regular_grid", Value::Bool(true)),
    ]);
    assert!(matches!(
        build("forge.oil_tank", &grid),
        Err(BuildError::Unsupported(_))
    ));
}

#[test]
fn polygon_parameters() {
    let m = build(
        "forge.polygon_prim",
        &p(&[
            ("width", Value::Float(40.0)),
            ("height", Value::Float(10.0)),
            ("segments", Value::Int(3)),
        ]),
    )
    .unwrap();
    m.validate().unwrap();
    assert_eq!((m.points.len(), m.polys.len()), (16, 9));
    let (lo, hi) = bbox(&m);
    assert_eq!((hi - lo).to_array(), [40.0, 0.0, 10.0]);
    let t = build(
        "forge.polygon_prim",
        &p(&[("triangle", Value::Bool(true)), ("segments", Value::Int(2))]),
    )
    .unwrap();
    t.validate().unwrap();
    assert_eq!((t.points.len(), t.polys.len()), (6, 4));
    let z = build("forge.polygon_prim", &p(&[("orientation", e("pos_z"))])).unwrap();
    let (lo, hi) = bbox(&z);
    assert!((hi.y - lo.y - 100.0).abs() < 1e-9 && (hi.z - lo.z).abs() < 1e-9);
}

/// Signed volume of the closed parts of a mesh (positive when faces point outwards).
fn volume(m: &PolyMesh) -> f64 {
    let mut v = 0.0;
    for f in &m.polys {
        let a = m.points[f[0] as usize];
        for w in 1..f.len() - 1 {
            let (b, c) = (m.points[f[w] as usize], m.points[f[w + 1] as usize]);
            v += a.dot(b.cross(c)) / 6.0;
        }
    }
    v
}

#[test]
fn figure_scales_with_height_and_changes_with_segments() {
    let base = build("forge.figure", &p(&[])).unwrap();
    base.validate().unwrap();
    assert!(volume(&base) > 0.0, "faces point outwards");
    let (lo, hi) = bbox(&base);
    assert!(lo.y.abs() < 1e-9 && (hi.y - 180.0).abs() < 1e-9);

    let tall = build("forge.figure", &p(&[("height", Value::Float(360.0))])).unwrap();
    assert_eq!(tall.points.len(), base.points.len());
    for (a, b) in base.points.iter().zip(&tall.points) {
        assert!((*a * 2.0 - *b).length() < 1e-9);
    }

    let coarse = build("forge.figure", &p(&[("segments", Value::Int(6))])).unwrap();
    let fine = build("forge.figure", &p(&[("segments", Value::Int(24))])).unwrap();
    for m in [&coarse, &fine] {
        m.validate().unwrap();
        assert!(volume(m) > 0.0);
    }
    assert!(coarse.polys.len() < base.polys.len() && base.polys.len() < fine.polys.len());
    assert_ne!(coarse, base);
    // the mannequin is left-right symmetric
    let (lo, hi) = bbox(&base);
    assert!((lo.x + hi.x).abs() < 1e-9);
}
