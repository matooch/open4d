//! Parity: docs/specs/MODIFIERS.md §20–21 (brief 055) against
//! tests/golden/{spline-wrap,spline-deformer,spline-rail}.
//!
//! The linked splines are not polygons, so the golden OBJ holds the cube only. Every case
//! is compared strictly (`compare_meshes` after `export_weld`) or pinned in `KNOWN_MISMATCH`
//! (the exact comparison must still fail) with the invariants the data supports.
//! `tests/golden/{wrap,wrap-f}` are the plain Wrap deformer's and are not discovered here.

mod common;

use std::path::Path;
use std::sync::Arc;

use forge_core::{Document, EvalCtx, NodeId, ParamKind, Registry, Tx, Value};
use forge_generators::editable_spline::SplineData;
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
    // the weights between 0 and Radius (the same pairing is exact, only the weights are 0.005 off)
    "spline-deformer/modifier-moved",
    // the role of Radius: points 50 away stay put at Radius 60 (20.2)
    "spline-deformer/radius-60",
    // with one Z spline linked the reference uses the unlinked helper of the scene (20.3)
    "spline-rail/one-spline",
    // the rule of the X splines (20.3)
    "spline-rail/x-splines",
];

/// Observed options we do not build.
const UNSUPPORTED: &[&str] = &[];

use common::forms::{Table, normalize, num, vec3};

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
            (ParamKind::Bool, toml::Value::Integer(i)) => Value::Bool(*i != 0),
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

/// A hand-made (editable) Linear spline from `points` / `closed`, placed by `position` /
/// `rotation_deg`.
fn editable_spline(tx: &mut Tx, t: &Table) -> NodeId {
    let id = tx.create("forge.spline", None, None).unwrap();
    let points = t["points"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| DVec3::from_array(vec3(p)));
    let data = SplineData::from_points(points.collect(), t["closed"].as_bool().unwrap());
    for (k, v) in data.to_values() {
        tx.set_param(id, &k, v).unwrap();
    }
    tx.set_param(id, "type", Value::Enum("linear".into()))
        .unwrap();
    if let Some(v) = t.get("position") {
        tx.set_param(id, "transform.pos", Value::Vec3(vec3(v)))
            .unwrap();
    }
    if let Some(v) = t.get("rotation_deg") {
        tx.set_param(
            id,
            "transform.rot",
            Value::Vec3(vec3(v).map(f64::to_radians)),
        )
        .unwrap();
    }
    id
}

