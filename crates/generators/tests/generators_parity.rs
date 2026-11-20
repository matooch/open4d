//! Parity: docs/specs/GENERATORS2.md §5 against tests/golden/{symmetry,array,connect,instance}.

use std::path::Path;
use std::sync::Arc;

mod common;

use common::{apply, resolved_params};
use forge_core::{Document, Registry, Value};
use forge_generators::{Primitives, evaluate};
use forge_parity_harness::{
    Mesh, Tolerance, compare_geometric, compare_meshes, discover_cases, export_weld,
};

const FEATURES: &[&str] = &[
    "symmetry", "array", "connect", "instance", "extrude", "lathe", "loft", "sweep",
];

/// Observed cases not derived yet or without the input data to build them.
const UNSUPPORTED: &[&str] = &[
    // pending: the case records no points for its "straight 2-point Linear spline of length 400"
    // (only `type = "Linear"` and the Intermediate Points settings), so the path is unknown.
    "sweep/r25-scale-end-only",
];

/// Cases we build but that differ, pinned so a fix shows up (GENERATORS2.md §9–10: a
/// 6-point loft section snaps two samples to segment starts; the sweep frame along a
/// non-planar path twists less than parallel transport). Not understood yet.
const KNOWN_MISMATCH: &[&str] = &[
    "loft/mesh-subdivision",
    "sweep/helix",
    // D18: R25 wins. Loft now has U points per ring and the R25 quad order (GENERATORS2.md
    // §9a), so the R20 loft cases (U - 1 points per ring) differ.
    "loft/defaults", // superseded by R25 (D18): loft/three-circles
    "loft/three",    // superseded by R25 (D18): loft/three-circles
    "loft/caps-off", // superseded by R25 (D18): loft/three-circles
    // brief 191 fixed the R25 uniform helix route (`splines-2/helix-uniform`, GENERATORS2.md
    // §6.1), which un-pinned `sweep/banking`. The 2-Rail mapping (`sweep/rail`) matched along
    // the reference's own ring centres (`tests/sweep_loft_r25.rs`) but its rail helix (end
    // radius 400) is not checked by `helix-uniform`. `sweep/scale-end` also has the
    // Banking-on frame mismatch of `sweep/helix` (GENERATORS2.md §10a).
    "sweep/rail",
    "sweep/scale-end",
    // Follow-up cases (generators-3.md, brief 042). The Flower (`sweep/r25-banking-planar`) and
    // the rotated Banking-off helix match; the helix routes below do not.
    // pending: the Bezier helix of the radius-changing rail is off by 0.29 (path alone, handles
    // now follow the previous chord as `evaluated_path` shows), the Sampler or the Banking-on frame
    // is not derived (GENERATORS2.md §10a).
    "sweep/r25-helix-radius",
    // pending: Helix Subdivision 4 (5 points, handles along z): the first ring's frame is
    // `Y x t0`, not our `t0 x Z`, for a tangent tilted 57 degrees from Z; the rule between the two
    // frames is not derived, so every ring is off (error 31).
    "sweep/r25-helix-sub4",
    // pending: as `r25-helix-sub4` (Subdivision 8, error 7.3).
    "sweep/r25-helix-sub8",
    // pending: as `sweep/rail`: the rail helix with End Radius 400 (error 1.1e-2).
    "sweep/r25-rail-rectangle",
];

/// Cases whose caps contain an exactly cocircular quad that the reference splits the other
/// way (GEOMETRY.md §6.1 R1): exact except the trailing cap triangles, plus a geometric
/// comparison. None now: the only one, `loft/three`, is superseded by R25 (D18).
const CAP_GEOMETRIC: &[&str] = &[];

