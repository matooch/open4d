//! The `forge` CLI (docs/specs/CLI.md §4).

use std::path::{Path, PathBuf};

use assert_cmd::Command;
use forge_parity_harness::{Tolerance, compare_meshes, parse_obj};
use serde_json::{Value, json};

fn dir(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("forge-cli-test-{}-{name}", std::process::id()));
    std::fs::remove_dir_all(&d).ok();
    std::fs::create_dir_all(&d).unwrap();
    d
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

fn golden_cube() -> forge_parity_harness::Mesh {
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden/cube/defaults/mesh.obj");
    parse_obj(&std::fs::read_to_string(path).unwrap()).unwrap()
}

fn assert_cube(path: &Path) {
    let got = parse_obj(&std::fs::read_to_string(path).unwrap()).unwrap();
    let report = compare_meshes(&golden_cube(), &got, &Tolerance::default());
    assert!(report.passed(), "{:?}", report.failures);
}

fn cube_scene(d: &Path) -> String {
    let scene = s(&d.join("a.forge")).to_string();
    assert_eq!(json_run(&["new", &scene]).0, 0);
    let args = json!({"type": "forge.cube"}).to_string();
    let (code, out) = json_run(&["cmd", &scene, "object.create", "--args", &args, "--save"]);
    assert_eq!(code, 0, "{out}");
    scene
}

#[test]
fn new_cmd_eval_export_matches_the_cube_golden() {
    let d = dir("flow");
    let scene = cube_scene(&d);
    let eval_obj = d.join("eval.obj");
    let (code, out) = json_run(&["eval", &scene, "--frame", "0", "--export", s(&eval_obj)]);
    assert_eq!(code, 0, "{out}");
    assert_eq!(out["ok"], true);
    assert_eq!(out["nodes"][0]["points"], 8);
    assert_eq!(out["nodes"][0]["polys"], 6);
    assert_eq!(out["export"]["objects"], 1);
    assert_cube(&eval_obj);

    let export_obj = d.join("export.obj");
    let (code, out) = json_run(&["export", &scene, "--format", "obj", "--out", s(&export_obj)]);
    assert_eq!(code, 0, "{out}");
    assert_eq!(out["objects"], 1);
    assert_cube(&export_obj);
    assert_eq!(
        std::fs::read_to_string(&eval_obj).unwrap(),
        std::fs::read_to_string(&export_obj).unwrap()
    );
}

#[test]
fn info_validate_and_commands() {
    let d = dir("info");
    let scene = cube_scene(&d);
    let (code, out) = json_run(&["info", &scene]);
    assert_eq!(code, 0);
    let cube = &out["roots"][0];
    assert_eq!(cube["type"], "forge.cube");
    assert_eq!(cube["name"], "Cube");
    assert!(cube["params"].get("size_x").is_some(), "{cube}");
    let (code, out) = json_run(&["validate", &scene]);
    assert_eq!((code, out["ok"].clone()), (0, json!(true)), "{out}");

    let (code, out) = json_run(&["commands"]);
    assert_eq!(code, 0);
    let ids: Vec<&str> = out["commands"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["id"].as_str().unwrap())
        .collect();
    let registry = forge_app::default_command_registry();
    assert_eq!(ids.len(), registry.len());
    assert!(ids.contains(&"object.create") && ids.contains(&"param.set"));
    assert_eq!(out["commands"][0]["args_schema"]["type"], "object");

    // Human output works too.
    let text = forge().args(["info", &scene]).output().unwrap();
    assert!(
        String::from_utf8(text.stdout)
            .unwrap()
            .contains("forge.cube")
    );
}

