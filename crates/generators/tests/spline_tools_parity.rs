//! Editable splines (docs/specs/SPLINES.md, brief 028) against tests/golden/spline-tools (O46):
//! the extrude (Offset 20, caps off) of a spline made editable, with the interpolation type
//! and Intermediate Points of `[params.settings]` applied afterwards.

use std::collections::BTreeSet;
use std::path::Path;
use std::sync::Arc;

mod common;

use common::{apply, resolved_params};
use forge_core::{Document, EvalCtx, Registry, Value};
use forge_generators::editable_spline::{self, SplineData, SplineType, tangent_readback};
use forge_generators::splines::{CIRCLE, RECTANGLE};
use forge_generators::{BuildError, Primitives, evaluate, make_editable};
use forge_parity_harness::{GoldenCase, Mesh, compare_meshes, discover_cases, export_weld};
use glam::DVec3;

fn registry() -> Arc<Registry> {
    let mut reg = Registry::with_builtins();
    reg.install(&Primitives).unwrap();
    Arc::new(reg)
}

/// Cases of this brief (editable objects and interpolation types).
const MATCHING: &[&str] = &[
    "editable-circle",
    "editable-rectangle",
    "interp-akima",
    "interp-bezier",
    "interp-bspline",
    "interp-linear",
];

/// Built but not matching.
const KNOWN_MISMATCH: &[&str] = &[];

/// Observed options whose rules are not derived.
const UNSUPPORTED: &[&str] = &[];

/// Cases of the folder that belong to other briefs: spline tools and booleans (029), the
/// Intermediate Points / primitive follow-ups of briefs 037 and 038 (`interp-f-*`,
/// `prim-f-*`, checked by their own tests when those briefs pick them up).
fn owned_elsewhere(name: &str) -> bool {
    name.starts_with("interp-f-")
        || name.starts_with("prim-f-")
        || [
            "and",
            "chamfer",
            "intersect",
            "or",
            "outline",
            "outline-neg",
            "reverse",
            "subdivide",
            "subtract",
            "union",
        ]
        .contains(&name)
}

fn extrude_of(case: &GoldenCase) -> Result<Mesh, BuildError> {
    let reg = registry();
    let m = case.manifest().unwrap();
    let params = resolved_params(case, &m);
    let mut input = params["input"].as_table().unwrap().clone();
    // `position` is the node's transform, which `apply` spells `pos`
    if let Some(p) = input.remove("position") {
        input.insert("pos".into(), p);
    }
    let settings = params.get("settings").and_then(|v| v.as_table()).cloned();
    let offset = params["extrude_offset"].as_float().unwrap();
    let ty = if input.contains_key("width") {
        RECTANGLE
    } else {
        CIRCLE
    };
    let mut doc = Document::new(reg.clone());
    let (ex, child) = doc
        .transact("case", |tx| {
            let ex = tx.create("forge.extrude", None, None)?;
            tx.set_param(ex, "movement", Value::Vec3([0.0, 0.0, offset]))?;
            for cap in ["cap_start", "cap_end"] {
                tx.set_param(ex, cap, Value::Enum("none".into()))?;
            }
            let child = tx.create(ty, Some(ex), None)?;
            apply(tx, &reg, child, ty, &input);
            Ok((ex, child))
        })
        .unwrap();
    // Make Editable, as the command does
    let editable = doc
        .transact("make editable", |tx| {
            make_editable(tx, child).map_err(|e| match e {
                forge_generators::EditableError::Edit(e) => e,
                other => panic!("{other}"),
            })
        })
        .unwrap();
    assert_eq!(
        doc.scene().node(editable).unwrap().type_id,
        editable_spline::TYPE
    );
    if let Some(s) = settings {
        doc.transact("settings", |tx| {
            if let Some(mode) = s.get("intermediate_points").and_then(|v| v.as_str()) {
                tx.set_param(editable, "intermediate", Value::Enum(mode.into()))?;
            }
            if let Some(t) = s.get("type").and_then(|v| v.as_str()) {
                tx.set_param(editable, "type", Value::Enum(t.into()))?;
            }
            Ok(())
        })
        .unwrap();
    }
    let mesh = evaluate(doc.scene(), ex, &EvalCtx::default())?.expect("geometry");
    Ok(Mesh {
        positions: mesh.points.iter().map(|p| p.to_array()).collect(),
        faces: mesh.polys.clone(),
    })
}

