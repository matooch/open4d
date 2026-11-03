//! docs/specs/PRIMITIVES3.md §6: Landscape registration, defaults and parameter behaviour.

use std::collections::BTreeMap;
use std::sync::Arc;

use forge_core::{Category, Document, EvalCtx, Registry, Value};
use forge_generators::{Primitives, build, evaluate};
use forge_geometry::PolyMesh;

const ID: &str = "forge.landscape";

fn p(pairs: &[(&str, Value)]) -> BTreeMap<String, Value> {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.clone()))
        .collect()
}

fn small(extra: &[(&str, Value)]) -> PolyMesh {
    let mut v = vec![
        ("width_segments", Value::Int(20)),
        ("depth_segments", Value::Int(20)),
    ];
    v.extend(extra.iter().map(|(k, x)| (*k, x.clone())));
    build(ID, &p(&v)).unwrap()
}

fn heights(m: &PolyMesh) -> Vec<f64> {
    m.points.iter().map(|q| q.y).collect()
}

#[test]
fn registers_with_the_r25_defaults_and_evaluates_headless() {
    let mut r = Registry::with_builtins();
    r.install(&Primitives).unwrap();
    let t = r.get(ID).unwrap();
    assert_eq!(t.category, Category::Generator);
    let d = |k: &str| t.find_param(k).unwrap().default.clone();
    assert_eq!(d("size_x"), Value::Float(600.0));
    assert_eq!(d("size_y"), Value::Float(100.0));
    assert_eq!(d("width_segments"), Value::Int(100));
    assert_eq!(d("rough_furrows"), Value::Float(0.5));
    assert_eq!(d("plateau_level"), Value::Float(1.0));
    assert_eq!(d("multifractal"), Value::Bool(true));
    assert_eq!(d("borders_at_sea_level"), Value::Bool(true));
    assert_eq!(d("spherical"), Value::Bool(false));
    assert_eq!(d("orientation"), Value::Enum("pos_y".into()));

    let mut doc = Document::new(Arc::new(r));
    let id = doc
        .transact("create", |tx| tx.create(ID, None, None))
        .unwrap();
    let mesh = evaluate(doc.scene(), id, &EvalCtx::default())
        .unwrap()
        .unwrap();
    assert_eq!((mesh.points.len(), mesh.polys.len()), (10201, 10000));
}

#[test]
fn deterministic_per_seed_and_seeds_differ() {
    let a = heights(&small(&[("seed", Value::Int(7))]));
    assert_eq!(a, heights(&small(&[("seed", Value::Int(7))])));
    assert_ne!(a, heights(&small(&[("seed", Value::Int(8))])));
}

#[test]
fn heights_stay_within_size_y_and_borders_are_zero_or_free() {
    let m = small(&[]);
    assert!(heights(&m).iter().all(|y| (0.0..=100.0).contains(y)));
    assert_eq!(m.points[0].y, 0.0);
    let free = small(&[("borders_at_sea_level", Value::Bool(false))]);
    let n = 21;
    assert!((0..n).any(|i| free.points[i].y > 0.0), "border row is free");
}

#[test]
fn sea_level_and_plateau_clamp() {
    let base = small(&[("borders_at_sea_level", Value::Bool(false))]);
    let sea = small(&[
        ("borders_at_sea_level", Value::Bool(false)),
        ("sea_level", Value::Float(0.5)),
    ]);
    let zeros = |m: &PolyMesh| heights(m).iter().filter(|y| **y == 0.0).count();
    assert!(zeros(&sea) > zeros(&base) + 20);
    let cap = small(&[
        ("borders_at_sea_level", Value::Bool(false)),
        ("plateau_level", Value::Float(0.4)),
    ]);
    let top = heights(&cap).into_iter().fold(0.0, f64::max);
    assert!((top - 40.0).abs() < 1e-9, "capped at 40, got {top}");
}

#[test]
fn furrows_change_the_detail_and_multifractal_changes_the_mesh() {
    let base = heights(&small(&[]));
    for (k, v) in [
        ("rough_furrows", Value::Float(0.9)),
        ("fine_furrows", Value::Float(0.0)),
        ("multifractal", Value::Bool(false)),
        ("scale", Value::Float(2.0)),
    ] {
        assert_ne!(base, heights(&small(&[(k, v)])), "{k}");
    }
}

#[test]
fn spherical_and_orientation_keep_the_topology() {
    let flat = small(&[]);
    let sph = small(&[("spherical", Value::Bool(true))]);
    assert_eq!(flat.polys, sph.polys);
    let r = 300.0;
    assert!(sph.points.iter().all(|q| q.length() >= r - 1e-9));
    let up = small(&[("orientation", Value::Enum("pos_z".into()))]);
    assert_eq!(up.points.len(), flat.points.len());
    assert!(up.points.iter().zip(&flat.points).all(|(a, b)| a.y == -b.z));
    flat.validate().unwrap();
    sph.validate().unwrap();
}
