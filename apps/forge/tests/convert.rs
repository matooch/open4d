//! `forge convert` and `forge export --option` (docs/specs/CLI.md, IO.md §3.6).

use std::path::{Path, PathBuf};

use assert_cmd::Command;
use serde_json::{Value, json};

fn dir(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("forge-convert-test-{}-{name}", std::process::id()));
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

/// A scene with one object of the given type at `path`.
fn scene(path: &Path, ty: &str) {
    assert_eq!(json_run(&["new", s(path), "--force"]).0, 0);
    let args = json!({"type": ty}).to_string();
    let (code, out) = json_run(&["cmd", s(path), "object.create", "--args", &args, "--save"]);
    assert_eq!(code, 0, "{out}");
}

fn names(d: &Path) -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(d)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    v.sort();
    v
}

#[test]
fn folder_of_scenes_becomes_a_folder_of_obj_files() {
    let d = dir("folder");
    let input = d.join("in");
    std::fs::create_dir_all(&input).unwrap();
    scene(&input.join("a.forge"), "forge.cube");
    scene(&input.join("b.FORGE"), "forge.sphere");
    std::fs::write(input.join("notes.txt"), "x").unwrap();
    let out = d.join("out");
    let (code, r) = json_run(&["convert", s(&input), "--out", s(&out)]);
    assert_eq!(code, 0, "{r}");
    assert_eq!(r["converted"], 2);
    assert_eq!(r["skipped"].as_array().unwrap().len(), 1);
    assert_eq!(names(&out), ["a.obj", "b.obj"]);
    // equal to `export` of the same scene
    let single = d.join("single.obj");
    assert_eq!(
        json_run(&["export", s(&input.join("a.forge")), "--out", s(&single)]).0,
        0
    );
    assert_eq!(
        std::fs::read(&single).unwrap(),
        std::fs::read(out.join("a.obj")).unwrap()
    );
    // a second run refuses to overwrite, --force allows it
    let (code, _) = json_run(&["convert", s(&input), "--out", s(&out)]);
    assert_eq!(code, 1);
    let (code, _) = json_run(&["convert", s(&input), "--out", s(&out), "--force"]);
    assert_eq!(code, 0);
}

#[test]
fn recursion_mirrors_folders_and_default_is_flat() {
    let d = dir("recursive");
    let input = d.join("in");
    std::fs::create_dir_all(input.join("sub/deeper")).unwrap();
    scene(&input.join("top.forge"), "forge.cube");
    scene(&input.join("sub/mid.forge"), "forge.cube");
    scene(&input.join("sub/deeper/low.forge"), "forge.cube");
    let flat = d.join("flat");
    assert_eq!(json_run(&["convert", s(&input), "--out", s(&flat)]).0, 0);
    assert_eq!(names(&flat), ["top.obj"]);
    let deep = d.join("deep");
    let (code, r) = json_run(&["convert", s(&input), "--out", s(&deep), "--recursive"]);
    assert_eq!(code, 0, "{r}");
    assert_eq!(r["converted"], 3);
    assert!(deep.join("sub/deeper/low.obj").is_file());
    assert!(deep.join("sub/mid.obj").is_file());
    // path order is reproducible
    let files: Vec<&str> = r["files"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["input"].as_str().unwrap())
        .collect();
    let mut sorted = files.clone();
    sorted.sort();
    assert_eq!(files, sorted);
}

