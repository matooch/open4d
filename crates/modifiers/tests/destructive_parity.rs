//! Parity: docs/specs/MODIFIERS.md §11 (Explosion, Shatter), §12 (Wind) and §13 against
//! tests/golden/{explosion,shatter,wind}.
//!
//! The `defaults` cases (strength 0) are compared exactly. The reference draws its
//! per-polygon motions from a random stream we cannot reproduce, and its wind noise is
//! unknown, so the other three cases are pinned in `KNOWN_MISMATCH` (the exact comparison
//! still fails) and checked by the *same statistical invariants* on the golden mesh and on
//! ours (spec §11, §12).

use std::path::Path;
use std::sync::Arc;

use forge_core::{Document, EvalCtx, NodeId, ParamKind, Registry, Value};
use forge_generators::{Primitives, evaluate};
use forge_modifiers::Deformers;
use forge_parity_harness::{GoldenCase, Mesh, compare_meshes, discover_cases, export_weld};
use glam::{DMat3, DVec3};

const FEATURES: &[&str] = &["explosion", "shatter", "wind"];

/// Cases whose exact comparison must still fail (random stream / noise not reproducible).
const KNOWN_MISMATCH: &[&str] = &[
    "explosion/strength-50",
    "shatter/strength-50",
    "wind/frame-10",
];

/// The wind case file has no `frame` / `fps` keys; the form (O19) says frame 10 at 30 fps.
const WIND_FRAME: i64 = 10;
const FPS: f64 = 30.0;

type Table = toml::map::Map<String, toml::Value>;

fn num(v: &toml::Value) -> f64 {
    v.as_float()
        .or(v.as_integer().map(|i| i as f64))
        .expect("number")
}

/// Sets the case's keys on `id`: `*_percent` / `*_deg` suffixes dropped, angles to radians.
fn apply(tx: &mut forge_core::Tx, reg: &Registry, id: NodeId, ty_id: &str, table: &Table) {
    let ty = reg.get(ty_id).unwrap();
    for (k, v) in table {
        if v.is_table() || matches!(k.as_str(), "type" | "frame" | "fps") {
            continue;
        }
        let key = k
            .strip_suffix("_deg")
            .or_else(|| k.strip_suffix("_percent"))
            .unwrap_or(k);
        let desc = ty
            .find_param(key)
            .unwrap_or_else(|| panic!("{ty_id}: {k:?} is not a param"));
        let value = match (&desc.kind, v) {
            (ParamKind::Bool, toml::Value::Boolean(b)) => Value::Bool(*b),
            (ParamKind::Int { .. }, toml::Value::Integer(i)) => Value::Int(*i),
            (ParamKind::Float { .. }, v) => {
                let x = num(v);
                Value::Float(if desc.ui.unit == forge_core::Unit::Angle {
                    x.to_radians()
                } else {
                    x
                })
            }
            other => panic!("{ty_id}.{k}: unexpected value {other:?}"),
        };
        tx.set_param(id, key, value).unwrap();
    }
}

/// The cube of the case, with the deformer child when `deformed`; returns the cube.
fn build(
    reg: &Arc<Registry>,
    case: &GoldenCase,
    params: &Table,
    deformed: bool,
) -> (Document, NodeId) {
    let mut doc = Document::new(reg.clone());
    let input = params["input"].as_table().unwrap();
    let id = doc
        .transact("case", |tx| {
            let cube = tx.create("forge.cube", None, None)?;
            assert_eq!(input["type"].as_str(), Some("cube"));
            apply(tx, reg, cube, "forge.cube", input);
            if deformed {
                let ty = format!("forge.{}", case.feature);
                let d = tx.create(&ty, Some(cube), None)?;
                apply(tx, reg, d, &ty, params);
            }
            Ok(cube)
        })
        .unwrap();
    (doc, id)
}

fn ctx(case: &GoldenCase, params: &Table) -> EvalCtx {
    let frame = params
        .get("frame")
        .map(|v| num(v) as i64)
        .unwrap_or(if case.feature == "wind" {
            WIND_FRAME
        } else {
            0
        });
    let fps = params.get("fps").map(num).unwrap_or(FPS);
    EvalCtx::at_frame(frame, fps)
}

fn to_mesh(m: &forge_geometry::PolyMesh) -> Mesh {
    Mesh {
        positions: m.points.iter().map(|p| p.to_array()).collect(),
        faces: m.polys.clone(),
    }
}

fn pts(m: &Mesh) -> Vec<DVec3> {
    m.positions.iter().map(|p| DVec3::from_array(*p)).collect()
}

// ---------------------------------------------------------------- polygon statistics

