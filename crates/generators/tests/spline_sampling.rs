//! Intermediate Points (docs/specs/GENERATORS2.md §6.2, brief 038): the shared sampler
//! against tests/golden/splines-2 (O17) and unit tests of every mode.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::sync::Arc;

mod common;

use common::{apply, resolved_params};
use forge_core::{Document, EvalCtx, Registry, Value};
use forge_generators::splines::{CIRCLE, HELIX, RECTANGLE, build_curve};
use forge_generators::{BuildError, Primitives, evaluate};
use forge_parity_harness::{Mesh, compare_meshes, discover_cases, export_weld};

fn registry() -> Arc<Registry> {
    let mut reg = Registry::with_builtins();
    reg.install(&Primitives).unwrap();
    Arc::new(reg)
}

/// Observed cases whose spline we build and match (all vertices, strict faces).
const MATCHING: &[&str] = &[
    "arc-angles",
    "arc-defaults",
    "arc-sector",
    "circle-ellipse",
    "circle-ring",
    "circle-uniform",
    "cissoid-defaults",
    "cycloid-defaults",
    "cycloid-epi",
    "flower-defaults",
    "fourside-defaults",
    "fourside-types",
    "nside-8",
    "nside-defaults",
    "nside-rounding",
    "profile-defaults",
    "rectangle-rounding",
    "star-defaults",
    "star-twist",
];

/// Built but not matching. `helix-uniform`: the 901 points of the first ring are within
/// 3.5e-4 of the export (tolerance 1e-4, GENERATORS2.md §6.1); the second ring is displaced
/// along a direction the extrude does not model (the spline is not planar), so only the
/// first ring is compared.
const KNOWN_MISMATCH: &[&str] = &["helix-uniform"];

/// Not built: the Cogwheel's tooth outline is not derived (GENERATORS2.md §6.3) and the
/// Formula needs the expression library of brief 052.
const UNSUPPORTED: &[&str] = &["cog-defaults", "formula-defaults"];

/// Extrude Direction Auto (brief 044 owns the real rule). Observed so far: the extrusion goes
/// to -z exactly when the spline, closed by its chord, runs clockwise (`cycloid-defaults` is the
/// only such case; the others have a positive or zero area). The harness applies that guess to
/// the first loop of the spline and the test below checks it against every export.
fn auto_direction_is_negative(
    doc: &Document,
    child: forge_core::NodeId,
) -> Result<bool, BuildError> {
    let node = doc.scene().node(child).unwrap();
    let loops = build_curve(&node.type_id, &node.params)?.loops();
    let ring = &loops[0];
    let area: f64 = (0..ring.len())
        .map(|i| {
            let (a, b) = (ring[i], ring[(i + 1) % ring.len()]);
            a.x * b.y - b.x * a.y
        })
        .sum();
    Ok(area < 0.0)
}

/// The extrude of one O17 case: offset along +Z (along -Z when Direction Auto goes there),
/// caps off.
fn extrude_of(case: &forge_parity_harness::GoldenCase) -> Result<Mesh, BuildError> {
    let reg = registry();
    let m = case.manifest().unwrap();
    let params = resolved_params(case, &m);
    let input = params["input"].as_table().unwrap().clone();
    let offset = params["extrude_offset"].as_float().unwrap();
    let ty = format!("forge.{}", input["type"].as_str().unwrap());
    let mut doc = Document::new(reg.clone());
    let (ex, child) = doc
        .transact("case", |tx| {
            let ex = tx.create("forge.extrude", None, None)?;
            for cap in ["cap_start", "cap_end"] {
                tx.set_param(ex, cap, Value::Enum("none".into()))?;
            }
            let child = tx.create(&ty, Some(ex), None)?;
            apply(tx, &reg, child, &ty, &input);
            Ok((ex, child))
        })
        .unwrap();
    let sign = if auto_direction_is_negative(&doc, child)? {
        -1.0
    } else {
        1.0
    };
    doc.transact("direction", |tx| {
        tx.set_param(ex, "movement", Value::Vec3([0.0, 0.0, sign * offset]))
    })
    .unwrap();
    let mesh = evaluate(doc.scene(), ex, &EvalCtx::default())?.expect("geometry");
    Ok(Mesh {
        positions: mesh.points.iter().map(|p| p.to_array()).collect(),
        faces: mesh.polys.clone(),
    })
}

