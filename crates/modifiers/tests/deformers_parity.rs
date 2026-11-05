//! Parity: docs/specs/MODIFIERS.md §0–6 and §9 against
//! tests/golden/{bend,twist,deformer-common,taper,bulge,shear,spherify,deformer-falloff,
//! squash-stretch,wrap} (§7).

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;

mod common;

use common::resolved_params;
use forge_core::{Document, NodeId, ParamKind, Registry, Value};
use forge_fields::FieldToolset;
use forge_generators::{Primitives, evaluate};
use forge_modifiers::Deformers;
use forge_parity_harness::{Mesh, compare_meshes, discover_cases, export_weld};

const FEATURES: &[&str] = &[
    "bend",
    "twist",
    "deformer-common",
    "taper",
    "bulge",
    "shear",
    "spherify",
    "deformer-falloff",
    "squash-stretch",
    "wrap",
];

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

/// The deformer-falloff case keys (O24) in our parameter names: `strength_deg` is
/// `strength` in degrees, `*_percent` is the percent param of that name, `keep_length` is
/// `keep_y_axis_length`; the rest is the same. `None` for keys that are not parameters.
fn falloff_deformer_table(
    params: &toml::map::Map<String, toml::Value>,
) -> toml::map::Map<String, toml::Value> {
    let mut out = toml::map::Map::new();
    for (k, v) in params {
        if v.is_table() || matches!(k.as_str(), "deformer" | "field_layers" | "alignment") {
            continue;
        }
        let key = match k.as_str() {
            "keep_length" => "keep_y_axis_length".to_string(),
            k => k
                .strip_suffix("_deg")
                .or_else(|| k.strip_suffix("_percent"))
                .unwrap_or(k)
                .to_string(),
        };
        let v = match (k.as_str(), v) {
            ("mode" | "curvature_type" | "wrap", toml::Value::String(m)) => {
                toml::Value::String(m.to_lowercase())
            }
            _ => v.clone(),
        };
        // Two keys of the same table may not collapse to one name.
        assert!(out.insert(key, v).is_none(), "duplicate key {k}");
    }
    out
}

/// The `[params.field]` table of an O24 case as a field node under the scene root;
/// returns its id. `size` is the Spherical radius, `*_percent` fractions, `layer_*` the
/// list row (returned as the row's strength; the row itself is stored on the deformer).
fn build_field(
    tx: &mut forge_core::Tx,
    reg: &Registry,
    table: &toml::map::Map<String, toml::Value>,
) -> (NodeId, f64) {
    let mut strength = 1.0;
    let name = table["type"].as_str().unwrap();
    let ty_id = format!("forge.field.{name}");
    let ty = reg.get(&ty_id).unwrap();
    let id = tx.create(&ty_id, None, None).unwrap();
    let num = |v: &toml::Value| {
        v.as_float()
            .or(v.as_integer().map(|i| i as f64))
            .expect("number")
    };
    for (k, v) in table {
        let (key, scale) = match k.as_str() {
            "type" | "layer_blend_mode" => continue,
            "layer_strength_percent" => {
                strength = num(v) / 100.0;
                continue;
            }
            // The colour and direction channel keys are not parameters (FIELDS.md F2).
            "color_mode"
            | "length_mode"
            | "length_scale_percent"
            | "invert_direction"
            | "strength_mode"
            | "strength_scale_percent"
            | "invert_rotations" => continue,
            "size" if name == "spherical" => ("radius".to_string(), 1.0),
            "position" => ("transform.pos".to_string(), 1.0),
            "rotation_deg" => ("transform.rot".to_string(), std::f64::consts::PI / 180.0),
            k => match k.strip_suffix("_percent") {
                Some(b) => (b.to_string(), 0.01),
                None => (k.to_string(), 1.0),
            },
        };
        let value = if key.starts_with("transform.") || key == "size" {
            let a = v.as_array().expect("vec3");
            Value::Vec3([num(&a[0]) * scale, num(&a[1]) * scale, num(&a[2]) * scale])
        } else {
            let desc = ty
                .find_param(&key)
                .unwrap_or_else(|| panic!("field {name}: {k:?} is not a param"));
            match (&desc.kind, v) {
                (ParamKind::Bool, toml::Value::Boolean(b)) => Value::Bool(*b),
                (ParamKind::Int { .. }, toml::Value::Integer(i)) => Value::Int(*i),
                (ParamKind::Float { .. }, v) => Value::Float(num(v) * scale),
                (ParamKind::Enum { options }, toml::Value::String(s)) => {
                    let o = options
                        .iter()
                        .find(|o| o.label.eq_ignore_ascii_case(s))
                        .unwrap_or_else(|| panic!("{key}: no option labelled {s:?}"));
                    Value::Enum(o.id.clone())
                }
                other => panic!("field {key}: unexpected value {other:?}"),
            }
        };
        tx.set_param(id, &key, value).unwrap();
    }
    (id, strength)
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
        if feature == "deformer-falloff" {
            let (_, input) = &inputs[0];
            let ty = type_id(input["type"].as_str().unwrap());
            let object = tx.create(&ty, None, None)?;
            apply(tx, reg, object, &ty, input);
            let deformer_ty = type_id(params["deformer"].as_str().unwrap());
            let deformer = tx.create(&deformer_ty, Some(object), None)?;
            apply(
                tx,
                reg,
                deformer,
                &deformer_ty,
                &falloff_deformer_table(params),
            );
            let layers = params["field_layers"].as_integer().unwrap_or(0);
            assert_eq!(layers, params.get("field").map_or(0, |_| 1), "one field");
            if let Some(field) = params.get("field").and_then(|v| v.as_table()) {
                let (f, strength) = build_field(tx, reg, field);
                let row = forge_core::LinkRow {
                    strength,
                    ..forge_core::LinkRow::new(f, "normal")
                };
                tx.set_param(deformer, "fields", Value::LinkRows(vec![row]))?;
            }
            return Ok(object);
        }
        if feature != "deformer-common" {
            let (_, input) = &inputs[0];
            let ty = type_id(input["type"].as_str().unwrap());
            let object = tx.create(&ty, None, None)?;
            apply(tx, reg, object, &ty, input);
            // `squash-stretch` is `forge.squash_stretch`; the 051 cases use the O9 / O11
            // keys (`*_percent`, `*_deg`), the same translation as the O24 cases.
            let deformer_ty = type_id(&feature.replace('-', "_"));
            let deformer = tx.create(&deformer_ty, Some(object), None)?;
            let table = match feature {
                "squash-stretch" | "wrap" => falloff_deformer_table(params),
                _ => params.clone(),
            };
            apply(tx, reg, deformer, &deformer_ty, &table);
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
    reg.install(&FieldToolset).unwrap();
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
        let params = resolved_params(&case, &m);

        let mut doc = Document::new(reg.clone());
        let target = build_case(&mut doc, &reg, &case.feature, &params);
        let mesh = match evaluate(doc.scene(), target, &forge_core::EvalCtx::default()) {
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
    assert_eq!(
        matched.len(),
        15 + 7 + 6 + 8 + 7 + 7 + 5 + 9 + 6 + 5,
        "{matched:?}"
    );
}
