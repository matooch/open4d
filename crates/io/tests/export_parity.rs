//! docs/specs/IO.md §1.5: golden scenes through our OBJ writer.
//!
//! Each primitive and generator golden case that matches in
//! `crates/generators/tests/{parity,generators_parity}.rs` is built as a scene, exported
//! with `export_scene` + `write`, parsed by the harness and compared with `mesh.obj`.

use std::collections::BTreeSet;
use std::path::Path;
use std::sync::Arc;

use forge_core::{Document, NodeId, ParamKind, Registry, Value};
use forge_generators::{BuildError, Primitives, evaluate};
use forge_io::obj::{ReadOptions, WriteOptions, read, write};
use forge_io::{export_scene, import_into};
use forge_modifiers::Deformers;
use forge_parity_harness::{
    Mesh, Tolerance, compare_geometric, compare_meshes, discover_cases, export_weld,
    max_normal_error, parse_obj, parse_obj_corner_normals,
};

const PRIMITIVES: &[&str] = &[
    "plane", "cube", "sphere", "cylinder", "cone", "disc", "torus", "tube", "capsule", "pyramid",
];
const GENERATORS: &[&str] = &[
    "symmetry", "array", "connect", "instance", "extrude", "lathe", "loft", "sweep",
];

/// Cases whose options are observed but not built (none today; asserted exactly).
const UNSUPPORTED: &[&str] = &[];

/// Built but differing from the reference, pinned like in `generators_parity.rs`
/// (GENERATORS2.md §9–10). Through the writer they still differ.
const KNOWN_MISMATCH: &[&str] = &["loft/mesh-subdivision", "sweep/helix"];

/// Exact except for trailing cut / cap triangles that the reference triangulates with an
/// underived method, plus a geometric comparison: of the whole mesh for the slices
/// (PRIMITIVES2.md §10), of the caps alone for the lofts (GENERATORS2.md §9–10).
const CUT_GEOMETRIC: &[&str] = &["cone/slice", "torus/slice", "capsule/slice"];
const CAP_GEOMETRIC: &[&str] = &["loft/defaults", "loft/three"];

/// Generator cases compared after the harness's tolerant weld on both sides, as in
/// `generators_parity.rs` (reference rounding noise creates near-duplicate points).
fn weld_both_sides(feature: &str) -> bool {
    GENERATORS.contains(&feature)
}

fn registry() -> Arc<Registry> {
    let mut r = Registry::with_builtins();
    r.install(&Primitives).unwrap();
    r.install(&Deformers).unwrap();
    Arc::new(r)
}

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

type Table = toml::map::Map<String, toml::Value>;

