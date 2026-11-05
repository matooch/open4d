//! Parity: docs/specs/MODIFIERS.md §20–21 (brief 055) against
//! tests/golden/{spline-wrap,spline-deformer,spline-rail}.
//!
//! The linked splines are not polygons, so the golden OBJ holds the cube only. Every case
//! is compared strictly (`compare_meshes` after `export_weld`) or pinned in `KNOWN_MISMATCH`
//! (the exact comparison must still fail) with the invariants the data supports.
//! `tests/golden/{wrap,wrap-f}` are the plain Wrap deformer's and are not discovered here.

use std::path::Path;
use std::sync::Arc;

use forge_core::{Document, EvalCtx, NodeId, ParamKind, Registry, Tx, Value};
use forge_generators::{Primitives, evaluate};
use forge_modifiers::Deformers;
use forge_parity_harness::{GoldenCase, Mesh, compare_meshes, discover_cases, export_weld};
use glam::DVec3;

const FEATURES: &[&str] = &["spline-wrap", "spline-deformer", "spline-rail"];

/// Cases that build but differ from the golden (reasons in MODIFIERS.md §20).
const KNOWN_MISMATCH: &[&str] = &[
    // the helix frame roll (20.1)
    "spline-wrap/defaults",
    // the whole rule matches, but the golden extends 99 spline lengths: the sampler's
    // last-digit error of the Arc's end point is amplified to 2e-3 (20.1)
    "spline-wrap/arc",
    // which spline points pair up (20.2)
    "spline-deformer/defaults",
];

/// Observed options we do not build.
const UNSUPPORTED: &[&str] = &[];

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

fn enum_id(s: &str) -> String {
    let s = s.replace('\u{2212}', "-");
    match s.as_str() {
        "+X" => "pos_x".into(),
        "+Y" => "pos_y".into(),
        "+Z" => "pos_z".into(),
        "-X" => "neg_x".into(),
        "-Y" => "neg_y".into(),
        "-Z" => "neg_z".into(),
        other => other.to_lowercase().replace(' ', "_"),
    }
}

/// Our key for a case-file key: `*_percent` and `*_deg` lose their suffix, the spline's
/// Intermediate Points settings have their own names.
fn our_key(k: &str) -> (String, f64) {
    let deg = std::f64::consts::PI / 180.0;
    match k {
        "intermediate_points" => ("intermediate".into(), 1.0),
        "number" => ("intermediate_number".into(), 1.0),
        "angle_deg" => ("intermediate_angle".into(), deg),
        "maximum_length" => ("intermediate_max_length".into(), 1.0),
        k => match (k.strip_suffix("_percent"), k.strip_suffix("_deg")) {
            (Some(b), _) => (b.into(), 1.0),
            (_, Some(b)) => (b.into(), deg),
            _ => (k.into(), 1.0),
        },
    }
}

/// Sets the keys of a `[params.*]` table that are parameters of `ty_id`.
fn set_params(tx: &mut Tx, reg: &Registry, id: NodeId, ty_id: &str, table: &Table) {
    let ty = reg.get(ty_id).unwrap();
    for (k, v) in table {
        match k.as_str() {
            "position" => {
                tx.set_param(id, "transform.pos", Value::Vec3(vec3(v)))
                    .unwrap();
                continue;
            }
            "rotation_deg" => {
                tx.set_param(
                    id,
                    "transform.rot",
                    Value::Vec3(vec3(v).map(f64::to_radians)),
                )
                .unwrap();
                continue;
            }
            _ => {}
        }
        let (key, factor) = our_key(k);
        let Some(desc) = ty.find_param(&key) else {
            continue; // `type`, `spline_type`, `plane` of a different spelling, ...
        };
        let value = match (&desc.kind, v) {
            (ParamKind::Int { .. }, toml::Value::Integer(i)) => Value::Int(*i),
            (ParamKind::Float { .. }, v) if v.is_float() || v.is_integer() => {
                Value::Float(num(v) * factor)
            }
            (ParamKind::Bool, toml::Value::Boolean(b)) => Value::Bool(*b),
            (ParamKind::Enum { .. }, toml::Value::String(s)) => Value::Enum(enum_id(s)),
            (ParamKind::Vec3 { .. }, v) if v.is_array() => Value::Vec3(vec3(v)),
            _ => continue,
        };
        tx.set_param(id, &key, value).unwrap();
    }
}