#[test]
fn options_apply_and_bad_ones_fail_before_writing() {
    let d = dir("options");
    scene(&d.join("a.forge"), "forge.cube");
    let out = d.join("out");
    let (code, _) = json_run(&[
        "convert",
        s(&d.join("a.forge")),
        "--out",
        s(&out),
        "--option",
        "normals=false",
        "--option",
        "weld=false",
    ]);
    assert_eq!(code, 0);
    let text = std::fs::read_to_string(out.join("a.obj")).unwrap();
    assert!(!text.contains("vn "));
    for bad in [
        ["--option", "nope=1"],
        ["--option", "weld=maybe"],
        ["--option", "weld"],
    ] {
        let out2 = d.join("out2");
        let (code, _) = json_run(&[
            "convert",
            s(&d.join("a.forge")),
            "--out",
            s(&out2),
            bad[0],
            bad[1],
        ]);
        assert_eq!(code, 1, "{bad:?}");
        assert!(!out2.exists(), "{bad:?}");
    }
    let (code, r) = json_run(&[
        "convert",
        s(&d.join("a.forge")),
        "--out",
        s(&out),
        "--format",
        "gltf",
    ]);
    assert_eq!(code, 1);
    assert!(
        r["error"]["message"]
            .as_str()
            .unwrap()
            .contains("supported: obj")
    );
}

#[test]
fn one_bad_scene_fails_the_run_but_the_others_are_written() {
    let d = dir("partial");
    let input = d.join("in");
    std::fs::create_dir_all(&input).unwrap();
    scene(&input.join("a.forge"), "forge.cube");
    std::fs::write(input.join("b.forge"), "not a scene").unwrap();
    // an empty scene has nothing to export
    assert_eq!(json_run(&["new", s(&input.join("c.forge"))]).0, 0);
    let out = d.join("out");
    let (code, r) = json_run(&["convert", s(&input), "--out", s(&out)]);
    assert_eq!(code, 1, "{r}");
    assert!(r["error"]["message"].as_str().unwrap().contains("2 of 3"));
    assert_eq!(names(&out), ["a.obj"]);
}

#[test]
fn nothing_convertible_and_missing_input_are_user_errors() {
    let d = dir("none");
    std::fs::write(d.join("x.txt"), "x").unwrap();
    let (code, r) = json_run(&["convert", s(&d), "--out", s(&d.join("out"))]);
    assert_eq!(code, 1);
    assert!(
        r["error"]["message"]
            .as_str()
            .unwrap()
            .contains("no convertible files")
    );
    let (code, _) = json_run(&["convert", s(&d.join("missing")), "--out", s(&d.join("out"))]);
    assert_eq!(code, 1);
}

#[test]
fn obj_files_import_into_native_scenes() {
    let d = dir("toforge");
    let input = d.join("in");
    std::fs::create_dir_all(&input).unwrap();
    std::fs::write(
        input.join("t.obj"),
        "o Tri\nv 0 0 1\nv 1 0 1\nv 0 1 1\nf 1 2 3\n",
    )
    .unwrap();
    scene(&input.join("skip.forge"), "forge.cube");
    let out = d.join("out");
    let (code, r) = json_run(&[
        "convert",
        s(&input),
        "--out",
        s(&out),
        "--format",
        "forge",
        "--option",
        "flip_z=true",
    ]);
    assert_eq!(code, 0, "{r}");
    assert_eq!(r["converted"], 1);
    assert_eq!(r["files"][0]["nodes"], 1);
    assert_eq!(names(&out), ["t.forge"]);
    // the saved scene round-trips back to OBJ with z mirrored
    let back = d.join("back.obj");
    assert_eq!(
        json_run(&["export", s(&out.join("t.forge")), "--out", s(&back)]).0,
        0
    );
    let text = std::fs::read_to_string(&back).unwrap();
    assert!(text.contains("v 0 0 -1"), "{text}");
}

#[test]
fn export_takes_options_and_rejects_unknown_ones() {
    let d = dir("export-opt");
    scene(&d.join("a.forge"), "forge.cube");
    let obj = d.join("a.obj");
    let (code, _) = json_run(&[
        "export",
        s(&d.join("a.forge")),
        "--out",
        s(&obj),
        "--option",
        "normals=false",
    ]);
    assert_eq!(code, 0);
    assert!(!std::fs::read_to_string(&obj).unwrap().contains("vn "));
    let (code, _) = json_run(&[
        "export",
        s(&d.join("a.forge")),
        "--out",
        s(&obj),
        "--option",
        "zzz=1",
    ]);
    assert_eq!(code, 1);
}
