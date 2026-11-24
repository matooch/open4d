//! Acceptance scenario S9 (docs/roadmap/README.md), driven only through the `forge` CLI:
//!
//! > Exchange: import FBX/glTF/Alembic exported from the reference app; geometry and
//! > animation match the reference within tolerance.
//!
//! For each of the six O44 cases (`tests/golden/exchange/<case>/`) and each file the
//! reference exported (`scene.fbx`, `scene.glb`, `scene.abc`, `scene.stl`) the test
//!
//! 1. imports the file with `forge convert --format forge`;
//! 2. evaluates it with `forge eval --export` and compares the OBJ with the reference's
//!    `mesh.obj` through `compare_geometric` at the case's tolerance;
//! 3. reads node transforms, cameras and lights from `forge info --json`, evaluated
//!    transforms and materials from a glTF written by `forge export --frame N`, and posed
//!    bounds from `forge eval --frame N`, and compares them with `case.toml`.
//!
//! What a format cannot carry (docs/specs/IO.md section 9) is asserted to be absent, so a
//! format that starts to carry more makes the test fail instead of passing unnoticed.
//! `crates/io/tests/{fbx,alembic,exchange_stl_ply}.rs` and `crates/app/tests/gltf_exchange.rs`
//! cover the same files at the library level (coverage matrix in the brief 171 report).

use std::path::{Path, PathBuf};

use assert_cmd::Command;
use forge_parity_harness::{
    CaseManifest, GoldenCase, Mesh, compare_geometric, discover_cases, parse_obj,
};
use serde_json::{Value, json};

const FORMATS: [&str; 4] = ["fbx", "glb", "abc", "stl"];

fn forge() -> Command {
    let mut cmd = Command::cargo_bin("forge").unwrap();
    // never pick up plugins from the machine running the tests
    cmd.env("FORGE_NO_PLUGINS", "1");
    cmd
}

/// Runs `forge --json args`; returns (exit code, stdout JSON, stderr).
fn run(args: &[&str]) -> (i32, Value, String) {
    let out = forge().arg("--json").args(args).output().unwrap();
    let text = String::from_utf8(out.stdout).unwrap();
    let value = serde_json::from_str(&text).unwrap_or_else(|e| panic!("{e}: {text:?}"));
    let err = String::from_utf8_lossy(&out.stderr).into_owned();
    (out.status.code().unwrap(), value, err)
}

fn ok(args: &[&str]) -> Value {
    let (code, value, err) = run(args);
    assert_eq!(code, 0, "forge {args:?}: {value} {err}");
    value
}

fn s(p: &Path) -> &str {
    p.to_str().unwrap()
}

fn case(name: &str) -> (GoldenCase, CaseManifest) {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden");
    let case = discover_cases(&root)
        .unwrap()
        .into_iter()
        .find(|c| c.feature == "exchange" && c.name == name)
        .unwrap_or_else(|| panic!("no exchange case {name}"));
    let manifest = case.manifest().unwrap();
    (case, manifest)
}

/// One scratch folder per case and process; every format gets a sub folder.
fn scratch(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("forge-s9-{}-{name}", std::process::id()));
    std::fs::remove_dir_all(&d).ok();
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// Step 1 (every case, every format): `forge convert --format forge`. `scale=1` because the
/// reference writes centimetre values into its glTF too (IO.md section 5); the other
/// importers' default is 1 already.
fn import(case: &GoldenCase, ext: &str, d: &Path) -> PathBuf {
    let src = case.dir.join(format!("scene.{ext}"));
    let out = d.join(ext);
    let result = ok(&[
        "convert",
        s(&src),
        "--out",
        s(&out),
        "--format",
        "forge",
        "--option",
        "scale=1",
    ]);
    assert_eq!(result["converted"], 1, "{ext}: {result}");
    out.join("scene.forge")
}

// ---------------------------------------------------------------------------
// Readers over the CLI's JSON
// ---------------------------------------------------------------------------

/// One node of `forge info --json`, flattened in tree order.
struct N {
    name: String,
    ty: String,
    /// Index of the parent in the list.
    parent: Option<usize>,
    params: Value,
    animated: Vec<String>,
}

