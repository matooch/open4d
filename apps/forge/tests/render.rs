//! `forge render` (docs/specs/CLI.md, RENDER.md R12).

use std::path::{Path, PathBuf};

use assert_cmd::Command;
use serde_json::{Value, json};

fn dir(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("forge-render-test-{}-{name}", std::process::id()));
    std::fs::remove_dir_all(&d).ok();
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn forge() -> Command {
    let mut cmd = Command::cargo_bin("forge").unwrap();
    cmd.env("FORGE_NO_PLUGINS", "1");
    cmd
}

fn json_run(args: &[&str]) -> (i32, Value) {
    let out = forge().arg("--json").args(args).output().unwrap();
    let text = String::from_utf8(out.stdout).unwrap();
    let value = serde_json::from_str(&text).unwrap_or_else(|e| panic!("{e}: {text:?}"));
    (out.status.code().unwrap(), value)
}

fn s(p: &Path) -> &str {
    p.to_str().unwrap()
}

fn cube_scene(d: &Path) -> String {
    let scene = s(&d.join("a.forge")).to_string();
    assert_eq!(json_run(&["new", &scene]).0, 0);
    let args = json!({"type": "forge.cube"}).to_string();
    let (code, out) = json_run(&["cmd", &scene, "object.create", "--args", &args, "--save"]);
    assert_eq!(code, 0, "{out}");
    scene
}

fn decode(p: &Path) -> forge_image::Image {
    forge_image::decode(&std::fs::read(p).unwrap(), &forge_image::Limits::default())
        .unwrap()
        .image
}

fn lit_pixels(img: &forge_image::Image) -> usize {
    img.data
        .as_chunks::<4>()
        .0
        .iter()
        .filter(|p| p[0] > 0.01)
        .count()
}

#[test]
fn renders_a_png_of_the_cube() {
    let d = dir("png");
    let scene = cube_scene(&d);
    let out = d.join("cube.png");
    let (code, r) = json_run(&[
        "render",
        &scene,
        "--out",
        s(&out),
        "--width",
        "64",
        "--height",
        "48",
        "--samples",
        "4",
    ]);
    assert_eq!(code, 0, "{r}");
    assert_eq!(r["ok"], true);
    let bytes = std::fs::read(&out).unwrap();
    assert_eq!(&bytes[..8], b"\x89PNG\r\n\x1a\n");
    let img = decode(&out);
    assert_eq!((img.width, img.height), (64, 48));
    let lit = lit_pixels(&img);
    assert!(
        lit > 100 && lit < 64 * 48,
        "{lit} lit pixels: framed, not blank, not full"
    );
    // Deterministic: a second render is byte-identical.
    let again = d.join("again.png");
    json_run(&[
        "render",
        &scene,
        "--out",
        s(&again),
        "--width",
        "64",
        "--height",
        "48",
        "--samples",
        "4",
    ]);
    assert_eq!(bytes, std::fs::read(&again).unwrap());
}

#[test]
fn renders_an_image_sequence_with_numbered_files() {
    let d = dir("seq");
    let scene = cube_scene(&d);
    let out = d.join("shot");
    let (code, r) = json_run(&[
        "render",
        &scene,
        "--out",
        s(&out),
        "--format",
        "png",
        "--start",
        "0",
        "--end",
        "2",
        "--width",
        "16",
        "--height",
        "12",
        "--samples",
        "1",
    ]);
    assert_eq!(code, 0, "{r}");
    assert_eq!(r["frames"], json!([0, 1, 2]));
    for n in ["shot0000.png", "shot0001.png", "shot0002.png"] {
        assert!(d.join(n).exists(), "{n}");
    }
}

