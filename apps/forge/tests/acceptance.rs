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
// S4: motion graphics through the CLI, `tests/acceptance/s4_motion_graphics.rhai`.
//
// "FORGE" is fractured into letters, cloned onto the 36 sampled points of a Circle spline,
// turned by a seeded Random effector and lifted by a Plain effector whose Linear field falloff
// is keyed across the ring. Everything below is read back from CLI output (OBJ and PNG files).
// ---------------------------------------------------------------------------------------------

fn s4_script() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/acceptance/s4_motion_graphics.rhai")
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
    let (code, result) = json_run(&[
        "cmd",
        s(&copy),
        "object.reparent",
        "--args",
        &args,
        "--save",
    ]);
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
                p0[i] * u * u * u
                    + 3.0 * p1[i] * u * u * t
                    + 3.0 * p2[i] * u * t * t
                    + p3[i] * t * t * t
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
fn s4_motion_graphics_letters_on_a_spline_effectors_and_sequence() {
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
        assert!(
            off < 1e-3,
            "clone {k} at {origin:?} is {off} off the spline"
        );
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
    for (k, origin) in origins.iter().enumerate() {
        let origin = *origin;
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

// ---------------------------------------------------------------------------------------------
// S5: volumes through the CLI, `tests/acceptance/s5_volume.rhai`.
//
// Three overlapping spheres and a Circle-spline tube in one Volume Builder, a light Gaussian
// smooth layer on top of the list, a Volume Mesher and Make Editable. Every number below is
// read back from CLI output; the volume reference is computed here from the analytic SDF of the
// same union, so no reference-app data is involved.
// ---------------------------------------------------------------------------------------------

/// The builder's voxel size in the S5 script.
const S5_VOXEL: f64 = 10.0;
const S5_SPHERE_RADIUS: f64 = 100.0;
/// Circle radius (major radius of the tube).
const S5_TUBE_MAJOR: f64 = 300.0;
/// `0.735 * spline_voxel_range * voxel_size` (docs/specs/VOLUME.md 6.3) with range 4.
const S5_TUBE_MINOR: f64 = 0.735 * 4.0 * S5_VOXEL;
/// Distance between neighbouring sphere centres.
const S5_SPHERE_SPACING: f64 = 120.0;

fn s5_script() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/acceptance/s5_volume.rhai")
}

fn s5_obj(path: &Path) -> forge_parity_harness::Mesh {
    parse_obj(&std::fs::read_to_string(path).unwrap()).unwrap()
}

/// Builds the S5 scene into `d` and returns the scene path and the polygon node id.
fn s5_build(d: &Path, name: &str) -> (PathBuf, u64) {
    let scene = d.join(format!("s5-{name}.forge"));
    let (code, out) = json_run(&["run", s(&s5_script()), "--save", s(&scene)]);
    assert_eq!(code, 0, "{out}");
    let line = out["output"][0]
        .as_str()
        .expect("the script prints its node ids");
    let polygon = line
        .split_whitespace()
        .find_map(|token| token.strip_prefix("polygon=")?.parse::<u64>().ok())
        .unwrap_or_else(|| panic!("no polygon= in {line:?}"));
    (scene, polygon)
}

/// Every undirected edge is used once in each direction and no face is degenerate.
fn s5_assert_closed_manifold(mesh: &forge_parity_harness::Mesh) {
    let mut directed: std::collections::BTreeMap<(u32, u32), usize> =
        std::collections::BTreeMap::new();
    for face in &mesh.faces {
        assert!(
            face.len() >= 3,
            "face with {} corners: {face:?}",
            face.len()
        );
        let distinct: std::collections::BTreeSet<u32> = face.iter().copied().collect();
        assert_eq!(distinct.len(), face.len(), "repeated index: {face:?}");
        // Newell normal: its length is twice the polygon's area.
        let mut normal = [0.0f64; 3];
        for i in 0..face.len() {
            let a = mesh.positions[face[i] as usize];
            let b = mesh.positions[face[(i + 1) % face.len()] as usize];
            normal[0] += (a[1] - b[1]) * (a[2] + b[2]);
            normal[1] += (a[2] - b[2]) * (a[0] + b[0]);
            normal[2] += (a[0] - b[0]) * (a[1] + b[1]);
        }
        let area =
            0.5 * (normal[0] * normal[0] + normal[1] * normal[1] + normal[2] * normal[2]).sqrt();
        assert!(area > 1e-9, "degenerate face {face:?} (area {area})");
        for i in 0..face.len() {
            *directed
                .entry((face[i], face[(i + 1) % face.len()]))
                .or_default() += 1;
        }
    }
    for (&(a, b), &count) in &directed {
        assert_eq!(count, 1, "edge {a} -> {b} is used {count} times");
        assert_eq!(
            directed.get(&(b, a)),
            Some(&1),
            "edge {a} -> {b} has no opposite half-edge"
        );
    }
}

/// Enclosed volume by the divergence theorem over the fan triangulation.
fn s5_signed_volume(mesh: &forge_parity_harness::Mesh) -> f64 {
    let mut total = 0.0;
    for face in &mesh.faces {
        let a = mesh.positions[face[0] as usize];
        for i in 1..face.len() - 1 {
            let b = mesh.positions[face[i] as usize];
            let c = mesh.positions[face[i + 1] as usize];
            total += (a[0] * (b[1] * c[2] - b[2] * c[1]) - a[1] * (b[0] * c[2] - b[2] * c[0])
                + a[2] * (b[0] * c[1] - b[1] * c[0]))
                / 6.0;
        }
    }
    total
}

/// The S5 union's volume from its fine analytic SDF: 2.5-unit lattice samples of the positive
/// octant (the union is symmetric in x, y and z), times the cell volume.
fn s5_reference_volume() -> f64 {
    const H: f64 = 2.5;
    const LIMIT: f64 = 340.0;
    let n = (LIMIT / H) as i64 + 1;
    let mut inside = 0u64;
    for i in 0..n {
        let x = (i as f64 + 0.5) * H;
        for j in 0..n {
            let y = (j as f64 + 0.5) * H;
            for k in 0..n {
                let z = (k as f64 + 0.5) * H;
                let mut d = f64::MAX;
                for centre in [-S5_SPHERE_SPACING, 0.0, S5_SPHERE_SPACING] {
                    let p = [x - centre, y, z];
                    d = d.min((p[0] * p[0] + p[1] * p[1] + p[2] * p[2]).sqrt() - S5_SPHERE_RADIUS);
                }
                let rho = (x * x + z * z).sqrt();
                d = d.min(((rho - S5_TUBE_MAJOR).powi(2) + y * y).sqrt() - S5_TUBE_MINOR);
                if d < 0.0 {
                    inside += 1;
                }
            }
        }
    }
    inside as f64 * H * H * H * 8.0
}

/// The same union in closed form: three spheres minus the two overlap lenses, plus the torus.
fn s5_closed_form_volume() -> f64 {
    let r = S5_SPHERE_RADIUS;
    let d = S5_SPHERE_SPACING;
    let lens = std::f64::consts::PI / 12.0 * (4.0 * r + d) * (2.0 * r - d).powi(2);
    let spheres = 3.0 * 4.0 / 3.0 * std::f64::consts::PI * r.powi(3) - 2.0 * lens;
    spheres
        + 2.0 * std::f64::consts::PI * std::f64::consts::PI * S5_TUBE_MAJOR * S5_TUBE_MINOR.powi(2)
}

#[test]
fn s5_volume_union_is_smoothed_meshed_and_made_editable() {
    let d = dir("s5");
    let (scene, polygon) = s5_build(&d, "a");

    // 1. Make Editable leaves one polygon object and no volume node.
    let (code, info) = json_run(&["info", s(&scene)]);
    assert_eq!(code, 0, "{info}");
    assert_eq!(info["roots"].as_array().unwrap().len(), 1, "{info}");
    let root = info["roots"][0].clone();
    assert_eq!(root["id"], polygon);
    assert_eq!(root["type"], "forge.polygon");
    assert!(
        root["params"]["mesh"]["polys"].as_u64().unwrap() > 1000,
        "the polygon holds the meshed volume: {root}"
    );
    let mut stack = vec![root];
    while let Some(node) = stack.pop() {
        let ty = node["type"].as_str().unwrap();
        assert!(
            !ty.starts_with("forge.volume_"),
            "{ty} is still in the scene after Make Editable"
        );
        for child in node["children"]
            .as_array()
            .map(Vec::as_slice)
            .unwrap_or(&[])
        {
            stack.push(child.clone());
        }
    }

    // 2. The exported mesh is closed, manifold and free of degenerate faces.
    let obj = d.join("s5.obj");
    export_at(&scene, "0", &obj);
    let mesh = s5_obj(&obj);
    assert!(
        mesh.positions.len() > 1000,
        "{} points",
        mesh.positions.len()
    );
    s5_assert_closed_manifold(&mesh);

    // 3. Every side of the bounding box is within one voxel of the sources' box.
    let mut lo = [f64::MAX; 3];
    let mut hi = [f64::MIN; 3];
    for p in &mesh.positions {
        for axis in 0..3 {
            lo[axis] = lo[axis].min(p[axis]);
            hi[axis] = hi[axis].max(p[axis]);
        }
    }
    let outer = [
        S5_TUBE_MAJOR + S5_TUBE_MINOR,
        S5_SPHERE_RADIUS,
        S5_TUBE_MAJOR + S5_TUBE_MINOR,
    ];
    for axis in 0..3 {
        assert!(
            (lo[axis] + outer[axis]).abs() <= S5_VOXEL,
            "axis {axis}: low {lo:?} is not within one voxel of {outer:?}"
        );
        assert!(
            (hi[axis] - outer[axis]).abs() <= S5_VOXEL,
            "axis {axis}: high {hi:?} is not within one voxel of {outer:?}"
        );
    }

    // 4. Enclosed volume within 5 % of the fine analytic reference.
    let reference = s5_reference_volume();
    let closed = s5_closed_form_volume();
    assert!(
        (reference - closed).abs() / closed < 0.005,
        "the fine grid reference {reference} disagrees with the closed form {closed}"
    );
    let volume = s5_signed_volume(&mesh);
    let relative = (volume - reference).abs() / reference;
    assert!(
        relative < 0.05,
        "mesh volume {volume} vs reference {reference}: {:.2} %",
        100.0 * relative
    );

    // 5. Reopen and export again: the same bytes, and the same bytes from a second run.
    let again = d.join("s5-again.obj");
    export_at(&scene, "0", &again);
    assert_eq!(
        std::fs::read(&obj).unwrap(),
        std::fs::read(&again).unwrap(),
        "save -> reopen -> export changed the OBJ"
    );
    let (scene_b, _) = s5_build(&d, "b");
    let second = d.join("s5-b.obj");
    export_at(&scene_b, "0", &second);
    assert_eq!(
        std::fs::read(&obj).unwrap(),
        std::fs::read(&second).unwrap(),
        "two runs of the script gave different OBJ files"
    );
}

// ---------------------------------------------------------------------------------------------
// S6: rigging through the CLI, `tests/acceptance/s6_rigging.rhai` (docs/roadmap/README.md S6):
// a joint chain bound to a cylinder with automatic weights, IK with a pole target, a Pose Morph,
// all animated. Pose Morph is our own design (D20, parity-pending O37); the IK pole side is the
// observed rule of RIG.md section 10 (the elbow goes opposite the pole).
// ---------------------------------------------------------------------------------------------

/// Chain geometry of the script: J1 at (0, -100, 0), two bones of 100.
const S6_ROOT: [f64; 3] = [0.0, -100.0, 0.0];
const S6_BONE: f64 = 100.0;
/// The goal and pole keys of the script at frames 0, 30 and 60.
const S6_KEYED: [(i64, [f64; 3], [f64; 3]); 3] = [
    (0, [60.0, 60.0, 0.0], [0.0, 0.0, 300.0]),
    (30, [0.0, 20.0, 80.0], [300.0, 0.0, 0.0]),
    (60, [-40.0, 70.0, 40.0], [0.0, 0.0, -300.0]),
];
/// The IK tag's default `threshold` (RIG.md section 10): the solver's stop rule.
const S6_IK_THRESHOLD: f64 = 0.01;

struct S6Ids {
    body: u64,
    m2: u64,
    m3: u64,
    morph: u64,
}

fn s6_script() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/acceptance/s6_rigging.rhai")
}

