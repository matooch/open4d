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
use serde_json::{Value, json};

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
fn s7_retime_edit_tangents_ease_speed_and_loop() {
    let d = dir("s7");
    let scene = d.join("s7.forge");
    let (code, result) = json_run(&["new", s(&scene)]);
    assert_eq!(code, 0, "{result}");
    let cmd = |name: &str, args: Value| {
        let (code, result) = json_run(&[
            "cmd",
            s(&scene),
            name,
            "--args",
            &args.to_string(),
            "--save",
        ]);
        assert_eq!(code, 0, "{name}: {result}");
        result["result"].clone()
    };
    let id = cmd("object.create", json!({"type": "forge.cube"}))["node"]
        .as_u64()
        .unwrap();
    let track = "transform.pos.x";
    for (frame, value) in [(0, 0), (15, 60), (30, 120)] {
        cmd(
            "key.set",
            json!({"node": id, "track": track, "frame": frame, "value": value}),
        );
    }
    let keys: Vec<_> = [0, 15, 30]
        .map(|frame| json!({"node": id, "track": track, "frame": frame}))
        .into();
    cmd(
        "key.scale",
        json!({"keys": keys, "pivot_frame": 0, "factor": 2}),
    );
    let at = |frame: i32| {
        let export = d.join(format!("frame-{frame}.obj"));
        export_at(&scene, &frame.to_string(), &export);
        max_x(&export)
    };
    let origin = at(0);
    assert!((at(30) - origin - 60.0).abs() < 1e-6);
    assert!((at(60) - origin - 120.0).abs() < 1e-6);
    let before = at(24);
    cmd(
        "key.set_tangent",
        json!({
            "node": id, "track": track, "frame": 30, "preset": "user", "broken": true, "weighted": true,
            "left": {"dt_frames": -6, "dv": -12}, "right": {"dt_frames": 9, "dv": 30},
        }),
    );
    let incoming = at(24);
    let outgoing = at(33);
    assert!(
        (incoming - before).abs() > 0.1,
        "value-graph edit should change the curve"
    );
    cmd(
        "key.set_speed",
        json!({"node": id, "track": track, "frame": 30, "side": "right", "speed": 0}),
    );
    assert!(
        (at(24) - incoming).abs() < 1e-6,
        "broken incoming handle should stay put"
    );
    assert!(
        at(33) < outgoing - 0.1,
        "zero outgoing speed should ease away from the middle key"
    );
    assert!((at(30) - origin - 60.0).abs() < 1e-6);
    let first_cycle = at(15);
    cmd(
        "track.set_behavior",
        json!({"node": id, "track": track, "after": "repeat"}),
    );
    assert!(
        (at(75) - first_cycle).abs() < 1e-6,
        "frame 75 should repeat frame 15"
    );
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

// ---------------------------------------------------------------------------------------------
// S4: motion graphics through the CLI, `tests/acceptance/s4_mograph.rhai`.
//
// "FORGE" is fractured into letters, cloned onto the 36 sampled points of a Circle spline,
// turned by a seeded Random effector and lifted by a Plain effector whose Linear field falloff
// is keyed across the ring. Everything below is read back from CLI output (OBJ and PNG files).
// ---------------------------------------------------------------------------------------------

fn s4_script() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/acceptance/s4_mograph.rhai")
}

/// Builds the S4 scene twice-identically into `d` and returns the scene path, the fracture's
/// node id and the cloner's node id (the script prints them).
fn s4_build(d: &Path, name: &str) -> (PathBuf, u64, u64) {
    let scene = d.join(format!("s4-{name}.forge"));
    let (code, out) = json_run(&["run", s(&s4_script()), "--save", s(&scene)]);
    assert_eq!(code, 0, "{out}");
    let line = out["output"][0]
        .as_str()
        .expect("the script prints its node ids");
    let id = |key: &str| {
        line.split_whitespace()
            .find_map(|token| token.strip_prefix(key)?.strip_prefix('='))
            .unwrap_or_else(|| panic!("no {key}= in {line:?}"))
            .parse::<u64>()
            .unwrap()
    };
    (scene, id("fracture"), id("cloner"))
}

fn s4_obj(path: &Path) -> forge_parity_harness::Mesh {
    parse_obj(&std::fs::read_to_string(path).unwrap()).unwrap()
}

