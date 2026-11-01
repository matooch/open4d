//! Acceptance scenario S1 (docs/roadmap/README.md), driven only through the `forge` CLI and
//! the Rhai script `tests/acceptance/s1.rhai`:
//!
//! > `forge` CLI builds a scene from a script (cube, cloner with random effector, bend
//! > deformer), evaluates frame 30, exports glTF and renders a PNG, headless.
//!
//! Passing today: script -> scene file, validate, evaluate frame 30, export OBJ (the glTF
//! stand-in), bend deformer animated, determinism. **Pending** (ignored tests below, each
//! names its brief): cloner with random effector (065, 069), glTF export (164), PNG render
//! (136, 139, 146). Until then the CLI stubs are pinned, so the day they start working the
//! pinning test fails and the matching ignored test must be filled in.

use std::path::{Path, PathBuf};

use assert_cmd::Command;
use forge_parity_harness::{Tolerance, compare_meshes, parse_obj};
use serde_json::Value;

fn dir(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("forge-s1-{}-{name}", std::process::id()));
    std::fs::remove_dir_all(&d).ok();
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn script() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/acceptance/s1.rhai")
}

fn forge() -> Command {
    Command::cargo_bin("forge").unwrap()
}

/// Runs `forge --json args`, returns (exit code, stdout JSON).
fn json_run(args: &[&str]) -> (i32, Value) {
    let out = forge().arg("--json").args(args).output().unwrap();
    let text = String::from_utf8(out.stdout).unwrap();
    let value = serde_json::from_str(&text).unwrap_or_else(|e| panic!("{e}: {text:?}"));
    (out.status.code().unwrap(), value)
}

fn s(p: &Path) -> &str {
    p.to_str().unwrap()
}

/// Step 1: the script builds the scene.
fn build(d: &Path) -> PathBuf {
    let scene = d.join("s1.forge");
    let (code, out) = json_run(&["run", s(&script()), "--save", s(&scene)]);
    assert_eq!(code, 0, "{out}");
    assert_eq!(out["ok"], true);
    assert!(out["output"][0].as_str().unwrap().starts_with("cube="));
    scene
}

fn export_at(scene: &Path, frame: &str, out: &Path) -> Value {
    let (code, result) = json_run(&["eval", s(scene), "--frame", frame, "--export", s(out)]);
    assert_eq!(code, 0, "{result}");
    result
}

fn max_x(path: &Path) -> f64 {
    let mesh = parse_obj(&std::fs::read_to_string(path).unwrap()).unwrap();
    mesh.positions.iter().map(|p| p[0]).fold(f64::MIN, f64::max)
}

#[test]
fn s1_script_builds_a_valid_scene_with_cube_and_bend() {
    let d = dir("build");
    let scene = build(&d);
    let (code, info) = json_run(&["info", s(&scene)]);
    assert_eq!(code, 0, "{info}");
    let cube = &info["roots"][0];
    assert_eq!(cube["type"], "forge.cube");
    // The smoothing tag is node 2; the bend is the cube's child.
    assert_eq!(cube["children"][0]["type"], "forge.bend");
    assert_eq!(cube["children"][0]["animated"][0], "strength");
    assert_eq!(info["end"], 60);
    let (code, v) = json_run(&["validate", s(&scene)]);
    assert_eq!(code, 0, "{v}");
}

#[test]
fn s1_frame_30_is_evaluated_bent_and_exported() {
    let d = dir("eval");
    let scene = build(&d);

    // Frame 0: bend strength 0, so the plain cube of the golden data.
    let f0 = d.join("f0.obj");
    export_at(&scene, "0", &f0);
    let golden =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden/cube/defaults/mesh.obj");
    let expected = parse_obj(&std::fs::read_to_string(golden).unwrap()).unwrap();
    let got = parse_obj(&std::fs::read_to_string(&f0).unwrap()).unwrap();
    let report = compare_meshes(&expected, &got, &Tolerance::default());
    assert!(report.passed(), "{:?}", report.failures);

    // Frame 30: halfway through the key range, bent. Frame 60 is bent further.
    let f30 = d.join("f30.obj");
    let result = export_at(&scene, "30", &f30);
    assert_eq!(result["frame"], 30);
    assert_eq!(result["export"]["objects"], 1);
    let f60 = d.join("f60.obj");
    export_at(&scene, "60", &f60);
    let (x0, x30, x60) = (max_x(&f0), max_x(&f30), max_x(&f60));
    assert!(x0 < x30 && x30 < x60, "{x0} {x30} {x60}");
    let bent = parse_obj(&std::fs::read_to_string(&f30).unwrap()).unwrap();
    assert_eq!(bent.positions.len(), 8);
    assert_eq!(bent.faces.len(), 6);

    // The `export` subcommand writes the same file as `eval --export`.
    let again = d.join("again.obj");
    let (code, out) = json_run(&[
        "export",
        s(&scene),
        "--format",
        "obj",
        "--out",
        s(&again),
        "--frame",
        "30",
    ]);
    assert_eq!(code, 0, "{out}");
    assert_eq!(
        std::fs::read_to_string(&again).unwrap(),
        std::fs::read_to_string(&f30).unwrap()
    );
}

#[test]
fn s1_is_deterministic() {
    let (a, b) = (dir("det-a"), dir("det-b"));
    let (scene_a, scene_b) = (build(&a), build(&b));
    let (obj_a, obj_b) = (a.join("o.obj"), b.join("o.obj"));
    export_at(&scene_a, "30", &obj_a);
    export_at(&scene_b, "30", &obj_b);
    assert_eq!(
        std::fs::read(&obj_a).unwrap(),
        std::fs::read(&obj_b).unwrap()
    );
}

/// The two unfinished outputs are stubs today. When this test fails, the stub is gone:
/// fill in the matching ignored test below and delete the stub assertion here.
#[test]
fn s1_pending_outputs_are_still_stubs() {
    let d = dir("stubs");
    let scene = build(&d);
    let gltf = d.join("s1.gltf");
    let (code, out) = json_run(&[
        "export",
        s(&scene),
        "--format",
        "gltf",
        "--out",
        s(&gltf),
        "--frame",
        "30",
    ]);
    assert_eq!(
        code, 1,
        "glTF export now exists (brief 164): enable s1_gltf_export {out}"
    );
    assert!(!gltf.exists());
    let (code, out) = json_run(&["render"]);
    assert_eq!(
        code, 1,
        "render now exists (briefs 136/139/146): enable s1_png_render {out}"
    );
}

#[test]
#[ignore = "pending briefs 065 (cloner) and 069 (random effector)"]
fn s1_cloner_with_random_effector() {
    // Extend tests/acceptance/s1.rhai: create the cloner above the cube, add a random
    // effector with a fixed seed, then assert the clone count at frame 30 and that two
    // runs give identical geometry.
    todo!("needs the cloner and the random effector");
}

#[test]
#[ignore = "pending brief 164 (glTF export)"]
fn s1_gltf_export() {
    // `forge export s1.forge --format gltf --out s1.gltf --frame 30`: the file parses as
    // glTF 2.0 with one mesh node whose positions equal the frame-30 OBJ export.
    todo!("needs glTF export");
}

#[test]
#[ignore = "pending briefs 136, 139, 146 (renderer, render settings, S8)"]
fn s1_png_render() {
    // `forge render s1.forge --frame 30 --out s1.png`: the file starts with the PNG
    // signature, has the configured size and is not blank.
    todo!("needs the renderer");
}
