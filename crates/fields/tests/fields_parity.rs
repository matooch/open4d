//! Parity: docs/specs/FIELDS.md against tests/golden/fields (level B, 729 samples per case)
//! for the Linear, Spherical, Box, Cylinder, Cone, Torus, Capsule and Radial fields and the
//! Remapping tab (exact), and the Random field (statistical, NOISE.md N7).

use std::collections::BTreeSet;
use std::path::Path;
use std::sync::Arc;

use forge_core::{Document, EvalCtx, ParamKind, Registry, Tx, Value, transform};
use forge_fields::{FieldList, FieldToolset};
use forge_parity_harness::{Samples, compare_samples, discover_cases};
use glam::DVec3;

/// Exact cases: the Linear, Spherical, Box (brief 057), Cylinder, Cone, Torus, Capsule and Radial
/// (brief 058) fields (`layer-opacity-50` is a Spherical field in a list at layer strength 50 %).
/// The Random cases are statistical (below); the multi-layer cases belong to 059 (blend modes,
/// groups, Formula, Shader).
fn in_scope(name: &str) -> bool {
    [
        "linear-",
        "spherical-",
        "box-",
        "cylinder-",
        "cone-",
        "torus-",
        "capsule-",
        "radial-",
    ]
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
        // Editor preview settings, not parameters (FIELDS.md F6).
        "viewplane_preview"
        | "show_x"
        | "show_y"
        | "show_z"
        | "viewplane_resolution"
        | "display_box" => return None,
        "size" if ty_id == forge_fields::SPHERICAL => ("radius".to_string(), 1.0),
        k if k.ends_with("_deg") && k != "rotation_deg" => (
            k.trim_end_matches("_deg").to_string(),
            std::f64::consts::PI / 180.0,
        ),
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
        (ParamKind::Int { .. }, toml::Value::Float(f)) => Value::Int(*f as i64),
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

/// A case built through the registry: the expected samples and our value at each point.
struct Built {
    points: Vec<DVec3>,
    expected: Samples,
    ours: Vec<f64>,
}

fn build(reg: &Arc<Registry>, case: &forge_parity_harness::GoldenCase) -> Built {
    let ctx = EvalCtx::default();
    let m = case.manifest().unwrap();
    let name = m.params["type"].as_str().unwrap().to_string();
    let ty = type_id(&name);
    let mut doc = Document::new(reg.clone());
    let node = doc
        .transact("case", |tx: &mut Tx| {
            let node = tx.create(&ty, None, None)?;
            for (key, raw) in &m.params {
                if let Some((id, v)) = param(reg, &ty, key, raw) {
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
    let Samples::Field { samples, .. } = &expected else {
        panic!("{}: not a field file", case.name)
    };
    let points: Vec<DVec3> = samples.iter().map(|(p, _)| DVec3::from_array(*p)).collect();
    let ours = list
        .sample_many(&points, &ctx)
        .iter()
        .map(|s| s.value)
        .collect();
    Built {
        points,
        expected,
        ours,
    }
}

#[test]
fn exact_field_types_match_the_reference() {
    let reg = registry();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden");
    let mut matched = BTreeSet::new();
    let mut skipped = 0;
    let mut mismatched = BTreeSet::new();
    for case in discover_cases(&root).unwrap() {
        if case.feature != "fields" {
            continue;
        }
        if !in_scope(&case.name) {
            skipped += 1;
            continue;
        }
        let b = build(&reg, &case);
        let Samples::Field { tolerance, .. } = &b.expected else {
            unreachable!()
        };
        let ours = Samples::Field {
            tolerance: *tolerance,
            samples: b
                .points
                .iter()
                .zip(&b.ours)
                .map(|(p, v)| (p.to_array(), *v))
                .collect(),
        };
        let report = compare_samples(&b.expected, &ours);
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
    // 27 (brief 057) + cylinder 2, cone 1, torus 1, capsule 1, radial 4 (brief 058).
    assert_eq!(matched.len(), 36, "{matched:?}");
    // 4 Random cases (statistical, below) + 17 blend / layer / group / formula / shader (059).
    assert_eq!(skipped, 21);
    assert_eq!(
        mismatched,
        KNOWN_MISMATCH.iter().map(|s| s.to_string()).collect()
    );
}

#[derive(Debug)]
struct Stats {
    mean: f64,
    std: f64,
    min: f64,
    max: f64,
    /// Correlation of neighbours one grid step (75) apart along x; x is the fastest index, rows of 9.
    lag1: f64,
}

fn stats(v: &[f64]) -> Stats {
    let n = v.len() as f64;
    let mean = v.iter().sum::<f64>() / n;
    let var = v.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / n;
    let (mut cov, mut cnt) = (0.0, 0.0);
    for row in v.chunks(9) {
        for w in row.windows(2) {
            cov += (w[0] - mean) * (w[1] - mean);
            cnt += 1.0;
        }
    }
    Stats {
        mean,
        std: var.sqrt(),
        min: v.iter().copied().fold(f64::MAX, f64::min),
        max: v.iter().copied().fold(f64::MIN, f64::max),
        lag1: cov / cnt / var,
    }
}

/// The Random cases are compared statistically (NOISE.md N7, FIELDS.md F6): the reference noise
/// cannot be reproduced value by value, so the distribution, the decorrelation at the grid step
/// and the exact Global = Field relation are checked.
#[test]
fn random_field_matches_the_reference_statistically() {
    let reg = registry();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden");
    let mut seen = BTreeSet::new();
    let mut by_name = std::collections::BTreeMap::new();
    for case in discover_cases(&root).unwrap() {
        if case.feature != "fields" || !case.name.starts_with("random-") {
            continue;
        }
        let b = build(&reg, &case);
        let Samples::Field { samples, .. } = &b.expected else {
            unreachable!()
        };
        let theirs: Vec<f64> = samples.iter().map(|(_, v)| *v).collect();
        assert_eq!(theirs.len(), 729);
        let (r, o) = (stats(&theirs), stats(&b.ours));
        println!(
            "{}: reference {r:?}\n{}  ours {o:?}",
            case.name,
            " ".repeat(case.name.len())
        );
        // Both inside [0, 1] and uncorrelated at the grid step.
        assert!(o.min >= 0.0 && o.max <= 1.0, "{}: {o:?}", case.name);
        assert!(r.lag1.abs() < 0.15 && o.lag1.abs() < 0.15, "{}", case.name);
        if case.name == "random-random-mode" {
            // Random mode is uniform: mean 0.5, std 1/sqrt(12) = 0.289.
            assert!((o.mean - 0.5).abs() < 0.05, "{o:?}");
            assert!((o.std - 0.2887).abs() < 0.04, "{o:?}");
            assert!(
                (r.mean - 0.5).abs() < 0.05 && (r.std - 0.2887).abs() < 0.04,
                "{r:?}"
            );
            assert!(o.min < 0.05 && o.max > 0.95, "{o:?}");
        } else {
            assert!(
                (o.mean - r.mean).abs() < 0.05,
                "{}: {o:?} vs {r:?}",
                case.name
            );
            assert!(
                (o.std - r.std).abs() < 0.25 * r.std,
                "{}: {o:?} vs {r:?}",
                case.name
            );
        }
        seen.insert(case.name.clone());
        by_name.insert(case.name.clone(), (theirs, b.ours));
    }
    assert_eq!(
        seen.into_iter().collect::<Vec<_>>(),
        [
            "random-defaults",
            "random-global",
            "random-random-mode",
            "random-seed"
        ]
    );
    // The field sits at the origin unrotated: Global and Field coincide, exactly, in both.
    assert_eq!(by_name["random-global"].0, by_name["random-defaults"].0);
    assert_eq!(by_name["random-global"].1, by_name["random-defaults"].1);
    // A different seed gives a different field.
    assert_ne!(by_name["random-seed"].1, by_name["random-defaults"].1);
}
