//! Acceptance scenario S1 (docs/roadmap/README.md), driven only through the `forge` CLI and
//! the Rhai script `tests/acceptance/s1.rhai`:
//!
//! > `forge` CLI builds a scene from a script (cube, cloner with random effector, bend
//! > deformer), evaluates frame 30, exports glTF and renders a PNG, headless.
//!
//! Covered: script -> scene file, validate, evaluate frame 30 (bend animated), OBJ export,
//! glTF (GLB) export, a cloner with a seeded random effector (`s1_cloner.rhai`), PNG render,
//! determinism.

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
    let mut cmd = Command::cargo_bin("forge").unwrap();
    // never pick up plugins from the machine running the tests
    cmd.env("FORGE_NO_PLUGINS", "1");
    cmd
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

fn script_named(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/acceptance")
        .join(name)
}

/// The second half of S1: `tests/acceptance/s1_cloner.rhai` (cloner, random effector, bend).
fn build_cloner(d: &Path) -> PathBuf {
    let scene = d.join("s1c.forge");
    let (code, out) = json_run(&[
        "run",
        s(&script_named("s1_cloner.rhai")),
        "--save",
        s(&scene),
    ]);
    assert_eq!(code, 0, "{out}");
    assert!(out["output"][0].as_str().unwrap().starts_with("cloner="));
    scene
}

#[test]
fn s1_cloner_with_random_effector() {
    let (a, b) = (dir("cloner-a"), dir("cloner-b"));
    let (scene_a, scene_b) = (build_cloner(&a), build_cloner(&b));
    let (code, v) = json_run(&["validate", s(&scene_a)]);
    assert_eq!(code, 0, "{v}");
    let (obj_a, obj_b) = (a.join("f30.obj"), b.join("f30.obj"));
    export_at(&scene_a, "30", &obj_a);
    export_at(&scene_b, "30", &obj_b);
    let mesh = parse_obj(&std::fs::read_to_string(&obj_a).unwrap()).unwrap();
    // Five clones of the 8-point, 6-face cube.
    assert_eq!(mesh.positions.len(), 5 * 8);
    assert_eq!(mesh.faces.len(), 5 * 6);
    // The random effector moved the clones off the straight row: the clone centres are not
    // all on one line (y and z differ between clones), and the same seed gives the same file.
    let centres: Vec<[f64; 3]> = (0..5)
        .map(|c| {
            let pts = &mesh.positions[c * 8..c * 8 + 8];
            let mut m = [0.0; 3];
            for p in pts {
                for k in 0..3 {
                    m[k] += p[k] / 8.0;
                }
            }
            m
        })
        .collect();
    let spread = |k: usize| {
        let v: Vec<f64> = centres.iter().map(|c| c[k]).collect();
        v.iter().cloned().fold(f64::MIN, f64::max) - v.iter().cloned().fold(f64::MAX, f64::min)
    };
    assert!(spread(1) > 1.0 && spread(2) > 1.0, "{centres:?}");
    assert_eq!(
        std::fs::read_to_string(&obj_a).unwrap(),
        std::fs::read_to_string(&obj_b).unwrap(),
        "same seed, same geometry"
    );
}

#[test]
fn s1_gltf_export() {
    // `forge export s1.forge --format gltf --out s1.glb --frame 30` writes binary glTF 2.0 (the
    // writer only writes GLB, so the path must end in `.glb`, IO.md §5). Converting it back
    // to OBJ gives the frame-30 points of the OBJ export.
    let d = dir("gltf");
    let scene = build(&d);
    let glb = d.join("s1.glb");
    let (code, out) = json_run(&[
        "export",
        s(&scene),
        "--format",
        "gltf",
        "--out",
        s(&glb),
        "--frame",
        "30",
    ]);
    assert_eq!(code, 0, "{out}");
    let bytes = std::fs::read(&glb).unwrap();
    assert_eq!(&bytes[0..4], b"glTF", "GLB magic");
    assert_eq!(
        u32::from_le_bytes(bytes[4..8].try_into().unwrap()),
        2,
        "glTF 2.0"
    );

    // `forge convert` imports foreign files into native scenes (`--format forge`) and exports
    // native scenes (IO.md §3), so GLB -> OBJ is two steps.
    let native = d.join("native");
    let (code, out) = json_run(&["convert", s(&glb), "--out", s(&native), "--format", "forge"]);
    assert_eq!(code, 0, "{out}");
    let back = d.join("back");
    let (code, out) = json_run(&["convert", s(&native), "--out", s(&back), "--format", "obj"]);
    assert_eq!(code, 0, "{out}");
    let f30 = d.join("f30.obj");
    export_at(&scene, "30", &f30);
    let points = |p: &Path| {
        let mesh = parse_obj(&std::fs::read_to_string(p).unwrap()).unwrap();
        let mut v: Vec<[i64; 3]> = mesh
            .positions
            .iter()
            .map(|q| [0, 1, 2].map(|k| (q[k] * 1e4).round() as i64))
            .collect();
        v.sort();
        v.dedup();
        v
    };
    let obj_back = std::fs::read_dir(&back)
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| p.extension().is_some_and(|x| x == "obj"))
        .expect("converted OBJ");
    assert_eq!(points(&obj_back), points(&f30));
}

#[test]
fn s1_png_render() {
    // `forge render s1.forge --frame 30 --out s1.png`: the file starts with the PNG
    // signature, has the configured size and is not blank.
    let d = dir("render");
    let scene = build(&d);
    let png = d.join("s1.png");
    let (code, out) = json_run(&[
        "render",
        s(&scene),
        "--frame",
        "30",
        "--out",
        s(&png),
        "--width",
        "96",
        "--height",
        "72",
        "--samples",
        "2",
    ]);
    assert_eq!(code, 0, "{out}");
    let bytes = std::fs::read(&png).unwrap();
    assert_eq!(&bytes[..8], b"\x89PNG\r\n\x1a\n");
    assert_eq!(u32::from_be_bytes(bytes[16..20].try_into().unwrap()), 96);
    assert_eq!(u32::from_be_bytes(bytes[20..24].try_into().unwrap()), 72);
    let lit = forge_image::decode(&bytes, &forge_image::Limits::default()).unwrap();
    assert!(
        lit.image
            .data
            .as_chunks::<4>()
            .0
            .iter()
            .any(|p| p[0] > 0.01),
        "not blank"
    );
}
