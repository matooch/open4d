//! Parity: clone-toolset spec C10 against tests/golden/cloner (level B clone data and
//! level A expanded geometry) for the Linear, Radial, Grid Array and Honeycomb modes.

use std::collections::BTreeSet;
use std::path::Path;
use std::sync::Arc;

use forge_clone_engine::CloneToolset;
use forge_core::{Category, Document, EvalCtx, NodeId, ParamKind, Registry, Tx, Value, transform};
use forge_fields::FieldToolset;
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
    reg.install(&FieldToolset).unwrap();
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

// ---------------------------------------------------------------------------------------
// Plain effector (spec E9)

/// Plain cases built but differing, with the reason (spec E9). None.
const PLAIN_KNOWN_MISMATCH: &[&str] = &[];

/// The case whose `case.toml` has no field table (the description says: a Linear field with
/// defaults, in the effector's list). It is rebuilt by its own test (spec E9).
const PLAIN_NO_FIELD_TABLE: &[&str] = &["linear-plain-field"];

/// Percent-valued effector keys that the oldest case files write without the suffix.
const OLD_PERCENT: &[&str] = &[
    "strength",
    "maximum",
    "minimum",
    "weight_transform",
    "u_transform",
    "v_transform",
    "modify_clone",
];

/// Keys of a field table that are not parameters (the colour/direction channel remapping).
const FIELD_SKIP: &[&str] = &[
    "type",
    "color_mode",
    "length_1005025",
    "scale_percent",
    "scale_percent_1005032",
    "invert_direction",
    "invert_rotations",
];

fn degrees(v: f64) -> f64 {
    v * std::f64::consts::PI / 180.0
}

/// One case key of a node table as `(param id, value)`; `None` for keys that are not
/// parameters of `ty_id`.
fn node_param(
    reg: &Registry,
    ty_id: &str,
    key: &str,
    raw: &toml::Value,
    renames: &[(&str, &str)],
) -> Option<(String, Value)> {
    let ty = reg.get(ty_id).unwrap();
    if key == "name" {
        return None;
    }
    let renames = [renames, &[("scale_1018", "scale_value")][..]].concat();
    let renamed = renames
        .iter()
        .find(|(from, _)| *from == key)
        .map_or(key, |(_, to)| *to);
    let (id, scale) = match renamed {
        "position" if raw.is_array() => (transform::POS.to_string(), 1.0),
        "rotation_deg" | "object_rotation_deg" => (transform::ROT.to_string(), degrees(1.0)),
        k if OLD_PERCENT.contains(&k) => (k.to_string(), 0.01),
        k => match (k.strip_suffix("_percent"), k.strip_suffix("_deg")) {
            (Some(b), _) => (b.to_string(), 0.01),
            (_, Some(b)) => (b.to_string(), degrees(1.0)),
            _ => (k.to_string(), 1.0),
        },
    };
    let desc = ty.find_param(&id)?;
    // The strings of the field's channel remapping ("Use Value", "No Remap") share ids
    // with numeric parameters.
    if raw.is_str() && matches!(desc.kind, ParamKind::Float { .. }) {
        return None;
    }
    let value = match (&desc.kind, raw) {
        (ParamKind::Bool, toml::Value::Boolean(b)) => Value::Bool(*b),
        (ParamKind::Int { .. }, toml::Value::Integer(i)) => Value::Int(*i),
        // The Random effector's Seed is a whole number the case files write as a float.
        (ParamKind::Int { .. }, toml::Value::Float(f)) => Value::Int(f.round() as i64),
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
        (ParamKind::String, toml::Value::String(s)) => Value::String(s.clone()),
        (ParamKind::Enum { options }, toml::Value::String(s)) => {
            let o = options
                .iter()
                .find(|o| o.label.eq_ignore_ascii_case(s))
                .unwrap_or_else(|| panic!("{id}: no option labelled {s:?}"));
            Value::Enum(o.id.clone())
        }
        // The strings of the field's channel remapping ("Use Value", "No Remap").
        (_, toml::Value::String(_)) => return None,
        other => panic!("{id}: unexpected value {other:?}"),
    };
    Some((id, desc.validate(value).expect("valid case value")))
}