/// Connected components of an OBJ mesh (points joined by a polygon are one piece).
fn s4_islands(mesh: &forge_parity_harness::Mesh) -> usize {
    let mut parent: Vec<usize> = (0..mesh.positions.len()).collect();
    fn find(parent: &mut [usize], mut x: usize) -> usize {
        while parent[x] != x {
            parent[x] = parent[parent[x]];
            x = parent[x];
        }
        x
    }
    for face in &mesh.faces {
        let Some(&first) = face.first() else {
            continue;
        };
        let root = find(&mut parent, first as usize);
        for &v in &face[1..] {
            let other = find(&mut parent, v as usize);
            if other != root {
                parent[other] = root;
            }
        }
    }
    let mut roots = std::collections::BTreeSet::new();
    for i in 0..mesh.positions.len() {
        roots.insert(find(&mut parent, i));
    }
    roots.len()
}

/// The fracture the cloner instances: `forge eval --node` cannot export a consumed child, so a
/// copy of the scene gets the fracture moved to the root and is exported instead.
fn s4_fracture_at(d: &Path, scene: &Path, fracture: u64, frame: i64, out: &Path) {
    let copy = d.join(format!("fracture-{frame}.forge"));
    std::fs::copy(scene, &copy).unwrap();
    let args = json!({"node": fracture, "parent": null}).to_string();
    let (code, result) = json_run(&["cmd", s(&copy), "object.reparent", "--args", &args, "--save"]);
    assert_eq!(code, 0, "{result}");
    let (code, result) = json_run(&[
        "eval",
        s(&copy),
        "--node",
        &fracture.to_string(),
        "--frame",
        &frame.to_string(),
        "--export",
        s(out),
    ]);
    assert_eq!(code, 0, "{result}");
}

/// The 4-byte quarter arcs of a Circle (docs/specs/SPLINES.md §6.1: quarters counter-clockwise
/// from (r, 0), handle length CIRCLE_KAPPA = 83/150 of the radius), densely sampled.
fn s4_circle(radius: f64) -> Vec<[f64; 2]> {
    let k = 83.0 / 150.0 * radius;
    let mut out = Vec::new();
    for quarter in 0..4 {
        let a0 = f64::from(quarter) * std::f64::consts::FRAC_PI_2;
        let a1 = f64::from(quarter + 1) * std::f64::consts::FRAC_PI_2;
        let p0 = [radius * a0.cos(), radius * a0.sin()];
        let p3 = [radius * a1.cos(), radius * a1.sin()];
        let p1 = [p0[0] - a0.sin() * k, p0[1] + a0.cos() * k];
        let p2 = [p3[0] + a1.sin() * k, p3[1] - a1.cos() * k];
        for step in 0..=4000 {
            let t = f64::from(step) / 4000.0;
            let u = 1.0 - t;
            out.push([0, 1].map(|i| {
                p0[i] * u * u * u + 3.0 * p1[i] * u * u * t + 3.0 * p2[i] * u * t * t + p3[i] * t * t * t
            }));
        }
    }
    out
}