#[test]
fn splines_2_cases_against_the_export() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden");
    let (mut matched, mut mismatched, mut unsupported) =
        (BTreeSet::new(), BTreeSet::new(), BTreeSet::new());
    for case in discover_cases(&root).unwrap() {
        if case.feature != "splines-2" {
            continue;
        }
        let tol = case.manifest().unwrap().tolerance;
        let expected = case.expected_mesh().unwrap().unwrap();
        let ours = match extrude_of(&case) {
            Err(BuildError::Unsupported(_)) => {
                unsupported.insert(case.name.clone());
                continue;
            }
            other => other.unwrap(),
        };
        // Direction Auto: -z exactly when the export went there
        let went_negative = expected.positions.iter().map(|p| p[2]).sum::<f64>() < 0.0;
        let ours_negative = ours.positions.iter().map(|p| p[2]).sum::<f64>() < 0.0;
        if !KNOWN_MISMATCH.contains(&case.name.as_str()) {
            assert_eq!(ours_negative, went_negative, "{}: direction", case.name);
        }
        assert_eq!(
            ours.positions.len(),
            expected.positions.len(),
            "{}",
            case.name
        );
        if KNOWN_MISMATCH.contains(&case.name.as_str()) {
            // the first ring only
            let n = expected.positions.len() / 2;
            let ring = |m: &Mesh| Mesh {
                positions: m.positions[..n].to_vec(),
                faces: Vec::new(),
            };
            let r = compare_meshes(&ring(&expected), &ring(&ours), &tol);
            assert!(
                !r.passed(),
                "{}: matches now, remove it from KNOWN_MISMATCH",
                case.name
            );
            let worst = r.max_position_error.unwrap();
            assert!(worst < 1e-3, "{}: error {worst}", case.name);
            mismatched.insert(case.name.clone());
            continue;
        }
        let r = compare_meshes(&export_weld(&expected), &export_weld(&ours), &tol);
        assert!(
            r.passed(),
            "{}: {:?} ({:?})",
            case.name,
            r.failures,
            r.max_position_error
        );
        matched.insert(case.name.clone());
    }
    let names = |l: &[&str]| l.iter().map(|s| s.to_string()).collect::<BTreeSet<_>>();
    assert_eq!(matched, names(MATCHING));
    assert_eq!(mismatched, names(KNOWN_MISMATCH));
    assert_eq!(unsupported, names(UNSUPPORTED));
}

fn values(pairs: &[(&str, Value)]) -> BTreeMap<String, Value> {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.clone()))
        .collect()
}

fn mode(m: &str) -> (&'static str, Value) {
    ("intermediate", Value::Enum(m.into()))
}

fn points(type_id: &str, extra: &[(&str, Value)]) -> Vec<glam::DVec3> {
    build_curve(type_id, &values(extra)).unwrap().points()
}

#[test]
fn circle_and_helix_default_to_uniform_and_rectangle_to_adaptive() {
    let reg = registry();
    for (ty, want) in [
        ("forge.circle_spline", "uniform"),
        ("forge.helix_spline", "uniform"),
        ("forge.rectangle_spline", "adaptive"),
    ] {
        let d = reg.get(ty).unwrap().find_param("intermediate").unwrap();
        assert_eq!(d.default, Value::Enum(want.into()), "{ty}");
    }
}

#[test]
fn none_gives_only_the_splines_own_points() {
    assert_eq!(points(CIRCLE, &[mode("none")]).len(), 4);
    assert_eq!(points(HELIX, &[mode("none")]).len(), 101);
}

#[test]
fn uniform_places_number_points_per_segment() {
    // closed circle: 4 quarters of (8 + 1), the first point not repeated
    let p = points(CIRCLE, &[]);
    assert_eq!(p.len(), 36);
    assert!((p[0] - glam::DVec3::new(200.0, 0.0, 0.0)).length() < 1e-9);
    assert!((p[9] - glam::DVec3::new(0.0, 200.0, 0.0)).length() < 1e-9);
    let q = points(
        CIRCLE,
        &[mode("uniform"), ("intermediate_number", Value::Int(3))],
    );
    assert_eq!(q.len(), 16);
    // open helix: 100 segments of 9 steps, plus the end point
    assert_eq!(points(HELIX, &[]).len(), 901);
    // Number 0 is None
    let z = [mode("uniform"), ("intermediate_number", Value::Int(0))];
    assert_eq!(points(HELIX, &z), points(HELIX, &[mode("none")]));
}