#[test]
fn generators_match_the_reference() {
    let mut reg = Registry::with_builtins();
    reg.install(&Primitives).unwrap();
    let reg = Arc::new(reg);
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden");
    let mut matched = Vec::new();
    let mut failed: Vec<String> = Vec::new();
    let mut unsupported = std::collections::BTreeSet::new();
    for case in discover_cases(&root).unwrap() {
        if !FEATURES.contains(&case.feature.as_str()) {
            continue;
        }
        let key = format!("{}/{}", case.feature, case.name);
        if UNSUPPORTED.contains(&key.as_str()) {
            println!("{key}: unsupported (input data not recorded)");
            unsupported.insert(key);
            continue;
        }
        let m = case.manifest().unwrap();
        let params = resolved_params(&case, &m);
        let mut inputs: Vec<(&String, &toml::map::Map<String, toml::Value>)> = params
            .iter()
            .filter_map(|(k, v)| v.as_table().map(|t| (k, t)))
            .collect();
        inputs.sort_by_key(|(k, _)| k.trim_start_matches("input").parse::<u32>().unwrap_or(1));

        let mut doc = Document::new(reg.clone());
        let mut reference = None;
        let gen_type = format!("forge.{}", case.feature);
        let generator = doc
            .transact("case", |tx| {
                let instance = case.feature == "instance";
                let generator = tx.create(&gen_type, None, None)?;
                apply(tx, &reg, generator, &gen_type, &params);
                for (_, t) in &inputs {
                    let ty = format!("forge.{}", t["type"].as_str().unwrap());
                    let parent = if instance { None } else { Some(generator) };
                    let child = tx.create(&ty, parent, None)?;
                    apply(tx, &reg, child, &ty, t);
                    if instance {
                        tx.set_param(generator, "reference", Value::Link(Some(child)))?;
                        reference = Some(child);
                    }
                }
                Ok(generator)
            })
            .unwrap();
        let mesh = match evaluate(doc.scene(), generator, &forge_core::EvalCtx::default()) {
            Err(forge_generators::BuildError::Unsupported(what)) => {
                println!("{key}: unsupported ({what})");
                unsupported.insert(key);
                continue;
            }
            other => other
                .unwrap()
                .unwrap_or_else(|| panic!("{key}: no geometry")),
        };
        let world = doc.scene().local_transform(generator).unwrap().matrix();
        let mut ours = Mesh {
            positions: mesh
                .points
                .iter()
                .map(|p| world.transform_point3(*p).to_array())
                .collect(),
            faces: mesh.polys.clone(),
        };
        // The scripted instance cases export the whole scene: the reference stays visible,
        // so its object comes first, then the instance (`instance/render-instance`).
        if let (true, Some(r)) = (m.params.contains_key("generator"), reference) {
            let rm = evaluate(doc.scene(), r, &forge_core::EvalCtx::default())
                .unwrap()
                .expect("reference geometry");
            let rw = doc.scene().local_transform(r).unwrap().matrix();
            let base = rm.points.len() as u32;
            let mut positions: Vec<[f64; 3]> = rm
                .points
                .iter()
                .map(|p| rw.transform_point3(*p).to_array())
                .collect();
            positions.extend(ours.positions);
            let mut faces = rm.polys.clone();
            faces.extend(
                ours.faces
                    .iter()
                    .map(|f| f.iter().map(|i| i + base).collect::<Vec<u32>>()),
            );
            ours = Mesh { positions, faces };
        }
        let ours = export_weld(&ours);
        let expected = export_weld(&case.expected_mesh().unwrap().unwrap());
        if KNOWN_MISMATCH.contains(&key.as_str()) {
            let r = compare_meshes(&expected, &ours, &m.tolerance);
            assert!(
                !r.passed(),
                "{key} matches now: remove it from KNOWN_MISMATCH"
            );
            println!("{key}: known mismatch {:?}", r.max_position_error);
            continue;
        }
        if CAP_GEOMETRIC.contains(&key.as_str()) {
            check_cap_geometric(&key, &expected, &ours, &m.tolerance);
            matched.push(key);
            continue;
        }
        let r = compare_meshes(&expected, &ours, &m.tolerance);
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
        matched.push(key);
    }
    println!("matched exactly: {}", matched.join(", "));
    assert!(failed.is_empty(), "unlisted mismatches: {failed:#?}");
    let want: std::collections::BTreeSet<String> =
        UNSUPPORTED.iter().map(|s| s.to_string()).collect();
    assert_eq!(unsupported, want, "unsupported set changed");
    // 36 before the follow-up cases, plus connect/tolerance-in, instance/render-instance,
    // lathe/r25-scaling-50, sweep/r25-banking-off-rotated and sweep/r25-banking-planar
    assert_eq!(matched.len(), 13 + 11 + 10 + 2 + 5, "{matched:?}");
}

/// Exact positions and faces up to a trailing run of triangles in both meshes (the
/// caps), and a geometric match of those caps.
fn check_cap_geometric(key: &str, expected: &Mesh, ours: &Mesh, tol: &Tolerance) {
    let points = |m: &Mesh| Mesh {
        positions: m.positions.clone(),
        faces: Vec::new(),
    };
    let same = compare_meshes(&points(expected), &points(ours), tol);
    assert!(same.passed(), "{key}: {:?}", same.failures);
    assert_eq!(expected.faces.len(), ours.faces.len(), "{key}: face count");
    let first_diff = expected
        .faces
        .iter()
        .zip(&ours.faces)
        .position(|(a, b)| a != b)
        .unwrap_or(expected.faces.len());
    for m in [expected, ours] {
        assert!(
            m.faces[first_diff..].iter().all(|f| f.len() == 3),
            "{key}: faces differ before the caps (face {first_diff}: expected {:?}, got {:?})",
            expected.faces.get(first_diff),
            ours.faces.get(first_diff)
        );
    }
    // everything before the caps matched exactly; compare the caps as surfaces
    let caps = |m: &Mesh| Mesh {
        positions: m.positions.clone(),
        faces: m.faces[first_diff..].to_vec(),
    };
    let r = compare_geometric(&caps(expected), &caps(ours), tol);
    assert!(r.passed(), "{key}: {:?}", r.failures);
}