/// Distance from a point to a polyline (the circle's densely sampled arcs).
fn s4_distance(p: [f64; 2], line: &[[f64; 2]]) -> f64 {
    let mut best = f64::MAX;
    for w in line.windows(2) {
        let ab = [w[1][0] - w[0][0], w[1][1] - w[0][1]];
        let ap = [p[0] - w[0][0], p[1] - w[0][1]];
        let len2 = ab[0] * ab[0] + ab[1] * ab[1];
        let t = if len2 > 0.0 {
            ((ap[0] * ab[0] + ap[1] * ab[1]) / len2).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let dx = ap[0] - ab[0] * t;
        let dy = ap[1] - ab[1] * t;
        best = best.min(dx.hypot(dy));
    }
    best
}

#[test]
fn s4_mograph_letters_on_a_spline_effectors_and_sequence() {
    let d = dir("s4");
    let (scene, fracture, _cloner) = s4_build(&d, "a");

    // 1. Fracture: one piece per letter. The cloner puts the whole fracture on the 36 sampled
    // points of the Circle, so the export has 36 times its points and 36 * 5 islands.
    let frac0_path = d.join("fracture-0.obj");
    s4_fracture_at(&d, &scene, fracture, 0, &frac0_path);
    let frac0 = s4_obj(&frac0_path);
    assert_eq!(s4_islands(&frac0), 5, "one piece per letter");
    let frac90_path = d.join("fracture-90.obj");
    s4_fracture_at(&d, &scene, fracture, 90, &frac90_path);
    let frac90 = s4_obj(&frac90_path);
    assert_eq!(frac0.positions, frac90.positions, "the fracture is static");

    let clones_path = d.join("clones-0.obj");
    export_at(&scene, "0", &clones_path);
    let clones = s4_obj(&clones_path);
    let n = frac0.positions.len();
    assert_eq!(clones.positions.len(), 36 * n, "36 clones of the fracture");
    assert_eq!(s4_islands(&clones), 36 * 5, "36 clones of the 5 pieces");

    // 2. At frame 0 the keyed field sits beyond the ring, so every clone's falloff is 0 and each
    // clone origin is a point of the spline. Clone k's slice is the fracture shifted by its
    // origin, so its position is the first point of the slice minus the fracture's first point.
    let curve = s4_circle(200.0);
    let mut moved = 0;
    let mut origins = Vec::new();
    for k in 0..36 {
        let a = clones.positions[k * n];
        let b = frac0.positions[0];
        let origin = [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
        let off = s4_distance([origin[0], origin[2]], &curve);
        assert!(off < 1e-3, "clone {k} at {origin:?} is {off} off the spline");
        if (0.5 + (origin[0] - 150.0) / 100.0).clamp(0.0, 1.0) > 0.0 {
            moved += 1;
        }
        origins.push(origin);
    }
    assert!(
        (10..36).contains(&moved),
        "the field sweep should lift part of the ring, not {moved} of 36"
    );

    // Frame 90: a clone the field covers rises by the effector's P (0, 100, 0) times its
    // falloff value; a clone at falloff 0 stays exactly where it was.
    let clones90_path = d.join("clones-90.obj");
    export_at(&scene, "90", &clones90_path);
    let clones90 = s4_obj(&clones90_path);
    assert_eq!(clones90.positions.len(), clones.positions.len());
    for k in 0..36 {
        let origin = origins[k];
        // Linear field, direction X+, length 50, remap strength 1: 0.5 + (x - p) / 100.
        let falloff = (0.5 + (origin[0] - 150.0) / 100.0).clamp(0.0, 1.0);
        for j in 0..n {
            let now = clones90.positions[k * n + j];
            let then = clones.positions[k * n + j];
            let delta = [now[0] - then[0], now[1] - then[1], now[2] - then[2]];
            let want = [0.0, 100.0 * falloff, 0.0];
            for axis in 0..3 {
                assert!(
                    (delta[axis] - want[axis]).abs() < 1e-6,
                    "clone {k} point {j}: {delta:?} != {want:?} (falloff {falloff})"
                );
            }
        }
    }

    // 3. A second run of the same script gives byte-identical OBJ exports at 0, 45 and 90.
    let (scene_b, _, _) = s4_build(&d, "b");
    for frame in [0, 45, 90] {
        let first = d.join(format!("determinism-a-{frame}.obj"));
        let second = d.join(format!("determinism-b-{frame}.obj"));
        export_at(&scene, &frame.to_string(), &first);
        export_at(&scene_b, &frame.to_string(), &second);
        assert_eq!(
            std::fs::read(&first).unwrap(),
            std::fs::read(&second).unwrap(),
            "frame {frame} is not deterministic"
        );
    }

    // 4. `forge render` writes a 1920x1080 sequence: frames, numbering, size and visible letters.
    let template = d.join("frame_{frame:04}.png");
    let (code, out) = json_run(&[
        "render",
        s(&scene),
        "--start",
        "0",
        "--end",
        "90",
        "--step",
        "45",
        "--width",
        "1920",
        "--height",
        "1080",
        "--samples",
        "1",
        "--alpha",
        "--out",
        s(&template),
    ]);
    assert_eq!(code, 0, "{out}");
    assert_eq!(out["frames"], json!([0, 45, 90]));
    assert_eq!(out["width"], 1920);
    assert_eq!(out["height"], 1080);
    for frame in [0, 45, 90] {
        let png = d.join(format!("frame_{frame:04}.png"));
        let bytes = std::fs::read(&png).unwrap_or_else(|e| panic!("{}: {e}", png.display()));
        assert_eq!(&bytes[..8], b"\x89PNG\r\n\x1a\n");
        assert_eq!(u32::from_be_bytes(bytes[16..20].try_into().unwrap()), 1920);
        assert_eq!(u32::from_be_bytes(bytes[20..24].try_into().unwrap()), 1080);
        let image = forge_image::decode(&bytes, &forge_image::Limits::default())
            .unwrap()
            .image;
        let covered = image
            .data
            .as_chunks::<4>()
            .0
            .iter()
            .filter(|p| p[3] > 0.5)
            .count();
        assert!(
            covered > 10_000,
            "frame {frame}: only {covered} pixels of letters"
        );
    }
}
