//! Parity: docs/specs/SUBDIVISION.md §5 against tests/golden/subdivision.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

mod common;

use common::{Table, enum_id, legacy_params, typed_params, with_defaults};
use forge_core::Registry;
use forge_generators::{BuildError, Primitives, build, subdivide_with_creases};
use forge_geometry::Edge;
use forge_geometry::PolyMesh;
use forge_geometry::normals::poly_normal;
use forge_parity_harness::{Mesh, compare_meshes, discover_cases, parse_obj};
use glam::DVec3;

/// Observed schemes whose geometry is not derived yet.
const UNSUPPORTED: &[&str] = &[];

/// The reference cylinder's caps are separate surfaces (docs/specs/SUBDIVISION.md §2,
/// `ngon-1`), but the export welds them: mark the rim — edges whose two faces meet at
/// 90° or more — as sharp to reconstruct that input.
fn separate_surface_edges(mesh: &PolyMesh) -> BTreeSet<Edge> {
    let mut faces: BTreeMap<Edge, Vec<usize>> = BTreeMap::new();
    for (f, p) in mesh.polys.iter().enumerate() {
        for i in 0..p.len() {
            faces
                .entry(Edge::new(p[i], p[(i + 1) % p.len()]))
                .or_default()
                .push(f);
        }
    }
    let n = |f: usize| poly_normal(&mesh.points, &mesh.polys[f]);
    faces
        .into_iter()
        .filter(|(_, fs)| fs.len() == 2 && n(fs[0]).dot(n(fs[1])) < 1e-6)
        .map(|(e, _)| e)
        .collect()
}

fn to_poly(m: &Mesh) -> PolyMesh {
    PolyMesh {
        points: m.positions.iter().map(|p| DVec3::from_array(*p)).collect(),
        polys: m.faces.clone(),
    }
}

#[test]
fn subdivision_matches_the_reference() {
    let mut reg = Registry::with_builtins();
    reg.install(&Primitives).unwrap();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden");
    let mut unsupported = BTreeSet::new();
    let mut matched = Vec::new();
    let mut failed: Vec<String> = Vec::new();
    for case in discover_cases(&root)
        .unwrap()
        .into_iter()
        .filter(|c| c.feature == "subdivision")
    {
        let key = format!("{}/{}", case.feature, case.name);
        let m = case.manifest().unwrap();
        // `input = "cube"` (older cases) or an `[params.input]` table (O20 cases).
        // the scripted follow-up case spells the generator by label (`type`, `subdivision_editor`)
        let params = legacy_params(m.params.clone().into_iter().collect());
        let input_table = params["input"].as_table();
        let input_kind = match input_table {
            Some(t) => t["type"].as_str().unwrap().to_string(),
            None => params["input"].as_str().unwrap().to_string(),
        };
        let input = match input_kind.as_str() {
            "cube" if input_table.is_some() => {
                let t = input_table.unwrap();
                let own: Table = t
                    .iter()
                    .filter(|(k, _)| *k != "type" && *k != "pos")
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect();
                let mut mesh =
                    build("forge.cube", &typed_params(&reg, "forge.cube", &own)).unwrap();
                let pos: Vec<f64> = t["pos"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|v| v.as_float().or(v.as_integer().map(|i| i as f64)).unwrap())
                    .collect();
                for p in &mut mesh.points {
                    *p += DVec3::new(pos[0], pos[1], pos[2]);
                }
                mesh
            }
            "pyramid" if input_table.is_some() => {
                let t = input_table.unwrap();
                let own: Table = t
                    .iter()
                    .filter(|(k, _)| *k != "type" && *k != "pos")
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect();
                build("forge.pyramid", &typed_params(&reg, "forge.pyramid", &own)).unwrap()
            }
            "cube" | "plane" => {
                let ty = format!("forge.{input_kind}");
                let table = with_defaults(&case, &m, &input_kind, &Table::new());
                build(&ty, &typed_params(&reg, &ty, &table)).unwrap()
            }
            _ => {
                let src = std::fs::read_to_string(case.dir.join("input.obj")).unwrap();
                to_poly(&parse_obj(&src).unwrap())
            }
        };
        let creases = if input_kind == "cylinder" {
            separate_surface_edges(&input)
        } else {
            BTreeSet::new()
        };
        let level = ["level", "level_view", "subdivision_editor"]
            .iter()
            .find_map(|k| params.get(*k))
            .and_then(|v| v.as_integer())
            .unwrap() as usize;
        let sub_ty = reg.get("forge.subdivision").unwrap();
        let scheme = params
            .get("scheme")
            .or(params.get("type"))
            .and_then(|v| v.as_str())
            .map(|s| enum_id(sub_ty.find_param("scheme").unwrap(), s))
            .unwrap_or_else(|| "catmull_clark_ngons".to_string());
        let scheme = scheme.as_str();
        match subdivide_with_creases(&input, level, scheme, &creases) {
            Err(BuildError::Unsupported(what)) => {
                println!("{key}: unsupported ({what})");
                unsupported.insert(key);
            }
            Err(e) => panic!("{key}: {e}"),
            Ok(ours) => {
                let ours = Mesh {
                    positions: ours.points.iter().map(|p| p.to_array()).collect(),
                    faces: ours.polys,
                };
                let expected = case.expected_mesh().unwrap().unwrap();
                let r = compare_meshes(&expected, &ours, &m.tolerance);
                println!(
                    "{key}: passed {} (points {} vs {}, faces {} vs {})",
                    r.passed(),
                    expected.positions.len(),
                    ours.positions.len(),
                    expected.faces.len(),
                    ours.faces.len()
                );
                if r.passed() {
                    matched.push(key);
                } else {
                    failed.push(format!(
                        "{key}: {:?} (max error {:?})",
                        r.failures, r.max_position_error
                    ));
                }
            }
        }
    }
    println!("matched exactly: {}", matched.join(", "));
    assert!(failed.is_empty(), "unlisted mismatches: {failed:#?}");
    let want: BTreeSet<String> = UNSUPPORTED.iter().map(|s| s.to_string()).collect();
    assert_eq!(unsupported, want, "unsupported set changed");
    assert_eq!(matched.len(), 17, "{matched:?}");
}