#[test]
fn writes_passes_separately_and_as_a_multipart_exr() {
    let d = dir("aov");
    let scene = cube_scene(&d);
    let out = d.join("p{frame:2}.exr");
    let (code, r) = json_run(&[
        "render",
        &scene,
        "--out",
        s(&out),
        "--aov",
        "depth,normal,object_id",
        "--width",
        "16",
        "--height",
        "12",
        "--samples",
        "2",
    ]);
    assert_eq!(code, 0, "{r}");
    for n in [
        "p00.exr",
        "p00_depth.exr",
        "p00_normal.exr",
        "p00_object_id.exr",
    ] {
        assert!(d.join(n).exists(), "{n}");
    }
    let m = d.join("multi.exr");
    let (code, r) = json_run(&[
        "render",
        &scene,
        "--out",
        s(&m),
        "--aov",
        "all",
        "--multipart",
        "--width",
        "16",
        "--height",
        "12",
        "--samples",
        "1",
    ]);
    assert_eq!(code, 0, "{r}");
    let layers = forge_image::decode_exr_layers(
        &std::fs::read(&m).unwrap(),
        &forge_image::Limits::default(),
    )
    .unwrap();
    assert_eq!(layers.len(), 11, "beauty and ten passes");
    let depth = layers
        .iter()
        .find(|l| l.name.as_deref() == Some("depth"))
        .unwrap();
    assert!(depth.channels[0].samples.iter().any(|v| *v > 0.0));
    let ids = layers
        .iter()
        .find(|l| l.name.as_deref() == Some("object_id"))
        .unwrap();
    assert!(
        ids.channels[0].samples.contains(&1.0),
        "first object has id 1"
    );
}

#[test]
fn usage_errors_are_exit_1_and_write_nothing() {
    let d = dir("errors");
    let scene = cube_scene(&d);
    let out = d.join("x.png");
    for args in [
        vec![
            "render",
            &scene,
            "--out",
            s(&out),
            "--frame",
            "1",
            "--start",
            "0",
            "--end",
            "2",
        ],
        vec![
            "render",
            &scene,
            "--out",
            s(&out),
            "--start",
            "3",
            "--end",
            "1",
        ],
        vec!["render", &scene, "--out", s(&out), "--aov", "bogus"],
        vec!["render", &scene, "--out", s(&out), "--width", "0"],
        vec![
            "render",
            &scene,
            "--out",
            s(&d.join("x.jpg")),
            "--depth",
            "16",
        ],
        vec!["render", &scene, "--out", s(&out), "--multipart"],
    ] {
        let (code, r) = json_run(&args);
        assert_eq!(code, 1, "{args:?}: {r}");
        assert_eq!(r["ok"], false);
    }
    assert!(!out.exists());
    let (code, _) = json_run(&["render", s(&d.join("missing.forge")), "--out", s(&out)]);
    assert_eq!(code, 2);
}

fn cmd(scene: &str, name: &str, args: Value) -> Value {
    let args = args.to_string();
    let (code, out) = json_run(&["cmd", scene, name, "--args", &args, "--save"]);
    assert_eq!(code, 0, "{name}: {out}");
    out
}

fn render_small(scene: &str, out: &Path, extra: &[&str]) -> Value {
    let mut a = vec!["render", scene, "--out", s(out), "--samples", "4"];
    a.extend_from_slice(extra);
    let (code, r) = json_run(&a);
    assert_eq!(code, 0, "{r}");
    r
}

#[test]
fn a_scene_camera_replaces_the_framing_camera() {
    let d = dir("camera");
    let scene = cube_scene(&d); // node 1: a cube of 200 units at the origin
    // fallback: the framing camera finds the cube
    let base = d.join("base.png");
    render_small(&scene, &base, &["--width", "64", "--height", "48"]);
    let lit = lit_pixels(&decode(&base));
    assert!(lit > 100);
    // a camera 600 units back looks at it: a 36 mm lens sees 600 units wide at 600 units,
    // so the 200 unit cube covers a third of the width
    let created = cmd(&scene, "object.create", json!({"type": "forge.camera"}));
    let cam = created["result"]["node"].as_u64().unwrap_or(2);
    cmd(
        &scene,
        "param.set",
        json!({"node": cam, "param": "transform.pos", "value": [0.0, 0.0, -600.0]}),
    );
    let seen = d.join("seen.png");
    render_small(&scene, &seen, &["--width", "64", "--height", "48"]);
    let img = decode(&seen);
    let lit_seen = lit_pixels(&img);
    assert!(
        lit_seen > 100 && lit_seen != lit,
        "{lit_seen} lit px through the camera, {lit} framed"
    );
    // turned around (heading 180) the camera sees nothing
    cmd(
        &scene,
        "param.set",
        json!({"node": cam, "param": "transform.rot", "value": [180.0, 0.0, 0.0]}),
    );
    let away = d.join("away.png");
    render_small(&scene, &away, &["--width", "64", "--height", "48"]);
    assert_eq!(lit_pixels(&decode(&away)), 0);
}