struct Motion {
    centre0: DVec3,
    centre1: DVec3,
    scale: f64,
    rigid_error: f64,
    /// Euler angles of `Ry · Rx · Rz` in degrees.
    euler: [f64; 3],
}

fn frame(p: [DVec3; 4]) -> DMat3 {
    let u = (p[1] - p[0]).normalize();
    let w = (p[1] - p[0]).cross(p[3] - p[0]).normalize();
    DMat3::from_cols(u, w.cross(u), w)
}

/// Per polygon: the rigid-plus-scale motion that takes `orig` to `got` (quads only).
fn motions(orig: &Mesh, got: &Mesh) -> Vec<Motion> {
    assert_eq!(orig.faces.len(), got.faces.len(), "face count");
    let (po, pg) = (pts(orig), pts(got));
    orig.faces
        .iter()
        .zip(&got.faces)
        .map(|(fo, fg)| {
            assert_eq!(fo.len(), 4);
            assert_eq!(fg.len(), 4);
            let a = [0, 1, 2, 3].map(|i| po[fo[i] as usize]);
            let b = [0, 1, 2, 3].map(|i| pg[fg[i] as usize]);
            let (ca, cb) = (a.iter().sum::<DVec3>() / 4.0, b.iter().sum::<DVec3>() / 4.0);
            let scale = (b[1] - b[0]).length() / (a[1] - a[0]).length();
            let rot = frame(b) * frame(a).transpose();
            let rigid_error = (0..4)
                .map(|i| (cb + scale * (rot * (a[i] - ca)) - b[i]).length())
                .fold(0.0, f64::max);
            let m = rot.to_cols_array_2d(); // m[col][row]
            let r = |row: usize, col: usize| m[col][row];
            let beta = (-r(1, 2)).asin();
            let alpha = r(0, 2).atan2(r(2, 2));
            let gamma = r(1, 0).atan2(r(1, 1));
            Motion {
                centre0: ca,
                centre1: cb,
                scale,
                rigid_error,
                euler: [alpha, beta, gamma].map(f64::to_degrees),
            }
        })
        .collect()
}

fn std(v: &[f64]) -> f64 {
    let m = v.iter().sum::<f64>() / v.len() as f64;
    (v.iter().map(|x| (x - m).powi(2)).sum::<f64>() / v.len() as f64).sqrt()
}

/// The invariants of spec §11 for the strength-50 cases (strength 50 %, speed 100,
/// angle speed 100 deg, end size 0, randomness 100 %). Panics with `who` on a violation.
fn check_blast(who: &str, shatter: bool, orig: &Mesh, got: &Mesh) {
    // Topology: every face owns its points, in order.
    assert_eq!(got.positions.len(), 4 * got.faces.len(), "{who}: points");
    for (i, f) in got.faces.iter().enumerate() {
        let want: Vec<u32> = (4 * i as u32..4 * i as u32 + 4).collect();
        assert_eq!(f, &want, "{who}: face {i}");
    }
    let ms = motions(orig, got);
    let h = 0.75 * 100.0 * 0.5; // half range of every Euler angle, degrees
    for (i, m) in ms.iter().enumerate() {
        assert!(
            (m.scale - 0.5).abs() < 1e-5,
            "{who}: face {i} scale {}",
            m.scale
        );
        assert!(
            m.rigid_error < 1e-5,
            "{who}: face {i} not rigid ({})",
            m.rigid_error
        );
        for a in m.euler {
            assert!(a.abs() <= h + 0.05, "{who}: face {i} angle {a} beyond {h}");
        }
    }
    for axis in 0..3 {
        let a: Vec<f64> = ms.iter().map(|m| m.euler[axis].abs()).collect();
        let max = a.iter().cloned().fold(0.0, f64::max);
        let mean = a.iter().sum::<f64>() / a.len() as f64;
        assert!(max >= 0.9 * h, "{who}: axis {axis} max angle {max}");
        assert!(
            (0.4 * h..=0.6 * h).contains(&mean),
            "{who}: axis {axis} mean {mean}"
        );
    }
    let d: Vec<DVec3> = ms.iter().map(|m| m.centre1 - m.centre0).collect();
    if shatter {
        for (i, m) in ms.iter().enumerate() {
            assert!(
                m.centre1.y.abs() < 1e-5,
                "{who}: face {i} centre y {}",
                m.centre1.y
            );
        }
        for (name, comp) in [("x", 0), ("z", 2)] {
            let v: Vec<f64> = d.iter().map(|d| d[comp]).collect();
            assert!(
                v.iter().all(|x| x.abs() <= 50.0 + 1e-6),
                "{who}: d{name} beyond 50"
            );
            let s = std(&v);
            assert!((14.0..=26.0).contains(&s), "{who}: d{name} std {s}");
        }
    } else {
        let l: Vec<f64> = d.iter().map(|d| d.length()).collect();
        let max = l.iter().cloned().fold(0.0, f64::max);
        assert!(
            (0.9 * 750.0..=750.0 + 1e-6).contains(&max),
            "{who}: max |d| {max}"
        );
        let mean = l.iter().sum::<f64>() / l.len() as f64;
        assert!((250.0..=500.0).contains(&mean), "{who}: mean |d| {mean}");
        let radial = ms
            .iter()
            .zip(&d)
            .map(|(m, d)| d.normalize().dot(m.centre0.normalize()))
            .sum::<f64>()
            / ms.len() as f64;
        assert!(
            (0.25..=0.6).contains(&radial),
            "{who}: radial bias {radial}"
        );
    }
}