fn s6_build(d: &Path, name: &str) -> (PathBuf, S6Ids) {
    let scene = d.join(format!("s6-{name}.forge"));
    let (code, out) = json_run(&["run", s(&s6_script()), "--save", s(&scene)]);
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
    let ids = S6Ids {
        body: id("body"),
        m2: id("m2"),
        m3: id("m3"),
        morph: id("morph"),
    };
    (scene, ids)
}

fn s6_sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn s6_dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn s6_len(a: [f64; 3]) -> f64 {
    s6_dot(a, a).sqrt()
}

/// The part of `v` perpendicular to the unit vector `c`.
fn s6_perp(v: [f64; 3], c: [f64; 3]) -> [f64; 3] {
    let k = s6_dot(v, c);
    [v[0] - c[0] * k, v[1] - c[1] * k, v[2] - c[2] * k]
}

fn s6_cos(a: [f64; 3], b: [f64; 3]) -> f64 {
    s6_dot(a, b) / (s6_len(a) * s6_len(b))
}

/// The centre of a node's evaluated bounding box at `frame`, from `forge eval --json`.
fn s6_center(scene: &Path, frame: i64, node: u64) -> [f64; 3] {
    let (code, out) = json_run(&[
        "eval",
        s(scene),
        "--frame",
        &frame.to_string(),
        "--node",
        &node.to_string(),
    ]);
    assert_eq!(code, 0, "{out}");
    let bbox = &out["nodes"][0]["bbox"];
    let c = |i: usize| (bbox[0][i].as_f64().unwrap() + bbox[1][i].as_f64().unwrap()) / 2.0;
    [c(0), c(1), c(2)]
}

