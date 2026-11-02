//! Parity of the selection commands with tests/golden/selection-ops (O26,
//! docs/specs/EDITABLE.md §2.8): the start selection of each case is rebuilt on a polygon
//! object holding the golden mesh, the command runs through the registry, and the three
//! stored sets are compared with `selection.json` exactly (level A).

use std::path::Path;

use forge_app::App;
use forge_app::select::{EditMode, stored_selection};
use forge_core::{NodeId, Value};
use forge_generators::from_poly_mesh;
use forge_geometry::PolyMesh;
use forge_parity_harness::{GoldenCase, discover_cases, parse_obj};
use glam::DVec3;
use serde_json::json;

/// Cases whose command we do not run, with the reason. Empty: all observed cases run.
const UNSUPPORTED: &[(&str, &str)] = &[];
/// Cases that run but differ. Empty.
const KNOWN_MISMATCH: &[(&str, &str)] = &[];
/// The observed cases (the other five O26 rows have no output).
const OBSERVED: usize = 6;

fn cases() -> Vec<GoldenCase> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden");
    discover_cases(&root)
        .unwrap()
        .into_iter()
        .filter(|c| c.feature == "selection-ops")
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

fn edge_ids(mesh: &PolyMesh, pairs: &[[u32; 2]]) -> Vec<u32> {
    let ids = mesh.edge_ids();
    pairs
        .iter()
        .map(|p| ids[&forge_geometry::Edge::new(p[0], p[1])])
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

/// A polygon object holding `mesh`, selected.
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

fn set_sets(app: &mut App, id: NodeId, points: &[u32], edges: &[u32], polys: &[u32]) {
    app.doc
        .transact("Start", |tx| {
            tx.set_param(id, "selection.points", Value::IndexList(points.to_vec()))?;
            tx.set_param(id, "selection.edges", Value::IndexList(edges.to_vec()))?;
            tx.set_param(id, "selection.polys", Value::IndexList(polys.to_vec()))
        })
        .unwrap();
}

/// Runs one case; returns `Err(message)` on a mismatch.
fn run_case(case: &GoldenCase) -> Result<(), String> {
    let manifest = case.manifest()?;
    let p = &manifest.params;
    let mesh = golden_mesh(case);
    let start_points = u32s(&p["start_points"]);
    let start_edges = edge_ids(&mesh, &pairs_of(&p["start_edges"]));
    let start_polys = u32s(&p["start_polys"]);

    let mut app = App::new_default();
    let id = object_with(&mut app, &mesh);
    set_sets(&mut app, id, &start_points, &start_edges, &start_polys);

    let name = p["command"].as_str().unwrap();
    let (mode, cmd, args) = match name {
        "Shrink Selection" => (EditMode::Polys, "select.shrink", json!({})),
        "Invert" => (EditMode::Polys, "select.invert", json!({})),
        "Select Connected" => (EditMode::Polys, "select.connected", json!({})),
        "Convert Selection" => {
            let mode = match p["from"].as_str().unwrap() {
                "points" => EditMode::Points,
                "edges" => EditMode::Edges,
                _ => EditMode::Polys,
            };
            let strict = !p["tolerant"].as_bool().unwrap();
            let to = match p["to"].as_str().unwrap() {
                "polygons" => "polys",
                other => other,
            };
            let m = if strict { "strict" } else { "tolerant" };
            (mode, "select.convert", json!({"to": to, "mode": m}))
        }
        other => return Err(format!("unknown command {other:?}")),
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
    let want_edges: Vec<[u32; 2]> = want["edges"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| [e[0].as_u64().unwrap() as u32, e[1].as_u64().unwrap() as u32])
        .collect();
    let mut want_edge_ids = edge_ids(&mesh, &want_edges);
    want_edge_ids.sort_unstable();
    let check = |what: &str, got: Vec<u32>, want: Vec<u32>| {
        if got == want {
            Ok(())
        } else {
            Err(format!("{what}: got {got:?}, want {want:?}"))
        }
    };
    check("points", got.points.iter().collect(), list("points"))?;
    check("edges", got.edges.iter().collect(), want_edge_ids)?;
    check("polys", got.polys.iter().collect(), list("polys"))
}

#[test]
fn selection_ops_match_the_reference() {
    let cases = cases();
    assert_eq!(cases.len(), OBSERVED);
    let listed = |n: &str, l: &[(&str, &str)]| l.iter().any(|(c, _)| *c == n);
    let mut matched = 0;
    for case in &cases {
        let result = if listed(&case.name, UNSUPPORTED) {
            continue;
        } else {
            run_case(case)
        };
        match (result, listed(&case.name, KNOWN_MISMATCH)) {
            (Ok(()), false) => matched += 1,
            (Err(e), false) => panic!("{}: {e}", case.name),
            (Ok(()), true) => panic!("{} now matches; remove it from KNOWN_MISMATCH", case.name),
            (Err(_), true) => {}
        }
    }
    assert_eq!(matched, OBSERVED - UNSUPPORTED.len() - KNOWN_MISMATCH.len());
}

/// Our editable cube has the reference's point and polygon numbering, so the indices of
/// the golden selections mean the same elements on our own cubes.
#[test]
fn editable_cube_numbering_matches_the_golden_meshes() {
    for case in cases() {
        if case.name == "select-connected" {
            continue; // the golden mesh was changed by Disconnect
        }
        let seg = u32s(&case.manifest().unwrap().params["segments"]);
        let mut app = App::new_default();
        app.run("object.create", json!({"type": "forge.cube"}))
            .unwrap();
        let cube = app.selection.active.unwrap();
        for (axis, n) in ["x", "y", "z"].iter().zip(&seg) {
            let args = json!({"node": cube.0, "param": format!("segments_{axis}"), "value": n});
            app.run("param.set", args).unwrap();
        }
        app.run("mesh.make_editable", json!({})).unwrap();
        let id = app.selection.active.unwrap();
        let ours = forge_app::select::base_mesh(app.doc.scene(), id).unwrap();
        let golden = golden_mesh(&case);
        assert_eq!(ours.polys, golden.polys, "{}", case.name);
        assert_eq!(ours.points.len(), golden.points.len(), "{}", case.name);
        for (a, b) in ours.points.iter().zip(&golden.points) {
            assert!(a.distance(*b) < 1e-4, "{}: {a:?} vs {b:?}", case.name);
        }
    }
}
