//! Parity: docs/specs/MODIFIERS.md §0–2 and §9 against
//! tests/golden/{bend,twist,deformer-common}.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;

use forge_core::{Document, NodeId, ParamKind, Registry, Value};
use forge_generators::{Primitives, evaluate};
use forge_modifiers::Deformers;
use forge_parity_harness::{Mesh, compare_meshes, discover_cases, export_weld};

const FEATURES: &[&str] = &["bend", "twist", "deformer-common"];

/// Observed cases not derived yet (MODIFIERS.md §9).
const UNSUPPORTED: &[&str] = &[];

/// Cases we build but that differ, pinned so a fix shows up (MODIFIERS.md §9).
const KNOWN_MISMATCH: &[&str] = &[];

/// Golden `type` keys that are not our id with `forge.` in front.
fn type_id(name: &str) -> String {
    match name {
        "subdivision_surface" => "forge.subdivision".to_string(),
        other => format!("forge.{other}"),
    }
}

fn vec3(v: &toml::Value) -> Value {
    let a = v.as_array().expect("vec3 is an array");
    let f = |i: usize| {
        a[i].as_float()
            .or(a[i].as_integer().map(|x| x as f64))
            .unwrap()
    };
    Value::Vec3([f(0), f(1), f(2)])
}

/// Case files hold angles as typed in the app (degrees); angle params store radians.
fn angle(desc: &forge_core::ParamDesc, v: f64) -> f64 {
    if desc.ui.unit == forge_core::Unit::Angle {
        v.to_radians()
    } else {
        v
    }
}

/// Sets a node's params from a case table: `pos` / `rot` are the transform (`rot` is
/// H, P, B in degrees), `type`, `parent` and `hierarchy` describe the scene instead of
/// the node, everything else a param of the node (`level` is the app's subdivision
/// level, our `level_view`).
fn apply(
    tx: &mut forge_core::Tx,
    reg: &Registry,
    id: NodeId,
    type_id: &str,
    table: &toml::map::Map<String, toml::Value>,
) {
    let ty = reg.get(type_id).unwrap();
    for (k, v) in table {
        if v.is_table() || matches!(k.as_str(), "type" | "parent" | "hierarchy") {
            continue;
        }
        if k == "pos" {
            tx.set_param(id, "transform.pos", vec3(v)).unwrap();
            continue;
        }
        if k == "rot" {
            let a = v.as_array().expect("rot is an array");
            let rad = |i: usize| {
                a[i].as_float()
                    .or(a[i].as_integer().map(|x| x as f64))
                    .unwrap()
                    .to_radians()
            };
            tx.set_param(id, "transform.rot", Value::Vec3([rad(0), rad(1), rad(2)]))
                .unwrap();
            continue;
        }
        let key = if k == "level" { "level_view" } else { k };
        let desc = ty
            .find_param(key)
            .unwrap_or_else(|| panic!("{type_id}: {k:?} is not a param"));
        let value = match (&desc.kind, v) {
            (ParamKind::Bool, toml::Value::Boolean(b)) => Value::Bool(*b),
            (ParamKind::Int { .. }, toml::Value::Integer(i)) => Value::Int(*i),
            (ParamKind::Float { .. }, toml::Value::Integer(i)) => {
                Value::Float(angle(desc, *i as f64))
            }
            (ParamKind::Float { .. }, toml::Value::Float(f)) => Value::Float(angle(desc, *f)),
            (ParamKind::Vec3 { .. }, v @ toml::Value::Array(_)) => vec3(v),
            (ParamKind::Enum { .. }, toml::Value::String(s)) => Value::Enum(s.clone()),
            other => panic!("{type_id}.{k}: unexpected value {other:?}"),
        };
        tx.set_param(id, key, value).unwrap();
    }
}