#[test]
fn an_open_uniform_spline_ends_a_ten_thousandth_short() {
    let p = points(HELIX, &[]);
    let last = *p.last().unwrap();
    // t = 0.99 of the 100th piece: 719.928 degrees, z = -199.98
    assert!((last.z + 199.98).abs() < 1e-3, "{last:?}");
    let none = points(HELIX, &[mode("none")]);
    assert!((none.last().unwrap().z + 200.0).abs() < 1e-9);
}

#[test]
fn adaptive_is_the_observed_walk() {
    // 17 points per quarter at 5 degrees (extrude/defaults)
    assert_eq!(points(CIRCLE, &[mode("adaptive")]).len(), 68);
    // a wider angle needs fewer points, a narrower one more
    let wide = points(
        CIRCLE,
        &[
            mode("adaptive"),
            ("intermediate_angle", Value::Float(20f64.to_radians())),
        ],
    );
    let narrow = points(
        CIRCLE,
        &[
            mode("adaptive"),
            ("intermediate_angle", Value::Float(2f64.to_radians())),
        ],
    );
    assert!(
        wide.len() < 68 && 68 < narrow.len(),
        "{} {}",
        wide.len(),
        narrow.len()
    );
    // straight pieces never get interior points, whatever the mode
    for m in ["none", "natural", "uniform", "adaptive", "subdivided"] {
        assert_eq!(points(RECTANGLE, &[mode(m)]).len(), 4, "{m}");
    }
}

#[test]
fn natural_steps_the_segment_parameter_and_uniform_the_arc_length() {
    let curve = build_curve(CIRCLE, &values(&[mode("natural")])).unwrap();
    let p = curve.points();
    assert_eq!(p.len(), 36);
    for (k, q) in p.iter().enumerate() {
        let (seg, i) = (k / 9, k % 9);
        let want = curve.at(seg as f64 + i as f64 / 9.0);
        assert!((*q - want).length() < 1e-9, "{k}");
    }
    // the same quarter by arc length: nine near-equal chords instead
    let u = points(CIRCLE, &[]);
    let chords: Vec<f64> = (0..9).map(|k| (u[k + 1] - u[k]).length()).collect();
    let mean = chords.iter().sum::<f64>() / 9.0;
    assert!(
        chords.iter().all(|c| (c - mean).abs() < 1e-3 * mean),
        "{chords:?}"
    );
    assert!((p[1] - u[1]).length() > 0.1, "natural and uniform differ");
}

#[test]
fn subdivided_respects_angle_and_maximum_length() {
    let p = points(
        CIRCLE,
        &[
            mode("subdivided"),
            ("intermediate_angle", Value::Float(10f64.to_radians())),
            ("intermediate_max_length", Value::Float(20.0)),
        ],
    );
    let n = p.len();
    assert_eq!(n % 4, 0);
    // each quarter: chord <= 20 needs 16+ pieces (arc 314), more than the 9 the angle needs
    assert!(n / 4 >= 16, "{n}");
    for k in 0..n {
        assert!((p[(k + 1) % n] - p[k]).length() <= 20.0 + 1e-6);
    }
    // a long maximum length leaves the angle in charge: the quarter turns by 90 degrees but
    // not evenly (handle 83/150), so 10 equal-t pieces are needed, not 9
    let q = points(
        CIRCLE,
        &[
            mode("subdivided"),
            ("intermediate_angle", Value::Float(10f64.to_radians())),
            ("intermediate_max_length", Value::Float(1.0e6)),
        ],
    );
    assert_eq!(q.len() / 4, 10);
}

#[test]
fn every_mode_builds_a_closed_extrude_ring() {
    use forge_generators::spline_sampling::Interpolation;
    let reg = registry();
    for m in Interpolation::ALL {
        let mut doc = Document::new(reg.clone());
        let ex = doc
            .transact("t", |tx| {
                let ex = tx.create("forge.extrude", None, None)?;
                let c = tx.create("forge.circle_spline", Some(ex), None)?;
                tx.set_param(c, "intermediate", Value::Enum(m.id().into()))?;
                Ok(ex)
            })
            .unwrap();
        let mesh = evaluate(doc.scene(), ex, &EvalCtx::default())
            .unwrap()
            .unwrap();
        assert!(mesh.points.len() >= 8, "{}", m.id());
    }
}