/// The spline type of a `[params.*]` helper table.
fn spline_type_id(t: &Table) -> &'static str {
    match t["type"].as_str().unwrap() {
        "helix" => "forge.helix_spline",
        "arc" => "forge.arc_spline",
        "spline" => match t.get("spline_type").and_then(|v| v.as_str()) {
            Some("Arc") => "forge.arc_spline",
            // spline-wrap/defaults names no kind; its keys are the Helix's
            None if t.contains_key("start_radius") => "forge.helix_spline",
            other => panic!("spline type {other:?}"),
        },
        other => panic!("helper type {other}"),
    }
}

fn spline(tx: &mut Tx, reg: &Registry, t: &Table) -> NodeId {
    let ty = spline_type_id(t);
    let id = tx.create(ty, None, None).unwrap();
    set_params(tx, reg, id, ty, t);
    id
}

/// How the deformer's box is given in the Spline Wrap cases.
#[derive(Clone, Copy, PartialEq)]
enum BoxMode {
    /// As recorded in the case file (no fixed box: the object's own extent).
    Plain,
    /// The recorded export had no bounding box (`t = z`): a fixed box `min 0, size 1` along z.
    Recorded,
}

struct Built {
    doc: Document,
    cube: NodeId,
    deformer: NodeId,
}

fn build(reg: &Arc<Registry>, case: &GoldenCase, params: &Table, bx: BoxMode) -> Built {
    let feature = case.feature.as_str();
    let mut doc = Document::new(reg.clone());
    let (cube, deformer) = doc
        .transact("case", |tx| {
            let cube = tx.create("forge.cube", None, None)?;
            set_params(
                tx,
                reg,
                cube,
                "forge.cube",
                params["input"].as_table().unwrap(),
            );
            let dty = match feature {
                "spline-wrap" => "forge.spline_wrap",
                "spline-deformer" => "forge.spline_deformer",
                _ => "forge.spline_rail",
            };
            let d = tx.create(dty, Some(cube), None)?;
            set_params(tx, reg, d, dty, params);
            let link = |tx: &mut Tx, key: &str, id: NodeId| {
                tx.set_param(d, key, Value::Link(Some(id))).unwrap();
            };
            match feature {
                "spline-wrap" => {
                    let s = spline(tx, reg, params["spline"].as_table().unwrap());
                    link(tx, "spline", s);
                    if bx == BoxMode::Recorded {
                        tx.set_param(d, "fix_bounding_box", Value::Bool(true))?;
                        tx.set_param(d, "bounding_box_center", Value::Vec3([0.0, 0.0, 0.5]))?;
                        tx.set_param(d, "bounding_box_size", Value::Vec3([200.0, 200.0, 1.0]))?;
                    }
                }
                "spline-deformer" => {
                    let a = spline(tx, reg, params["original_spline"].as_table().unwrap());
                    let b = spline(tx, reg, params["modifying_spline"].as_table().unwrap());
                    link(tx, "original_spline", a);
                    link(tx, "modifying_spline", b);
                }
                _ => {
                    let l = spline(tx, reg, params["left_helix"].as_table().unwrap());
                    let r = spline(tx, reg, params["right_helix"].as_table().unwrap());
                    link(tx, "z_spline_left", l);
                    link(tx, "z_spline_right", r);
                }
            }
            Ok((cube, d))
        })
        .unwrap();
    Built {
        doc,
        cube,
        deformer,
    }
}

fn cube_mesh(b: &mut Built, enabled: bool) -> Mesh {
    let d = b.deformer;
    b.doc
        .transact("enable", |tx| {
            tx.set_param(d, "enabled", Value::Bool(enabled))
        })
        .unwrap();
    let m = evaluate(b.doc.scene(), b.cube, &EvalCtx::default())
        .unwrap()
        .unwrap();
    Mesh {
        positions: m.points.iter().map(|p| p.to_array()).collect(),
        faces: m.polys,
    }
}

fn registry() -> Arc<Registry> {
    let mut reg = Registry::with_builtins();
    reg.install(&Primitives).unwrap();
    reg.install(&Deformers).unwrap();
    Arc::new(reg)
}