impl N {
    fn vec3(&self, key: &str) -> [f64; 3] {
        let v = &self.params[key];
        std::array::from_fn(|i| {
            v[i].as_f64()
                .unwrap_or_else(|| panic!("{}: {key}", self.name))
        })
    }
    fn num(&self, key: &str) -> f64 {
        self.params[key]
            .as_f64()
            .unwrap_or_else(|| panic!("{}: {key}", self.name))
    }
}

fn tree(scene: &Path) -> Vec<N> {
    fn walk(v: &Value, parent: Option<usize>, out: &mut Vec<N>) {
        let me = out.len();
        out.push(N {
            name: v["name"].as_str().unwrap().to_string(),
            ty: v["type"].as_str().unwrap().to_string(),
            parent,
            params: v["params"].clone(),
            animated: v["animated"]
                .as_array()
                .map(|a| a.iter().map(|t| t.as_str().unwrap().to_string()).collect())
                .unwrap_or_default(),
        });
        for c in v["children"].as_array().unwrap() {
            walk(c, Some(me), out);
        }
    }
    let info = ok(&["info", s(scene)]);
    let mut out = Vec::new();
    for r in info["roots"].as_array().unwrap() {
        walk(r, None, &mut out);
    }
    out
}

fn of_type<'a>(nodes: &'a [N], ty: &str) -> Vec<&'a N> {
    nodes.iter().filter(|n| n.ty == ty).collect()
}

/// Names match across formats up to case, spaces and underscores (`Arm Mesh`, `Arm_Mesh`).
fn same_name(a: &str, b: &str) -> bool {
    let norm = |t: &str| t.to_lowercase().replace([' ', '_'], "");
    norm(a) == norm(b)
}

fn find<'a>(nodes: &'a [N], name: &str) -> &'a N {
    nodes
        .iter()
        .find(|n| same_name(&n.name, name))
        .unwrap_or_else(|| panic!("no node {name}"))
}

fn close3(a: [f64; 3], b: [f64; 3], tol: f64) -> bool {
    a.iter().zip(&b).all(|(x, y)| (x - y).abs() <= tol)
}

fn triple(v: &toml::Value) -> [f64; 3] {
    let a = v.as_array().unwrap();
    std::array::from_fn(|i| {
        a[i].as_float()
            .or_else(|| a[i].as_integer().map(|n| n as f64))
            .unwrap()
    })
}

fn float(v: &toml::Value) -> f64 {
    v.as_float().unwrap()
}

/// Local transform of a node must equal the `position` / `rotation_degrees` / `scale` of its
/// `case.toml` table (HPB degrees).
fn assert_transform(n: &N, want: &toml::Value, tol: f64) {
    let pos = triple(&want["position"]);
    let rot = triple(&want["rotation_degrees"]);
    let scale = triple(&want["scale"]);
    assert!(
        close3(n.vec3("transform.pos"), pos, tol),
        "{}: pos {:?} vs {pos:?}",
        n.name,
        n.vec3("transform.pos")
    );
    assert!(
        close3(n.vec3("transform.rot"), rot, tol),
        "{}: rot {:?} vs {rot:?}",
        n.name,
        n.vec3("transform.rot")
    );
    assert!(
        close3(n.vec3("transform.scale"), scale, tol),
        "{}: scale {:?} vs {scale:?}",
        n.name,
        n.vec3("transform.scale")
    );
}

/// `forge eval --frame N --export`, the OBJ parsed; also returns the JSON.
fn eval_obj(scene: &Path, frame: u32, obj: &Path) -> (Mesh, Value) {
    let result = ok(&[
        "eval",
        s(scene),
        "--frame",
        &frame.to_string(),
        "--export",
        s(obj),
    ]);
    let mesh = parse_obj(&std::fs::read_to_string(obj).unwrap()).unwrap();
    (mesh, result)
}

/// What the reference's STL holds of `mesh.obj`: triangles only (quads fanned from the first
/// corner, which the writer test in `exchange_stl_ply.rs` pins against the golden STL bytes).
fn triangulated(mesh: &Mesh) -> Vec<[[f64; 3]; 3]> {
    mesh.faces
        .iter()
        .flat_map(|f| (1..f.len() - 1).map(|i| [f[0], f[i], f[i + 1]]))
        .map(|t| t.map(|v| mesh.positions[v as usize]))
        .collect()
}