#[test]
fn animated_cube_exports_the_evaluated_frame() {
    let d = dir("anim");
    let scene = cube_scene(&d);
    // Keys cannot be set by a command with explicit times, so write them with the library.
    let mut app = forge_app::App::new_default();
    app.open(&scene).unwrap();
    let cube = app.doc.scene().roots()[0];
    app.doc
        .transact("Keys", |tx| {
            for (time, x) in [(0.0, 0.0), (1.0, 300.0)] {
                let key = forge_core::Key {
                    interp: forge_core::Interp::Linear,
                    ..forge_core::Key::new(time, x)
                };
                tx.set_key(cube, "transform.pos.x", key)?;
            }
            Ok(())
        })
        .unwrap();
    app.save(&scene).unwrap();

    let min_x = |frame: &str| {
        let out = d.join(format!("f{frame}.obj"));
        let (code, v) = json_run(&[
            "export",
            &scene,
            "--format",
            "obj",
            "--out",
            s(&out),
            "--frame",
            frame,
        ]);
        assert_eq!(code, 0, "{v}");
        let mesh = parse_obj(&std::fs::read_to_string(out).unwrap()).unwrap();
        mesh.positions.iter().map(|p| p[0]).fold(f64::MAX, f64::min)
    };
    assert!((min_x("0") + 100.0).abs() < 1e-4);
    assert!((min_x("15") - 50.0).abs() < 1e-4);

    let (_, info) = json_run(&["info", &scene]);
    assert_eq!(info["roots"][0]["animated"][0], "transform.pos.x");
}

#[test]
fn cmd_without_save_leaves_the_file_alone() {
    let d = dir("dry");
    let scene = cube_scene(&d);
    let before = std::fs::read_to_string(&scene).unwrap();
    let (code, out) = json_run(&[
        "cmd",
        &scene,
        "object.create",
        "--args",
        r#"{"type":"forge.sphere"}"#,
    ]);
    assert_eq!(code, 0, "{out}");
    assert!(out["result"]["node"].is_u64());
    assert_eq!(std::fs::read_to_string(&scene).unwrap(), before);
}