/// The invariants of spec §12 for wind at frame 10 (defaults).
fn check_wind(who: &str, orig: &Mesh, got: &Mesh) {
    assert_eq!(orig.faces, got.faces, "{who}: faces");
    assert_eq!(orig.positions.len(), got.positions.len(), "{who}: points");
    let mut by_xy: Vec<((i64, i64), f64)> = Vec::new();
    for (a, b) in orig.positions.iter().zip(&got.positions) {
        assert!(
            (a[0] - b[0]).abs() < 1e-9 && (a[1] - b[1]).abs() < 1e-9,
            "{who}: x/y moved"
        );
        let dz = b[2] - a[2];
        assert!(dz.abs() <= 75.0 + 1e-6, "{who}: |dz| {dz}");
        if a[0] == 0.0 {
            assert!(dz.abs() < 1e-9, "{who}: x = 0 moved by {dz}");
        } else {
            assert!(dz * a[0] > 0.0 || dz.abs() < 1e-9, "{who}: sign");
        }
        let key = (
            (a[0] * 1000.0).round() as i64,
            (a[1] * 1000.0).round() as i64,
        );
        match by_xy.iter().find(|(k, _)| *k == key) {
            Some((_, prev)) => assert!((prev - dz).abs() < 1e-6, "{who}: dz depends on z"),
            None => by_xy.push((key, dz)),
        }
    }
    assert!(
        by_xy.iter().any(|(_, dz)| dz.abs() > 10.0),
        "{who}: nothing moved"
    );
}

#[test]
fn explosion_shatter_wind_match_the_reference() {
    let mut reg = Registry::with_builtins();
    reg.install(&Primitives).unwrap();
    reg.install(&Deformers).unwrap();
    let reg = Arc::new(reg);
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden");
    let mut exact = Vec::new();
    let mut statistical = Vec::new();
    for case in discover_cases(&root).unwrap() {
        if !FEATURES.contains(&case.feature.as_str()) {
            continue;
        }
        let key = format!("{}/{}", case.feature, case.name);
        let m = case.manifest().unwrap();
        let params: Table = m.params.clone().into_iter().collect();
        let ctx = ctx(&case, &params);

        let eval = |deformed: bool| {
            let (doc, id) = build(&reg, &case, &params, deformed);
            evaluate(doc.scene(), id, &ctx).unwrap().expect("geometry")
        };
        let ours = to_mesh(&eval(true));
        let orig = to_mesh(&eval(false));
        let expected = case.expected_mesh().unwrap().unwrap();

        let r = compare_meshes(&export_weld(&expected), &export_weld(&ours), &m.tolerance);
        if KNOWN_MISMATCH.contains(&key.as_str()) {
            assert!(
                !r.passed(),
                "{key} matches exactly now: remove it from KNOWN_MISMATCH"
            );
            let shatter = case.feature == "shatter";
            if case.feature == "wind" {
                check_wind(&format!("{key} (golden)"), &orig, &expected);
                check_wind(&format!("{key} (ours)"), &orig, &ours);
            } else {
                check_blast(&format!("{key} (golden)"), shatter, &orig, &expected);
                check_blast(&format!("{key} (ours)"), shatter, &orig, &ours);
            }
            statistical.push(key);
            continue;
        }
        assert!(
            r.passed(),
            "{key}: {:?} (max error {:?})",
            r.failures,
            r.max_position_error
        );
        exact.push(key);
    }
    println!("exact: {exact:?}; statistical: {statistical:?}");
    assert_eq!(exact.len(), 2, "{exact:?}");
    assert_eq!(statistical.len(), 3, "{statistical:?}");
}