/// STL is a triangle soup, so planes of nearly planar quads cannot be grouped reliably
/// (`compare_geometric` merges the two triangles of a sphere quad on the f64 side and splits
/// them on the f32 side). Every expected triangle must instead be present, corner for corner
/// in the same winding, within the case tolerance, and nothing else.
fn assert_triangles(label: &str, expected: &Mesh, got: &Mesh, tol: f64) {
    let want = triangulated(expected);
    let have = triangulated(got);
    assert_eq!(want.len(), have.len(), "{label}: triangle count");
    let mut used = vec![false; have.len()];
    for (i, w) in want.iter().enumerate() {
        let hit = have.iter().enumerate().position(|(k, h)| {
            !used[k] && (0..3).any(|r| (0..3).all(|c| close3(w[c], h[(c + r) % 3], tol)))
        });
        let k = hit.unwrap_or_else(|| panic!("{label}: triangle {i} {w:?} missing"));
        used[k] = true;
    }
}

fn assert_geometry(label: &str, expected: &Mesh, got: &Mesh, m: &CaseManifest) {
    let report = compare_geometric(expected, got, &m.tolerance);
    assert!(report.passed(), "{label}: {:?}", report.failures);
    // not vacuous: the expectation has points unless the case is the empty camera scene
    assert_eq!(report.points.0, report.points.1, "{label}");
}

/// Step 2 for a case whose `mesh.obj` is the scene at frame 0: every format's evaluated
/// geometry equals it (STL against the triangulated expectation).
fn check_frame0_geometry(
    case: &GoldenCase,
    m: &CaseManifest,
    d: &Path,
    scenes: &[(&str, PathBuf)],
) {
    let expected = case.expected_mesh().unwrap().unwrap();
    assert!(!expected.positions.is_empty(), "{}", case.name);
    for (ext, scene) in scenes {
        let (got, _) = eval_obj(scene, 0, &d.join(format!("{ext}-f0.obj")));
        let label = format!("{} {ext}", case.name);
        if *ext == "stl" {
            assert_triangles(&label, &expected, &got, m.tolerance.position);
        } else {
            assert_geometry(&label, &expected, &got, m);
        }
    }
}

fn import_all(case: &GoldenCase, d: &Path) -> Vec<(&'static str, PathBuf)> {
    FORMATS
        .iter()
        .map(|ext| (*ext, import(case, ext, d)))
        .collect()
}

fn scene_of<'a>(scenes: &'a [(&str, PathBuf)], ext: &str) -> &'a Path {
    &scenes.iter().find(|(e, _)| *e == ext).unwrap().1
}

/// A glTF written by `forge export --frame N`: the scene evaluated at that frame, in metres
/// with Z reflected. Returns its JSON chunk.
fn evaluated_gltf(scene: &Path, frame: u32, out: &Path) -> Value {
    ok(&[
        "export",
        s(scene),
        "--format",
        "gltf",
        "--frame",
        &frame.to_string(),
        "--out",
        s(out),
    ]);
    let bytes = std::fs::read(out).unwrap();
    assert_eq!(&bytes[..4], b"glTF");
    let len = u32::from_le_bytes(bytes[12..16].try_into().unwrap()) as usize;
    assert_eq!(&bytes[16..20], b"JSON");
    serde_json::from_slice(&bytes[20..20 + len]).unwrap()
}

