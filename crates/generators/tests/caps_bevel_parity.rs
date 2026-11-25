//! Parity: docs/specs/GENERATORS2.md §7, §7a against tests/golden/caps-bevel (O56, R25): the
//! shared Caps tab of Extrude, Lathe, Loft and Sweep, and the R25 Extrude Direction / Offset.

use std::collections::BTreeSet;
use std::path::Path;
use std::sync::Arc;

mod common;

use common::{Table, apply, create_spline, resolved_params};
use forge_core::{Document, EvalCtx, Registry};
use forge_generators::{BuildError, Primitives, evaluate};
use forge_parity_harness::{GoldenCase, Mesh, compare_meshes, discover_cases, export_weld};

/// Observed cases the generators return `Unsupported` for, with the reason (GENERATORS2.md §7a).
const UNSUPPORTED: &[&str] = &[
    // pending: the reference's Delaunay fill adds a centre point per cap and lists its triangles in an
    // order seen for one outline only (a rectangle); no general rule.
    "caps-bevel/caps-delaunay",
    // pending: the Regular Grid fill (730 points, 20 x 20 cells per cap) is not derived.
    "caps-bevel/caps-grid",
    // pending: bevel rings of Solid and Step (point and face order) and of the sweep's circle
    // profile are not derived; those bevels with Size > 0 are refused (the Round bevel of a
    // 4-point extrude outline is derived, GENERATORS2.md 7b).
    "caps-bevel/bevel-chamfer",
    "caps-bevel/bevel-concave",
    "caps-bevel/bevel-step",
    "caps-bevel/sweep-caps-bevel",
    // The two ellipse sections of the case are recorded without their positions (the export has 79
    // distinct points: the sections are not the same), so the input is not recorded.
    "caps-bevel/tri-ellipse-loft",
    // The case file records neither the second section's rotation nor its height (the mesh is 200 high):
    // the input is not recorded.
    "caps-bevel/tri-loft-rotated",
    // Exported from editable Polygon objects (a sliced Cylinder, a Dodecahedron): Make Editable
    // triangulation of primitives (EDITABLE.md, GEOMETRY.md §6), not the Caps tab.
    "caps-bevel/tri-slice-orient-x",
    "caps-bevel/tri-dodeca-seg2",
];

/// Input positions that the case file lacks and the observation row and the case description state
/// ([caps-bevel.md](../../../docs/parity/caps-bevel.md): "Lathe of a default Rectangle at x 300").
const POSITIONS_FROM_DESCRIPTION: &[(&str, [f64; 3])] =
    &[("caps-bevel/lathe-caps", [300.0, 0.0, 0.0])];

/// Built but differing. `tri-extrude-ngon`: the hexagon is regular, so every cap vertex is
/// cocircular and the reference's triangle choice (a fan around the second ring point) is a
/// tie that the constrained Delaunay rule cannot decide (GEOMETRY.md §6.1 R1).
const KNOWN_MISMATCH: &[&str] = &["caps-bevel/tri-extrude-ngon"];

fn registry() -> Arc<Registry> {
    let mut reg = Registry::with_builtins();
    reg.install(&Primitives).unwrap();
    Arc::new(reg)
}

/// The case's input tables in child order.
fn inputs(params: &Table) -> Vec<Table> {
    let mut v: Vec<(&String, &Table)> = params
        .iter()
        .filter_map(|(k, v)| v.as_table().map(|t| (k, t)))
        .collect();
    v.sort_by_key(|(k, _)| k.trim_start_matches("input").parse::<u32>().unwrap_or(1));
    v.into_iter()
        .map(|(_, t)| {
            let mut t = t.clone();
            // the case spells the primitive by its short name
            let ty = t["type"].as_str().unwrap().to_string();
            t.insert("type".into(), format!("{ty}_spline").into());
            t
        })
        .collect()
}

fn build(case: &GoldenCase, key: &str) -> Result<Mesh, BuildError> {
    let reg = registry();
    let m = case.manifest().unwrap();
    let params = resolved_params(case, &m);
    let gen_type = format!("forge.{}", params["type"].as_str().unwrap());
    let mut own = Table::new();
    for (k, v) in &params {
        if !v.is_table() {
            own.insert(k.clone(), v.clone());
        }
    }
    let mut doc = Document::new(reg.clone());
    let generator = doc
        .transact("case", |tx| {
            let generator = tx.create(&gen_type, None, None)?;
            apply(tx, &reg, generator, &gen_type, &own);
            for mut t in inputs(&params) {
                if let Some((_, pos)) = POSITIONS_FROM_DESCRIPTION.iter().find(|(k, _)| *k == key) {
                    t.insert("pos".into(), pos.to_vec().into());
                }
                create_spline(tx, &reg, Some(generator), &t);
            }
            Ok(generator)
        })
        .unwrap();
    let mesh = evaluate(doc.scene(), generator, &EvalCtx::default())?.expect("geometry");
    let world = doc.scene().local_transform(generator).unwrap().matrix();
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
fn caps_and_bevel_cases_match_the_reference() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden");
    let (mut matched, mut unsupported) = (BTreeSet::new(), BTreeSet::new());
    let mut failed: Vec<String> = Vec::new();
    let mut seen = 0;
    for case in discover_cases(&root).unwrap() {
        if case.feature != "caps-bevel" {
            continue;
        }
        seen += 1;
        let key = format!("{}/{}", case.feature, case.name);
        let params = case.manifest().unwrap().params;
        // Make Editable cases of other types have no generator under test.
        let not_a_generator = params
            .get("type")
            .and_then(|t| t.as_str())
            .is_some_and(|t| t == "polygon")
            || params.contains_key("primitive");
        if not_a_generator || key.ends_with("tri-loft-rotated") || key.ends_with("tri-ellipse-loft")
        {
            println!("{key}: unsupported (not built)");
            unsupported.insert(key);
            continue;
        }
        let tol = case.manifest().unwrap().tolerance;
        let ours = match build(&case, &key) {
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
    assert_eq!(seen, 22);
    assert_eq!(matched.len(), 11, "{matched:?}");
}
