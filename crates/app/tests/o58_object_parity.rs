//! Parity of Duplicate, Arrange (linear), Randomize, Transfer and the Array command with the
//! O58 cases `tests/golden/modeling-tools/arr-f-*` (docs/specs/MODELING.md §9, brief 187).
//! Each case is rebuilt through the registered commands, the scene is exported with the OBJ
//! writer and compared with `mesh.obj` (positions within the case tolerance, identical face
//! lists). Randomize cannot be matched number for number (the reference stream is unknown):
//! it is compared statistically (level C), see `randomize_is_within_the_observed_ranges`.
//! Cases are discovered by their `tool`.

use std::path::Path;

use forge_app::App;
use forge_core::transform::hpb_from_rotation;
use forge_core::{NodeId, Value};
use forge_io::export_scene;
use forge_io::obj::{WriteOptions, write};
use forge_parity_harness::{GoldenCase, Mesh, compare_meshes, discover_cases, parse_obj};
use glam::{DMat3, DVec3};
use serde_json::{Value as Json, json};

/// The tools of this test.
const TOOLS: &[&str] = &[
    "array_command",
    "duplicate",
    "arrange",
    "randomize",
    "transfer",
];
/// Cases whose options we do not run (none).
const UNSUPPORTED: &[(&str, &str)] = &[];
/// Cases that run but differ (none).
const KNOWN_MISMATCH: &[(&str, &str)] = &[];
/// Cases compared statistically instead of number for number.
const STATISTICAL: &[&str] = &["modeling-tools/arr-f-randomize"];
/// The `arr-f` cases of the observation form O58.
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

fn key(case: &GoldenCase) -> String {
    format!("{}/{}", case.feature, case.name)
}

fn floats(v: &toml::Value) -> Vec<f64> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|x| {
            x.as_float()
                .unwrap_or_else(|| x.as_integer().unwrap() as f64)
        })
        .collect()
}

fn ints(v: &toml::Value) -> Vec<i64> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_integer().unwrap())
        .collect()
}

fn degrees(v: &[f64]) -> Vec<f64> {
    v.iter().map(|r| r.to_degrees()).collect()
}

/// The scene as the harness mesh, through our OBJ writer. The reference puts a new object on
/// top of the hierarchy, so the files of cases that create several objects list the newest
/// first (`arr-f-arrange`: `Cube.2`, `Cube.1`, `Cube`); `newest_first` writes ours that way.
fn exported(app: &App, newest_first: bool) -> Mesh {
    let mut objects = export_scene(app.doc.scene(), None);
    if newest_first {
        objects.reverse();
    }
    // the reference file keeps coincident points of copies apart (arr-f-array-mode)
    let opts = WriteOptions {
        normals: false,
        weld: false,
    };
    parse_obj(&write(&objects, &opts)).expect("our own file parses")
}

fn create_cube(app: &mut App) -> NodeId {
    app.run("object.create", json!({"type": "forge.cube"}))
        .unwrap();
    app.selection.active.unwrap()
}

fn nodes_json(ids: &[NodeId]) -> Json {
    json!(ids.iter().map(|n| n.0).collect::<Vec<_>>())
}

fn set_transform(app: &mut App, id: NodeId, pos: [f64; 3], rot_deg: [f64; 3]) {
    app.run(
        "transform.set",
        json!({"node": id.0, "pos": pos, "rot": rot_deg}),
    )
    .unwrap();
}

