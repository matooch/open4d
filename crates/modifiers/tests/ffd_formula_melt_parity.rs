//! Parity: docs/specs/MODIFIERS.md §16 (FFD), §17 (Formula), §18 (Melt) and §19 against
//! tests/golden/{ffd,formula,melt}.
//!
//! FFD and Formula match exactly. The reference Melt noise is not reproduced, so the two
//! Melt cases are pinned in `KNOWN_MISMATCH` (the exact comparison must still fail) and
//! checked by the structural invariants of spec §18, on the golden mesh and on ours.

use std::path::Path;
use std::sync::Arc;

use forge_core::{Document, EvalCtx, NodeId, Registry, Value};
use forge_generators::{Primitives, evaluate};
use forge_modifiers::Deformers;
use forge_modifiers::ffd::regular_grid;
use forge_parity_harness::{GoldenCase, Mesh, compare_meshes, discover_cases, export_weld};
use glam::DVec3;

const FEATURES: &[&str] = &["ffd", "formula", "melt"];

/// Cases whose exact comparison must still fail (Melt noise not reproduced, spec §18).
const KNOWN_MISMATCH: &[&str] = &["melt/defaults", "melt/strength"];

/// Observed options we do not derive (none: the Formula effects other than Y Radial are
/// unobserved, not unsupported).
const UNSUPPORTED: &[&str] = &[];

/// One Melt case: its key, then `F` of the cap points for the golden and for ours.
type CapF = (String, Vec<Option<f64>>, Vec<Option<f64>>);

type Table = toml::map::Map<String, toml::Value>;

fn num(v: &toml::Value) -> f64 {
    v.as_float()
        .or(v.as_integer().map(|i| i as f64))
        .expect("number")
}

fn vec3(v: &toml::Value) -> [f64; 3] {
    let a = v.as_array().expect("vec3");
    [num(&a[0]), num(&a[1]), num(&a[2])]
}

/// The `[params.input]` cube.
fn build_cube(tx: &mut forge_core::Tx, input: &Table) -> NodeId {
    assert_eq!(input["type"].as_str(), Some("cube"));
    let cube = tx.create("forge.cube", None, None).unwrap();
    for (k, v) in input {
        if k == "type" {
            continue;
        }
        let value = match v {
            toml::Value::Integer(i) if k.starts_with("segments") => Value::Int(*i),
            other => Value::Float(num(other)),
        };
        tx.set_param(cube, k, value).unwrap();
    }
    cube
}

/// The case's deformer parameters in our names; returns the frame (default 0) and fps.
fn set_deformer(tx: &mut forge_core::Tx, id: NodeId, feature: &str, params: &Table) -> (i64, f64) {
    let (mut frame, mut fps) = (0, 30.0);
    let mut moved: (Option<usize>, Option<[f64; 3]>) = (None, None);
    for (k, v) in params {
        if v.is_table() {
            continue;
        }
        match (feature, k.as_str()) {
            (_, "frame") => frame = v.as_integer().unwrap(),
            (_, "fps") => fps = num(v),
            // Observer bookkeeping, not parameters.
            ("ffd", "pos" | "rot" | "point_count" | "point62_before") => {}
            ("ffd", "moved_point_index" | "point_index_moved") => {
                moved.0 = Some(v.as_integer().unwrap() as usize)
            }
            ("ffd", "moved_point_delta" | "point_moved_by") => moved.1 = Some(vec3(v)),
            ("ffd", "grid_size") => tx.set_param(id, k, Value::Vec3(vec3(v))).unwrap(),
            ("ffd", _) => tx
                .set_param(id, k, Value::Int(v.as_integer().unwrap()))
                .unwrap(),
            ("formula", "size") => tx.set_param(id, k, Value::Vec3(vec3(v))).unwrap(),
            ("formula", "effect") => {
                let id_of = v.as_str().unwrap().to_lowercase().replace(' ', "_");
                tx.set_param(id, k, Value::Enum(id_of)).unwrap()
            }
            ("formula", _) => tx
                .set_param(id, k, Value::String(v.as_str().unwrap().to_string()))
                .unwrap(),
            ("melt", _) => {
                let key = k.strip_suffix("_percent").unwrap_or(k);
                tx.set_param(id, key, Value::Float(num(v))).unwrap()
            }
            other => panic!("unexpected key {other:?}"),
        }
    }
    if feature == "ffd" {
        // The observer moved one lattice point by a delta (deformer at the origin).
        let get = |axis: &str| params[axis].as_integer().unwrap() as usize;
        let counts = [
            get("grid_points_x"),
            get("grid_points_y"),
            get("grid_points_z"),
        ];
        let size = DVec3::from_array(vec3(&params["grid_size"]));
        let mut grid = regular_grid(size, counts);
        if let (Some(i), Some(d)) = moved {
            grid[i] += DVec3::from_array(d);
        }
        let list = grid.iter().map(|p| p.to_array()).collect();
        tx.set_param(id, "grid", Value::Vec3List(list)).unwrap();
    }
    (frame, fps)
}

