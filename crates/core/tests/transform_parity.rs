//! Parity: docs/specs/TRANSFORM.md §4 against docs/parity/transform.md golden data.

mod common;

use std::path::{Path, PathBuf};

use forge_core::{Document, NodeId, Value};
use forge_parity_harness::{GoldenCase, Mesh, compare_meshes, discover_cases};

fn golden_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden")
}

fn cases(feature: &str) -> Vec<GoldenCase> {
    discover_cases(&golden_root())
        .unwrap()
        .into_iter()
        .filter(|c| c.feature == feature)
        .collect()
}

/// Reads `<prefix>pos_k`, `rot_k` (degrees), `scale_k` into the three transform params.
fn transform_values(
    params: &std::collections::BTreeMap<String, toml::Value>,
    prefix: &str,
) -> [[f64; 3]; 3] {
    let mut out = [[0.0; 3], [0.0; 3], [1.0; 3]];
    for (group, name) in out.iter_mut().zip(["pos", "rot", "scale"]) {
        for (k, slot) in group.iter_mut().enumerate() {
            if let Some(v) = params.get(&format!("{prefix}{name}_{}", k + 1)) {
                let x = v
                    .as_float()
                    .or_else(|| v.as_integer().map(|i| i as f64))
                    .expect("number");
                *slot = if name == "rot" { x.to_radians() } else { x };
            }
        }
    }
    out
}

fn set_transform(
    tx: &mut forge_core::Tx,
    id: NodeId,
    t: [[f64; 3]; 3],
) -> Result<(), forge_core::EditError> {
    tx.set_param(id, "transform.pos", Value::Vec3(t[0]))?;
    tx.set_param(id, "transform.rot", Value::Vec3(t[1]))?;
    tx.set_param(id, "transform.scale", Value::Vec3(t[2]))
}

fn check(case: &GoldenCase, local: &Mesh) {
    let manifest = case.manifest().unwrap();
    let has_parent = manifest.params.keys().any(|k| k.starts_with("parent_"));
    let mut doc = Document::new(common::registry());
    let id = doc
        .transact("build", |tx| {
            let parent = if has_parent {
                let p = tx.create("forge.null", None, None)?;
                set_transform(tx, p, transform_values(&manifest.params, "parent_"))?;
                Some(p)
            } else {
                None
            };
            let id = tx.create("forge.null", parent, None)?;
            set_transform(tx, id, transform_values(&manifest.params, ""))?;
            Ok(id)
        })
        .unwrap();
    let m = doc.scene().world_matrix(id).unwrap();
    let ours = Mesh {
        positions: local
            .positions
            .iter()
            .map(|p| m.transform_point3(glam::DVec3::from_array(*p)).to_array())
            .collect(),
        faces: local.faces.clone(),
    };
    let expected = case.expected_mesh().unwrap().unwrap();
    let report = compare_meshes(&expected, &ours, &manifest.tolerance);
    assert!(
        report.passed(),
        "{}/{}: {:?} (max error {:?})",
        case.feature,
        case.name,
        report.failures,
        report.max_position_error
    );
}

#[test]
fn transform_cases_match_the_reference() {
    let all = cases("transform");
    assert!(
        all.len() >= 8,
        "expected the 8 observed transform cases, found {}",
        all.len()
    );
    let identity = all
        .iter()
        .find(|c| c.name == "identity")
        .expect("identity case");
    let local = identity.expected_mesh().unwrap().unwrap();
    for case in all.iter().chain(&cases("calibration")) {
        check(case, &local);
    }
}
