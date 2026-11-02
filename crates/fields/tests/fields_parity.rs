//! Parity: docs/specs/FIELDS.md against tests/golden/fields (level B, 729 samples per case)
//! for the Linear, Spherical and Box fields and the Remapping tab.

use std::collections::BTreeSet;
use std::path::Path;
use std::sync::Arc;

use forge_core::{Document, EvalCtx, ParamKind, Registry, Tx, Value, transform};
use forge_fields::{FieldList, FieldToolset};
use forge_parity_harness::{Samples, compare_samples, discover_cases};
use glam::DVec3;

/// Cases of this brief: the Linear, Spherical and Box fields (`layer-opacity-50` is a Spherical
/// field in a list at layer strength 50 %). Other field types and the multi-layer cases belong
/// to 058 (Cylinder, Cone, Torus, Capsule, Radial, Random) and 059 (blend modes, groups,
/// Formula, Shader).
fn in_scope(name: &str) -> bool {
    ["linear-", "spherical-", "box-"]
        .iter()
        .any(|p| name.starts_with(p))
        || name == "layer-opacity-50"
}

/// Cases built but differing from the observation (pinned so a fix shows up).
const KNOWN_MISMATCH: &[&str] = &[];

/// Keys of the colour and direction channels, which no case samples (spec F2).
const CHANNEL_KEYS: &[&str] = &[
    "color_mode",
    "length_mode",
    "length_scale_percent",
    "invert_direction",
    "strength_mode",
    "strength_scale_percent",
    "invert_rotations",
];

fn floats(v: &toml::Value) -> Vec<f64> {
    v.as_array()
        .expect("array")
        .iter()
        .map(|x| x.as_float().or(x.as_integer().map(|i| i as f64)).unwrap())
        .collect()
}

fn registry() -> Arc<Registry> {
    let mut reg = Registry::with_builtins();
    reg.install(&FieldToolset).unwrap();
    Arc::new(reg)
}

fn type_id(name: &str) -> String {
    format!("forge.field.{name}")
}

/// One case key as a field parameter; `None` for keys that are not parameters.
fn param(reg: &Registry, ty_id: &str, key: &str, raw: &toml::Value) -> Option<(String, Value)> {
    let (id, scale) = match key {
        "type" | "layer_strength_percent" | "layer_blend_mode" => return None,
        "size" if ty_id == forge_fields::SPHERICAL => ("radius".to_string(), 1.0),
        "position" => (transform::POS.to_string(), 1.0),
        "rotation_deg" => (transform::ROT.to_string(), std::f64::consts::PI / 180.0),
        "scale" => (transform::SCALE.to_string(), 1.0),
        k if CHANNEL_KEYS.contains(&k) => return None,
        k => match k.strip_suffix("_percent") {
            Some(b) => (b.to_string(), 0.01),
            None => (k.to_string(), 1.0),
        },
    };
    let ty = reg.get(ty_id).unwrap();
    let desc = ty
        .find_param(&id)
        .unwrap_or_else(|| panic!("case key {key:?} has no parameter on {ty_id}"));
    let value = match (&desc.kind, raw) {
        (ParamKind::Bool, toml::Value::Boolean(b)) => Value::Bool(*b),
        (ParamKind::Int { .. }, toml::Value::Integer(i)) => Value::Int(*i),
        (ParamKind::Float { .. }, v) => Value::Float(
            v.as_float()
                .unwrap_or_else(|| v.as_integer().expect("number") as f64)
                * scale,
        ),
        (ParamKind::Vec3 { .. }, v) => {
            let f = floats(v);
            Value::Vec3([f[0] * scale, f[1] * scale, f[2] * scale])
        }
        (ParamKind::Enum { options }, toml::Value::String(s)) => {
            let o = options
                .iter()
                .find(|o| o.label.eq_ignore_ascii_case(s))
                .unwrap_or_else(|| panic!("{id}: no option labelled {s:?}"));
            Value::Enum(o.id.clone())
        }
        other => panic!("{id}: unexpected value {other:?}"),
    };
    Some((id.clone(), desc.validate(value).expect("valid case value")))
}

#[test]
fn linear_spherical_and_box_match_the_reference() {
    let reg = registry();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden");
    let ctx = EvalCtx::default();
    let mut matched = BTreeSet::new();
    let mut skipped = 0;
    let mut mismatched = BTreeSet::new();
    for case in discover_cases(&root).unwrap() {
        if case.feature != "fields" {
            continue;
        }
        let m = case.manifest().unwrap();
        if !in_scope(&case.name) {
            skipped += 1;
            continue;
        }
        let name = m.params["type"].as_str().unwrap().to_string();
        let ty = type_id(&name);
        let mut doc = Document::new(reg.clone());
        let node = doc
            .transact("case", |tx: &mut Tx| {
                let node = tx.create(&ty, None, None)?;
                for (key, raw) in &m.params {
                    if let Some((id, v)) = param(&reg, &ty, key, raw) {
                        tx.set_param(node, &id, v)?;
                    }
                }
                Ok(node)
            })
            .unwrap();
        let mut list = FieldList::from_nodes(doc.scene(), &[node]).unwrap();
        if let Some(pct) = m.params.get("layer_strength_percent") {
            assert_eq!(m.params["layer_blend_mode"].as_str(), Some("Normal"));
            list = list.with_strength(0, pct.as_float().unwrap() / 100.0);
        }
        let (_, expected) = case.samples().unwrap().remove(0);
        let Samples::Field { samples, tolerance } = &expected else {
            panic!("{}: not a field file", case.name)
        };
        let points: Vec<DVec3> = samples.iter().map(|(p, _)| DVec3::from_array(*p)).collect();
        let ours = Samples::Field {
            tolerance: *tolerance,
            samples: list
                .sample_many(&points, &ctx)
                .iter()
                .zip(samples)
                .map(|(s, (p, _))| (*p, s.value))
                .collect(),
        };
        let report = compare_samples(&expected, &ours);
        if KNOWN_MISMATCH.contains(&case.name.as_str()) {
            assert!(
                !report.passed(),
                "{} matches now: remove it from KNOWN_MISMATCH",
                case.name
            );
            mismatched.insert(case.name.clone());
            continue;
        }
        assert!(
            report.passed(),
            "{}: {:?} (max error {})",
            case.name,
            report.failures,
            report.max_error
        );
        matched.insert(case.name.clone());
    }
    assert_eq!(matched.len(), 27, "{matched:?}");
    assert_eq!(skipped, 30);
    assert_eq!(
        mismatched,
        KNOWN_MISMATCH.iter().map(|s| s.to_string()).collect()
    );
}