/// The points of a node at `frame`, exported to OBJ in `d`.
fn s6_points(d: &Path, scene: &Path, frame: i64, node: u64, tag: &str) -> Vec<[f64; 3]> {
    let obj = d.join(format!("{tag}-{node}-{frame}.obj"));
    let (code, out) = json_run(&[
        "eval",
        s(scene),
        "--frame",
        &frame.to_string(),
        "--node",
        &node.to_string(),
        "--export",
        s(&obj),
    ]);
    assert_eq!(code, 0, "{out}");
    parse_obj(&std::fs::read_to_string(&obj).unwrap())
        .unwrap()
        .positions
}

/// A copy of `scene` after one command.
fn s6_variant(d: &Path, scene: &Path, name: &str, command: &str, args: Value) -> PathBuf {
    let copy = d.join(format!("s6-variant-{name}.forge"));
    std::fs::copy(scene, &copy).unwrap();
    let (code, out) = json_run(&[
        "cmd",
        s(&copy),
        command,
        "--args",
        &args.to_string(),
        "--save",
    ]);
    assert_eq!(code, 0, "{out}");
    copy
}

#[test]
fn s6_rigging_skin_follows_the_joints_ik_pole_and_pose_morph() {
    let d = dir("s6");
    let (scene, ids) = s6_build(&d, "a");

    // 1. IK: the tip (the J3 marker rides on the end joint) reaches the keyed goal within the
    // solver's threshold; the bones keep their length (the chain never stretches); the elbow lies
    // in the plane of the chord and the pole, on the side opposite the pole (RIG.md section 10).
    let mut elbow_dirs = Vec::new();
    for (frame, goal, pole) in S6_KEYED {
        let tip = s6_center(&scene, frame, ids.m3);
        let elbow = s6_center(&scene, frame, ids.m2);
        let miss = s6_len(s6_sub(tip, goal));
        assert!(
            miss <= S6_IK_THRESHOLD,
            "frame {frame}: the tip is {miss} from the goal"
        );
        let upper = s6_len(s6_sub(elbow, S6_ROOT));
        let lower = s6_len(s6_sub(tip, elbow));
        assert!(
            (upper - S6_BONE).abs() < 1e-6 && (lower - S6_BONE).abs() < 1e-6,
            "frame {frame}: bones {upper}, {lower}"
        );
        let to_goal = s6_sub(goal, S6_ROOT);
        let chord = to_goal.map(|v| v / s6_len(to_goal));
        let bend = s6_perp(s6_sub(elbow, S6_ROOT), chord);
        let toward_pole = s6_perp(s6_sub(pole, S6_ROOT), chord);
        let cosine = s6_cos(bend, toward_pole);
        assert!(
            cosine < -0.999_999,
            "frame {frame}: the elbow must bend away from the pole (cos {cosine})"
        );
        elbow_dirs.push(bend);
    }
    // the pole moves from +Z to +X to -Z: the bend plane turns with it
    assert!(s6_cos(elbow_dirs[0], elbow_dirs[1]) < 0.9);
    assert!(s6_cos(elbow_dirs[0], elbow_dirs[2]) < 0.9);

    // 2. Skin: the cap centres sit on the chain's ends (automatic weights give them to the end
    // joints), so the skinned cylinder's ends follow the root and the IK-driven tip.
    for (frame, goal, _) in S6_KEYED {
        let pts = s6_points(&d, &scene, frame, ids.body, "skin");
        assert_eq!(pts.len(), 110, "2 cap centres + 12 rings of 9");
        let bottom = s6_len(s6_sub(pts[0], S6_ROOT));
        let top = s6_len(s6_sub(pts[1], goal));
        assert!(
            bottom < 0.5,
            "frame {frame}: bottom cap centre {bottom} from J1"
        );
        assert!(
            top < 0.5,
            "frame {frame}: top cap centre {top} from the tip"
        );
    }
    let rest = s6_points(&d, &scene, 0, ids.body, "rest-check");
    let bent = s6_points(&d, &scene, 30, ids.body, "rest-check");
    let moved = rest
        .iter()
        .zip(&bent)
        .map(|(a, b)| s6_len(s6_sub(*a, *b)))
        .fold(0.0, f64::max);
    assert!(
        moved > 50.0,
        "the skin follows the animated joints ({moved})"
    );

    // 3. Pose Morph: compared with the tag switched off. Strength 0 % (frame 0) changes nothing;
    // at 100 % (frame 30) the widened upper half moves, the lower half and the cap centres do not;
    // the skin is linear, so doubling a key doubles the displacement at the same frame.
    let tag = ids.morph;
    let off = s6_variant(
        &d,
        &scene,
        "off",
        "param.set",
        json!({"node": tag, "param": "enabled", "value": false}),
    );
    let displacement = |frame: i64, other: &Path, with: &Path| -> Vec<[f64; 3]> {
        let a = s6_points(&d, with, frame, ids.body, "morph-on");
        let b = s6_points(&d, other, frame, ids.body, "morph-off");
        assert_eq!(a.len(), b.len());
        a.iter().zip(&b).map(|(p, q)| s6_sub(*p, *q)).collect()
    };
    let zero = displacement(0, &off, &scene);
    assert!(zero.iter().all(|v| s6_len(*v) < 1e-9), "0 % moves nothing");
    let full = displacement(30, &off, &scene);
    // point 0 is the bottom cap centre; ring points j = 0..4 (y <= 0 at rest) are not in the target
    let untouched = |i: usize| i == 0 || i == 1 || (i >= 2 && (i - 2) % 9 <= 4);
    for (i, v) in full.iter().enumerate() {
        if untouched(i) {
            assert!(s6_len(*v) < 1e-9, "point {i} is outside the target: {v:?}");
        }
    }
    let max_full = full.iter().map(|v| s6_len(*v)).fold(0.0, f64::max);
    // at 100 % a rest-pose ring point moves by 20 (radius 20 -> 40); the skin blend can only shorten it
    assert!(
        max_full > 5.0 && max_full <= 20.0 + 1e-6,
        "the morph moves the upper half by at most its 20 units ({max_full})"
    );
    // frame 60 holds 50 %; the same scene with that key at 100 % moves every point twice as far
    let half = displacement(60, &off, &scene);
    let doubled = s6_variant(
        &d,
        &scene,
        "double",
        "key.set",
        json!({"node": tag, "track": "strength_0", "frame": 60, "value": 100.0}),
    );
    let double = displacement(60, &off, &doubled);
    assert!(half.iter().any(|v| s6_len(*v) > 1.0));
    for (i, (h, f)) in half.iter().zip(&double).enumerate() {
        for axis in 0..3 {
            assert!(
                (2.0 * h[axis] - f[axis]).abs() < 1e-6,
                "point {i}: 50 % {h:?} vs 100 % {f:?}"
            );
        }
    }
}

