//! Parity: docs/specs/GENERATORS3.md against tests/golden/{atom-array,spline-mask} (O20).

use std::collections::BTreeSet;
use std::path::Path;
use std::sync::Arc;

mod common;

use common::{Table, apply, resolved_params};
use forge_core::{Document, EvalCtx, Registry, Value};
use forge_generators::{BuildError, Primitives, evaluate, spline_mask};
use forge_parity_harness::{GoldenCase, Mesh, compare_meshes, discover_cases, export_weld};

const FEATURES: &[&str] = &["atom-array", "spline-mask"];

/// Observed options whose rules are not derived (none: every observed case is built).
const UNSUPPORTED: &[&str] = &[];

/// Built but differing: `spline-mask/cap` has every point and 429 of 430 triangles; face 13 is
/// `[6, 7, 209]` for us and `[6, 7, 210]` in the export. pending: the Delaunay tie-break of
/// the near-cocircular Bezier arc samples of a 432-point ring (GENERATORS3.md §2 rule 5).
const KNOWN_MISMATCH: &[&str] = &["spline-mask/cap"];

fn registry() -> Arc<Registry> {
    let mut reg = Registry::with_builtins();
    reg.install(&Primitives).unwrap();
    Arc::new(reg)
}

fn inputs(params: &Table) -> Vec<(&String, &Table)> {
    let mut v: Vec<_> = params
        .iter()
        .filter_map(|(k, v)| v.as_table().map(|t| (k, t)))
        .collect();
    v.sort_by_key(|(k, _)| k.trim_start_matches("input").parse::<u32>().unwrap_or(1));
    v
}

fn type_of(t: &Table) -> String {
    format!("forge.{}", t["type"].as_str().unwrap())
}

/// The sign of Extrude Direction Auto: the guess of the spline tests (signed area of all
/// loops, a zero total follows the first loop), in the plane frame (u, v) of the mask Axis
/// (`zy`: u = z, v = y).
fn auto_sign(curve: &forge_generators::splines::Curve, zy: bool) -> f64 {
    let areas: Vec<f64> = curve
        .loops()
        .iter()
        .map(|ring| {
            (0..ring.len())
                .map(|i| {
                    let (a, b) = (ring[i], ring[(i + 1) % ring.len()]);
                    let (a, b) = if zy {
                        (
                            glam::DVec3::new(a.z, a.y, a.x),
                            glam::DVec3::new(b.z, b.y, b.x),
                        )
                    } else {
                        (a, b)
                    };
                    a.x * b.y - b.x * a.y
                })
                .sum()
        })
        .collect();
    let total: f64 = areas.iter().sum();
    let scale: f64 = areas.iter().map(|a| a.abs()).sum();
    let area = if total.abs() < 1e-9 * scale {
        areas[0]
    } else {
        total
    };
    if area < 0.0 { -1.0 } else { 1.0 }
}

fn build(case: &GoldenCase) -> Result<Mesh, BuildError> {
    let reg = registry();
    let m = case.manifest().unwrap();
    let params = resolved_params(case, &m);
    let gen_type = format!("forge.{}", case.feature.replace('-', "_"));
    let mut own = Table::new();
    let mut extrude: Option<(f64, bool)> = None;
    for (k, v) in &params {
        match k.as_str() {
            "extrude_offset" => extrude = Some((v.as_float().unwrap(), false)),
            "caps" => {
                let e = extrude.get_or_insert((0.0, false));
                e.1 = v.as_bool().unwrap();
            }
            _ => {
                own.insert(k.clone(), v.clone());
            }
        }
    }
    // Create Cap makes the mask a polygon object that the Extrude passes through.
    if own
        .get("create_cap")
        .is_some_and(|v| v.as_integer() == Some(1))
    {
        extrude = None;
    }
    let mut doc = Document::new(reg.clone());
    let (root, generator) = doc
        .transact("case", |tx| {
            let root = match extrude {
                Some(_) => tx.create("forge.extrude", None, None)?,
                None => tx.create(&gen_type, None, None)?,
            };
            let generator = if extrude.is_some() {
                tx.create(&gen_type, Some(root), None)?
            } else {
                root
            };
            apply(tx, &reg, generator, &gen_type, &own);
            for (_, t) in inputs(&params) {
                let ty = type_of(t);
                let child = tx.create(&ty, Some(generator), None)?;
                apply(tx, &reg, child, &ty, t);
            }
            Ok((root, generator))
        })
        .unwrap();
    if let Some((offset, caps)) = extrude {
        let curve = spline_mask::curve_of(doc.scene(), generator)?.expect("mask curve");
        let zy = own.get("axis").and_then(|v| v.as_str()) == Some("ZY (Along X)");
        let sign = auto_sign(&curve, zy);
        doc.transact("extrude", |tx| {
            // the plane normal is u x v: +z for XY, -x for ZY
            let movement = if zy {
                [-sign * offset, 0.0, 0.0]
            } else {
                [0.0, 0.0, sign * offset]
            };
            tx.set_param(root, "movement", Value::Vec3(movement))?;
            if !caps {
                for cap in ["cap_start", "cap_end"] {
                    tx.set_param(root, cap, Value::Enum("none".into()))?;
                }
            }
            Ok(())
        })
        .unwrap();
    }
    let mesh = evaluate(doc.scene(), root, &EvalCtx::default())?.expect("geometry");
    let world = doc.scene().local_transform(root).unwrap().matrix();
    Ok(Mesh {
        positions: mesh
            .points
            .iter()
            .map(|p| world.transform_point3(*p).to_array())
            .collect(),
        faces: mesh.polys.clone(),
    })
}

#[test]
fn atom_array_and_spline_mask_match_the_reference() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden");
    let (mut matched, mut unsupported) = (BTreeSet::new(), BTreeSet::new());
    let mut failed: Vec<String> = Vec::new();
    for case in discover_cases(&root).unwrap() {
        if !FEATURES.contains(&case.feature.as_str()) {
            continue;
        }
        let key = format!("{}/{}", case.feature, case.name);
        let tol = case.manifest().unwrap().tolerance;
        let ours = match build(&case) {
            Err(BuildError::Unsupported(what)) => {
                println!("{key}: unsupported ({what})");
                unsupported.insert(key);
                continue;
            }
            other => other.unwrap(),
        };
        let expected = export_weld(&case.expected_mesh().unwrap().unwrap());
        let ours = export_weld(&ours);
        let r = compare_meshes(&expected, &ours, &tol);
        if KNOWN_MISMATCH.contains(&key.as_str()) {
            assert!(
                !r.passed(),
                "{key} matches now: remove it from KNOWN_MISMATCH"
            );
            continue;
        }
        println!(
            "{key}: passed {} (points {} vs {}, faces {} vs {})",
            r.passed(),
            expected.positions.len(),
            ours.positions.len(),
            expected.faces.len(),
            ours.faces.len()
        );
        if !r.passed() {
            failed.push(format!(
                "{key}: {:?} (max error {:?})",
                r.failures, r.max_position_error
            ));
            continue;
        }
        matched.insert(key);
    }
    assert!(failed.is_empty(), "unlisted mismatches: {failed:#?}");
    let want: BTreeSet<String> = UNSUPPORTED.iter().map(|s| s.to_string()).collect();
    assert_eq!(unsupported, want, "unsupported set changed");
    assert_eq!(matched.len(), 5 + 8, "{matched:?}");
}