/// Builds the case's scene through the commands; returns the app.
fn build(case: &GoldenCase) -> Result<App, String> {
    let manifest = case.manifest()?;
    let p = &manifest.params;
    let s = &p["settings"];
    let tool = p["tool"].as_str().unwrap();
    let mut app = App::new_default();
    match tool {
        "array_command" => {
            let seg = ints(&p["segments"]);
            let cube = create_cube(&mut app);
            for (axis, n) in ["x", "y", "z"].iter().zip(&seg) {
                let args = json!({"node": cube.0, "param": format!("segments_{axis}"), "value": n});
                app.run("param.set", args).unwrap();
            }
            app.run("mesh.make_editable", json!({})).unwrap();
            let id = app.selection.active.unwrap();
            let polys: Vec<u32> = ints(&p["select_polys"]).iter().map(|&i| i as u32).collect();
            app.doc
                .transact("Start", |tx| {
                    tx.set_param(id, "selection.polys", Value::IndexList(polys))
                })
                .unwrap();
            let count = ints(&s["count"]);
            let args = json!({"settings": {
                "count_x": count[0], "count_y": count[1], "count_z": count[2],
                "offset": floats(&s["offset"]),
                "rotation_deg": floats(&s["rotation"]),
                "scale": floats(&s["scale"]),
            }});
            app.run("mesh.array_command", args)
                .map_err(|e| e.to_string())?;
        }
        "duplicate" => {
            create_cube(&mut app);
            if s["layout"].as_str() != Some("linear") || s["per_step"].as_bool() != Some(true) {
                return Err("only the linear per-step layout was observed".into());
            }
            let args = json!({
                "copies": s["copies"].as_integer().unwrap(),
                "offset": floats(&s["move"]),
                "rotation_deg": floats(&s["rotation"]),
                "scale": floats(&s["scale"]),
            });
            app.run("object.duplicate", args)
                .map_err(|e| e.to_string())?;
        }
        "arrange" => {
            let ids: Vec<NodeId> = (0..3).map(|_| create_cube(&mut app)).collect();
            let l = &s["linear"];
            let args = json!({
                "nodes": nodes_json(&ids), "mode": "line",
                "spacing": floats(&l["move"]),
            });
            app.run("object.arrange", args).map_err(|e| e.to_string())?;
        }
        "randomize" => {
            let ids: Vec<NodeId> = (0..3).map(|_| create_cube(&mut app)).collect();
            let args = json!({
                "nodes": nodes_json(&ids), "seed": s["seed"].as_integer().unwrap(),
                "position": floats(&s["move_range"]),
                "rotation_deg": degrees(&floats(&s["rotate_range_rad"])),
                "scale": floats(&s["scale_range"]),
            });
            app.run("object.randomize", args)
                .map_err(|e| e.to_string())?;
        }
        _ => {
            let a = create_cube(&mut app);
            let b = create_cube(&mut app);
            // The case text says "P 20 deg", but the exported cube is turned about the Z axis
            // (its x axis maps to (cos 20, -sin 20, 0)), which is a bank of 20 degrees in our
            // convention: the file is the ground truth (docs/specs/MODELING.md §9.5).
            set_transform(&mut app, a, [50.0, 0.0, 20.0], [0.0, 0.0, 20.0]);
            set_transform(&mut app, b, [200.0, 100.0, 0.0], [45.0, 0.0, 0.0]);
            let args = json!({
                "target": b.0, "source": a.0,
                "position": s["enable_move"].as_bool().unwrap(),
                "rotation": s["enable_rotation"].as_bool().unwrap(),
                "scale": s["enable_scale"].as_bool().unwrap(),
                "space": "world",
            });
            app.run("object.transfer", args)
                .map_err(|e| e.to_string())?;
        }
    }
    Ok(app)
}

fn run_case(case: &GoldenCase) -> Result<(), String> {
    let manifest = case.manifest()?;
    let golden = case.expected_mesh()?.expect("mesh.obj");
    let app = build(case)?;
    if STATISTICAL.contains(&key(case).as_str()) {
        return check_ranges(&manifest.params["settings"], &golden, &app);
    }
    let tool = manifest.params["tool"].as_str().unwrap();
    let ours = exported(&app, ["arrange", "transfer"].contains(&tool));
    let r = compare_meshes(&golden, &ours, &manifest.tolerance);
    if r.passed() {
        Ok(())
    } else {
        Err(format!("{:?}", r.failures))
    }
}

