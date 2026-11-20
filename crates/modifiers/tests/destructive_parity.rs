//! Parity: docs/specs/MODIFIERS.md §11 (Explosion, Shatter), §12 (Wind) and §13 against
//! tests/golden/{explosion,shatter,wind}.
//!
//! The `defaults` cases (strength 0) are compared exactly. The reference draws its
//! per-polygon motions from a random stream we cannot reproduce, and its wind noise is
//! unknown, so every other case is pinned in `KNOWN_MISMATCH` (the exact comparison still
//! fails) and checked by the *same invariants* on the golden mesh and on ours (spec §11, §12):
//! the scale, rigidity, rotation range, offset bounds and the cases' deterministic parts
//! (Randomness 0, Speed 0) exactly.
//!
//! Cases are read through `common::normalize` (older and menu-created forms).

mod common;

use std::path::Path;
use std::sync::Arc;

use common::forms::{Table, normalize, num};
use forge_core::{Document, EvalCtx, NodeId, ParamKind, Registry, Value};
use forge_generators::{Primitives, evaluate};
use forge_modifiers::Deformers;
use forge_parity_harness::{GoldenCase, Mesh, compare_meshes, discover_cases, export_weld};
use glam::{DMat3, DVec3};

const FEATURES: &[&str] = &["explosion", "shatter", "wind"];

/// Cases whose exact comparison must still fail: the random stream of Explosion and Shatter
/// and the wind noise are not reproducible (spec §11, §12). Every case except the strength-0
/// `defaults` ones.
fn known_mismatch(key: &str) -> bool {
    !key.ends_with("/defaults")
}

/// The wind case file of the older form has no `frame` / `fps` keys; the form (O19) says
/// frame 10 at 30 fps.
const WIND_FRAME: i64 = 10;
const FPS: f64 = 30.0;

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
            (ParamKind::Bool, toml::Value::Integer(i)) => Value::Bool(*i != 0),
            (ParamKind::Int { .. }, toml::Value::Integer(i)) => Value::Int(*i),
            (ParamKind::Enum { options }, toml::Value::String(s)) => {
                let o = options
                    .iter()
                    .find(|o| o.label == *s)
                    .unwrap_or_else(|| panic!("{ty_id}.{k}: no option {s:?}"));
                Value::Enum(o.id.clone())
            }
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
            let ty = format!("forge.{}", input["type"].as_str().unwrap());
            let cube = tx.create(&ty, None, None)?;
            apply(tx, reg, cube, &ty, input);
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

/// What a blast case sets (spec §11): `s` strength fraction, `r` randomness fraction.
struct Blast {
    shatter: bool,
    s: f64,
    speed: f64,
    angle_speed: f64,
    end_size: f64,
    r: f64,
}

impl Blast {
    fn of(feature: &str, p: &Table) -> Blast {
        let get = |k: &str, d: f64| p.get(k).map_or(d, num);
        Blast {
            shatter: feature == "shatter",
            s: get("strength_percent", 0.0) / 100.0,
            speed: get("speed", 100.0),
            angle_speed: get("angle_speed_deg", 100.0),
            end_size: get("end_size", 0.0),
            r: get("randomness_percent", 100.0) / 100.0,
        }
    }

    /// Half range of every Euler angle, degrees (§11): `angle_speed · s (1 + s) / 2`.
    fn half_angle(&self) -> f64 {
        0.5 * self.angle_speed * self.s * (1.0 + self.s)
    }

    /// Scale of every polygon (§11): `1 + (end_size − 1) s`.
    fn scale(&self) -> f64 {
        1.0 + (self.end_size - 1.0) * self.s
    }
}