#[test]
fn lights_in_the_scene_replace_the_default_light() {
    let d = dir("light");
    let scene = cube_scene(&d);
    cmd(&scene, "object.create", json!({"type": "forge.light"}));
    let created = d.join("l.png");
    let r = render_small(&scene, &created, &["--width", "32", "--height", "24"]);
    assert_eq!(r["warnings"], json!([]), "{r}");
    // an omni light at the origin sits inside the cube: the outside is dark, so no head light
    // was added
    assert_eq!(lit_pixels(&decode(&created)), 0);
}

#[test]
fn stored_render_settings_are_the_defaults_and_flags_override_them() {
    let d = dir("settings");
    let scene = cube_scene(&d);
    cmd(
        &scene,
        "render.settings",
        json!({"width": 40, "height": 30, "samples": 2}),
    );
    let out = d.join("s.png");
    let (code, r) = json_run(&["render", &scene, "--out", s(&out)]);
    assert_eq!(code, 0, "{r}");
    let img = decode(&out);
    assert_eq!((img.width, img.height), (40, 30));
    let out2 = d.join("s2.png");
    let r = render_small(&scene, &out2, &["--width", "24"]);
    assert_eq!(
        (r["width"].as_u64(), r["height"].as_u64()),
        (Some(24), Some(30))
    );
    // a scene without stored settings keeps the old defaults
    let plain = cube_scene(&dir("settings-plain"));
    let out3 = d.join("s3.png");
    let (code, _) = json_run(&["render", &plain, "--out", s(&out3), "--samples", "2"]);
    assert_eq!(code, 0);
    let img = decode(&out3);
    assert_eq!((img.width, img.height), (320, 240));
}

#[test]
fn sky_and_floor_objects_light_and_fill_the_background() {
    let d = dir("sky");
    let scene = cube_scene(&d);
    // no sky: the background is black, the cube is lit by the head light
    let before = d.join("before.png");
    render_small(&scene, &before, &["--width", "32", "--height", "24"]);
    let lit_before = lit_pixels(&decode(&before));
    cmd(&scene, "object.create", json!({"type": "forge.sky"}));
    let after = d.join("after.png");
    let r = render_small(&scene, &after, &["--width", "32", "--height", "24"]);
    assert_eq!(r["warnings"], json!([]), "{r}");
    // the visible sky fills the background
    let lit_after = lit_pixels(&decode(&after));
    assert!(lit_after > lit_before + 200, "{lit_before} -> {lit_after}");
    // a floor under the cube adds geometry: it is hit by rays (object ids start after meshes)
    cmd(&scene, "object.create", json!({"type": "forge.floor"}));
    let floored = d.join("floor.png");
    let r = render_small(&scene, &floored, &["--width", "32", "--height", "24"]);
    assert_eq!(r["ok"], true);
    assert_ne!(
        std::fs::read(&after).unwrap(),
        std::fs::read(&floored).unwrap(),
        "the floor changes the picture"
    );
}

// ---- S8 look-dev acceptance (brief 146, RENDER.md R19) ------------------------------------

/// Builds the S8 scene through the CLI: a cube with a noise+fresnel shader tree, an Area
/// light and a physical camera with the thin lens on.
fn s8_scene(d: &Path) -> (String, u64) {
    let scene = s(&d.join("s8.forge")).to_string();
    assert_eq!(json_run(&["new", &scene]).0, 0);
    let created = cmd(&scene, "object.create", json!({"type": "forge.cube"}));
    let cube = created["result"]["node"].as_u64().unwrap();
    let created = cmd(&scene, "material.create", json!({}));
    let material = created["result"]["material"].as_u64().unwrap();
    // The node-graph material has no noise/fresnel nodes: use the shader node tree.
    cmd(
        &scene,
        "shader.set",
        json!({
            "type": "forge.shader.noise", "material": material, "channel": "color",
            "params": {
                "space": "object", "global_scale": 0.05,
                "color_1": [0.8, 0.1, 0.1], "color_2": [0.1, 0.3, 0.8],
            },
        }),
    );
    cmd(
        &scene,
        "shader.set",
        json!({
            "type": "forge.shader.fresnel", "material": material, "channel": "bump",
            "params": {"ior": 1.5},
        }),
    );
    cmd(
        &scene,
        "material.assign",
        json!({"material": material, "nodes": [cube]}),
    );
    let created = cmd(&scene, "object.create", json!({"type": "forge.camera"}));
    let camera = created["result"]["node"].as_u64().unwrap();
    for (param, value) in [
        ("transform.pos", json!([0.0, 0.0, -600.0])),
        ("transform.rot", json!([10.0, 15.0, 0.0])),
        ("f_stop", json!(2.8)),
        ("focus_distance", json!(600.0)),
    ] {
        cmd(
            &scene,
            "param.set",
            json!({"node": camera, "param": param, "value": value}),
        );
    }
    let created = cmd(&scene, "object.create", json!({"type": "forge.light"}));
    let light = created["result"]["node"].as_u64().unwrap();
    for (param, value) in [
        ("type", json!("area")),
        ("intensity", json!(3.0)),
        ("size_x", json!(300.0)),
        ("size_y", json!(300.0)),
        ("transform.pos", json!([0.0, 300.0, -400.0])),
    ] {
        cmd(
            &scene,
            "param.set",
            json!({"node": light, "param": param, "value": value}),
        );
    }
    (scene, camera)
}