/// Creates a node of `ty` at the root with the case table's parameters.
fn create_node(
    tx: &mut Tx,
    reg: &Registry,
    ty: &str,
    table: &Table,
    skip: &[&str],
    renames: &[(&str, &str)],
) -> NodeId {
    let id = tx.create(ty, None, None).unwrap();
    for (key, raw) in table {
        if raw.is_table() || skip.contains(&key.as_str()) {
            continue;
        }
        if let Some((param, v)) = node_param(reg, ty, key, raw, renames) {
            tx.set_param(id, &param, v).unwrap();
        }
    }
    id
}

fn field_type_of(label: &str) -> &'static str {
    match label {
        "Spherical Field" => "forge.field.spherical",
        "Linear Field" => "forge.field.linear",
        other => panic!("field type {other:?} is not part of brief 068"),
    }
}

/// The cloner with its 20 cm cube, its Plain effectors in list order and their fields.
fn build_with_effectors(
    doc: &mut Document,
    reg: &Registry,
    mode: &str,
    table: &Table,
    effectors: &[&Table],
) -> NodeId {
    let cloner = build(doc, reg, mode, table);
    doc.transact("effectors", |tx: &mut Tx| {
        let mut ids = Vec::new();
        for e in effectors {
            let ty = match e.get("type").and_then(|t| t.as_str()) {
                Some("random") => "forge.effector.random",
                _ => "forge.effector.plain",
            };
            let normalised = normalise_random(e);
            let e = if ty == "forge.effector.random" {
                &normalised
            } else {
                *e
            };
            let eff = create_node(tx, reg, ty, e, &["type", "use_alpha_strength_1071"], &[]);
            if let Some(toml::Value::Table(f)) = e.get("field") {
                let field = create_node(
                    tx,
                    reg,
                    field_type_of(f["type"].as_str().unwrap()),
                    f,
                    FIELD_SKIP,
                    &[("size", "radius")],
                );
                tx.set_param(eff, "fields", Value::LinkList(vec![field]))
                    .unwrap();
            }
            ids.push(eff);
        }
        tx.set_param(cloner, "effectors", Value::LinkList(ids))
    })
    .unwrap();
    cloner
}

/// The effector tables of a case in list order (`effector`, `effector_2`, ...).
fn effector_tables(params: &std::collections::BTreeMap<String, toml::Value>) -> Vec<&Table> {
    let mut out = Vec::new();
    for key in std::iter::once("effector".to_string()).chain((2..).map(|n| format!("effector_{n}")))
    {
        match params.get(&key) {
            Some(toml::Value::Table(t)) => out.push(t),
            _ => break,
        }
    }
    out
}

