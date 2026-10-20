//! Parity: docs/specs/GENERATORS2.md §5 against tests/golden/{symmetry,array,connect,instance}.

use std::path::Path;
use std::sync::Arc;

use forge_core::{Document, NodeId, ParamKind, Registry, Value};
use forge_generators::{Primitives, evaluate};
use forge_parity_harness::{Mesh, compare_meshes, discover_cases, export_weld};

const FEATURES: &[&str] = &[
    "symmetry", "array", "connect", "instance", "extrude", "lathe", "loft", "sweep",
];

/// Observed cases not derived yet (GENERATORS2.md §9–10).
const UNSUPPORTED: &[&str] = &[
    "loft/caps-off",
    "loft/defaults",
    "loft/mesh-subdivision",
    "loft/three",
    "sweep/defaults",
    "sweep/end-rotation",
    "sweep/end-scale",
    "sweep/helix",
    "sweep/rectangle-profile",
];

fn vec3(v: &toml::Value) -> Value {
    let a = v.as_array().expect("pos is an array");
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

/// Sets a node's params from a case table: `pos` is the position, `type` is skipped,
/// sub-tables are inputs, everything else a param of the node.
fn apply(
    tx: &mut forge_core::Tx,
    reg: &Registry,
    id: NodeId,
    type_id: &str,
    table: &toml::map::Map<String, toml::Value>,
) {
    let ty = reg.get(type_id).unwrap();
    for (k, v) in table {
        if k == "type" || v.is_table() {
            continue;
        }
        if k == "pos" {
            tx.set_param(id, "transform.pos", vec3(v)).unwrap();
            continue;
        }
        let desc = ty
            .find_param(k)
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
        tx.set_param(id, k, value).unwrap();
    }
}

#[test]
fn generators_match_the_reference() {
    let mut reg = Registry::with_builtins();
    reg.install(&Primitives).unwrap();
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
        let mut inputs: Vec<(&String, &toml::map::Map<String, toml::Value>)> = params
            .iter()
            .filter_map(|(k, v)| v.as_table().map(|t| (k, t)))
            .collect();
        inputs.sort_by_key(|(k, _)| k.trim_start_matches("input").parse::<u32>().unwrap_or(1));

        let mut doc = Document::new(reg.clone());
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
                    }
                }
                Ok(generator)
            })
            .unwrap();
        let mesh = match evaluate(doc.scene(), generator) {
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
        let ours = export_weld(&Mesh {
            positions: mesh
                .points
                .iter()
                .map(|p| world.transform_point3(*p).to_array())
                .collect(),
            faces: mesh.polys.clone(),
        });
        let expected = export_weld(&case.expected_mesh().unwrap().unwrap());
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
    assert_eq!(matched.len(), 13 + 11, "{matched:?}");
}
