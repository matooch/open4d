//! Parity of Grow Selection, Convert Selection (tolerant) and Select Connected with the O58
//! cases `tests/golden/modeling-tools/sel-*` (docs/specs/EDITABLE.md §2.8, brief 187). The
//! golden mesh is stored on a polygon object, the start selection is rebuilt from the case,
//! the command runs through the registry, and the three stored sets are compared with
//! `selection.json` exactly (level A). Cases are discovered by their `tool`.

use std::path::Path;

use forge_app::App;
use forge_app::select::{EditMode, stored_selection};
use forge_core::{NodeId, Value};
use forge_generators::from_poly_mesh;
use forge_geometry::{Edge, PolyMesh};
use forge_parity_harness::{GoldenCase, discover_cases, parse_obj};
use glam::DVec3;
use serde_json::json;

/// The tools of this test.
const TOOLS: &[&str] = &["grow", "convert_selection", "select_connected"];
/// Cases whose options we do not run (none).
const UNSUPPORTED: &[(&str, &str)] = &[];
/// Cases that run but differ (none).
const KNOWN_MISMATCH: &[(&str, &str)] = &[];
/// The cases of the observation form O58 that name these tools.
const OBSERVED: usize = 6;

fn cases() -> Vec<GoldenCase> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden");
    discover_cases(&root)
        .unwrap()
        .into_iter()
        .filter(|c| c.feature == "modeling-tools")
        .filter(|c| {
            let m = c.manifest().unwrap();
            m.params
                .get("tool")
                .and_then(|t| t.as_str())
                .is_some_and(|t| TOOLS.contains(&t))
        })
        .collect()
}

fn golden_mesh(case: &GoldenCase) -> PolyMesh {
    let src = std::fs::read_to_string(case.dir.join("mesh.obj")).unwrap();
    let g = parse_obj(&src).unwrap();
    PolyMesh {
        points: g.positions.iter().map(|p| DVec3::from_array(*p)).collect(),
        polys: g.faces,
    }
}

fn u32s(v: &toml::Value) -> Vec<u32> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_integer().unwrap() as u32)
        .collect()
}

fn pairs_of(v: &toml::Value) -> Vec<[u32; 2]> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|p| {
            let a = u32s(p);
            [a[0], a[1]]
        })
        .collect()
}

fn edge_ids(mesh: &PolyMesh, pairs: &[[u32; 2]]) -> Vec<u32> {
    let ids = mesh.edge_ids();
    pairs.iter().map(|p| ids[&Edge::new(p[0], p[1])]).collect()
}

/// A polygon object holding `mesh`.
fn object_with(app: &mut App, mesh: &PolyMesh) -> NodeId {
    app.run("object.create", json!({"type": "forge.cube"}))
        .unwrap();
    app.run("mesh.make_editable", json!({})).unwrap();
    let id = app.selection.active.unwrap();
    app.doc
        .transact("Golden mesh", |tx| {
            tx.set_param(id, "mesh", Value::Mesh(from_poly_mesh(mesh).into()))
        })
        .unwrap();
    id
}

fn run_case(case: &GoldenCase) -> Result<(), String> {
    let manifest = case.manifest()?;
    let p = &manifest.params;
    let tool = p["tool"].as_str().unwrap();
    let settings = &p["settings"];
    let mesh = golden_mesh(case);
    let get = |k: &str| p.get(k);
    let points = get("select_points")
        .or(get("start_points"))
        .map(u32s)
        .unwrap_or_default();
    let edges = get("select_edges")
        .or(get("start_edges"))
        .map(|v| edge_ids(&mesh, &pairs_of(v)))
        .unwrap_or_default();
    let polys = get("select_polys").map(u32s).unwrap_or_default();

    let mut app = App::new_default();
    let id = object_with(&mut app, &mesh);
    app.doc
        .transact("Start", |tx| {
            tx.set_param(id, "selection.points", Value::IndexList(points.clone()))?;
            tx.set_param(id, "selection.edges", Value::IndexList(edges.clone()))?;
            tx.set_param(id, "selection.polys", Value::IndexList(polys.clone()))
        })
        .unwrap();

    let flag = |k: &str| settings.get(k).and_then(|v| v.as_bool()).unwrap_or(false);
    let (mode, cmd, args) = match tool {
        "grow" => {
            if flag("use_edge") {
                return Err("Use Edge is not implemented".into());
            }
            let mode = if !points.is_empty() {
                EditMode::Points
            } else if !edges.is_empty() {
                EditMode::Edges
            } else {
                EditMode::Polys
            };
            (mode, "select.grow", json!({}))
        }
        "select_connected" => (EditMode::Polys, "select.connected", json!({})),
        _ => {
            let mode = if flag("from_points") {
                EditMode::Points
            } else if flag("from_edges") {
                EditMode::Edges
            } else {
                EditMode::Polys
            };
            let to = if flag("to_polygons") {
                "polys"
            } else {
                "edges"
            };
            let m = if flag("tolerant") {
                "tolerant"
            } else {
                "strict"
            };
            (mode, "select.convert", json!({"to": to, "mode": m}))
        }
    };
    app.run(
        "select.mode",
        json!({"mode": forge_command::ModeArg::from(mode)}),
    )
    .map_err(|e| e.to_string())?;
    app.run(cmd, args).map_err(|e| e.to_string())?;

    let got = stored_selection(app.doc.scene(), id);
    let src = std::fs::read_to_string(case.dir.join("selection.json")).unwrap();
    let want: serde_json::Value = serde_json::from_str(&src).unwrap();
    let list = |k: &str| -> Vec<u32> {
        want[k]
            .as_array()
            .unwrap()
            .iter()
            .map(|x| x.as_u64().unwrap() as u32)
            .collect()
    };
    let want_pairs: Vec<[u32; 2]> = want["edges"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| [e[0].as_u64().unwrap() as u32, e[1].as_u64().unwrap() as u32])
        .collect();
    let mut want_edges = edge_ids(&mesh, &want_pairs);
    want_edges.sort_unstable();
    let check = |what: &str, got: Vec<u32>, want: Vec<u32>| {
        if got == want {
            Ok(())
        } else {
            Err(format!("{what}: got {got:?}, want {want:?}"))
        }
    };
    check("points", got.points.iter().collect(), list("points"))?;
    check("edges", got.edges.iter().collect(), want_edges)?;
    check("polys", got.polys.iter().collect(), list("polys"))
}

#[test]
fn o58_selection_cases_match_the_reference() {
    let cases = cases();
    assert_eq!(cases.len(), OBSERVED);
    let listed = |n: &str, l: &[(&str, &str)]| l.iter().any(|(c, _)| *c == n);
    let mut matched = 0;
    for case in &cases {
        if listed(&case.name, UNSUPPORTED) {
            continue;
        }
        match (run_case(case), listed(&case.name, KNOWN_MISMATCH)) {
            (Ok(()), false) => matched += 1,
            (Err(e), false) => panic!("{}: {e}", case.name),
            (Ok(()), true) => panic!("{} now matches; remove it from KNOWN_MISMATCH", case.name),
            (Err(_), true) => {}
        }
    }
    assert_eq!(matched, OBSERVED - UNSUPPORTED.len() - KNOWN_MISMATCH.len());
}
