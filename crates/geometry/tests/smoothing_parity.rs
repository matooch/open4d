//! Parity: docs/specs/SMOOTHING.md §5 against tests/golden/smoothing (docs/parity/smoothing.md).

use std::path::Path;

use forge_geometry::PolyMesh;
use forge_geometry::normals::{Weighting, corner_normals};
use forge_parity_harness::{discover_cases, max_normal_error, parse_obj, parse_obj_corner_normals};
use glam::DVec3;

const DEFAULT_ANGLE_DEG: f64 = 40.0;

#[test]
fn corner_normals_match_exported_normals() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden");
    let cases: Vec<_> = discover_cases(&root)
        .unwrap()
        .into_iter()
        .filter(|c| c.feature == "smoothing")
        .collect();
    assert_eq!(cases.len(), 5, "the observed smoothing cases");
    for case in cases {
        let manifest = case.manifest().unwrap();
        let src =
            std::fs::read_to_string(case.dir.join(manifest.mesh.as_deref().unwrap())).unwrap();
        let golden = parse_obj(&src).unwrap();
        let mesh = PolyMesh {
            points: golden
                .positions
                .iter()
                .map(|p| DVec3::from_array(*p))
                .collect(),
            polys: golden.faces.clone(),
        };
        let expected: Vec<Vec<[f64; 3]>> = parse_obj_corner_normals(&src)
            .unwrap()
            .into_iter()
            .map(|f| {
                f.into_iter()
                    .map(|n| n.expect("every corner has a normal"))
                    .collect()
            })
            .collect();
        let angle = manifest
            .params
            .get("smoothing_angle")
            .and_then(|v| v.as_float())
            .unwrap_or(DEFAULT_ANGLE_DEG)
            .to_radians();
        let ours: Vec<Vec<[f64; 3]>> = corner_normals(&mesh, angle, Weighting::Angle)
            .into_iter()
            .map(|f| f.into_iter().map(|n| n.to_array()).collect())
            .collect();
        let err = max_normal_error(&expected, &ours).expect("same shape");
        assert!(err < 1e-5, "{}: max normal error {err}", case.name);
    }
}
