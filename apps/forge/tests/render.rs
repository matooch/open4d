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