/// Translation (centimetres, our axes), unit quaternion and scale of the glTF node `name`.
fn gltf_node(json: &Value, name: &str) -> ([f64; 3], [f64; 4], [f64; 3]) {
    let node = json["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|n| same_name(n["name"].as_str().unwrap_or(""), name))
        .unwrap_or_else(|| panic!("no glTF node {name}"));
    let get = |key: &str, default: &[f64]| -> Vec<f64> {
        node.get(key)
            .map(|v| {
                v.as_array()
                    .unwrap()
                    .iter()
                    .map(|x| x.as_f64().unwrap())
                    .collect()
            })
            .unwrap_or_else(|| default.to_vec())
    };
    let t = get("translation", &[0.0; 3]);
    let r = get("rotation", &[0.0, 0.0, 0.0, 1.0]);
    let sc = get("scale", &[1.0; 3]);
    (
        [t[0] * 100.0, t[1] * 100.0, -t[2] * 100.0],
        [r[0], r[1], r[2], r[3]],
        [sc[0], sc[1], sc[2]],
    )
}

/// The unit quaternion the glTF export writes for our HPB angles when only one of them is
/// set: H turns about +Y, B about -Z (the Z reflection of the export, IO.md section 8.2).
fn heading_quaternion(deg: f64) -> [f64; 4] {
    let (s, c) = (deg.to_radians() / 2.0).sin_cos();
    [0.0, s, 0.0, c]
}

fn bank_quaternion(deg: f64) -> [f64; 4] {
    let (s, c) = (deg.to_radians() / 2.0).sin_cos();
    [0.0, 0.0, -s, c]
}

fn close_quat(a: [f64; 4], b: [f64; 4], tol: f64) -> bool {
    // q and -q are the same rotation
    let direct = a.iter().zip(&b).all(|(x, y)| (x - y).abs() <= tol);
    let flipped = a.iter().zip(&b).all(|(x, y)| (x + y).abs() <= tol);
    direct || flipped
}

fn bbox_of(eval: &Value) -> [[f64; 3]; 2] {
    let nodes = eval["nodes"].as_array().unwrap();
    assert_eq!(nodes.len(), 1, "{eval}");
    let b = &nodes[0]["bbox"];
    std::array::from_fn(|i| std::array::from_fn(|k| b[i][k].as_f64().unwrap()))
}

fn bbox_error(got: [[f64; 3]; 2], want: &toml::Value) -> f64 {
    let want = want.as_array().unwrap();
    (0..2)
        .map(|i| {
            let w = triple(&want[i]);
            (0..3).map(|k| (got[i][k] - w[k]).abs()).fold(0.0, f64::max)
        })
        .fold(0.0, f64::max)
}

// ---------------------------------------------------------------------------
// The six cases
// ---------------------------------------------------------------------------

#[test]
fn s9_static_mesh() {
    let (case, m) = case("static-mesh");
    let d = scratch(&case.name);
    let scenes = import_all(&case, &d);
    check_frame0_geometry(&case, &m, &d, &scenes);
    for ext in ["fbx", "glb", "abc"] {
        let nodes = tree(scene_of(&scenes, ext));
        let polys = of_type(&nodes, "forge.polygon");
        assert_eq!(polys.len(), 2, "{ext}");
        for key in ["cube", "sphere"] {
            let n = find(&nodes, key);
            assert!(n.parent.is_none(), "{ext} {key}");
            assert_transform(n, &m.params[key], 1e-4);
        }
    }
    // STL has no hierarchy or names: one polygon object with the geometry in world space
    let nodes = tree(scene_of(&scenes, "stl"));
    assert_eq!(nodes.len(), 1);
    assert_eq!(nodes[0].ty, "forge.polygon");
    assert!(close3(nodes[0].vec3("transform.pos"), [0.0; 3], 0.0));
}

#[test]
fn s9_hierarchy() {
    let (case, m) = case("hierarchy");
    let d = scratch(&case.name);
    let scenes = import_all(&case, &d);
    check_frame0_geometry(&case, &m, &d, &scenes);
    for ext in ["fbx", "glb", "abc"] {
        let nodes = tree(scene_of(&scenes, ext));
        let root = find(&nodes, "root");
        assert_eq!(root.ty, "forge.null", "{ext}");
        assert!(root.parent.is_none());
        assert_transform(root, &m.params["root"], 1e-4);
        let cube = find(&nodes, "cube");
        assert_eq!(cube.ty, "forge.polygon", "{ext}");
        let parent = cube.parent.expect("cube is a child");
        assert_eq!(nodes[parent].name, root.name, "{ext}");
        assert_transform(cube, &m.params["cube"], 1e-4);
    }
    // STL: the hierarchy is flattened into world-space triangles (checked by the geometry)
    let nodes = tree(scene_of(&scenes, "stl"));
    assert_eq!(nodes.len(), 1);
    assert_eq!(nodes[0].ty, "forge.polygon");
}

#[test]
fn s9_anim_transform() {
    let (case, m) = case("anim-transform");
    let d = scratch(&case.name);
    let scenes = import_all(&case, &d);
    check_frame0_geometry(&case, &m, &d, &scenes);
    let keys = m.params["keyframes"].as_array().unwrap();
    let key = |frame: i64| {
        keys.iter()
            .find(|k| k["frame"].as_integer() == Some(frame))
            .unwrap()
    };
    let expected = case.expected_mesh().unwrap().unwrap();
    let mut mid = Vec::new();
    for ext in ["fbx", "glb", "abc"] {
        let scene = scene_of(&scenes, ext);
        let nodes = tree(scene);
        let cube = of_type(&nodes, "forge.polygon");
        assert_eq!(cube.len(), 1, "{ext}");
        // position, rotation and scale are animated; the file's static values are the start
        assert!(
            ["transform.pos.x", "transform.rot.x", "transform.scale.x"]
                .iter()
                .all(|t| cube[0].animated.iter().any(|a| a == t)),
            "{ext}: {:?}",
            cube[0].animated
        );
        let name = cube[0].name.clone();
        // the evaluated transform at the recorded frames, read from the evaluated glTF
        for frame in [0_i64, 30] {
            let k = key(frame);
            let json = evaluated_gltf(scene, frame as u32, &d.join(format!("{ext}-{frame}.glb")));
            let (t, q, sc) = gltf_node(&json, &name);
            let pos = triple(&k["position"]);
            let rot = triple(&k["rotation_degrees"]);
            let scale = triple(&k["scale"]);
            assert!(close3(t, pos, 1e-4), "{ext} f{frame}: pos {t:?} vs {pos:?}");
            assert!(close3(sc, scale, 1e-6), "{ext} f{frame}: scale {sc:?}");
            assert!(
                close_quat(q, heading_quaternion(rot[0]), 1e-6),
                "{ext} f{frame}: rotation {q:?} for H {}",
                rot[0]
            );
        }
        // the evaluated geometry at frame 30: the cube scaled by 1.5 and moved
        let (got, _) = eval_obj(scene, 30, &d.join(format!("{ext}-f30.obj")));
        let k = key(30);
        let (pos, scale) = (
            triple(&k["position"]),
            float(&k["scale"].as_array().unwrap()[0]),
        );
        let want = Mesh {
            positions: expected
                .positions
                .iter()
                .map(|p| std::array::from_fn(|i| p[i] * scale + pos[i]))
                .collect(),
            faces: expected.faces.clone(),
        };
        assert_geometry(&format!("{ext} f30"), &want, &got, &m);
        // frame 15 lies between the keys, and the three formats agree on it
        let json = evaluated_gltf(scene, 15, &d.join(format!("{ext}-15.glb")));
        let (t, _, sc) = gltf_node(&json, &name);
        assert!(
            t[0] > 0.0 && t[0] < 200.0 && sc[0] > 1.0 && sc[0] < 1.5,
            "{ext}: {t:?} {sc:?}"
        );
        mid.push((ext, t, sc));
    }
    for (ext, t, sc) in &mid[1..] {
        assert!(
            close3(*t, mid[0].1, 1e-2),
            "f15 position {ext}: {t:?} vs {:?}",
            mid[0].1
        );
        assert!(
            close3(*sc, mid[0].2, 1e-4),
            "f15 scale {ext}: {sc:?} vs {:?}",
            mid[0].2
        );
    }
    // STL has no animation: the geometry stays at frame 0
    let (still, _) = eval_obj(scene_of(&scenes, "stl"), 30, &d.join("stl-f30.obj"));
    assert_triangles("stl f30", &expected, &still, m.tolerance.position);
}

#[test]
fn s9_skinned_arm() {
    let (case, m) = case("skinned-arm");
    let d = scratch(&case.name);
    let scenes = import_all(&case, &d);
    check_frame0_geometry(&case, &m, &d, &scenes);
    let posed = &m.params["posed_samples"];
    let tol = m.tolerance.position;
    let rest = bbox_of(&ok(&["eval", s(scene_of(&scenes, "stl")), "--frame", "0"]));

    // FBX: joints, skin and the J2 keys are carried; both poses match the recorded bounds
    {
        let scene = scene_of(&scenes, "fbx");
        let nodes = tree(scene);
        for j in ["J1", "J2", "J3"] {
            assert_eq!(find(&nodes, j).ty, "forge.joint", "{j}");
        }
        assert_eq!(of_type(&nodes, "forge.skin").len(), 1);
        for frame in [15_u32, 30] {
            let got = bbox_of(&ok(&["eval", s(scene), "--frame", &frame.to_string()]));
            let err = bbox_error(got, &posed[format!("f{frame}_bbox")]);
            assert!(err <= tol, "fbx f{frame}: bounds off by {err:e}");
            let json = evaluated_gltf(scene, frame, &d.join(format!("fbx-{frame}.glb")));
            let (_, q, _) = gltf_node(&json, "J2");
            let deg = triple(&posed[format!("f{frame}_j2_rotation_degrees")]);
            assert!(
                close_quat(q, bank_quaternion(deg[2]), 1e-5),
                "fbx f{frame}: J2 {q:?} vs {deg:?}"
            );
        }
    }

    // Alembic: geometry only; the exporter baked the deformed points per frame, joints
    // arrive as nulls without a skin
    {
        let scene = scene_of(&scenes, "abc");
        let nodes = tree(scene);
        assert!(of_type(&nodes, "forge.skin").is_empty());
        assert!(of_type(&nodes, "forge.joint").is_empty());
        assert!(
            find(&nodes, "arm mesh")
                .animated
                .iter()
                .any(|a| a == "mesh.points")
        );
        for frame in [15_u32, 30] {
            let got = bbox_of(&ok(&["eval", s(scene), "--frame", &frame.to_string()]));
            let err = bbox_error(got, &posed[format!("f{frame}_bbox")]);
            assert!(err <= tol, "abc f{frame}: bounds off by {err:e}");
        }
    }

    // glTF: skin and weights survive, the animation does not (the exporter bakes none).
    // The recorded J2 keys are restored through the CLI, then the posed bounds are compared.
    {
        let scene = scene_of(&scenes, "glb");
        let nodes = tree(scene);
        assert_eq!(of_type(&nodes, "forge.joint").len(), 3);
        assert_eq!(of_type(&nodes, "forge.skin").len(), 1);
        assert!(
            nodes.iter().all(|n| n.animated.is_empty()),
            "the reference GLB has no animation"
        );
        let still = bbox_of(&ok(&["eval", s(scene), "--frame", "15"]));
        assert!(
            bbox_error(still, &posed["f0_bbox"]) <= tol,
            "unkeyed glb stays in the bind pose"
        );
        // restore the keys
        let j2 = {
            let info = ok(&["info", s(scene)]);
            fn find_id(v: &Value) -> Option<u64> {
                if v["name"] == "J2" {
                    return v["id"].as_u64();
                }
                v["children"].as_array()?.iter().find_map(find_id)
            }
            info["roots"]
                .as_array()
                .unwrap()
                .iter()
                .find_map(find_id)
                .unwrap()
        };
        let keyed = d.join("glb-keyed.forge");
        std::fs::copy(scene, &keyed).unwrap();
        for k in m.params["animation"]["keys"].as_array().unwrap() {
            let frame = k["frame"].as_integer().unwrap();
            let args = json!({"node": j2, "param": "transform.rot", "frame": frame,
                "value": [0.0, 0.0, float(&k["degrees"])]});
            ok(&[
                "cmd",
                s(&keyed),
                "key.set_parameter",
                "--args",
                &args.to_string(),
                "--save",
            ]);
        }
        let keys: Vec<Value> = [0, 30]
            .iter()
            .map(|f| json!({"frame": f, "node": j2, "track": "transform.rot.z"}))
            .collect();
        let args = json!({"interp": "linear", "keys": keys});
        ok(&[
            "cmd",
            s(&keyed),
            "key.set_interp",
            "--args",
            &args.to_string(),
            "--save",
        ]);
        // KNOWN_MISMATCH (IO.md section 5): the recorded poses used the authoring rig; the
        // GLB's weight rows sum to 0.9999847 and the skin is normalised, so the bounds
        // differ by up to 5e-4 (the round trip through our own GLB is exact, see
        // `gltf_exchange.rs`).
        let mut mismatched = Vec::new();
        for frame in [15_u32, 30] {
            let got = bbox_of(&ok(&["eval", s(&keyed), "--frame", &frame.to_string()]));
            let err = bbox_error(got, &posed[format!("f{frame}_bbox")]);
            assert!(err < 5e-4, "glb f{frame}: bounds off by {err:e}");
            if err > tol {
                mismatched.push(frame);
            }
        }
        assert_eq!(
            mismatched,
            [15, 30],
            "glb skin: KNOWN_MISMATCH changed, update IO.md section 5"
        );
    }

    // STL: every skinning channel is lost, the mesh stays in the bind pose
    {
        let scene = scene_of(&scenes, "stl");
        assert_eq!(tree(scene).len(), 1);
        for frame in [15_u32, 30] {
            let got = bbox_of(&ok(&["eval", s(scene), "--frame", &frame.to_string()]));
            assert_eq!(got, rest, "stl f{frame}");
        }
        assert!(bbox_error(rest, &posed["f0_bbox"]) <= tol);
    }
}

#[test]
fn s9_camera_light() {
    let (case, m) = case("camera-light");
    let d = scratch(&case.name);
    let expected = case.expected_mesh().unwrap().unwrap();
    assert!(
        expected.positions.is_empty(),
        "the reference mesh.obj is empty"
    );
    let cam = &m.params["camera"];
    let fov = float(&cam["parameters"]["field_of_view_horizontal_degrees"]);
    let sensor = float(&cam["parameters"]["sensor_size_film_gate"]);
    for ext in ["fbx", "glb", "abc"] {
        let scene = import(&case, ext, &d);
        // no geometry: nothing to export, and eval reports no node
        let result = ok(&["eval", s(&scene), "--frame", "0"]);
        assert!(
            result["nodes"].as_array().unwrap().is_empty(),
            "{ext}: {result}"
        );
        let (code, _, _) = run(&["eval", s(&scene), "--export", s(&d.join("empty.obj"))]);
        assert_eq!(code, 1, "{ext}: an empty scene cannot be exported");
        let nodes = tree(&scene);
        let cameras = of_type(&nodes, "forge.camera");
        let lights = of_type(&nodes, "forge.light");
        // the FBX also carries the exporter's own viewport camera
        assert_eq!(cameras.len(), if ext == "fbx" { 2 } else { 1 }, "{ext}");
        let camera = find(&nodes, "camera");
        assert_eq!(camera.ty, "forge.camera");
        assert!(
            close3(camera.vec3("transform.pos"), triple(&cam["position"]), 1e-4),
            "{ext}"
        );
        assert!(
            close3(
                camera.vec3("transform.rot"),
                triple(&cam["rotation_degrees"]),
                1e-4
            ),
            "{ext}"
        );
        assert!((camera.num("sensor_size") - sensor).abs() < 1e-9, "{ext}");
        let got_fov = camera.num("fov_horizontal"); // `info` shows degrees
        assert!(
            (got_fov - fov).abs() < 1e-5,
            "{ext}: fov {got_fov} vs {fov}"
        );
        if ext == "fbx" {
            assert_eq!(lights.len(), 2);
            for (key, name, kind) in [("omni", "Omni", "omni"), ("spot", "Spot", "spot")] {
                let l = find(&nodes, name);
                assert_eq!(l.ty, "forge.light");
                assert_eq!(l.params["type"], kind);
                let want = &m.params[key];
                assert!(
                    close3(l.vec3("transform.pos"), triple(&want["position"]), 1e-4),
                    "{name}"
                );
                let p = &want["parameters"];
                assert!(
                    (l.num("intensity") - float(&p["intensity_percent"])).abs() < 1e-9,
                    "{name}"
                );
                if kind == "spot" {
                    let outer = l.num("outer_angle"); // degrees in `info`
                    assert!(
                        (outer - float(&p["outer_angle_degrees"])).abs() < 1e-6,
                        "{name}"
                    );
                }
            }
        } else {
            // glTF has no light export option and Alembic no light schema (IO.md section 9)
            assert!(lights.is_empty(), "{ext} carries no lights");
        }
    }
    // STL of an empty scene has no triangle: nothing to import
    let src = case.dir.join("scene.stl");
    let out = forge()
        .args([
            "convert",
            s(&src),
            "--out",
            s(&d.join("stl")),
            "--format",
            "forge",
        ])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&out.stderr).contains("no polygons found"));
}

