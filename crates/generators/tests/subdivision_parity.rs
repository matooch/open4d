//! Parity: docs/specs/SUBDIVISION.md §5 against tests/golden/subdivision.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use forge_generators::{BuildError, build, subdivide_with_creases};
use forge_geometry::Edge;
use forge_geometry::PolyMesh;
use forge_geometry::normals::poly_normal;
use forge_parity_harness::{Mesh, compare_meshes, discover_cases, parse_obj};
use glam::DVec3;

/// Observed schemes whose geometry is not derived yet.
const UNSUPPORTED: &[&str] = &[
    "subdivision/scheme-osd_bilinear",
    "subdivision/scheme-osd_catmull_clark",
    "subdivision/scheme-osd_loop",
];

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
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden");
    let mut unsupported = BTreeSet::new();
    let mut matched = Vec::new();
    for case in discover_cases(&root)
        .unwrap()
        .into_iter()
        .filter(|c| c.feature == "subdivision")
    {
        let key = format!("{}/{}", case.feature, case.name);
        let m = case.manifest().unwrap();
        let input_kind = m.params["input"].as_str().unwrap().to_string();
        let input = match input_kind.as_str() {
            "cube" => build("forge.cube", &BTreeMap::new()).unwrap(),
            "plane" => build("forge.plane", &BTreeMap::new()).unwrap(),
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
        let level = m.params["level"].as_integer().unwrap() as usize;
        let scheme = m
            .params
            .get("scheme")
            .and_then(|v| v.as_str())
            .unwrap_or("catmull_clark_ngons");
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
                assert!(
                    r.passed(),
                    "{key}: {:?} (max error {:?})",
                    r.failures,
                    r.max_position_error
                );
                matched.push(key);
            }
        }
    }
    println!("matched exactly: {}", matched.join(", "));
    let want: BTreeSet<String> = UNSUPPORTED.iter().map(|s| s.to_string()).collect();
    assert_eq!(unsupported, want, "unsupported set changed");
    assert_eq!(matched.len(), 8, "{matched:?}");
}