#[test]
fn exit_codes() {
    let d = dir("codes");
    let scene = cube_scene(&d);
    // 1: unknown command, bad args, unknown node, bad usage, stubs
    assert_eq!(json_run(&["cmd", &scene, "no.such_command"]).0, 1);
    let (code, out) = json_run(&["cmd", &scene, "object.create", "--args", r#"{"bogus":1}"#]);
    assert_eq!(
        (code, out["ok"].clone(), out["error"]["code"].clone()),
        (1, json!(false), json!(1))
    );
    assert_eq!(
        json_run(&["cmd", &scene, "object.create", "--args", "not json"]).0,
        1
    );
    assert_eq!(json_run(&["eval", &scene, "--node", "9999"]).0, 1);
    assert_eq!(
        forge().arg("bogus").output().unwrap().status.code(),
        Some(1)
    );
    for stub in [vec!["run", "x.rhai"], vec!["render"]] {
        assert_eq!(json_run(&stub).0, 1, "{stub:?}");
    }
    // 2: scene cannot be loaded
    assert_eq!(json_run(&["info", s(&d.join("missing.forge"))]).0, 2);
    let bad = d.join("bad.forge");
    std::fs::write(&bad, "not a scene").unwrap();
    assert_eq!(json_run(&["validate", s(&bad)]).0, 2);
    // new refuses to overwrite without --force
    assert_eq!(json_run(&["new", &scene]).0, 1);
    assert_eq!(json_run(&["new", &scene, "--force"]).0, 0);
    // help is ok
    forge().arg("--help").assert().success();
}

/// `forge --json commands | head -c N`: a reader that closes the pipe early is not an error
/// (no panic, no backtrace, exit code 0). Run once with the reader gone before the tool
/// writes (the case that used to panic) and once after it read a few bytes.
#[test]
fn closed_stdout_pipe_is_a_quiet_success() {
    use std::io::Read;
    use std::process::Stdio;

    for read_first in [false, true] {
        let bin = forge().get_program().to_owned();
        let mut child = std::process::Command::new(bin)
            .args(["--json", "commands"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let mut stdout = child.stdout.take().unwrap();
        if read_first {
            let mut head = [0u8; 16];
            stdout.read_exact(&mut head).unwrap();
        }
        drop(stdout);
        let out = child.wait_with_output().unwrap();
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(!stderr.contains("panicked"), "{stderr}");
        assert_eq!(out.status.code(), Some(0), "{stderr}");
    }
}

/// `forge rpc` against a server started in this process (port 0, token in the environment).
#[test]
fn rpc_calls_a_running_server() {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};

    let (mut server, inbox) = forge_rpc::Server::start(forge_rpc::ServerConfig {
        port: 0,
        token: Some("cli-token".into()),
        insecure_local: false,
    })
    .unwrap();
    let port = server.port().to_string();
    let stop = Arc::new(AtomicBool::new(false));
    let flag = stop.clone();
    let pump = std::thread::spawn(move || {
        let mut app = forge_app::App::new_default();
        inbox.run_until(&mut app, &flag);
    });
    let rpc = |extra: &[&str], token: Option<&str>| {
        let mut cmd = forge();
        cmd.env("FORGE_SETTINGS", "/nonexistent/forge-settings.json")
            .env_remove("FORGE_RPC_TOKEN")
            .env("FORGE_RPC_PORT", &port);
        if let Some(t) = token {
            cmd.env("FORGE_RPC_TOKEN", t);
        }
        let out = cmd.arg("--json").arg("rpc").args(extra).output().unwrap();
        let text = String::from_utf8(out.stdout).unwrap();
        (
            out.status.code().unwrap(),
            serde_json::from_str::<Value>(&text).unwrap(),
        )
    };

    let (code, out) = rpc(&["app.version"], Some("cli-token"));
    assert_eq!(code, 0, "{out}");
    assert_eq!(out["method"], "app.version");
    assert_eq!(out["result"]["name"], "forge3d");

    let params = json!({"id": "object.create", "args": {"type": "forge.cube"}}).to_string();
    let (code, out) = rpc(&["command.run", "--params", &params], Some("cli-token"));
    assert_eq!(code, 0, "{out}");
    let node = out["result"]["result"]["node"].as_u64().unwrap();
    let (code, out) = rpc(
        &["eval.mesh", "--params", &json!({"node": node}).to_string()],
        Some("cli-token"),
    );
    assert_eq!(code, 0, "{out}");
    assert_eq!(out["result"]["points"].as_array().unwrap().len(), 8);

    // --port and --token flags work without the environment.
    let out = forge()
        .env("FORGE_SETTINGS", "/nonexistent/forge-settings.json")
        .env_remove("FORGE_RPC_TOKEN")
        .env_remove("FORGE_RPC_PORT")
        .args([
            "--json",
            "rpc",
            "timeline.get",
            "--port",
            &port,
            "--token",
            "cli-token",
        ])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(0));

    // A wrong token and an unknown method are user errors (exit 1) carrying the message.
    let (code, out) = rpc(&["app.version"], Some("wrong"));
    assert_eq!((code, out["ok"].clone()), (1, json!(false)));
    assert!(
        out["error"]["message"]
            .as_str()
            .unwrap()
            .contains("unauthorized")
    );
    let (code, out) = rpc(&["no.such"], Some("cli-token"));
    assert_eq!(code, 1);
    assert!(
        out["error"]["message"]
            .as_str()
            .unwrap()
            .contains("unknown method")
    );
    let (code, _) = rpc(&["app.version", "--params", "{oops"], Some("cli-token"));
    assert_eq!(code, 1);

    stop.store(true, Ordering::SeqCst);
    server.shutdown();
    pump.join().unwrap();
}

#[test]
fn rpc_without_an_editor_is_a_user_error() {
    // A port that was just free: nothing listens on it.
    let port = {
        let l = std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap();
        l.local_addr().unwrap().port().to_string()
    };
    let out = forge()
        .env("FORGE_SETTINGS", "/nonexistent/forge-settings.json")
        .args(["--json", "rpc", "app.version", "--port", &port])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1));
    let body: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert!(
        body["error"]["message"]
            .as_str()
            .unwrap()
            .contains("cannot connect")
    );
}

fn example(name: &str) -> String {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples/scripts")
        .join(name)
        .to_string_lossy()
        .into_owned()
}