#[test]
fn s9_materials() {
    let (case, m) = case("materials");
    let d = scratch(&case.name);
    let scenes = import_all(&case, &d);
    check_frame0_geometry(&case, &m, &d, &scenes);
    let materials = m.params["materials"].as_array().unwrap();
    assert_eq!(materials.len(), 3);
    // (material of the cube at this x) in the glTF written from each import
    for ext in FORMATS {
        let nodes = tree(scene_of(&scenes, ext));
        let json = evaluated_gltf(scene_of(&scenes, ext), 0, &d.join(format!("{ext}-mat.glb")));
        let mats = json["materials"].as_array().cloned().unwrap_or_default();
        let polys: Vec<&N> = of_type(&nodes, "forge.polygon");
        match ext {
            "fbx" | "glb" => {
                assert_eq!(polys.len(), 3, "{ext}");
                assert_eq!(mats.len(), 3, "{ext}");
                for entry in materials {
                    let want = &entry["material"];
                    let x = triple(&entry["object"]["position"])[0];
                    // the glTF node of the cube at this x and its material
                    let node = json["nodes"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .find(|n| {
                            n.get("mesh").is_some()
                                && (n["translation"][0].as_f64().unwrap() * 100.0 - x).abs() < 1e-4
                        })
                        .unwrap_or_else(|| panic!("{ext}: no cube at x = {x}"));
                    let primitive =
                        &json["meshes"][node["mesh"].as_u64().unwrap() as usize]["primitives"][0];
                    let mat = &mats[primitive["material"].as_u64().unwrap() as usize];
                    assert_eq!(mat["name"].as_str(), want["name"].as_str(), "{ext} x = {x}");
                    let pbr = &mat["pbrMetallicRoughness"];
                    let color = triple(&want["color"]);
                    let got: Vec<f64> = (0..3)
                        .map(|i| pbr["baseColorFactor"][i].as_f64().unwrap())
                        .collect();
                    assert!(
                        close3([got[0], got[1], got[2]], color, 1e-6),
                        "{ext} {}: {got:?}",
                        want["name"]
                    );
                    if ext == "glb" {
                        // the reference maps the reflection switch to the metallic/roughness pair
                        let reflective = want["use_reflection"].as_integer() == Some(1);
                        let (metal, rough) = if reflective { (0.0, 0.1) } else { (1.0, 1.0) };
                        assert!((pbr["metallicFactor"].as_f64().unwrap() - metal).abs() < 1e-6);
                        assert!((pbr["roughnessFactor"].as_f64().unwrap() - rough).abs() < 1e-6);
                    }
                }
            }
            "abc" => {
                // geometry and transforms only: three cubes, no material, no texture tag
                assert_eq!(polys.len(), 3);
                assert!(mats.is_empty(), "Alembic carries no materials");
                assert!(nodes.iter().all(|n| n.ty != "forge.tag.texture"));
                let mut xs: Vec<f64> = polys.iter().map(|n| n.vec3("transform.pos")[0]).collect();
                xs.sort_by(f64::total_cmp);
                assert_eq!(xs, [-150.0, 0.0, 150.0]);
            }
            _ => {
                // STL: triangles of all three cubes in one object, no material
                assert_eq!(nodes.len(), 1);
                assert!(mats.is_empty(), "STL carries no materials");
            }
        }
    }
}