#[test]
fn plain_effector_cases_match_the_reference() {
    let reg = registry();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden");
    let mut matched = Vec::new();
    let mut mismatched = BTreeSet::new();
    let ctx = EvalCtx::default();
    for case in discover_cases(&root).unwrap() {
        if case.feature != "cloner" {
            continue;
        }
        let m = case.manifest().unwrap();
        let effectors = effector_tables(&m.params);
        let all_plain = effectors
            .iter()
            .all(|e| e.get("type").and_then(|t| t.as_str()) == Some("plain"));
        if effectors.is_empty() || !all_plain {
            continue;
        }
        if PLAIN_NO_FIELD_TABLE.contains(&case.name.as_str()) {
            continue;
        }
        let mode = m.params["mode"].as_str().unwrap().to_string();
        let table: Table = m
            .params
            .iter()
            .filter(|(k, v)| {
                !v.is_table() && !k.starts_with("plain") && k.as_str() != "document_frame"
            })
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        let mut doc = Document::new(reg.clone());
        let cloner = build_with_effectors(&mut doc, &reg, &mode, &table, &effectors);

        let data = clone_data(doc.scene(), cloner, &ctx).unwrap().unwrap();
        assert!(data.is_consistent(), "{}", case.name);
        let ours = Samples::Clones {
            tolerance: 1e-4,
            clones: (0..data.len()).map(|i| sample_of(&data, i)).collect(),
        };
        let (_, expected) = case.samples().unwrap().remove(0);
        let samples = compare_samples(&expected, &ours);

        let mesh = evaluate(doc.scene(), cloner, &ctx)
            .unwrap()
            .expect("geometry");
        let ours = export_weld(&Mesh {
            positions: mesh.points.iter().map(|p| p.to_array()).collect(),
            faces: mesh.polys.clone(),
        });
        let want = export_weld(&case.expected_mesh().unwrap().unwrap());
        let geo = compare_meshes(&want, &ours, &m.tolerance);
        let ok = samples.passed() && geo.passed();
        if PLAIN_KNOWN_MISMATCH.contains(&case.name.as_str()) {
            assert!(
                !ok,
                "{} matches now: remove it from PLAIN_KNOWN_MISMATCH",
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
        PLAIN_KNOWN_MISMATCH.iter().map(|s| s.to_string()).collect()
    );
    assert_eq!(matched.len(), 13, "{matched:?}");
}

/// The effector list is in order and the order matters when the channels interact:
/// Absolute Scale 2 then Relative Scale 1 gives 3, the swapped list gives 2 (spec E1, E5).
#[test]
fn the_effector_list_applies_in_list_order() {
    let reg = registry();
    let mut doc = Document::new(reg.clone());
    let (cloner, first, second) = doc
        .transact("scene", |tx| {
            let cloner = tx.create("forge.cloner", None, None)?;
            tx.set_param(cloner, "mode", Value::Enum("linear".into()))?;
            tx.create("forge.cube", Some(cloner), None)?;
            let mk = |tx: &mut Tx, absolute: bool| -> Result<NodeId, forge_core::EditError> {
                let e = tx.create("forge.effector.plain", None, None)?;
                tx.set_param(e, "position", Value::Bool(false))?;
                tx.set_param(e, "scale", Value::Bool(true))?;
                tx.set_param(e, "uniform_scale", Value::Bool(true))?;
                tx.set_param(e, "absolute_scale", Value::Bool(absolute))?;
                tx.set_param(
                    e,
                    "scale_value",
                    Value::Float(if absolute { 2.0 } else { 1.0 }),
                )?;
                Ok(e)
            };
            let first = mk(tx, true)?;
            let second = mk(tx, false)?;
            Ok((cloner, first, second))
        })
        .unwrap();
    let scale_of = |doc: &Document| {
        clone_data(doc.scene(), cloner, &EvalCtx::default())
            .unwrap()
            .unwrap()
            .matrices[0]
            .x_axis
            .length()
    };
    let link = |doc: &mut Document, list: Vec<NodeId>| {
        doc.transact("link", |tx| {
            tx.set_param(cloner, "effectors", Value::LinkList(list))
        })
        .unwrap();
    };
    link(&mut doc, vec![first, second]);
    assert!((scale_of(&doc) - 3.0).abs() < 1e-12);
    link(&mut doc, vec![second, first]);
    assert!((scale_of(&doc) - 2.0).abs() < 1e-12);
    // The categories are checked: a cube cannot be an effector.
    let cube = doc.scene().node(cloner).unwrap().children[0];
    let err = doc.transact("bad", |tx| {
        tx.set_param(cloner, "effectors", Value::LinkList(vec![cube]))
    });
    assert!(err.is_err());
    // A node appears at most once, and never links to itself.
    let twice = doc.transact("twice", |tx| {
        tx.set_param(cloner, "effectors", Value::LinkList(vec![first, first]))
    });
    assert!(twice.is_err());
    assert_eq!(
        doc.scene().node_type(first).unwrap().category,
        Category::Effector
    );
}

/// `linear-plain-field`: the case file holds no field table, only the description (a Linear
/// field with defaults as the effector's only layer). Rebuilt here with the default Linear
/// field at the origin. Its Color is not recorded either, so it is read back from the samples
/// with the rule of spec E6 (`sample = 1 + (field colour - 1) * value`, value 0.5 at x = 0);
/// positions, the expanded mesh and the equal colours of all ten clones are checked
/// independently of that.
#[test]
fn linear_plain_field_matches_with_the_default_linear_field() {
    let reg = registry();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden");
    let case = discover_cases(&root)
        .unwrap()
        .into_iter()
        .find(|c| c.feature == "cloner" && c.name == "linear-plain-field")
        .unwrap();
    let m = case.manifest().unwrap();
    let (_, expected) = case.samples().unwrap().remove(0);
    let Samples::Clones { clones, .. } = &expected else {
        panic!("clone samples expected");
    };
    let first = clones[0].color;
    assert!(clones.iter().all(|c| c.color == first));
    let field_color = first.map(|c| 1.0 + (c - 1.0) / 0.5);

    let mode = m.params["mode"].as_str().unwrap().to_string();
    let table: Table = m
        .params
        .iter()
        .filter(|(k, v)| !v.is_table() && !k.starts_with("plain"))
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    let effector = effector_tables(&m.params)[0];
    let mut doc = Document::new(reg.clone());
    let cloner = build_with_effectors(&mut doc, &reg, &mode, &table, &[effector]);
    let eff = match doc.scene().param(cloner, "effectors") {
        Some(Value::LinkList(l)) => l[0],
        other => panic!("no effector list: {other:?}"),
    };
    doc.transact("field", |tx| {
        let f = tx.create("forge.field.linear", None, None)?;
        tx.set_param(f, "color", Value::Color(field_color))?;
        tx.set_param(eff, "fields", Value::LinkList(vec![f]))
    })
    .unwrap();

    let ctx = EvalCtx::default();
    let data = clone_data(doc.scene(), cloner, &ctx).unwrap().unwrap();
    let ours = Samples::Clones {
        tolerance: 1e-4,
        clones: (0..data.len()).map(|i| sample_of(&data, i)).collect(),
    };
    let samples = compare_samples(&expected, &ours);
    assert!(samples.passed(), "{:?}", samples.failures);
    let mesh = evaluate(doc.scene(), cloner, &ctx).unwrap().unwrap();
    let ours = export_weld(&Mesh {
        positions: mesh.points.iter().map(|p| p.to_array()).collect(),
        faces: mesh.polys.clone(),
    });
    let want = export_weld(&case.expected_mesh().unwrap().unwrap());
    let geo = compare_meshes(&want, &ours, &m.tolerance);
    assert!(geo.passed(), "{:?}", geo.failures);
}

// ---------------------------------------------------------------------------------------
// Random effector (spec E11-E16)

/// A Random effector table in one spelling: the older case files write the percent keys
/// without the suffix, and call the noise Scale `scale` / `scale_percent` and the Scale
/// checkbox `scale_1061` / `scale`.
fn normalise_random(e: &Table) -> Table {
    let mut out = Table::new();
    for (k, v) in e {
        let key = match (k.as_str(), v) {
            ("animation_speed", _) => "animation_speed_percent",
            ("scale", toml::Value::Float(_)) | ("scale_percent", _) => "noise_scale_percent",
            ("scale_1061", _) => "scale",
            (other, _) => other,
        };
        out.insert(key.to_string(), v.clone());
    }
    out
}

fn random_effector(m: &forge_parity_harness::CaseManifest) -> Option<Table> {
    let e = effector_tables(&m.params);
    match e.as_slice() {
        [t] if t.get("type").and_then(|t| t.as_str()) == Some("random") => Some((*t).clone()),
        _ => None,
    }
}

/// The values the effector drew, per randomised channel, read back from clone data: the
/// offset divided by P, the angle divided by R, the axis length minus 1.
#[derive(Default, Debug)]
struct Drawn {
    position: Vec<f64>,
    rotation: Vec<f64>,
    scale: Vec<f64>,
}

/// Compares `data` with the unaffected `base` clone by clone. Channels the effector does not
/// touch must be equal within `tol` (the exact part); the others are read back as values.
fn read_back(
    data: &forge_clone_engine::CloneData,
    base: &forge_clone_engine::CloneData,
    e: &Table,
    tol: f64,
    what: &str,
) -> Drawn {
    let on = |k: &str| e.get(k).and_then(|v| v.as_bool()).unwrap_or(false);
    let vec3 = |k: &str, scale: f64| -> glam::DVec3 {
        let f = floats(&e[k]);
        glam::DVec3::new(f[0], f[1], f[2]) * scale
    };
    let mut out = Drawn::default();
    assert_eq!(data.len(), base.len(), "{what}: clone count");
    for i in 0..data.len() {
        let (a, b) = (data.matrices[i], base.matrices[i]);
        let close = |x: glam::DVec3, y: glam::DVec3, name: &str| {
            assert!(
                (x - y).length() < tol,
                "{what}: clone {i} {name}: {x:?} != {y:?}"
            );
        };
        assert_eq!(data.visible[i], base.visible[i], "{what}: visible {i}");
        assert!(
            (data.weights[i] - base.weights[i]).abs() < tol,
            "{what}: weight {i}"
        );
        for k in 0..3 {
            assert!(
                (data.colors[i][k] - base.colors[i][k]).abs() < tol,
                "{what}: colour {i}"
            );
        }
        if on("position") {
            let p = vec3("p", 1.0);
            let d = (a.w_axis - b.w_axis).truncate();
            out.position.extend((0..3).map(|k| d[k] / p[k]));
        } else {
            close(a.w_axis.truncate(), b.w_axis.truncate(), "position");
        }
        if on("rotation") {
            let r = vec3("r_deg", std::f64::consts::PI / 180.0);
            let h = forge_core::transform::hpb_from_rotation(glam::DMat3::from_mat4(a));
            out.rotation.extend((0..3).map(|k| h[k] / r[k]));
        } else {
            // Directions (the lengths change when Scale is on).
            let dir = |v: glam::DVec4| v.truncate().normalize();
            close(dir(a.x_axis), dir(b.x_axis), "x axis");
            close(dir(a.y_axis), dir(b.y_axis), "y axis");
            close(dir(a.z_axis), dir(b.z_axis), "z axis");
            if !on("scale") {
                close(a.x_axis.truncate(), b.x_axis.truncate(), "x axis length");
            }
        }
        if on("scale") {
            let (lx, ly, lz) = (a.x_axis.length(), a.y_axis.length(), a.z_axis.length());
            let base_len = b.x_axis.length();
            out.scale
                .extend([lx - base_len, ly - base_len, lz - base_len]);
        }
    }
    out
}

fn mean_var(v: &[f64]) -> (f64, f64) {
    let n = v.len() as f64;
    let mean = v.iter().sum::<f64>() / n;
    (mean, v.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / n)
}

/// Spec E16: mean and variance of `n` values against `U[-1, 1]` within 3 standard errors
/// of the model (the standard deviation of the sample variance is `0.298 / sqrt(n)`).
fn uniform_band(v: &[f64]) -> Result<(), String> {
    let n = v.len() as f64;
    let (mean, var) = mean_var(v);
    let mean_tol = 3.0 * (1.0 / 3.0_f64 / n).sqrt();
    let var_tol = 3.0 * 0.298 / n.sqrt();
    if mean.abs() > mean_tol || (var - 1.0 / 3.0).abs() > var_tol {
        return Err(format!(
            "n {n}: mean {mean:.3} (tolerance {mean_tol:.3}), variance {var:.3} (1/3 +- {var_tol:.3})"
        ));
    }
    Ok(())
}

fn rms_step(v: &[f64]) -> f64 {
    let steps: Vec<f64> = v
        .chunks(3)
        .collect::<Vec<_>>()
        .windows(2)
        .flat_map(|w| (0..3).map(|k| w[1][k] - w[0][k]).collect::<Vec<_>>())
        .collect();
    (steps.iter().map(|s| s * s).sum::<f64>() / steps.len() as f64).sqrt()
}

/// Cases whose exact part matches but whose statistics do not fit the model, with the
/// reason (spec E16): Indexed.
const RANDOM_KNOWN_MISMATCH: &[&str] = &["random-indexed"];

#[test]
fn random_effector_cases_match_the_reference() {
    let reg = registry();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden");
    let ctx = EvalCtx::default();
    let mut exact = Vec::new();
    let mut drawn_ref: std::collections::BTreeMap<String, Drawn> = Default::default();
    let mut drawn_ours: std::collections::BTreeMap<String, Drawn> = Default::default();
    for case in discover_cases(&root).unwrap() {
        if case.feature != "cloner" {
            continue;
        }
        let m = case.manifest().unwrap();
        let Some(e) = random_effector(&m) else {
            continue;
        };
        let mode = m.params["mode"].as_str().unwrap().to_string();
        let table: Table = m
            .params
            .iter()
            .filter(|(k, v)| {
                !v.is_table()
                    && !k.starts_with("plain")
                    && !k.starts_with("random")
                    && k.as_str() != "document_frame"
            })
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        let normalised = normalise_random(&e);
        let mut doc = Document::new(reg.clone());
        let cloner = build_with_effectors(&mut doc, &reg, &mode, &table, &[&e]);
        let mut base_doc = Document::new(reg.clone());
        let base_cloner = build(&mut base_doc, &reg, &mode, &table);
        let ours = clone_data(doc.scene(), cloner, &ctx).unwrap().unwrap();
        let base = clone_data(base_doc.scene(), base_cloner, &ctx)
            .unwrap()
            .unwrap();
        assert!(ours.is_consistent(), "{}", case.name);

        // The reference, as clone data.
        let (_, expected) = case.samples().unwrap().remove(0);
        let Samples::Clones { clones, .. } = &expected else {
            panic!("{}: not clone samples", case.name);
        };
        let mut reference = base.clone();
        for (i, c) in clones.iter().enumerate() {
            reference.matrices[i] = glam::DMat4::from_cols(
                glam::DVec3::from(c.x).extend(0.0),
                glam::DVec3::from(c.y).extend(0.0),
                glam::DVec3::from(c.z).extend(0.0),
                glam::DVec3::from(c.pos).extend(1.0),
            );
            reference.colors[i] = c.color;
            reference.weights[i] = c.weight;
            reference.visible[i] = c.visible;
        }
        assert_eq!(clones.len(), base.len(), "{}: clone count", case.name);
        // Exact part: counts, visibility, colour, weight, and every channel that is not
        // randomised, for the reference and for us.
        let r = read_back(&reference, &base, &normalised, 1e-4, &case.name);
        let o = read_back(&ours, &base, &normalised, 1e-9, &case.name);
        // Level A counts of the expanded mesh.
        let mesh = evaluate(doc.scene(), cloner, &ctx)
            .unwrap()
            .expect("geometry");
        let ours_mesh = export_weld(&Mesh {
            positions: mesh.points.iter().map(|p| p.to_array()).collect(),
            faces: mesh.polys.clone(),
        });
        let want = export_weld(&case.expected_mesh().unwrap().unwrap());
        assert_eq!(
            want.positions.len(),
            ours_mesh.positions.len(),
            "{}",
            case.name
        );
        assert_eq!(want.faces.len(), ours_mesh.faces.len(), "{}", case.name);
        exact.push(case.name.clone());
        drawn_ref.insert(case.name.clone(), r);
        drawn_ours.insert(case.name.clone(), o);
    }
    exact.sort();
    assert_eq!(exact.len(), 8, "{exact:?}");

    // Statistical part (spec E16). Every value inside the range, ours and the reference.
    for (name, d) in drawn_ref.iter().chain(drawn_ours.iter()) {
        for v in d.position.iter().chain(&d.rotation).chain(&d.scale) {
            assert!((-1.0 - 1e-6..=1.0 + 1e-6).contains(v), "{name}: {v}");
        }
    }
    // The two files with the same seed hold the same values.
    let a = &drawn_ref["grid-random"].position;
    let b = &drawn_ref["grid-random-seed"].position;
    assert!(a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1e-6));
    // Random mode: 27 + 27 position values and the 5 uniform scales (n = 59).
    let mut pooled = drawn_ref["grid-random-seed"].position.clone();
    pooled.extend(&drawn_ref["grid-random-seed-7"].position);
    pooled.extend(drawn_ref["random-scale"].scale.iter().step_by(3));
    assert_eq!(pooled.len(), 59);
    uniform_band(&pooled).unwrap();
    uniform_band(&drawn_ref["grid-random-rot"].rotation).unwrap();
    println!("random pooled {:?}", mean_var(&pooled));
    println!(
        "rotation {:?}",
        mean_var(&drawn_ref["grid-random-rot"].rotation)
    );
    println!(
        "gaussian {:?}",
        mean_var(&drawn_ref["random-gaussian"].position)
    );
    println!(
        "indexed {:?}",
        mean_var(&drawn_ref["random-indexed"].position)
    );
    // Gaussian (n = 15).
    let g = &drawn_ref["random-gaussian"].position;
    let (mean, var) = mean_var(g);
    let n = g.len() as f64;
    assert!(mean.abs() <= 3.0 / 6.0 / n.sqrt(), "gaussian mean {mean}");
    assert!(
        (var - 1.0 / 36.0).abs() <= 3.0 / 36.0 * (2.0 / n).sqrt(),
        "gaussian variance {var}"
    );
    // Noise: smoothness of the clones along Y within a factor 2 of ours.
    let (rs, os) = (
        rms_step(&drawn_ref["random-noise"].position),
        rms_step(&drawn_ours["random-noise"].position),
    );
    println!("noise rms step: reference {rs:.4}, ours {os:.4}");
    assert!(
        rs > os / 2.0 && rs < os * 2.0,
        "noise steps {rs} vs ours {os}"
    );
    // Indexed: range only (checked above); the reference is not a uniform (known mismatch).
    assert!(uniform_band(&drawn_ref["random-indexed"].position).is_err());
    assert_eq!(RANDOM_KNOWN_MISMATCH, ["random-indexed"]);
}