/// PSNR of two linear RGB images, peak 1.
fn psnr(a: &forge_image::Image, b: &forge_image::Image) -> f64 {
    assert_eq!((a.width, a.height), (b.width, b.height));
    let mut sum = 0.0;
    let mut n = 0.0;
    for (p, q) in a
        .data
        .as_chunks::<4>()
        .0
        .iter()
        .zip(b.data.as_chunks::<4>().0.iter())
    {
        for k in 0..3 {
            let d = f64::from(p[k]) - f64::from(q[k]);
            sum += d * d;
            n += 1.0;
        }
    }
    if sum == 0.0 {
        f64::INFINITY
    } else {
        10.0 * (n / sum).log10()
    }
}

/// Per-channel mean value ratio in dB (20 log10(mean / reference)).
fn mean_db(a: &forge_image::Image, b: &forge_image::Image) -> [f64; 3] {
    let sum = |img: &forge_image::Image| {
        let mut s = [0.0f64; 3];
        for p in img.data.as_chunks::<4>().0 {
            for (k, v) in s.iter_mut().enumerate() {
                *v += f64::from(p[k]);
            }
        }
        s
    };
    let (sa, sb) = (sum(a), sum(b));
    std::array::from_fn(|k| 20.0 * (sa[k] / sb[k]).log10())
}

fn render_s8(scene: &str, out: &Path, extra: &[&str]) -> Value {
    let mut a = vec![
        "render",
        scene,
        "--out",
        s(out),
        "--width",
        "96",
        "--height",
        "72",
        "--samples",
        "8",
        "--max-depth",
        "3",
        "--seed",
        "7",
    ];
    a.extend_from_slice(extra);
    let (code, r) = json_run(&a);
    assert_eq!(code, 0, "{r}");
    r
}