/// Sets a node's params from a case table (same rules as `generators_parity.rs`).
fn apply(tx: &mut forge_core::Tx, reg: &Registry, id: NodeId, type_id: &str, table: &Table) {
    let ty = reg.get(type_id).unwrap();
    for (k, v) in table {
        // `type` names an input's node type, unless the node has a `type` param itself
        if (k == "type" && ty.find_param("type").is_none()) || v.is_table() {
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

/// Builds the case's scene; returns the document and the node under test.
fn build_scene(reg: &Arc<Registry>, feature: &str, params: &Table) -> (Document, NodeId) {
    let mut inputs: Vec<(&String, &Table)> = params
        .iter()
        .filter_map(|(k, v)| v.as_table().map(|t| (k, t)))
        .collect();
    inputs.sort_by_key(|(k, _)| k.trim_start_matches("input").parse::<u32>().unwrap_or(1));
    let mut doc = Document::new(reg.clone());
    let type_id = format!("forge.{feature}");
    let id = doc
        .transact("case", |tx| {
            let instance = feature == "instance";
            let id = tx.create(&type_id, None, None)?;
            apply(tx, reg, id, &type_id, params);
            for (_, t) in &inputs {
                let ty = format!("forge.{}", t["type"].as_str().unwrap());
                let parent = if instance { None } else { Some(id) };
                let child = tx.create(&ty, parent, None)?;
                apply(tx, reg, child, &ty, t);
                if instance {
                    tx.set_param(id, "reference", Value::Link(Some(child)))?;
                }
            }
            Ok(id)
        })
        .unwrap();
    (doc, id)
}

/// The harness mesh our writer produces for the scene (or the selected node).
fn exported(doc: &Document, node: Option<NodeId>) -> Mesh {
    let roots = node.map(|n| vec![n]);
    let objects = export_scene(doc.scene(), roots.as_deref());
    assert_eq!(objects.len(), 1, "exactly one exported object");
    let text = write(&objects, &WriteOptions::default());
    parse_obj(&text).expect("our own file parses")
}

/// Exact positions and faces up to a trailing run of triangles in both meshes, and a
/// geometric match of the tail (`caps_only`) or of the whole mesh.
fn check_triangle_tail(key: &str, expected: &Mesh, ours: &Mesh, tol: &Tolerance, caps_only: bool) {
    let points = |m: &Mesh| Mesh {
        positions: m.positions.clone(),
        faces: Vec::new(),
    };
    let same = compare_meshes(&points(expected), &points(ours), tol);
    assert!(same.passed(), "{key}: {:?}", same.failures);
    assert_eq!(expected.faces.len(), ours.faces.len(), "{key}: face count");
    let first_diff = expected
        .faces
        .iter()
        .zip(&ours.faces)
        .position(|(a, b)| a != b)
        .unwrap_or(expected.faces.len());
    for m in [expected, ours] {
        assert!(
            m.faces[first_diff..].iter().all(|f| f.len() == 3),
            "{key}: faces differ before the triangle tail (face {first_diff})"
        );
    }
    let tail = |m: &Mesh| Mesh {
        positions: m.positions.clone(),
        faces: m.faces[if caps_only { first_diff } else { 0 }..].to_vec(),
    };
    let r = compare_geometric(&tail(expected), &tail(ours), tol);
    assert!(r.passed(), "{key}: {:?}", r.failures);
}

#[test]
fn golden_scenes_through_the_writer_match_the_reference() {
    let reg = registry();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden");
    let mut matched = Vec::new();
    let mut unsupported = BTreeSet::new();
    let mut mismatched = BTreeSet::new();
    for case in discover_cases(&root).unwrap() {
        let feature = case.feature.as_str();
        let is_gen = GENERATORS.contains(&feature);
        if !is_gen && !PRIMITIVES.contains(&feature) {
            continue;
        }
        let key = format!("{}/{}", case.feature, case.name);
        let m = case.manifest().unwrap();
        let params: Table = m.params.clone().into_iter().collect();
        let (doc, id) = build_scene(&reg, feature, &params);
        match evaluate(doc.scene(), id) {
            Err(BuildError::Unsupported(what)) => {
                println!("{key}: unsupported ({what})");
                unsupported.insert(key);
                continue;
            }
            Err(e) => panic!("{key}: {e}"),
            Ok(None) => panic!("{key}: no geometry"),
            Ok(Some(_)) => {}
        }
        // generators export the selected node (the instance reference stays out of it),
        // primitives export the whole scene
        let ours = exported(&doc, is_gen.then_some(id));
        let expected = case.expected_mesh().unwrap().unwrap();
        let (expected, ours) = if weld_both_sides(feature) {
            (export_weld(&expected), export_weld(&ours))
        } else {
            (expected, ours)
        };
        if KNOWN_MISMATCH.contains(&key.as_str()) {
            let r = compare_meshes(&expected, &ours, &m.tolerance);
            assert!(
                !r.passed(),
                "{key} matches now: remove it from KNOWN_MISMATCH"
            );
            mismatched.insert(key);
            continue;
        }
        let cut = CUT_GEOMETRIC.contains(&key.as_str());
        if cut || CAP_GEOMETRIC.contains(&key.as_str()) {
            check_triangle_tail(&key, &expected, &ours, &m.tolerance, !cut);
            matched.push(key);
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
    println!(
        "through the writer: {} cases: {}",
        matched.len(),
        matched.join(", ")
    );
    let set = |l: &[&str]| l.iter().map(|s| s.to_string()).collect::<BTreeSet<_>>();
    assert_eq!(unsupported, set(UNSUPPORTED), "unsupported set changed");
    assert_eq!(
        mismatched,
        set(KNOWN_MISMATCH),
        "known-mismatch set changed"
    );
    assert_eq!(matched.len(), 75 + 31, "{matched:?}");
}

#[test]
fn offset_cube_keeps_its_world_position() {
    let reg = registry();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden");
    let case = discover_cases(&root)
        .unwrap()
        .into_iter()
        .find(|c| c.feature == "calibration" && c.name == "offset-cube")
        .expect("calibration case");
    let m = case.manifest().unwrap();
    let pos = ["pos_1", "pos_2", "pos_3"].map(|k| m.params[k].as_float().unwrap());
    assert_eq!(pos, [10.0, 20.0, 30.0]);
    let mut doc = Document::new(reg);
    doc.transact("cube", |tx| {
        let id = tx.create("forge.cube", None, None)?;
        tx.set_param(id, "transform.pos", Value::Vec3(pos))
    })
    .unwrap();
    let ours = exported(&doc, None);
    let expected = case.expected_mesh().unwrap().unwrap();
    let r = compare_meshes(&expected, &ours, &m.tolerance);
    assert!(r.passed(), "{:?}", r.failures);
    assert_eq!(ours.positions[0], [-90.0, -80.0, -70.0]);
    // the exporter's settings: same numbers, same winding as the golden file (flip_z off)
    assert_eq!(ours.faces, expected.faces);
}

#[test]
fn smoothing_cases_write_the_exported_normals() {
    let reg = registry();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden");
    let mut checked = 0;
    for case in discover_cases(&root).unwrap() {
        if case.feature != "smoothing" {
            continue;
        }
        let key = format!("{}/{}", case.feature, case.name);
        let m = case.manifest().unwrap();
        let src = std::fs::read_to_string(case.dir.join(m.mesh.as_deref().unwrap())).unwrap();
        let golden = parse_obj(&src).unwrap();
        // golden file -> polygon object with a smoothing tag -> writer
        let objects = read(&src, &ReadOptions::default()).unwrap();
        assert_eq!(objects.len(), 1, "{key}");
        let mut doc = Document::new(reg.clone());
        let degrees = m
            .params
            .get("smoothing_angle")
            .and_then(|v| v.as_float())
            .unwrap_or(40.0);
        let ids = doc
            .transact("import", |tx| {
                let ids = import_into(tx, &objects)?;
                let tag = tx.scene().node(ids[0]).unwrap().tags[0];
                tx.set_param(tag, "angle", Value::Float(degrees.to_radians()))?;
                Ok(ids)
            })
            .unwrap();
        assert_eq!(ids.len(), 1);
        let text = write(&export_scene(doc.scene(), None), &WriteOptions::default());
        let ours = parse_obj(&text).unwrap();
        let r = compare_meshes(&golden, &ours, &m.tolerance);
        assert!(r.passed(), "{key}: {:?}", r.failures);
        let expected: Vec<Vec<[f64; 3]>> = parse_obj_corner_normals(&src)
            .unwrap()
            .into_iter()
            .map(|f| f.into_iter().map(|n| n.expect("normal")).collect())
            .collect();
        let got: Vec<Vec<[f64; 3]>> = parse_obj_corner_normals(&text)
            .unwrap()
            .into_iter()
            .map(|f| f.into_iter().map(|n| n.expect("normal")).collect())
            .collect();
        let err = max_normal_error(&expected, &got).expect("same shape");
        assert!(err < 1e-5, "{key}: max normal error {err}");
        checked += 1;
    }
    assert_eq!(checked, 5);
}

#[test]
fn export_selected_and_skipped_nodes() {
    let reg = registry();
    let mut doc = Document::new(reg);
    let (a, b, bend, boole_a) = doc
        .transact("scene", |tx| {
            let a = tx.create("forge.cube", None, None)?;
            tx.rename(a, "A")?;
            let null = tx.create("forge.null", None, None)?;
            let b = tx.create("forge.sphere", Some(null), None)?;
            tx.rename(b, "B")?;
            let bend = tx.create("forge.bend", Some(b), None)?;
            let boole = tx.create("forge.boole", None, None)?;
            let boole_a = tx.create("forge.cube", Some(boole), None)?;
            let boole_b = tx.create("forge.cube", Some(boole), None)?;
            tx.set_param(boole_b, "transform.pos", Value::Vec3([100.0, 0.0, 0.0]))?;
            Ok((a, b, bend, boole_a))
        })
        .unwrap();
    let names = |roots: Option<&[NodeId]>| -> Vec<String> {
        export_scene(doc.scene(), roots)
            .into_iter()
            .map(|o| o.name)
            .collect()
    };
    let all = names(None);
    // A, B (deformer child and Null skipped), and the boolean result; its inputs are consumed
    assert_eq!(all.len(), 3, "{all:?}");
    assert_eq!(&all[..2], ["A", "B"]);
    assert_eq!(names(Some(&[a])), ["A"]);
    assert_eq!(names(Some(&[b])), ["B"]);
    // a selected ancestor and its descendant are taken once
    let null = doc.scene().node(b).unwrap().parent.unwrap();
    assert_eq!(names(Some(&[null, b, bend])), ["B"]);
    // consumed inputs and deformers alone give nothing
    assert!(names(Some(&[boole_a, bend])).is_empty());
    assert!(names(Some(&[NodeId(9999)])).is_empty());
}