struct Built {
    doc: Document,
    cube: NodeId,
    ctx: EvalCtx,
}

fn build(reg: &Arc<Registry>, case: &GoldenCase, params: &Table) -> Built {
    let mut doc = Document::new(reg.clone());
    let mut ctx = EvalCtx::default();
    let cube = doc
        .transact("case", |tx| {
            let cube = build_cube(tx, params["input"].as_table().unwrap());
            let d = tx.create(&format!("forge.{}", case.feature), Some(cube), None)?;
            let (frame, fps) = set_deformer(tx, d, &case.feature, params);
            ctx = EvalCtx::at_frame(frame, fps);
            Ok(cube)
        })
        .unwrap();
    Built { doc, cube, ctx }
}

fn undeformed_cube(reg: &Arc<Registry>, params: &Table) -> Vec<DVec3> {
    let mut doc = Document::new(reg.clone());
    let cube = doc
        .transact("cube", |tx| {
            Ok(build_cube(tx, params["input"].as_table().unwrap()))
        })
        .unwrap();
    evaluate(doc.scene(), cube, &EvalCtx::default())
        .unwrap()
        .unwrap()
        .points
}

/// The structure of spec §18 (facts 1 and 3) on one Melt result against the cube: `strength`
/// is the case's fraction, `other` the same cube melted at the other case's strength.
fn melt_structure(name: &str, cube: &[DVec3], out: &[DVec3]) {
    assert_eq!(cube.len(), out.len());
    for (p, q) in cube.iter().zip(out) {
        let (r0, r1) = (p.x.hypot(p.z), q.x.hypot(q.z));
        if r0 > 1e-9 {
            // Same direction in xz, farther from the axis.
            assert!(r1 > r0, "{name}: {p:?} -> {q:?} does not spread");
            assert!(
                (p.x * q.z - p.z * q.x).abs() < 1e-3 * r0 * r1,
                "{name}: direction of {p:?} changed to {q:?}"
            );
        }
        if p.y <= 0.0 {
            assert_eq!(q.y, 0.0, "{name}: {p:?} did not collapse");
        } else {
            assert!(q.y > 0.0 && q.y < p.y, "{name}: {p:?} -> {q:?}");
        }
    }
}

/// `ln(y/y')/(8 s²)` of the points of the top cap (`y = 200`), `None` when `y'` is too small
/// for the 6 decimals of the export.
fn cap_f(cube: &[DVec3], out: &[DVec3], s: f64) -> Vec<Option<f64>> {
    cube.iter()
        .zip(out)
        .filter(|(p, _)| p.y == 200.0)
        .map(|(p, q)| (q.y > 0.2).then(|| (p.y / q.y).ln() / (8.0 * s * s)))
        .collect()
}

/// `k − 1` per ray (by direction angle) of a Melt result.
fn spread(cube: &[DVec3], out: &[DVec3]) -> Vec<f64> {
    cube.iter()
        .zip(out)
        .filter(|(p, _)| p.x.hypot(p.z) > 1e-9 && p.y == -200.0)
        .map(|(p, q)| q.x.hypot(q.z) / p.x.hypot(p.z) - 1.0)
        .collect()
}

