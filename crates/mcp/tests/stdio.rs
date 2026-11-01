//! Drives the `forge-mcp` binary over stdio (docs/specs/RPC.md section 8.5).

use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::JoinHandle;

use forge_app::App;
use forge_rpc::{Server, ServerConfig};
use serde_json::{Value, json};

struct Mcp {
    child: Child,
    stdin: Option<ChildStdin>,
    stdout: BufReader<ChildStdout>,
    next: u64,
}

impl Mcp {
    fn spawn(args: &[&str], env: &[(&str, &str)]) -> Mcp {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_forge-mcp"));
        cmd.args(args)
            .env_remove("FORGE_RPC_PORT")
            .env_remove("FORGE_RPC_TOKEN")
            .env("FORGE_SETTINGS", "/nonexistent/forge-settings.json")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit());
        for (k, v) in env {
            cmd.env(k, v);
        }
        let mut child = cmd.spawn().unwrap();
        let stdin = child.stdin.take();
        let stdout = BufReader::new(child.stdout.take().unwrap());
        Mcp {
            child,
            stdin,
            stdout,
            next: 1,
        }
    }

    fn send_raw(&mut self, text: &str) {
        let stdin = self.stdin.as_mut().unwrap();
        stdin.write_all(text.as_bytes()).unwrap();
        stdin.write_all(b"\n").unwrap();
        stdin.flush().unwrap();
    }

    fn recv(&mut self) -> Value {
        let mut line = String::new();
        assert!(
            self.stdout.read_line(&mut line).unwrap() > 0,
            "server closed"
        );
        serde_json::from_str(&line).unwrap()
    }

    fn request(&mut self, method: &str, params: Value) -> Value {
        let id = self.next;
        self.next += 1;
        self.send_raw(
            &json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params}).to_string(),
        );
        let reply = self.recv();
        assert_eq!(reply["id"], json!(id));
        reply
    }

    fn initialize(&mut self) -> Value {
        let r = self.request(
            "initialize",
            json!({"protocolVersion": "2025-06-18", "capabilities": {},
                "clientInfo": {"name": "test", "version": "0"}}),
        );
        self.send_raw(r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#);
        r
    }

    /// Calls a tool; returns (is_error, parsed text or raw text).
    fn tool(&mut self, name: &str, arguments: Value) -> (bool, Value) {
        let r = self.request("tools/call", json!({"name": name, "arguments": arguments}));
        let result = &r["result"];
        assert!(result.is_object(), "no result: {r}");
        let text = result["content"][0]["text"].as_str().unwrap();
        let value = serde_json::from_str(text).unwrap_or_else(|_| json!(text));
        (result["isError"].as_bool().unwrap(), value)
    }

    fn finish(mut self) -> std::process::ExitStatus {
        drop(self.stdin.take());
        self.child.wait().unwrap()
    }
}

impl Drop for Mcp {
    fn drop(&mut self) {
        let _ = self.child.kill();
    }
}