#[test]
fn editable_and_interpolation_cases_against_the_export() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden");
    let (mut matched, mut mismatched, mut unsupported) =
        (BTreeSet::new(), BTreeSet::new(), BTreeSet::new());
    for case in discover_cases(&root).unwrap() {
        if case.feature != "spline-tools" || owned_elsewhere(&case.name) {
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
        let r = compare_meshes(&export_weld(&expected), &export_weld(&ours), &tol);
        if KNOWN_MISMATCH.contains(&case.name.as_str()) {
            assert!(!r.passed(), "{}: matches now, remove it", case.name);
            mismatched.insert(case.name.clone());
            continue;
        }
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

/// The folder has no case that is neither claimed here nor listed as another brief's.
#[test]
fn every_case_of_the_folder_is_claimed() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden");
    for case in discover_cases(&root).unwrap() {
        if case.feature != "spline-tools" {
            continue;
        }
        assert!(
            MATCHING.contains(&case.name.as_str()) || owned_elsewhere(&case.name),
            "{}: new spline-tools case; claim it or list its brief",
            case.name
        );
    }
}

/// The editable data of the observation form's Notes ("Editable spline facts").
fn editable_of(ty: &str, set: &[(&str, Value)]) -> (SplineData, SplineType) {
    let reg = registry();
    let mut doc = Document::new(reg);
    let id = doc
        .transact("make", |tx| {
            let src = tx.create(ty, None, None)?;
            for (k, v) in set {
                tx.set_param(src, k, v.clone())?;
            }
            Ok(make_editable(tx, src).unwrap())
        })
        .unwrap();
    let data = editable_spline::data_of(doc.scene(), id).unwrap();
    let Some(Value::Enum(t)) = doc.scene().param(id, "type") else {
        panic!("no type")
    };
    (data, SplineType::from_id(t).unwrap())
}

fn near(a: DVec3, b: DVec3) {
    near_to(a, b, 1e-9);
}

fn near_to(a: DVec3, b: DVec3, tol: f64) {
    assert!((a - b).length() < tol, "{a:?} vs {b:?}");
}

#[test]
fn editable_circle_points_and_tangents_match_the_notes() {
    let (d, ty) = editable_of(CIRCLE, &[]);
    assert_eq!(ty, SplineType::Bezier);
    assert_eq!(d.segments.len(), 1);
    assert!(d.segments[0].closed);
    let pos = [(200.0, 0.0), (0.0, 200.0), (-200.0, 0.0), (0.0, -200.0)];
    // (left, right) handle vectors as read by the app: 83 long, along the tangent
    let read = [
        ((0.0, -83.0), (0.0, 83.0)),
        ((83.0, 0.0), (-83.0, 0.0)),
        ((0.0, 83.0), (0.0, -83.0)),
        ((-83.0, 0.0), (83.0, 0.0)),
    ];
    for i in 0..4 {
        near(d.points[i], DVec3::new(pos[i].0, pos[i].1, 0.0));
        near(
            tangent_readback(d.left[i]),
            DVec3::new(read[i].0.0, read[i].0.1, 0.0),
        );
        near(
            tangent_readback(d.right[i]),
            DVec3::new(read[i].1.0, read[i].1.1, 0.0),
        );
    }
}

#[test]
fn editable_rectangle_is_linear_without_handles() {
    let (d, ty) = editable_of(RECTANGLE, &[]);
    assert_eq!(ty, SplineType::Linear);
    let want = [
        (200.0, 200.0),
        (-200.0, 200.0),
        (-200.0, -200.0),
        (200.0, -200.0),
    ];
    for (p, w) in d.points.iter().zip(want) {
        near(*p, DVec3::new(w.0, w.1, 0.0));
    }
    assert!(d.left.iter().chain(&d.right).all(|h| *h == DVec3::ZERO));
}

#[test]
fn a_ring_becomes_two_segments() {
    let (d, _) = editable_of(CIRCLE, &[("ring", Value::Bool(true))]);
    assert_eq!(d.segments.iter().map(|s| s.len).collect::<Vec<_>>(), [4, 4]);
    near(d.points[4], DVec3::new(100.0, 0.0, 0.0));
}

/// `interp-f/helix-handles` (Notes, form only): 101 points, Bezier; the read-back factor
/// 0.75 holds for the helix too.
#[test]
fn editable_helix_points_and_tangents_match_the_notes() {
    let (d, ty) = editable_of(forge_generators::splines::HELIX, &[]);
    assert_eq!(ty, SplineType::Bezier);
    assert_eq!(d.points.len(), 101);
    assert!(!d.segments[0].closed);
    near(d.points[0], DVec3::new(200.0, 0.0, 0.0));
    near(d.points[100], DVec3::new(200.0, 0.0, -200.0));
    near_to(d.points[1], DVec3::new(198.422940, 25.066647, -2.0), 1e-5);
    // our helix handle is a third of the piece length (GENERATORS2.md 6.1); the app's is
    // 0.07 % shorter, so the read-back agrees to a relative 1e-3, not to the form's digits
    let close = |a: DVec3, b: DVec3| near_to(a, b, 1e-3 * b.length());
    close(
        tangent_readback(d.right[0]),
        DVec3::new(0.0, 6.278974, -0.500982),
    );
    close(
        tangent_readback(d.right[1]),
        DVec3::new(-0.786964, 6.229462, -0.500982),
    );
    close(
        tangent_readback(d.left[1]),
        DVec3::new(0.786964, -6.229462, 0.500982),
    );
    // open ends: the unused handle mirrors the used one (points 0 and 100 of the form)
    close(
        tangent_readback(d.left[0]),
        DVec3::new(0.0, -6.278974, 0.500982),
    );
    close(
        tangent_readback(d.right[100]),
        DVec3::new(0.0, 6.278974, -0.500982),
    );
}
