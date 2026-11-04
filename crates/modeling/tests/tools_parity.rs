//! Parity: docs/specs/MODELING.md §1–3 against the Extrude, Extrude Inner and Bevel cases and
//! §4 against the mesh-command cases of tests/golden/modeling-tools.

use std::path::Path;
use std::sync::Arc;

use forge_core::{Document, EvalCtx, Registry};
use forge_generators::{Primitives, evaluate};
use forge_geometry::PolyMesh;
use forge_geometry::selection::{IndexSet, Selection};
use forge_modeling::{Settings, tool_desc, tool_fn};
use forge_parity_harness::{Mesh, compare_meshes, discover_cases, export_weld};

/// Tools this test covers; every other `modeling-tools` case belongs to later briefs.
const TOOLS: &[&str] = &["extrude", "extrude_inner", "inset", "bevel"];

/// Observed options we return `Unsupported` for (none among the golden cases).
const UNSUPPORTED: &[&str] = &[];

/// Cases that build but differ (none).
const KNOWN_MISMATCH: &[&str] = &[];

fn cube(segments: [i64; 3]) -> PolyMesh {
    let mut reg = Registry::with_builtins();
    reg.install(&Primitives).unwrap();
    let reg = Arc::new(reg);
    let mut doc = Document::new(reg);
    let id = doc
        .transact("cube", |tx| {
            let id = tx.create("forge.cube", None, None)?;
            for (axis, n) in ["x", "y", "z"].iter().zip(segments) {
                tx.set_param(id, &format!("segments_{axis}"), forge_core::Value::Int(n))?;
            }
            Ok(id)
        })
        .unwrap();
    evaluate(doc.scene(), id, &EvalCtx::default())
        .unwrap()
        .expect("cube geometry")
}

fn as_mesh(m: &PolyMesh) -> Mesh {
    Mesh {
        positions: m.points.iter().map(|p| p.to_array()).collect(),
        faces: m.polys.clone(),
    }
}

fn json_of(v: &toml::Value) -> serde_json::Value {
    match v {
        toml::Value::Boolean(b) => (*b).into(),
        toml::Value::Integer(i) => (*i).into(),
        toml::Value::Float(f) => (*f).into(),
        toml::Value::String(s) => s.clone().into(),
        toml::Value::Array(a) => a.iter().map(json_of).collect::<Vec<_>>().into(),
        other => panic!("unexpected setting {other:?}"),
    }
}

#[test]
fn extrude_and_inset_match_the_reference() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden");
    let mut matched = Vec::new();
    let mut unsupported = std::collections::BTreeSet::new();
    for case in discover_cases(&root).unwrap() {
        // `rechecks/extrude-bevel-subdiv` (O15) is an Extrude case
        if case.feature != "modeling-tools" && case.feature != "rechecks" {
            continue;
        }
        let m = case.manifest().unwrap();
        let Some(tool) = m.params.get("tool").and_then(|t| t.as_str()) else {
            continue;
        };
        let tool = tool.to_string();
        if !TOOLS.contains(&tool.as_str()) {
            continue;
        }
        let key = format!("{}/{}", case.feature, case.name);
        let seg: Vec<i64> = m.params["segments"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_integer().unwrap())
            .collect();
        let base = cube([seg[0], seg[1], seg[2]]);
        let mut sel = Selection::default();
        if let Some(polys) = m.params.get("select_polys") {
            sel.polys = polys
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_integer().unwrap() as u32)
                .collect();
        }
        if let Some(pairs) = m.params.get("select_edges") {
            let ids = base.edge_ids();
            sel.edges = pairs
                .as_array()
                .unwrap()
                .iter()
                .map(|p| {
                    let p = p.as_array().unwrap();
                    let (a, b) = (p[0].as_integer().unwrap(), p[1].as_integer().unwrap());
                    ids[&forge_geometry::Edge::new(a as u32, b as u32)]
                })
                .collect::<IndexSet>();
        }
        let desc = tool_desc(&tool).unwrap();
        let object: serde_json::Map<String, serde_json::Value> = m.params["settings"]
            .as_table()
            .unwrap()
            .iter()
            .map(|(k, v)| (k.clone(), json_of(v)))
            .collect();
        let settings = Settings::from_json(&desc, &object).unwrap();
        let result = tool_fn(desc.id).unwrap()(&base, &sel, &settings);
        let ours = match result {
            Err(forge_modeling::ToolError::Unsupported(what)) => {
                println!("{key}: unsupported ({what})");
                unsupported.insert(key);
                continue;
            }
            other => other.unwrap().0,
        };
        let expected = case.expected_mesh().unwrap().unwrap();
        let ours = as_mesh(&ours);
        let mismatch = KNOWN_MISMATCH.contains(&key.as_str());
        // the raw exported file, and the weld every other parity test compares through
        let raw = compare_meshes(&expected, &ours, &m.tolerance);
        let welded = compare_meshes(&export_weld(&expected), &export_weld(&ours), &m.tolerance);
        if mismatch {
            assert!(!(raw.passed() && welded.passed()), "{key} matches now");
            continue;
        }
        assert!(raw.passed(), "{key} raw: {:?}", raw.failures);
        assert!(welded.passed(), "{key} welded: {:?}", welded.failures);
        matched.push(key);
    }
    println!("matched exactly: {}", matched.join(", "));
    let want: std::collections::BTreeSet<String> =
        UNSUPPORTED.iter().map(|s| s.to_string()).collect();
    assert_eq!(unsupported, want, "unsupported set changed");
    assert_eq!(matched.len(), 25, "{matched:?}");
}