#[test]
fn ffd_formula_melt_match_the_reference() {
    let mut reg = Registry::with_builtins();
    reg.install(&Primitives).unwrap();
    reg.install(&Deformers).unwrap();
    let reg = Arc::new(reg);
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden");
    let (mut matched, mut pinned) = (Vec::new(), Vec::new());
    let mut unsupported = std::collections::BTreeSet::new();
    let mut melt_spreads: Vec<(String, Vec<f64>)> = Vec::new();
    let mut caps: Vec<CapF> = Vec::new();
    for case in discover_cases(&root).unwrap() {
        if !FEATURES.contains(&case.feature.as_str()) {
            continue;
        }
        let key = format!("{}/{}", case.feature, case.name);
        let m = case.manifest().unwrap();
        let params: Table = m.params.clone().into_iter().collect();
        let b = build(&reg, &case, &params);
        let mesh = match evaluate(b.doc.scene(), b.cube, &b.ctx) {
            Err(forge_generators::BuildError::Unsupported(what)) => {
                println!("{key}: unsupported ({what})");
                unsupported.insert(key);
                continue;
            }
            other => other
                .unwrap()
                .unwrap_or_else(|| panic!("{key}: no geometry")),
        };
        let world = b.doc.scene().world_matrix(b.cube).unwrap();
        let ours = export_weld(&Mesh {
            positions: mesh
                .points
                .iter()
                .map(|p| world.transform_point3(*p).to_array())
                .collect(),
            faces: mesh.polys.clone(),
        });
        let golden = case.expected_mesh().unwrap().unwrap();
        let expected = export_weld(&golden);
        let r = compare_meshes(&expected, &ours, &m.tolerance);
        if KNOWN_MISMATCH.contains(&key.as_str()) {
            assert!(
                !r.passed(),
                "{key} matches now: remove it from KNOWN_MISMATCH"
            );
            // Spec §18: the structure holds for the golden and for ours.
            let cube = undeformed_cube(&reg, &params);
            let gold: Vec<DVec3> = golden
                .positions
                .iter()
                .map(|p| DVec3::from_array(*p))
                .collect();
            assert_eq!(golden.faces, mesh.polys, "{key}: topology");
            // The exported vertex order is not ours: the undeformed export (`ffd/defaults`,
            // "every vertex is where a plain cube has it", O9) is the reference cube.
            let plain = discover_cases(&root)
                .unwrap()
                .into_iter()
                .find(|c| c.feature == "ffd" && c.name == "defaults")
                .and_then(|c| c.expected_mesh().unwrap())
                .unwrap();
            let plain: Vec<DVec3> = plain
                .positions
                .iter()
                .map(|p| DVec3::from_array(*p))
                .collect();
            melt_structure(&format!("{key} golden"), &plain, &gold);
            melt_structure(&format!("{key} ours"), &cube, &mesh.points);
            melt_spreads.push((key.clone(), spread(&plain, &gold)));
            let s = num(&params["strength_percent"]) / 100.0;
            // The axis point (0, 200, 0): 200·e^(−8 s²), golden and ours.
            for pts in [(&plain, &gold), (&cube, &mesh.points)] {
                let i = pts
                    .0
                    .iter()
                    .position(|p| *p == DVec3::new(0.0, 200.0, 0.0))
                    .unwrap();
                let want = 200.0 * (-8.0 * s * s).exp();
                assert!(
                    (pts.1[i].y - want).abs() < 1e-3,
                    "{key}: axis point {}",
                    pts.1[i].y
                );
            }
            caps.push((
                key.clone(),
                cap_f(&plain, &gold, s),
                cap_f(&cube, &mesh.points, s),
            ));
            pinned.push(key);
            continue;
        }
        assert!(
            r.passed(),
            "{key}: {:?} (max error {:?})",
            r.failures,
            r.max_position_error
        );
        matched.push(key);
    }
    println!("exact: {matched:?}; pinned: {pinned:?}");
    // Spec §18 fact 2: the spread scales with strength by one factor for every ray.
    let find = |k: &str| &melt_spreads.iter().find(|(n, _)| n == k).unwrap().1;
    let (a, b) = (find("melt/defaults"), find("melt/strength"));
    for (x, y) in a.iter().zip(b) {
        assert!((y / x - 1.6905).abs() < 5e-4, "ratio {} at {x}", y / x);
    }
    // Spec §18 fact 3: `F` on the cap does not depend on the strength (golden and ours).
    let cap = |k: &str| caps.iter().find(|(n, _, _)| n == k).unwrap();
    let (c25, c50) = (cap("melt/defaults"), cap("melt/strength"));
    for (which, f25, f50) in [("golden", &c25.1, &c50.1), ("ours", &c25.2, &c50.2)] {
        let mut compared = 0;
        for (x, y) in f25.iter().zip(f50) {
            if let (Some(x), Some(y)) = (x, y) {
                assert!((x - y).abs() < 2e-3 * x.max(1.0), "{which}: F {x} vs {y}");
                compared += 1;
            }
        }
        assert!(
            compared >= if which == "golden" { 14 } else { 5 },
            "{which}: only {compared} cap points compared"
        );
    }
    let want: std::collections::BTreeSet<String> =
        UNSUPPORTED.iter().map(|s| s.to_string()).collect();
    assert_eq!(unsupported, want, "unsupported set changed");
    assert_eq!(matched.len(), 3 + 2, "{matched:?}");
    assert_eq!(pinned.len(), KNOWN_MISMATCH.len(), "{pinned:?}");
}