#[test]
fn s6_all_frames_evaluate_and_two_runs_agree() {
    let d = dir("s6-frames");
    let (a, ids) = s6_build(&d, "a");
    let (b, _) = s6_build(&d, "b");
    // every frame of the animation evaluates without error; the measured mean frame time is
    // reported, not asserted (HERMES-IMPLEMENTER 5a: no wall-clock assertions)
    let started = std::time::Instant::now();
    let frames = 0..=60;
    let count = frames.clone().count();
    for frame in frames {
        let (code, out) = json_run(&["eval", s(&a), "--frame", &frame.to_string()]);
        assert_eq!(code, 0, "frame {frame}: {out}");
        assert_eq!(out["ok"], true, "frame {frame}: {out}");
    }
    eprintln!(
        "S6: {count} frames, mean {:.1} ms per frame (one forge process per frame, debug build)",
        started.elapsed().as_secs_f64() * 1000.0 / count as f64
    );
    // determinism: the same script gives the same skinned points and joint positions
    for frame in [0, 15, 30, 45, 60] {
        assert_eq!(
            s6_points(&d, &a, frame, ids.body, "run-a"),
            s6_points(&d, &b, frame, ids.body, "run-b"),
            "frame {frame}"
        );
        for node in [ids.m2, ids.m3] {
            assert_eq!(s6_center(&a, frame, node), s6_center(&b, frame, node));
        }
    }
}