fn spline(tx: &mut Tx, reg: &Registry, t: &Table) -> NodeId {
    if t["type"].as_str() == Some("editable") {
        return editable_spline(tx, t);
    }
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
                    for (table, key) in [
                        ("left_helix", "z_spline_left"),
                        ("right_helix", "z_spline_right"),
                        ("x_top", "x_spline_top"),
                        ("x_bottom", "x_spline_bottom"),
                    ] {
                        if let Some(t) = params.get(table).and_then(|t| t.as_table()) {
                            let id = spline(tx, reg, t);
                            link(tx, key, id);
                        }
                    }
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

/// Drops the vertices no face uses (the staged cases with editable helper splines export the
/// splines' own points, 4 unused vertices) and renumbers the faces, keeping the order.
fn compact(m: Mesh) -> Mesh {
    let mut used = vec![false; m.positions.len()];
    for &i in m.faces.iter().flatten() {
        used[i as usize] = true;
    }
    let mut map = vec![0u32; used.len()];
    let mut positions = Vec::new();
    for (i, p) in m.positions.iter().enumerate() {
        if used[i] {
            map[i] = positions.len() as u32;
            positions.push(*p);
        }
    }
    let faces = m
        .faces
        .iter()
        .map(|f| f.iter().map(|&i| map[i as usize]).collect())
        .collect();
    Mesh { positions, faces }
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
        let params = normalize(&m.params.clone().into_iter().collect());
        let expected = compact(case.expected_mesh().unwrap().unwrap());

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
    assert_eq!(
        exact,
        [
            "spline-deformer/lines",
            "spline-deformer/use-length",
            "spline-rail/defaults",
            "spline-rail/limited",
            "spline-rail/scale-before-50",
            "spline-rail/uneven",
            "spline-rail/within-box",
        ]
    );
    assert_eq!(pinned.len(), 7, "{pinned:?}");
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
        "spline-deformer/modifier-moved" => {
            // The original Arc is the one of `defaults`; the modifier is the same Arc moved up by 50:
            // the same four points move (within the radius 30), only along +y, by at most 50 and
            // by less the farther they are from the spline, in the golden and in ours.
            let near = |i: usize| {
                let p = v(orig, i);
                let ang = p.y.atan2(p.x);
                if (0.0..=std::f64::consts::FRAC_PI_2).contains(&ang) {
                    (((p.x * p.x + p.y * p.y).sqrt() - 200.0).powi(2) + p.z * p.z).sqrt()
                } else {
                    f64::INFINITY
                }
            };
            for (who, m) in [("golden", gold), ("ours", ours)] {
                let mut by: Vec<(f64, f64)> = Vec::new();
                for i in 0..orig.positions.len() {
                    let d = v(m, i) - v(orig, i);
                    if d.length() > 1e-9 {
                        assert!(
                            near(i) < 30.0,
                            "{key} {who}: moved point {i} at {}",
                            near(i)
                        );
                        assert!(d.x.abs() < 1e-9 && d.z.abs() < 1e-9, "{key} {who}: {d:?}");
                        assert!(d.y > 0.0 && d.y <= 50.0 + 1e-9, "{key} {who}: {d:?}");
                        by.push((near(i), d.y));
                    }
                }
                assert_eq!(by.len(), 4, "{key} {who}: moved points");
                by.sort_by(|a, b| a.0.total_cmp(&b.0));
                assert!(
                    by.windows(2).all(|w| w[0].1 >= w[1].1),
                    "{key} {who}: {by:?}"
                );
                assert!((by[0].1 - 50.0).abs() < 0.1, "{key} {who}: nearest {by:?}");
            }
        }
        "spline-deformer/radius-60" => {
            // The golden moves only the one point on the original line, by the full 100; the
            // points 50 away do not move although 50 < Radius 60 (pending: the role of Radius).
            // Ours moves that point the same way, and more.
            let moved = |m: &Mesh| -> Vec<usize> {
                (0..orig.positions.len())
                    .filter(|i| (v(m, *i) - v(orig, *i)).length() > 1e-9)
                    .collect()
            };
            let gm = moved(gold);
            assert_eq!(gm.len(), 1, "{key}: golden moved {gm:?}");
            let d = v(gold, gm[0]) - v(orig, gm[0]);
            assert!(
                (d - DVec3::new(0.0, 100.0, 0.0)).length() < 1e-4,
                "{key}: {d:?}"
            );
            let d = v(ours, gm[0]) - v(orig, gm[0]);
            assert!(
                (d - DVec3::new(0.0, 100.0, 0.0)).length() < 1e-4,
                "{key}: ours {d:?}"
            );
        }
        "spline-rail/one-spline" | "spline-rail/x-splines" => {
            // y is carried in the world y direction by every case of the rail (20.3): the
            // offset `y' - y` is the same for every y of one (x, z) column, golden and ours
            // (the helices' own y is 0 or the end rise of -0.25).
            for (who, m) in [("golden", gold), ("ours", ours)] {
                for i in 0..orig.positions.len() {
                    let (o, p) = (v(orig, i), v(m, i));
                    let j = (0..orig.positions.len())
                        .find(|j| {
                            let q = v(orig, *j);
                            q.x == o.x && q.z == o.z && q.y == -200.0
                        })
                        .unwrap();
                    let base = v(m, j).y + 200.0;
                    assert!((p.y - (base + o.y)).abs() < 1e-3, "{key} {who}: y at {o:?}");
                }
            }
            // X Spline Top and Bottom (straight lines along x at z = 0) make every point's
            // z come out 0 (pending: the rule of the X splines).
            if key == "spline-rail/x-splines" {
                assert!(gold.positions.iter().all(|p| p[2].abs() < 1e-6), "{key}");
            }
        }
        other => panic!("no invariants for {other}"),
    }
}