/// One cube's transform recovered from its eight world-space points (the cube's points are
/// in the fixed order of the default primitive): position, heading/pitch/bank in degrees,
/// per-axis scale for the unit-200 cube.
fn decompose(points: &[[f64; 3]]) -> (DVec3, DVec3, DVec3) {
    let p = |i: usize| DVec3::from_array(points[i]);
    let centre = (0..8).map(p).sum::<DVec3>() / 8.0;
    // point 0 (-,-,-), 2 (+,-,-), 1 (-,+,-), 6 (-,-,+) of the default cube
    let (x, y, z) = (p(2) - p(0), p(1) - p(0), p(6) - p(0));
    let scale = DVec3::new(x.length(), y.length(), z.length()) / 200.0;
    let rot = DMat3::from_cols(x.normalize(), y.normalize(), z.normalize());
    let hpb = hpb_from_rotation(rot).map(f64::to_degrees);
    (centre, hpb, scale)
}

fn cubes_of(mesh: &Mesh) -> Vec<(DVec3, DVec3, DVec3)> {
    mesh.positions.chunks(8).map(decompose).collect()
}

/// Level C: every recovered transform stays inside the ranges of the case, in both the
/// reference file and ours, and the same seed repeats exactly (the observation's repeat).
fn check_ranges(s: &toml::Value, golden: &Mesh, app: &App) -> Result<(), String> {
    let mv = floats(&s["move_range"]);
    let rot = degrees(&floats(&s["rotate_range_rad"]));
    let sc = floats(&s["scale_range"]);
    let within = |what: &str, mesh: &Mesh| -> Result<(), String> {
        let cubes = cubes_of(mesh);
        if cubes.len() != 3 {
            return Err(format!("{what}: {} cubes", cubes.len()));
        }
        for (i, (pos, hpb, scale)) in cubes.iter().enumerate() {
            for a in 0..3 {
                let ok = pos[a].abs() <= mv[a] + 1e-3
                    && hpb[a].abs() <= rot[a] + 1e-3
                    && (scale[a] - 1.0).abs() <= sc[a] + 1e-3;
                if !ok {
                    return Err(format!(
                        "{what}: cube {i} axis {a}: {pos:?} {hpb:?} {scale:?}"
                    ));
                }
            }
        }
        Ok(())
    };
    within("reference", golden)?;
    within("ours", &exported(app, false))?;
    // the same seed gives the same transforms, a different seed does not
    let again = exported(&build_with_seed(s, 1), false);
    if exported(app, false).positions != again.positions {
        return Err("the same seed gave other transforms".into());
    }
    let other = exported(&build_with_seed(s, 2), false);
    if other.positions == again.positions {
        return Err("another seed gave the same transforms".into());
    }
    Ok(())
}

fn build_with_seed(s: &toml::Value, seed: i64) -> App {
    let mut app = App::new_default();
    let ids: Vec<NodeId> = (0..3).map(|_| create_cube(&mut app)).collect();
    app.run(
        "object.randomize",
        json!({
            "nodes": nodes_json(&ids), "seed": seed,
            "position": floats(&s["move_range"]),
            "rotation_deg": degrees(&floats(&s["rotate_range_rad"])),
            "scale": floats(&s["scale_range"]),
        }),
    )
    .unwrap();
    app
}

#[test]
fn o58_object_cases_match_the_reference() {
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

/// Transfer: both cubes end with the position, rotation and scale of the source, and the
/// source keeps its own (the recovered transforms of the two exported cubes agree).
#[test]
fn transfer_gives_the_target_the_sources_placement() {
    let case = cases()
        .into_iter()
        .find(|c| c.name == "arr-f-transfer")
        .unwrap();
    let app = build(&case).unwrap();
    let cubes = cubes_of(&exported(&app, true));
    assert_eq!(cubes.len(), 2);
    let (a, b) = (cubes[1], cubes[0]);
    assert!((a.0 - DVec3::new(50.0, 0.0, 20.0)).length() < 1e-9);
    assert!((a.0 - b.0).length() < 1e-9);
    assert!((a.1 - b.1).length() < 1e-6);
    assert!((a.2 - b.2).length() < 1e-9);
}