#[test]
fn run_builds_a_scene_and_saves_only_on_request() {
    let d = dir("run");
    let out = d.join("out.forge");
    let (code, v) = json_run(&["run", &example("cube_grid.rhai"), "--save", s(&out)]);
    assert_eq!(code, 0, "{v}");
    assert_eq!(v["ok"], true);
    assert_eq!(v["output"], json!(["built 27 cubes"]));
    assert_eq!(v["commands"], 110);
    assert_eq!(v["saved"], s(&out));
    let (_, info) = json_run(&["info", s(&out)]);
    let grid = &info["roots"][0];
    assert_eq!(grid["name"], "Grid");
    assert_eq!(grid["children"].as_array().unwrap().len(), 27);

    // a plain run is a dry run
    let (code, v) = json_run(&["run", &example("animated_sphere.rhai")]);
    assert_eq!(code, 0, "{v}");
    assert_eq!(v["saved"], Value::Null);
    assert!(v["value"].is_u64());
}

#[test]
fn run_on_an_existing_scene_with_text_output() {
    let d = dir("run-scene");
    let scene = cube_scene(&d);
    let script = d.join("s.rhai");
    std::fs::write(
        &script,
        r#"
        for id in scene().nodes() { print(`${id}: ${scene().node(id).name}`); }
        set_param(scene().nodes()[0], "size_x", 10.0);
        "#,
    )
    .unwrap();
    let before = std::fs::read(&scene).unwrap();
    let out = forge()
        .args(["run", s(&script), "--scene", &scene])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(0));
    assert_eq!(String::from_utf8(out.stdout).unwrap(), "1: Cube\n");
    assert_eq!(std::fs::read(&scene).unwrap(), before, "dry run");
    // --save may overwrite the input
    assert_eq!(
        json_run(&["run", s(&script), "--scene", &scene, "--save", &scene]).0,
        0
    );
    let (_, info) = json_run(&["info", &scene]);
    assert_eq!(info["roots"][0]["params"]["size_x"], 10.0);
}

#[test]
fn run_failures_are_exit_1_and_save_nothing() {
    let d = dir("run-fail");
    let out = d.join("out.forge");
    for (name, src) in [
        ("syntax.rhai", "create(\"forge.cube\");\nlet = ;"),
        (
            "runtime.rhai",
            "create(\"forge.cube\");\nset_param(99, \"size_x\", 1.0);",
        ),
        ("limit.rhai", "create(\"forge.cube\"); loop { }"),
    ] {
        let script = d.join(name);
        std::fs::write(&script, src).unwrap();
        let res = forge()
            .args(["run", s(&script), "--save", s(&out)])
            .output()
            .unwrap();
        assert_eq!(res.status.code(), Some(1), "{name}");
        let err = String::from_utf8(res.stderr).unwrap();
        assert!(err.contains(name) && err.contains("line "), "{name}: {err}");
        assert!(!out.exists(), "{name} saved");
    }
    let err = forge()
        .args(["run", s(&d.join("limit.rhai"))])
        .output()
        .unwrap();
    assert!(
        String::from_utf8(err.stderr)
            .unwrap()
            .contains("operation limit")
    );
    // JSON error body
    let (code, v) = json_run(&["run", s(&d.join("runtime.rhai"))]);
    assert_eq!(code, 1);
    assert_eq!(v["ok"], false);
    // missing script: 1; missing scene: 2
    assert_eq!(json_run(&["run", s(&d.join("none.rhai"))]).0, 1);
    assert_eq!(
        json_run(&[
            "run",
            &example("cube_grid.rhai"),
            "--scene",
            s(&d.join("none.forge"))
        ])
        .0,
        2
    );
}

#[test]
fn run_prints_output_before_a_failure_on_stderr() {
    let d = dir("run-partial");
    let script = d.join("p.rhai");
    std::fs::write(&script, "print(\"before\"); run(\"no.such_command\");").unwrap();
    let res = forge().args(["run", s(&script)]).output().unwrap();
    assert_eq!(res.status.code(), Some(1));
    let err = String::from_utf8(res.stderr).unwrap();
    assert!(err.starts_with("before\n"), "{err}");
    assert!(res.stdout.is_empty());
}