/// The invariants of spec §11 on one blast result: topology, scale, rigidity, the rotation range
/// (independent of Randomness), the offset bounds, and what Randomness 0 and Speed 0 fix exactly.
/// A polygon that shrinks to a point (scale 0) has no rotation to read. Panics with `who`.
fn check_blast(who: &str, b: &Blast, orig: &Mesh, got: &Mesh) {
    // Topology: every face owns its points, in order.
    assert_eq!(got.positions.len(), 4 * got.faces.len(), "{who}: points");
    for (i, f) in got.faces.iter().enumerate() {
        let want: Vec<u32> = (4 * i as u32..4 * i as u32 + 4).collect();
        assert_eq!(f, &want, "{who}: face {i}");
    }
    let k = b.scale();
    let degenerate = k.abs() < 1e-9;
    let ms = if degenerate {
        Vec::new()
    } else {
        motions(orig, got)
    };
    let h = b.half_angle();
    for (i, m) in ms.iter().enumerate() {
        assert!(
            (m.scale - k).abs() < 1e-5 * k.max(1.0),
            "{who}: face {i} scale {} want {k}",
            m.scale
        );
        assert!(
            m.rigid_error < 1e-5 * k.max(1.0),
            "{who}: face {i} not rigid ({})",
            m.rigid_error
        );
        for a in m.euler {
            assert!(a.abs() <= h + 0.05, "{who}: face {i} angle {a} beyond {h}");
        }
    }
    if ms.len() >= 100 {
        // Distribution over many polygons: each axis fills the range, mean |angle| = h/2.
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
    }
    // Centres from the points (a polygon of scale 0 is a point).
    let centre = |m: &Mesh, f: &[u32]| {
        f.iter()
            .map(|&i| DVec3::from_array(m.positions[i as usize]))
            .sum::<DVec3>()
            / f.len() as f64
    };
    let c0: Vec<DVec3> = orig.faces.iter().map(|f| centre(orig, f)).collect();
    let c1: Vec<DVec3> = got.faces.iter().map(|f| centre(got, f)).collect();
    let d: Vec<DVec3> = c1.iter().zip(&c0).map(|(a, b)| *a - *b).collect();
    if b.shatter {
        for (i, c) in c1.iter().enumerate() {
            let want = c0[i].y * (1.0 - (2.0 * b.s).min(1.0));
            assert!(
                (c.y - want).abs() < 1e-5,
                "{who}: face {i} centre y {}",
                c.y
            );
        }
        let bound = 100.0 * b.s * b.r;
        for (name, comp) in [("x", 0), ("z", 2)] {
            let v: Vec<f64> = d.iter().map(|d| d[comp]).collect();
            assert!(
                v.iter().all(|x| x.abs() <= bound + 1e-5),
                "{who}: d{name} beyond {bound}"
            );
            if b.r == 0.0 {
                assert!(v.iter().all(|x| x.abs() < 1e-5), "{who}: d{name} at R = 0");
            } else if v.len() >= 100 {
                let s = std(&v);
                assert!((14.0..=26.0).contains(&s), "{who}: d{name} std {s}");
            }
        }
    } else {
        let range = 15.0 * b.speed * b.s;
        let l: Vec<f64> = d.iter().map(|d| d.length()).collect();
        assert!(
            l.iter().all(|x| *x <= range + 1e-5),
            "{who}: |d| beyond {range}: {l:?}"
        );
        if b.speed == 0.0 {
            assert!(l.iter().all(|x| *x < 1e-5), "{who}: moved at speed 0");
        }
        if b.r == 0.0 {
            // Randomness 0: half the range, straight away from the origin.
            for (i, (d, c)) in d.iter().zip(&c0).enumerate() {
                assert!((d.length() - range / 2.0).abs() < 1e-5, "{who}: |d| {i}");
                if c.length() > 1e-9 {
                    assert!(
                        (d.normalize() - c.normalize()).length() < 1e-6,
                        "{who}: d {i} not radial"
                    );
                }
            }
        }
        if l.len() >= 100 {
            let max = l.iter().cloned().fold(0.0, f64::max);
            assert!(
                (0.9 * range..=range + 1e-6).contains(&max),
                "{who}: max |d| {max}"
            );
            let mean = l.iter().sum::<f64>() / l.len() as f64;
            assert!(
                (range / 3.0..=range * 2.0 / 3.0).contains(&mean),
                "{who}: mean |d| {mean}"
            );
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
}

/// The invariants of spec §12 on one wind result: topology, only z moves, the offset depends on
/// `(x, y)` only, finite and bounded; with Flag on it vanishes on the plane `x = 0`, with Flag off
/// it does not (every point of the golden cases moves).
fn check_wind(who: &str, flag: bool, amplitude: f64, orig: &Mesh, got: &Mesh) {
    assert_eq!(orig.faces, got.faces, "{who}: faces");
    assert_eq!(orig.positions.len(), got.positions.len(), "{who}: points");
    let mut by_xy: Vec<((i64, i64), f64)> = Vec::new();
    let mut on_axis = Vec::new();
    for (a, b) in orig.positions.iter().zip(&got.positions) {
        assert!(
            (a[0] - b[0]).abs() < 1e-5 && (a[1] - b[1]).abs() < 1e-5,
            "{who}: x/y moved"
        );
        let dz = b[2] - a[2];
        // Measured up to 2.55 amplitudes (Flag off, §12).
        assert!(dz.abs() <= 2.6 * amplitude + 1e-6, "{who}: |dz| {dz}");
        if a[0] == 0.0 {
            on_axis.push(dz);
        }
        let key = (
            (a[0] * 1000.0).round() as i64,
            (a[1] * 1000.0).round() as i64,
        );
        match by_xy.iter().find(|(k, _)| *k == key) {
            Some((_, prev)) => assert!((prev - dz).abs() < 1e-5, "{who}: dz depends on z"),
            None => by_xy.push((key, dz)),
        }
    }
    if flag {
        assert!(on_axis.iter().all(|d| d.abs() < 1e-5), "{who}: x = 0 moved");
    } else {
        assert!(
            on_axis.iter().all(|d| d.abs() > 1.0),
            "{who}: x = 0 pinned with Flag off"
        );
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
        let params = normalize(&m.params.clone().into_iter().collect());
        let ctx = ctx(&case, &params);

        let eval = |deformed: bool| {
            let (doc, id) = build(&reg, &case, &params, deformed);
            evaluate(doc.scene(), id, &ctx).unwrap().expect("geometry")
        };
        let ours = to_mesh(&eval(true));
        let orig = to_mesh(&eval(false));
        let expected = case.expected_mesh().unwrap().unwrap();

        let r = compare_meshes(&export_weld(&expected), &export_weld(&ours), &m.tolerance);
        if known_mismatch(&key) {
            assert!(
                !r.passed(),
                "{key} matches exactly now: remove it from the pinned cases"
            );
            if case.feature == "wind" {
                let flag = params
                    .get("flag")
                    .is_none_or(|v| v.as_bool().unwrap_or_else(|| v.as_integer() != Some(0)));
                let amp = num(&params["amplitude"]);
                check_wind(&format!("{key} (golden)"), flag, amp, &orig, &expected);
                check_wind(&format!("{key} (ours)"), flag, amp, &orig, &ours);
            } else {
                let b = Blast::of(&case.feature, &params);
                check_blast(&format!("{key} (golden)"), &b, &orig, &expected);
                check_blast(&format!("{key} (ours)"), &b, &orig, &ours);
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
    assert_eq!(statistical.len(), 3 + 7 + 3 + 6, "{statistical:?}");
}