/// Tools of the mesh commands (spec §4).
const COMMANDS: &[&str] = &[
    "subdivide",
    "triangulate",
    "untriangulate",
    "reverse_normals",
    "optimize",
    "weld",
    "melt",
    "split",
    "disconnect",
    "connect_points",
    "mirror",
    "center_axis",
];

/// Mesh-command cases we return `Unsupported` for (none).
const COMMAND_UNSUPPORTED: &[&str] = &[];

/// Mesh-command cases that build but differ (none).
const COMMAND_KNOWN_MISMATCH: &[&str] = &[];

fn index_set(v: &toml::Value) -> IndexSet {
    v.as_array()
        .unwrap()
        .iter()
        .map(|i| i.as_integer().unwrap() as u32)
        .collect()
}

#[test]
fn mesh_commands_match_the_reference() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden");
    let mut matched = Vec::new();
    let mut unsupported = std::collections::BTreeSet::new();
    for case in discover_cases(&root).unwrap() {
        if case.feature != "modeling-tools" {
            continue;
        }
        let m = case.manifest().unwrap();
        let Some(tool) = m.params.get("tool").and_then(|t| t.as_str()) else {
            continue;
        };
        if !COMMANDS.contains(&tool) {
            continue;
        }
        let key = format!("{}/{}", case.feature, case.name);
        let seg: Vec<i64> = m.params["segments"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_integer().unwrap())
            .collect();
        let mut base = cube([seg[0], seg[1], seg[2]]);
        let mut sel = Selection::default();
        if let Some(v) = m.params.get("select_polys") {
            sel.polys = index_set(v);
        }
        if let Some(v) = m.params.get("select_points") {
            sel.points = index_set(v);
        }
        if let Some(v) = m.params.get("move_points") {
            let d: Vec<f64> = v
                .as_array()
                .unwrap()
                .iter()
                .map(|x| {
                    x.as_float()
                        .unwrap_or_else(|| x.as_integer().unwrap() as f64)
                })
                .collect();
            for p in &mut base.points {
                *p += glam::DVec3::new(d[0], d[1], d[2]);
            }
        }
        let run = |id: &str, mesh: &PolyMesh, sel: &Selection, settings: &Settings| {
            tool_fn(id).unwrap()(mesh, sel, settings)
        };
        let defaults = |id: &str| Settings::defaults(&tool_desc(id).unwrap());
        // set-up steps the reference case did before the command under test
        if m.params.contains_key("triangulate_first") {
            base = run("triangulate", &base, &sel, &defaults("triangulate"))
                .unwrap()
                .0;
            sel = Selection::default();
        }
        if m.params.contains_key("disconnect_first") {
            base = run("disconnect", &base, &sel, &defaults("disconnect"))
                .unwrap()
                .0;
            sel = Selection::default();
        }
        let desc = tool_desc(tool).unwrap();
        let object: serde_json::Map<String, serde_json::Value> = m
            .params
            .get("settings")
            .and_then(|t| t.as_table())
            .map(|t| t.iter().map(|(k, v)| (k.clone(), json_of(v))).collect())
            .unwrap_or_default();
        let settings = Settings::from_json(&desc, &object).unwrap();
        let result = run(tool, &base, &sel, &settings);
        let ours = match result {
            Err(forge_modeling::ToolError::Unsupported(what)) => {
                println!("{key}: unsupported ({what})");
                unsupported.insert(key);
                continue;
            }
            other => other.unwrap().0,
        };
        let ours = match tool {
            // the file holds source and copy (two objects), copy indices offset
            "split" => {
                let copy = forge_modeling::split_copy(&base, &sel).unwrap();
                let offset = ours.points.len() as u32;
                let mut both = ours;
                both.points.extend(copy.points);
                both.polys.extend(
                    copy.polys
                        .into_iter()
                        .map(|p| p.into_iter().map(|c| c + offset).collect()),
                );
                both
            }
            // the file holds world coordinates: position (the centre) plus object-space point
            "center_axis" => {
                let (mesh, c) = forge_modeling::center_axis(&base, &settings).unwrap();
                assert_eq!(
                    c,
                    glam::DVec3::new(50.0, 0.0, 0.0),
                    "{key}: observed position"
                );
                PolyMesh {
                    points: mesh.points.iter().map(|p| *p + c).collect(),
                    polys: mesh.polys,
                }
            }
            _ => ours,
        };
        let expected = case.expected_mesh().unwrap().unwrap();
        let ours = as_mesh(&ours);
        let mismatch = COMMAND_KNOWN_MISMATCH.contains(&key.as_str());
        let raw = compare_meshes(&expected, &ours, &m.tolerance);
        let welded = compare_meshes(&export_weld(&expected), &export_weld(&ours), &m.tolerance);
        if mismatch {
            assert!(!(raw.passed() && welded.passed()), "{key} matches now");
            continue;
        }
        assert!(raw.passed(), "{key} raw: {:?}", raw.failures);
        assert!(welded.passed(), "{key} welded: {:?}", welded.failures);
        matched.push(key);
    }
    println!("matched exactly: {}", matched.join(", "));
    let want: std::collections::BTreeSet<String> =
        COMMAND_UNSUPPORTED.iter().map(|s| s.to_string()).collect();
    assert_eq!(unsupported, want, "unsupported set changed");
    assert_eq!(matched.len(), 15, "{matched:?}");
}
