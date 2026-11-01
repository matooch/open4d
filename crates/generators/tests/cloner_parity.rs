//! Parity: clone-toolset spec C10 against tests/golden/cloner (level B clone data and
//! level A expanded geometry) for the Linear, Radial, Grid Array and Honeycomb modes.

use std::collections::BTreeSet;
use std::path::Path;
use std::sync::Arc;

use forge_clone_engine::CloneToolset;
use forge_core::{Document, EvalCtx, ParamKind, Registry, Tx, Value};
use forge_generators::{BuildError, Primitives, clone_data, evaluate};
use forge_parity_harness::{
    Mesh, Samples, compare_meshes, compare_samples, discover_cases, export_weld,
    samples::Clone as SampleClone,
};

/// Cases not run here, with the brief that owns them (clone-toolset spec C10).
const OUT_OF_SCOPE_OBJECT: &[&str] = &[
    "object-axis",
    "object-edge",
    "object-polycenter",
    "object-surface",
    "object-surface-seed",
    "object-vertex",
    "object-vertex-align-off",
    "object-volume",
];
/// Several children (Clones modes) and nesting: brief 067.
const OUT_OF_SCOPE_ORDER: &[&str] = &["order-iterate", "order-random", "order-sort", "nested"];

/// Observed cases we build but that differ, pinned so a fix shows up.
const KNOWN_MISMATCH: &[&str] = &[];

type Table = toml::map::Map<String, toml::Value>;

fn registry() -> Arc<Registry> {
    let mut reg = Registry::with_builtins();
    reg.install(&Primitives).unwrap();
    reg.install(&CloneToolset).unwrap();
    Arc::new(reg)
}

/// Case keys that other cases of the same form name differently (linear-defaults was
/// recorded with the field labels of the Linear tab).
fn alias(key: &str, table: &Table) -> String {
    let per_step = table.contains_key("step_rotation_per_step_deg");
    match key {
        "step_position" => "transform_position".into(),
        "step_scale_percent" => "transform_scale_percent".into(),
        "step_rotation_deg" if per_step => "transform_rotation_deg".into(),
        "step_rotation_per_step_deg" => "step_rotation_deg".into(),
        other => other.into(),
    }
}

fn floats(v: &toml::Value) -> Vec<f64> {
    v.as_array()
        .expect("array")
        .iter()
        .map(|x| x.as_float().or(x.as_integer().map(|i| i as f64)).unwrap())
        .collect()
}