/// Rebuilds the case's scene and returns the node whose mesh was exported
/// (`deformer-common/generator` exports the subdivision, everything else the object).
fn build_case(
    doc: &mut Document,
    reg: &Arc<Registry>,
    feature: &str,
    params: &toml::map::Map<String, toml::Value>,
) -> NodeId {
    let mut inputs: Vec<(&String, &toml::map::Map<String, toml::Value>)> = params
        .iter()
        .filter_map(|(k, v)| v.as_table().map(|t| (k, t)))
        .filter(|(k, _)| k.starts_with("input"))
        .collect();
    inputs.sort_by_key(|(k, _)| k.trim_start_matches("input").parse::<u32>().unwrap_or(1));
    let generator = params.get("generator").and_then(|v| v.as_table());

    doc.transact("case", |tx| {
        // `forge.bend` / `forge.twist` cases: `[params.input]` is the object and the
        // case's `[params]` are the deformer's, as a child of the object.
        if feature != "deformer-common" {
            let (_, input) = &inputs[0];
            let ty = type_id(input["type"].as_str().unwrap());
            let object = tx.create(&ty, None, None)?;
            apply(tx, reg, object, &ty, input);
            let deformer_ty = type_id(feature);
            let deformer = tx.create(&deformer_ty, Some(object), None)?;
            apply(tx, reg, deformer, &deformer_ty, params);
            return Ok(object);
        }

        let mut roots: BTreeMap<String, NodeId> = BTreeMap::new();
        let mut null = None;
        if let Some(g) = generator {
            let ty = type_id(g["type"].as_str().unwrap());
            let id = tx.create(&ty, None, None)?;
            apply(tx, reg, id, &ty, g);
            roots.insert("generator".to_string(), id);
        }
        for (key, table) in &inputs {
            let ty = type_id(table["type"].as_str().unwrap());
            let parent = match table.get("parent").and_then(|v| v.as_str()) {
                Some("null") => {
                    let n = *null
                        .get_or_insert_with(|| tx.create("forge.null", None, None).expect("null"));
                    Some(n)
                }
                Some(p) => Some(
                    *roots
                        .get(p)
                        .unwrap_or_else(|| panic!("{key}: unknown parent {p:?}")),
                ),
                None => None,
            };
            let id = tx.create(&ty, parent, None)?;
            apply(tx, reg, id, &ty, table);
            roots.insert(key.to_string(), id);
        }
        let target = roots
            .get("generator")
            .copied()
            .or_else(|| roots.get("input").copied())
            .expect("an input node");
        Ok(target)
    })
    .unwrap()
}

#[test]
fn deformers_match_the_reference() {
    let mut reg = Registry::with_builtins();
    reg.install(&Primitives).unwrap();
    reg.install(&Deformers).unwrap();
    let reg = Arc::new(reg);
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden");
    let mut matched = Vec::new();
    let mut unsupported = std::collections::BTreeSet::new();
    for case in discover_cases(&root).unwrap() {
        if !FEATURES.contains(&case.feature.as_str()) {
            continue;
        }
        let key = format!("{}/{}", case.feature, case.name);
        let m = case.manifest().unwrap();
        let params: toml::map::Map<String, toml::Value> = m.params.clone().into_iter().collect();

        let mut doc = Document::new(reg.clone());
        let target = build_case(&mut doc, &reg, &case.feature, &params);
        let mesh = match evaluate(doc.scene(), target) {
            Err(forge_generators::BuildError::Unsupported(what)) => {
                println!("{key}: unsupported ({what})");
                unsupported.insert(key);
                continue;
            }
            other => other
                .unwrap()
                .unwrap_or_else(|| panic!("{key}: no geometry")),
        };
        let world = doc.scene().world_matrix(target).unwrap();
        let ours = export_weld(&Mesh {
            positions: mesh
                .points
                .iter()
                .map(|p| world.transform_point3(*p).to_array())
                .collect(),
            faces: mesh.polys.clone(),
        });
        let expected = export_weld(&case.expected_mesh().unwrap().unwrap());
        if KNOWN_MISMATCH.contains(&key.as_str()) {
            let r = compare_meshes(&expected, &ours, &m.tolerance);
            assert!(
                !r.passed(),
                "{key} matches now: remove it from KNOWN_MISMATCH"
            );
            println!("{key}: known mismatch");
            continue;
        }
        let r = compare_meshes(&expected, &ours, &m.tolerance);
        assert!(
            r.passed(),
            "{key}: {:?} (max error {:?})",
            r.failures,
            r.max_position_error
        );
        matched.push(key);
    }
    println!("matched exactly: {}", matched.join(", "));
    let want: std::collections::BTreeSet<String> =
        UNSUPPORTED.iter().map(|s| s.to_string()).collect();
    assert_eq!(unsupported, want, "unsupported set changed");
    assert_eq!(matched.len(), 10 + 7 + 6, "{matched:?}");
}