fn tool_names(mcp: &mut Mcp) -> Vec<String> {
    let r = mcp.request("tools/list", json!({}));
    r["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap().to_string())
        .collect()
}

#[test]
fn headless_initialize_list_create_and_read_the_scene() {
    let mut mcp = Mcp::spawn(&["--headless"], &[]);
    let init = mcp.initialize();
    assert_eq!(init["result"]["protocolVersion"], "2025-06-18");
    assert_eq!(init["result"]["serverInfo"]["name"], "forge-mcp");
    assert!(init["result"]["capabilities"]["tools"].is_object());

    let list = mcp.request("tools/list", json!({}));
    let tools = list["result"]["tools"].as_array().unwrap();
    let names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
    for want in [
        "object_create",
        "param_set",
        "scene_get",
        "node_get",
        "eval_mesh_summary",
        "export_obj",
        "frame_set",
        "edit_undo",
    ] {
        assert!(names.contains(&want), "missing tool {want}");
    }
    let mut unique = names.clone();
    unique.sort();
    unique.dedup();
    assert_eq!(unique.len(), names.len(), "tool names are unique");
    for t in tools {
        let n = t["name"].as_str().unwrap();
        assert!(n.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') && n.len() <= 64);
        assert_eq!(t["inputSchema"]["type"], "object", "{n}");
        assert!(!t["description"].as_str().unwrap().is_empty());
    }
    let create = tools.iter().find(|t| t["name"] == "object_create").unwrap();
    assert!(create["inputSchema"]["properties"]["type"].is_object());

    let (err, made) = mcp.tool("object_create", json!({"type": "forge.cube"}));
    assert!(!err, "{made}");
    let node = made["node"].as_u64().unwrap();

    let (err, scene) = mcp.tool("scene_get", json!({}));
    assert!(!err);
    assert_eq!(scene["roots"].as_array().unwrap().len(), 1);
    assert_eq!(scene["roots"][0]["id"], node);

    let (_, n) = mcp.tool("node_get", json!({"id": node}));
    assert_eq!(n["id"], node);

    assert!(mcp.finish().success());
}

#[test]
fn query_tools_summarize_move_the_frame_and_export() {
    let mut mcp = Mcp::spawn(&["--headless"], &[]);
    mcp.initialize();
    let (_, made) = mcp.tool("object_create", json!({"type": "forge.cube"}));
    let node = made["node"].as_u64().unwrap();

    let (err, sum) = mcp.tool("eval_mesh_summary", json!({"node": node}));
    assert!(!err);
    assert_eq!(sum["points"], 8);
    assert_eq!(sum["polys"], 6);
    assert!(sum.get("polys").is_some() && sum.get("positions").is_none());
    let min = sum["bbox"]["min"].as_array().unwrap();
    let max = sum["bbox"]["max"].as_array().unwrap();
    for a in 0..3 {
        let (lo, hi) = (min[a].as_f64().unwrap(), max[a].as_f64().unwrap());
        assert!(
            hi > lo && (lo + hi).abs() < 1e-9,
            "centered box on axis {a}"
        );
    }

    let (err, set) = mcp.tool("frame_set", json!({"frame": 3}));
    assert!(!err, "{set}");
    assert_eq!(set["frame"], 3);
    let (_, scene) = mcp.tool("scene_get", json!({}));
    assert_eq!(scene["frame"], 3);

    let dir = std::env::temp_dir().join(format!("forge-mcp-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("cube.obj");
    let (err, out) = mcp.tool("export_obj", json!({"path": path.to_str().unwrap()}));
    assert!(!err, "{out}");
    let obj = std::fs::read_to_string(&path).unwrap();
    assert!(obj.lines().filter(|l| l.starts_with("v ")).count() == 8);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn errors_are_results_or_protocol_errors_and_the_server_keeps_serving() {
    let mut mcp = Mcp::spawn(&["--headless"], &[]);
    mcp.initialize();

    let (err, msg) = mcp.tool("object_create", json!({"type": "no.such.type"}));
    assert!(err, "a refused command is an isError result");
    assert!(msg.as_str().is_some_and(|m| !m.is_empty()));
    let (err, _) = mcp.tool("node_get", json!({"id": 99999}));
    assert!(err);
    let (err, _) = mcp.tool("object_create", json!({}));
    assert!(err, "missing required argument");

    let unknown = mcp.request(
        "tools/call",
        json!({"name": "no_such_tool", "arguments": {}}),
    );
    assert_eq!(unknown["error"]["code"], -32602);
    let bad_args = mcp.request("tools/call", json!({"name": "scene_get", "arguments": 5}));
    assert_eq!(bad_args["error"]["code"], -32602);
    let no_name = mcp.request("tools/call", json!({}));
    assert_eq!(no_name["error"]["code"], -32602);
    let unknown = mcp.request("resources/list", json!({}));
    assert_eq!(unknown["error"]["code"], -32601);
    assert_eq!(mcp.request("ping", json!({}))["result"], json!({}));

    mcp.send_raw("this is not json");
    let parse = mcp.recv();
    assert_eq!(parse["error"]["code"], -32700);
    assert_eq!(parse["id"], Value::Null);
    mcp.send_raw("[1,2]");
    assert_eq!(mcp.recv()["error"]["code"], -32600);
    mcp.send_raw(r#"{"jsonrpc":"2.0","id":7,"method":5}"#);
    assert_eq!(mcp.recv()["error"]["code"], -32600);

    // A notification gets no reply: the next reply belongs to the next request.
    mcp.send_raw(r#"{"jsonrpc":"2.0","method":"notifications/cancelled","params":{}}"#);
    let (err, _) = mcp.tool("scene_get", json!({}));
    assert!(!err);
}

#[test]
fn an_unknown_protocol_version_is_answered_with_ours() {
    let mut mcp = Mcp::spawn(&["--headless"], &[]);
    let r = mcp.request("initialize", json!({"protocolVersion": "1999-01-01"}));
    assert_eq!(r["result"]["protocolVersion"], "2025-06-18");
    let r = mcp.request("initialize", json!({"protocolVersion": "2024-11-05"}));
    assert_eq!(r["result"]["protocolVersion"], "2024-11-05");
}

#[test]
fn headless_can_start_from_a_scene_file() {
    let dir = std::env::temp_dir().join(format!("forge-mcp-scene-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("scene.forge");
    let path = path.to_str().unwrap();
    {
        let mut app = App::new_default();
        app.run("object.create", json!({"type": "forge.sphere"}))
            .unwrap();
        app.save(path).unwrap();
    }
    let mut mcp = Mcp::spawn(&["--headless", path], &[]);
    mcp.initialize();
    let (_, scene) = mcp.tool("scene_get", json!({}));
    assert_eq!(scene["roots"].as_array().unwrap().len(), 1);
    assert_eq!(scene["roots"][0]["type"], "forge.sphere");
    drop(mcp);

    let bad = Command::new(env!("CARGO_BIN_EXE_forge-mcp"))
        .args(["--headless", "/nonexistent/x.forge"])
        .output()
        .unwrap();
    assert_eq!(bad.status.code(), Some(1));
    assert!(bad.stdout.is_empty(), "stdout stays protocol-only");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn bad_arguments_and_help() {
    for args in [
        &["--bogus"][..],
        &["scene.forge"],
        &["--port", "x"],
        &["--headless", "--port", "1"],
    ] {
        let out = Command::new(env!("CARGO_BIN_EXE_forge-mcp"))
            .args(args)
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(2), "{args:?}");
    }
    let out = Command::new(env!("CARGO_BIN_EXE_forge-mcp"))
        .arg("--help")
        .output()
        .unwrap();
    assert!(out.status.success());
    assert!(String::from_utf8_lossy(&out.stdout).contains("--headless"));
}

/// An RPC server with an in-process `App` on a pump thread (no window).
struct Editor {
    server: Server,
    stop: Arc<AtomicBool>,
    pump: Option<JoinHandle<()>>,
}

impl Editor {
    fn start(token: &str) -> Editor {
        let (server, inbox) = Server::start(ServerConfig {
            port: 0,
            token: Some(token.to_string()),
            insecure_local: false,
        })
        .unwrap();
        let stop = Arc::new(AtomicBool::new(false));
        let flag = stop.clone();
        let pump = std::thread::spawn(move || {
            let mut app = App::new_default();
            inbox.run_until(&mut app, &flag);
        });
        Editor {
            server,
            stop,
            pump: Some(pump),
        }
    }
}

impl Drop for Editor {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        self.server.shutdown();
        if let Some(p) = self.pump.take() {
            let _ = p.join();
        }
    }
}

#[test]
fn editor_mode_forwards_over_rpc_with_the_token() {
    let editor = Editor::start("mcp-test-token");
    let port = editor.server.port().to_string();
    let mut mcp = Mcp::spawn(&["--port", &port], &[("FORGE_RPC_TOKEN", "mcp-test-token")]);
    mcp.initialize();
    assert!(tool_names(&mut mcp).contains(&"object_create".to_string()));

    let (err, made) = mcp.tool("object_create", json!({"type": "forge.cube"}));
    assert!(!err, "{made}");
    let (_, scene) = mcp.tool("scene_get", json!({}));
    assert_eq!(scene["roots"].as_array().unwrap().len(), 1);
    let (_, sum) = mcp.tool("eval_mesh_summary", json!({"node": made["node"]}));
    assert_eq!(sum["points"], 8);

    // The port also comes from the environment.
    let mut env = Mcp::spawn(
        &[],
        &[
            ("FORGE_RPC_PORT", &port),
            ("FORGE_RPC_TOKEN", "mcp-test-token"),
        ],
    );
    env.initialize();
    let (_, scene) = env.tool("scene_get", json!({}));
    assert_eq!(
        scene["roots"].as_array().unwrap().len(),
        1,
        "same editor, same document"
    );
}

#[test]
fn editor_mode_reports_a_wrong_token_and_a_missing_editor_as_tool_errors() {
    let editor = Editor::start("right-token");
    let port = editor.server.port().to_string();
    let mut mcp = Mcp::spawn(&["--port", &port], &[("FORGE_RPC_TOKEN", "wrong-token")]);
    mcp.initialize();
    let (err, msg) = mcp.tool("scene_get", json!({}));
    assert!(err);
    assert!(
        !msg.to_string().contains("wrong-token"),
        "token is never echoed"
    );
    let list = mcp.request("tools/list", json!({}));
    assert!(list["error"].is_object(), "listing needs the editor");
    drop(mcp);
    drop(editor);

    // Nothing listens any more on that port.
    let mut gone = Mcp::spawn(&["--port", &port], &[("FORGE_RPC_TOKEN", "x")]);
    gone.initialize();
    let (err, msg) = gone.tool("scene_get", json!({}));
    assert!(err);
    assert!(msg.as_str().unwrap().contains("cannot connect"));
    assert_eq!(gone.request("ping", json!({}))["result"], json!({}));
}