fn v(m: &Mesh, i: usize) -> DVec3 {
    DVec3::from_array(m.positions[i])
}

/// The index of cube point `(x, y, z)` in the 4/8/4 cube listing of the cases (y fastest).
fn column(x: i32, z: i32) -> usize {
    let xs = [-100, -50, 0, 50, 100];
    let ring: Vec<(i32, i32)> = xs
        .iter()
        .map(|x| (*x, -100))
        .chain([-50, 0, 50, 100].map(|z| (100, z)))
        .chain([50, 0, -50, -100].map(|x| (x, 100)))
        .chain([50, 0, -50].map(|z| (-100, z)))
        .collect();
    ring.iter().position(|c| *c == (x, z)).expect("ring point") * 9
}

#[test]
fn spline_deformers_match_the_reference() {
    let reg = registry();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden");
    let (mut exact, mut pinned) = (Vec::new(), Vec::new());
    for case in discover_cases(&root).unwrap() {
        if !FEATURES.contains(&case.feature.as_str()) {
            continue;
        }
        let key = format!("{}/{}", case.feature, case.name);
        assert!(!UNSUPPORTED.contains(&key.as_str()));
        let m = case.manifest().unwrap();
        let params: Table = m.params.clone().into_iter().collect();
        let expected = case.expected_mesh().unwrap().unwrap();

        let mut b = build(&reg, &case, &params, BoxMode::Plain);
        let ours = cube_mesh(&mut b, true);
        let orig = cube_mesh(&mut b, false);
        // a deformer never changes the topology
        assert_eq!(ours.faces, orig.faces, "{key}: topology");
        assert_eq!(ours.faces, expected.faces, "{key}: golden topology");
        assert!(ours.positions.iter().flatten().all(|c| c.is_finite()));

        // Spline Wrap is compared under the box the recorded export had (§20.1): the case
        // is exact only there; with the object's own box it is a different (sensible) wrap.
        let (ours_cmp, plain_matches) = if case.feature == "spline-wrap" {
            let mut r = build(&reg, &case, &params, BoxMode::Recorded);
            let recorded = cube_mesh(&mut r, true);
            let plain = compare_meshes(&export_weld(&expected), &export_weld(&ours), &m.tolerance);
            (recorded, plain.passed())
        } else {
            (ours.clone(), false)
        };
        assert!(
            !plain_matches,
            "{key}: matches with the object's own box; the recorded box is a degenerate one"
        );
        let r = compare_meshes(
            &export_weld(&expected),
            &export_weld(&ours_cmp),
            &m.tolerance,
        );
        if KNOWN_MISMATCH.contains(&key.as_str()) {
            assert!(
                !r.passed(),
                "{key} matches exactly now: remove it from KNOWN_MISMATCH"
            );
            check_invariants(&key, &orig, &ours_cmp, &expected);
            pinned.push(key);
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
    println!("exact: {exact:?}; pinned: {pinned:?}");
    assert_eq!(exact, ["spline-rail/defaults"]);
    assert_eq!(pinned.len(), 3, "{pinned:?}");
}

/// What the golden data supports for the pinned cases, asserted on the golden and on ours.
fn check_invariants(key: &str, orig: &Mesh, ours: &Mesh, gold: &Mesh) {
    match key {
        "spline-wrap/defaults" => {
            // (a) the offsets keep unit length and the three columns stay orthogonal:
            // in the golden and in ours, the y step of a column is a unit vector `B`,
            // the x step `N` too, and `N ⟂ B`
            for m in [gold, ours] {
                for z in [-100, 100] {
                    let c0 = column(-100, z);
                    let c1 = column(100, z);
                    let b = (v(m, c0 + 8) - v(m, c0)) / 400.0;
                    let n = (v(m, c1) - v(m, c0)) / 200.0;
                    assert!((b.length() - 1.0).abs() < 1e-6, "{key}: |B| {}", b.length());
                    assert!((n.length() - 1.0).abs() < 1e-6, "{key}: |N| {}", n.length());
                    assert!(n.dot(b).abs() < 1e-6, "{key}: N.B {}", n.dot(b));
                }
            }
            // (b) the start rows extend along the first chord of the sampled helix, in the
            // golden and in ours: row z = -100 lies 100 L behind row z = 0
            let t0 = DVec3::new(-0.0069667, 0.996813, -0.0794711);
            for m in [gold, ours] {
                let d = v(m, column(100, -100) + 4) - v(m, column(100, 0) + 4);
                assert!((d.normalize() + t0).length() < 1e-4, "{key}: start {d:?}");
            }
            // (c) the start frame of ours is within 0.05 rad of the golden's (measured:
            // 0.0066 rad, 20.1) and the end frame within 0.1 rad (measured 0.029 + 0.0066)
            let normal = |m: &Mesh, z: i32| {
                (v(m, column(100, z)) - v(m, column(-100, z)).lerp(v(m, column(100, z)), 0.0))
                    .normalize()
            };
            let _ = normal;
            let n_at = |m: &Mesh, z: i32| {
                let c = column(100, z);
                let d = column(-100, z);
                (v(m, c) - v(m, d)).normalize()
            };
            for (z, bound) in [(0, 0.05), (100, 0.1)] {
                let angle = n_at(gold, z).angle_between(n_at(ours, z));
                assert!(angle < bound, "{key}: normal angle at z {z}: {angle}");
            }
            assert!(orig.positions != ours.positions);
        }
        "spline-wrap/arc" => {
            // the rows behind the start are exact, everything is within the sampler's
            // accuracy times the 99 lengths of extension
            let mut worst = 0.0f64;
            for z in [-100, -50, 0] {
                for x in [-100, 100] {
                    let i = column(x, z);
                    for k in 0..9 {
                        let e = (v(ours, i + k) - v(gold, i + k)).length();
                        assert!(e < 1e-4, "{key}: start row z {z}: {e}");
                    }
                }
            }
            for i in 0..ours.positions.len() {
                worst = worst.max((v(ours, i) - v(gold, i)).length());
            }
            assert!(worst < 5e-3, "{key}: worst error {worst}");
            assert!(worst > 1e-4);
        }
        "spline-deformer/defaults" => {
            let near = |i: usize| {
                // distance to the original Arc (radius 200, quarter circle, z = 0 plane)
                let p = v(orig, i);
                let r = (p.x * p.x + p.y * p.y).sqrt();
                let ang = p.y.atan2(p.x);
                if (0.0..=std::f64::consts::FRAC_PI_2).contains(&ang) {
                    ((r - 200.0).powi(2) + p.z * p.z).sqrt()
                } else {
                    f64::INFINITY
                }
            };
            let moved = |m: &Mesh| -> Vec<usize> {
                (0..orig.positions.len())
                    .filter(|i| (v(m, *i) - v(orig, *i)).length() > 1e-9)
                    .collect()
            };
            let within: Vec<usize> = (0..orig.positions.len())
                .filter(|i| near(*i) < 30.0)
                .collect();
            // the golden moves exactly the points within `radius` of the original spline
            assert_eq!(moved(gold), within, "{key}: golden moved set");
            assert_eq!(within.len(), 4, "{key}: four points inside the radius");
            // ours moves the same ones, and no other
            assert_eq!(moved(ours), within, "{key}: our moved set");
            // golden: each moves along (0, -1, -1) by d, d falls with the distance to the spline
            let mut by_dist: Vec<(f64, f64)> = Vec::new();
            for i in &within {
                let d = v(gold, *i) - v(orig, *i);
                assert!(d.x.abs() < 1e-9 && (d.y - d.z).abs() < 1e-9 && d.y < 0.0);
                by_dist.push((near(*i), -d.y));
            }
            by_dist.sort_by(|a, b| a.0.total_cmp(&b.0));
            assert!((by_dist[0].0).abs() < 1e-9 && (by_dist[0].1 - 200.0).abs() < 1e-3);
            assert!(by_dist.windows(2).all(|w| w[0].1 > w[1].1), "{by_dist:?}");
            // ours: weights fall with the distance as well and the nearest moves most
            let mut ours_by: Vec<(f64, f64)> = within
                .iter()
                .map(|i| (near(*i), (v(ours, *i) - v(orig, *i)).length()))
                .collect();
            ours_by.sort_by(|a, b| a.0.total_cmp(&b.0));
            assert!(ours_by.windows(2).all(|w| w[0].1 >= w[1].1), "{ours_by:?}");
        }
        other => panic!("no invariants for {other}"),
    }
}