/// One case key as a cloner parameter: `(id, value)`; `None` for keys that are not
/// parameters of the node.
fn param(reg: &Registry, mode: &str, table: &Table, key: &str) -> Option<(String, Value)> {
    if matches!(key, "mode" | "effector_count") {
        return None;
    }
    let key = alias(key, table);
    let (base, scale) = if let Some(b) = key.strip_suffix("_percent") {
        (b, 0.01)
    } else if let Some(b) = key.strip_suffix("_deg") {
        (b, std::f64::consts::PI / 180.0)
    } else {
        (key.as_str(), 1.0)
    };
    let ty = reg.get("forge.cloner").unwrap();
    let id = [
        format!("{mode}_{base}"),
        base.to_string(),
        format!("clone_{base}"),
    ]
    .into_iter()
    .find(|c| ty.find_param(c).is_some())
    .unwrap_or_else(|| panic!("case key {key:?} has no cloner parameter"));
    let desc = ty.find_param(&id).unwrap();
    let raw = &table[&key_of(table, &key)];
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
        (ParamKind::Color, v) => {
            let f = floats(v);
            Value::Color([f[0], f[1], f[2]])
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
    Some((id, desc.validate(value).expect("valid case value")))
}

/// The table key a (possibly aliased) key came from.
fn key_of(table: &Table, aliased: &str) -> String {
    table
        .keys()
        .find(|k| alias(k, table) == aliased)
        .unwrap_or_else(|| panic!("no key for {aliased}"))
        .clone()
}

fn has_effector(params: &std::collections::BTreeMap<String, toml::Value>) -> bool {
    params.contains_key("effector")
        || params
            .get("effector_count")
            .and_then(|v| v.as_integer())
            .is_some_and(|n| n > 0)
}

fn sample_of(c: &forge_clone_engine::CloneData, i: usize) -> SampleClone {
    let m = c.matrices[i];
    SampleClone {
        pos: m.w_axis.truncate().to_array(),
        x: m.x_axis.truncate().to_array(),
        y: m.y_axis.truncate().to_array(),
        z: m.z_axis.truncate().to_array(),
        color: c.colors[i],
        weight: c.weights[i],
        visible: c.visible[i],
    }
}

/// A cloner with a 20 cm cube child and the case's params.
fn build(doc: &mut Document, reg: &Registry, mode: &str, table: &Table) -> forge_core::NodeId {
    doc.transact("case", |tx: &mut Tx| {
        let cloner = tx.create("forge.cloner", None, None)?;
        tx.set_param(cloner, "mode", Value::Enum(mode.into()))?;
        for key in table.keys() {
            if let Some((id, v)) = param(reg, mode, table, key) {
                tx.set_param(cloner, &id, v)?;
            }
        }
        let cube = tx.create("forge.cube", Some(cloner), None)?;
        for axis in ["size_x", "size_y", "size_z"] {
            tx.set_param(cube, axis, Value::Float(20.0))?;
        }
        Ok(cloner)
    })
    .unwrap()
}

#[test]
fn cloner_modes_match_the_reference() {
    let reg = registry();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden");
    let mut matched = Vec::new();
    let mut effects = BTreeSet::new();
    let mut skipped = BTreeSet::new();
    let mut mismatched = BTreeSet::new();
    let ctx = EvalCtx::default();
    for case in discover_cases(&root).unwrap() {
        if case.feature != "cloner" {
            continue;
        }
        let m = case.manifest().unwrap();
        let mode = m.params["mode"].as_str().unwrap().to_string();
        if has_effector(&m.params) {
            effects.insert(case.name.clone());
            continue;
        }
        if mode == "object"
            || OUT_OF_SCOPE_OBJECT.contains(&case.name.as_str())
            || OUT_OF_SCOPE_ORDER.contains(&case.name.as_str())
        {
            skipped.insert(case.name.clone());
            continue;
        }
        let table: Table = m
            .params
            .iter()
            .filter(|(_, v)| !v.is_table())
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        let mut doc = Document::new(reg.clone());
        let cloner = build(&mut doc, &reg, &mode, &table);

        // Level B: the clone data.
        let data = clone_data(doc.scene(), cloner, &ctx).unwrap().unwrap();
        assert!(data.is_consistent(), "{}", case.name);
        let ours = Samples::Clones {
            tolerance: 1e-4,
            clones: (0..data.len()).map(|i| sample_of(&data, i)).collect(),
        };
        let (_, expected) = case.samples().unwrap().remove(0);
        let samples = compare_samples(&expected, &ours);

        // Level A: the expanded geometry.
        let mesh = match evaluate(doc.scene(), cloner, &ctx) {
            Err(BuildError::Unsupported(what)) => panic!("{}: unsupported ({what})", case.name),
            other => other.unwrap().expect("geometry"),
        };
        let ours = export_weld(&Mesh {
            positions: mesh.points.iter().map(|p| p.to_array()).collect(),
            faces: mesh.polys.clone(),
        });
        let want = export_weld(&case.expected_mesh().unwrap().unwrap());
        let geo = compare_meshes(&want, &ours, &m.tolerance);

        let ok = samples.passed() && geo.passed();
        if KNOWN_MISMATCH.contains(&case.name.as_str()) {
            assert!(
                !ok,
                "{} matches now: remove it from KNOWN_MISMATCH",
                case.name
            );
            mismatched.insert(case.name.clone());
            continue;
        }
        assert!(
            samples.passed(),
            "{}: samples {:?} (max error {})",
            case.name,
            samples.failures,
            samples.max_error
        );
        assert!(
            geo.passed(),
            "{}: mesh {:?} (max error {:?})",
            case.name,
            geo.failures,
            geo.max_position_error
        );
        matched.push(case.name.clone());
    }
    println!("matched: {}", matched.join(", "));
    assert_eq!(
        mismatched,
        KNOWN_MISMATCH.iter().map(|s| s.to_string()).collect()
    );
    assert_eq!(matched.len(), 19, "{matched:?}");
    assert_eq!(effects.len(), 35, "{effects:?}");
    assert_eq!(skipped.len(), 12, "{skipped:?}");
}

/// Nesting needs no extra code (the inner cloner is a prototype with geometry): the
/// `nested` golden mesh (outer Linear x3 of an inner Radial x4 of a cube) matches. The
/// case file does not record the inner cloner, so it is built here; brief 067 owns the
/// nesting rules and their samples.
#[test]
fn a_cloner_inside_a_cloner_matches_the_nested_mesh() {
    let reg = registry();
    let mut doc = Document::new(reg.clone());
    let outer = doc
        .transact("nested", |tx| {
            let outer = tx.create("forge.cloner", None, None)?;
            tx.set_param(outer, "mode", Value::Enum("linear".into()))?;
            let inner = tx.create("forge.cloner", Some(outer), None)?;
            tx.set_param(inner, "mode", Value::Enum("radial".into()))?;
            tx.set_param(inner, "radial_count", Value::Int(4))?;
            let cube = tx.create("forge.cube", Some(inner), None)?;
            for axis in ["size_x", "size_y", "size_z"] {
                tx.set_param(cube, axis, Value::Float(20.0))?;
            }
            Ok(outer)
        })
        .unwrap();
    let mesh = evaluate(doc.scene(), outer, &EvalCtx::default())
        .unwrap()
        .unwrap();
    let ours = export_weld(&Mesh {
        positions: mesh.points.iter().map(|p| p.to_array()).collect(),
        faces: mesh.polys.clone(),
    });
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden");
    let case = discover_cases(&root)
        .unwrap()
        .into_iter()
        .find(|c| c.feature == "cloner" && c.name == "nested")
        .unwrap();
    let want = export_weld(&case.expected_mesh().unwrap().unwrap());
    let r = compare_meshes(&want, &ours, &case.manifest().unwrap().tolerance);
    assert!(r.passed(), "{:?}", r.failures);
}