#[test]
fn s8_lookdev_area_light_camera_dof_material_and_aovs() {
    let d = dir("s8");
    let (scene, camera) = s8_scene(&d);
    let out = d.join("s8-{pass}.png");
    let r = render_s8(&scene, &out, &["--dof", "--aov", "all"]);
    let files: Vec<String> = r["files"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f.as_str().unwrap().to_string())
        .collect();
    assert_eq!(files.len(), 11, "beauty and ten passes: {files:?}");
    let beauty = decode(&d.join("s8-beauty.png"));
    let lit = lit_pixels(&beauty);
    assert!(
        lit > 500 && lit < 96 * 72,
        "{lit} lit pixels: the camera frames the cube"
    );
    // not uniform: a shader tree with noise gives more than one colour
    let first = beauty.data.as_chunks::<4>().0[0];
    assert!(
        beauty
            .data
            .as_chunks::<4>()
            .0
            .iter()
            .any(|p| p[..3] != first[..3]),
        "the beauty is a flat colour"
    );

    // Every pass decodes, and the data passes differ from the beauty.
    for pass in [
        "depth",
        "normal",
        "albedo",
        "position",
        "object_id",
        "material_id",
        "shadow",
        "diffuse",
        "specular",
        "emission",
    ] {
        let img = decode(&d.join(format!("s8-{pass}.png")));
        assert_eq!((img.width, img.height), (96, 72), "{pass}");
        assert_eq!(img.data.len(), beauty.data.len(), "{pass}");
        assert_ne!(
            std::fs::read(d.join(format!("s8-{pass}.png"))).unwrap(),
            std::fs::read(d.join("s8-beauty.png")).unwrap(),
            "{pass} is not a copy of the beauty"
        );
    }
    let non_black = |img: &forge_image::Image| {
        img.data
            .as_chunks::<4>()
            .0
            .iter()
            .filter(|p| p[0] > 0.01 || p[1] > 0.01 || p[2] > 0.01)
            .count()
    };
    assert!(non_black(&decode(&d.join("s8-depth.png"))) > 500);
    let normal = decode(&d.join("s8-normal.png"));
    assert!(
        normal.data.iter().any(|v| v.abs() > 0.1),
        "the normal pass has no direction"
    );
    assert!(non_black(&decode(&d.join("s8-albedo.png"))) > 500);
    // Object and material ids: background 0 plus at least one geometry id.
    for pass in ["object_id", "material_id"] {
        let img = decode(&d.join(format!("s8-{pass}.png")));
        let ids: std::collections::BTreeSet<u32> = img
            .data
            .as_chunks::<4>()
            .0
            .iter()
            .map(|p| (p[0] * 255.0).round() as u32)
            .collect();
        assert!(
            ids.contains(&0) && ids.iter().any(|v| *v > 0),
            "{pass} ids {ids:?}"
        );
    }

    // Deterministic: the same scene, seed and settings render byte-identically.
    let again_dir = d.join("again");
    std::fs::create_dir_all(&again_dir).unwrap();
    render_s8(
        &scene,
        &again_dir.join("s8-{pass}.png"),
        &["--dof", "--aov", "all"],
    );
    for file in &files {
        let name = Path::new(file).file_name().unwrap();
        assert_eq!(
            std::fs::read(d.join(name)).unwrap(),
            std::fs::read(again_dir.join(name)).unwrap(),
            "{} differs on the second run",
            name.to_string_lossy()
        );
    }

    // The thin lens is live: moving the focus to the near field changes the beauty.
    cmd(
        &scene,
        "param.set",
        json!({"node": camera, "param": "focus_distance", "value": 60.0}),
    );
    let focused = d.join("focused.png");
    render_s8(&scene, &focused, &["--dof"]);
    assert_ne!(
        std::fs::read(&focused).unwrap(),
        std::fs::read(d.join("s8-beauty.png")).unwrap(),
        "the focus distance did not change the render"
    );

    // Cross-OS: compare with the checked-in reference (REGENERATE writes it).
    let reference =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/acceptance/s8-reference.png");
    if std::env::var_os("FORGE_S8_WRITE_REFERENCE").is_some() {
        std::fs::copy(d.join("s8-beauty.png"), &reference).unwrap();
    } else if reference.exists() {
        let expected = decode(&reference);
        let db = mean_db(&beauty, &expected);
        for (k, db) in db.iter().enumerate() {
            assert!(
                db.abs() <= 1.0,
                "channel {k} is {db:.3} dB from the reference (budget 1 dB)"
            );
        }
        let p = psnr(&beauty, &expected);
        assert!(p >= 30.0, "PSNR {p:.1} dB against the reference (min 30)");
    }
}

#[test]
fn renders_with_the_real_time_engine() {
    if forge_render_rt::request_default_device().is_err() {
        if std::env::var("FORGE_REQUIRE_GPU").as_deref() == Ok("1") {
            panic!("FORGE_REQUIRE_GPU=1 but no GPU adapter for --engine rt");
        }
        eprintln!("skipping --engine rt: no GPU adapter");
        return;
    }
    let d = dir("rt");
    let scene = cube_scene(&d);
    let out = d.join("rt.png");
    let (code, r) = json_run(&[
        "render",
        &scene,
        "--engine",
        "rt",
        "--out",
        s(&out),
        "--width",
        "64",
        "--height",
        "48",
    ]);
    assert_eq!(code, 0, "{r}");
    let lit = lit_pixels(&decode(&out));
    assert!(lit > 100, "only {lit} lit pixels");
    // an unknown engine is a usage error and writes nothing
    let bad = d.join("bad.png");
    let out = forge()
        .args(["render", &scene, "--engine", "nope", "--out", s(&bad)])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1));
    assert!(!bad.exists());
}
