//! JSON-RPC server and client (docs/specs/RPC.md §7). Every server binds port 0.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::JoinHandle;
use std::time::Duration;

use forge_app::App;
use forge_rpc::{
    Client, ClientError, MAX_CONNECTIONS, MAX_MESSAGE_BYTES, RpcSettings, Server, ServerConfig,
    StartError, code, ct_eq,
};
use serde_json::{Value, json};

const TOKEN: &str = "s3cret-token";

/// A server with an in-process `App` (no window) served on its own thread.
struct Fixture {
    server: Server,
    stop: Arc<AtomicBool>,
    pump: Option<JoinHandle<()>>,
}

impl Fixture {
    fn start(token: Option<&str>, insecure_local: bool) -> Fixture {
        let (server, inbox) = Server::start(ServerConfig {
            port: 0,
            token: token.map(String::from),
            insecure_local,
        })
        .unwrap();
        let stop = Arc::new(AtomicBool::new(false));
        let flag = stop.clone();
        let pump = std::thread::spawn(move || {
            let mut app = App::new_default();
            inbox.run_until(&mut app, &flag);
        });
        Fixture {
            server,
            stop,
            pump: Some(pump),
        }
    }

    fn token() -> Fixture {
        Fixture::start(Some(TOKEN), false)
    }

    fn client(&self) -> Client {
        Client::connect(self.server.port(), Some(TOKEN)).unwrap()
    }

    fn raw(&self) -> Raw {
        let stream = TcpStream::connect(("127.0.0.1", self.server.port())).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(10)))
            .unwrap();
        Raw {
            reader: BufReader::new(stream.try_clone().unwrap()),
            writer: stream,
        }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        self.server.shutdown();
        if let Some(p) = self.pump.take() {
            let _ = p.join();
        }
    }
}

/// A bare socket for protocol-level tests.
struct Raw {
    reader: BufReader<TcpStream>,
    writer: TcpStream,
}

impl Raw {
    fn send(&mut self, text: &str) {
        self.writer.write_all(text.as_bytes()).unwrap();
        self.writer.write_all(b"\n").unwrap();
    }

    fn recv(&mut self) -> Value {
        let mut line = String::new();
        self.reader.read_line(&mut line).unwrap();
        serde_json::from_str(&line).unwrap_or_else(|e| panic!("{e}: {line:?}"))
    }

    fn ask(&mut self, request: Value) -> Value {
        self.send(&request.to_string());
        self.recv()
    }

    /// True once the server has closed the connection.
    fn closed(&mut self) -> bool {
        let mut rest = Vec::new();
        self.reader.read_to_end(&mut rest).is_ok()
    }
}

fn request(id: u64, method: &str, params: Value) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params, "token": TOKEN})
}

fn error_code(e: &ClientError) -> i64 {
    match e {
        ClientError::Rpc { code, .. } => *code,
        other => panic!("not an rpc error: {other}"),
    }
}

#[test]
fn create_a_cube_set_a_param_read_the_scene_and_evaluate_the_mesh() {
    let fx = Fixture::token();
    let mut c = fx.client();

    let version = c.version().unwrap();
    assert_eq!(version["name"], "forge3d");
    assert_eq!(version["protocol"], 1);

    let made = c
        .command_run("object.create", json!({"type": "forge.cube"}))
        .unwrap();
    let node = made["node"].as_u64().unwrap();

    let mesh = c.eval_mesh(node, None).unwrap();
    assert_eq!(mesh.points.len(), 8);
    assert_eq!(mesh.polys.len(), 6);
    let width = |m: &forge_rpc::MeshData| {
        let xs: Vec<f64> = m.points.iter().map(|p| p[0]).collect();
        xs.iter().cloned().fold(f64::MIN, f64::max) - xs.iter().cloned().fold(f64::MAX, f64::min)
    };
    assert!((width(&mesh) - 200.0).abs() < 1e-9);

    c.command_run(
        "param.set",
        json!({"node": node, "param": "size_x", "value": 400.0}),
    )
    .unwrap();

    let scene = c.scene_get().unwrap();
    let roots = scene["roots"].as_array().unwrap();
    assert_eq!(roots.len(), 1);
    assert_eq!(roots[0]["id"], node);
    assert_eq!(roots[0]["type"], "forge.cube");
    assert_eq!(roots[0]["params"]["size_x"], 400.0);
    assert_eq!(scene["selected"], json!([node]));
    assert_eq!(c.node_get(node).unwrap()["params"]["size_x"], 400.0);

    let wide = c.eval_mesh(node, Some(0)).unwrap();
    assert_eq!(wide.points.len(), 8);
    assert!((width(&wide) - 400.0).abs() < 1e-9);

    let timeline = c.timeline_get().unwrap();
    assert_eq!(timeline["frame"], 0);
    assert_eq!(timeline["playing"], false);
    assert!(timeline["fps"].as_f64().unwrap() > 0.0);

    // One undo step per edit: undo brings the width back.
    c.command_run("edit.undo", json!({})).unwrap();
    assert!((width(&c.eval_mesh(node, None).unwrap()) - 200.0).abs() < 1e-9);
}

#[test]
fn command_list_matches_the_registry() {
    let fx = Fixture::token();
    let mut c = fx.client();
    let listed = c.command_list().unwrap();
    let ids: Vec<&str> = listed["commands"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["id"].as_str().unwrap())
        .collect();
    let app = App::new_default();
    let expected: Vec<&str> = app.commands.iter().map(|h| h.id()).collect();
    assert_eq!(ids, expected);
    assert!(listed["commands"][0]["args_schema"].is_object());
}

#[test]
fn the_token_is_enforced() {
    let fx = Fixture::token();

    let mut raw = fx.raw();
    let mut no_token = request(1, "app.version", json!({}));
    no_token.as_object_mut().unwrap().remove("token");
    let reply = raw.ask(no_token);
    assert_eq!(reply["error"]["code"], code::UNAUTHORIZED);
    assert_eq!(reply["id"], 1);
    assert!(reply.get("result").is_none());
    assert!(raw.closed());

    let mut c = Client::connect(fx.server.port(), Some("wrong")).unwrap();
    let err = c.version().unwrap_err();
    assert_eq!(error_code(&err), code::UNAUTHORIZED);
    let mut c = Client::connect(fx.server.port(), None).unwrap();
    assert_eq!(
        error_code(&c.command_list().unwrap_err()),
        code::UNAUTHORIZED
    );

    // A bad token runs nothing, and the error does not echo either token.
    let mut raw = fx.raw();
    let mut bad = request(
        2,
        "command.run",
        json!({"id": "object.create", "args": {"type": "forge.cube"}}),
    );
    bad["token"] = json!("nope-nope");
    let reply = raw.ask(bad);
    let text = reply.to_string();
    assert!(
        !text.contains(TOKEN) && !text.contains("nope-nope"),
        "{text}"
    );
    assert_eq!(fx.client().scene_get().unwrap()["nodes"], 0);
}

#[test]
fn no_token_means_no_server_unless_insecure_local() {
    let refused = Server::start(ServerConfig {
        port: 0,
        token: None,
        insecure_local: false,
    });
    assert!(matches!(refused, Err(StartError::NoToken)));
    let empty = Server::start(ServerConfig {
        port: 0,
        token: Some(String::new()),
        insecure_local: false,
    });
    assert!(matches!(empty, Err(StartError::NoToken)));

    let fx = Fixture::start(None, true);
    let mut c = Client::connect(fx.server.port(), None).unwrap();
    assert_eq!(c.version().unwrap()["name"], "forge3d");

    // A configured token is still required with the flag.
    let fx = Fixture::start(Some(TOKEN), true);
    let mut c = Client::connect(fx.server.port(), None).unwrap();
    assert_eq!(error_code(&c.version().unwrap_err()), code::UNAUTHORIZED);
}

#[test]
fn the_server_listens_on_localhost_only() {
    let fx = Fixture::token();
    assert!(fx.server.local_addr().ip().is_loopback());
}

#[test]
fn malformed_requests_get_errors_and_the_connection_survives() {
    let fx = Fixture::token();
    let mut raw = fx.raw();

    raw.send("{not json");
    let reply = raw.recv();
    assert_eq!(reply["error"]["code"], code::PARSE_ERROR);
    assert_eq!(reply["id"], Value::Null);

    let reply = raw.ask(json!([1, 2]));
    assert_eq!(reply["error"]["code"], code::INVALID_REQUEST);

    let mut no_id = request(0, "app.version", json!({}));
    no_id.as_object_mut().unwrap().remove("id");
    assert_eq!(raw.ask(no_id)["error"]["code"], code::INVALID_REQUEST);

    let mut wrong_version = request(3, "app.version", json!({}));
    wrong_version["jsonrpc"] = json!("1.0");
    let reply = raw.ask(wrong_version);
    assert_eq!(reply["error"]["code"], code::INVALID_REQUEST);
    assert_eq!(reply["id"], 3);

    let reply = raw.ask(request(4, "no.such.method", json!({})));
    assert_eq!(reply["error"]["code"], code::METHOD_NOT_FOUND);

    let reply = raw.ask(request(5, "node.get", json!({})));
    assert_eq!(reply["error"]["code"], code::INVALID_PARAMS);
    let reply = raw.ask(request(6, "node.get", json!({"id": "one"})));
    assert_eq!(reply["error"]["code"], code::INVALID_PARAMS);
    let reply = raw.ask(request(7, "app.version", json!([1])));
    assert_eq!(reply["error"]["code"], code::INVALID_PARAMS);

    let reply = raw.ask(request(
        8,
        "command.run",
        json!({"id": "no.such", "args": {}}),
    ));
    assert_eq!(reply["error"]["code"], code::FAILED);
    assert_eq!(reply["error"]["data"]["kind"], "command");
    let reply = raw.ask(request(
        9,
        "command.run",
        json!({"id": "object.create", "args": {"bogus": 1}}),
    ));
    assert_eq!(reply["error"]["code"], code::INVALID_PARAMS);
    let reply = raw.ask(request(10, "node.get", json!({"id": 999})));
    assert_eq!(reply["error"]["code"], code::FAILED);
    assert_eq!(reply["error"]["data"]["kind"], "node");
    let reply = raw.ask(request(11, "eval.mesh", json!({"node": 999})));
    assert_eq!(reply["error"]["code"], code::FAILED);

    // String ids come back unchanged; blank lines are ignored; still alive.
    raw.send("");
    let mut ok = request(0, "app.version", json!({}));
    ok["id"] = json!("abc");
    let reply = raw.ask(ok);
    assert_eq!(reply["id"], "abc");
    assert_eq!(reply["result"]["name"], "forge3d");
}

#[test]
fn an_oversized_message_is_refused_and_the_server_keeps_serving() {
    let fx = Fixture::token();
    let mut raw = fx.raw();
    let writer = raw.writer.try_clone().unwrap();
    let sender = std::thread::spawn(move || {
        let mut writer = writer;
        let chunk = vec![b'a'; 64 * 1024];
        let mut sent = 0;
        // Past the limit, without a newline; the server may close before we finish.
        while sent < MAX_MESSAGE_BYTES + 128 * 1024 {
            if writer.write_all(&chunk).is_err() {
                break;
            }
            sent += chunk.len();
        }
    });
    let reply = raw.recv();
    assert_eq!(reply["error"]["code"], code::TOO_LARGE);
    assert!(raw.closed());
    sender.join().unwrap();
    assert_eq!(fx.client().version().unwrap()["name"], "forge3d");
}

#[test]
fn a_message_just_under_the_limit_is_accepted() {
    let fx = Fixture::token();
    let mut raw = fx.raw();
    // Padding inside a valid request: an unknown field is ignored.
    let mut req = request(1, "app.version", json!({}));
    req["pad"] = json!("x".repeat(MAX_MESSAGE_BYTES - 512));
    let reply = raw.ask(req);
    assert_eq!(reply["result"]["name"], "forge3d");
}

#[test]
fn the_fifth_connection_is_refused() {
    let fx = Fixture::token();
    let mut held: Vec<Client> = (0..MAX_CONNECTIONS)
        .map(|_| {
            let mut c = fx.client();
            c.version().unwrap();
            c
        })
        .collect();
    let mut fifth = fx.client();
    let err = fifth.version().unwrap_err();
    assert_eq!(error_code(&err), code::TOO_MANY_CONNECTIONS);

    // Freeing a slot lets a new connection in.
    held.pop();
    let mut again = None;
    for _ in 0..100 {
        let mut c = fx.client();
        if c.version().is_ok() {
            again = Some(c);
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    assert!(again.is_some());
}

#[test]
fn a_burst_is_rate_limited_per_connection() {
    let fx = Fixture::token();
    let mut raw = fx.raw();
    let burst = 200;
    for i in 0..burst {
        raw.send(&request(i, "app.version", json!({})).to_string());
    }
    let (mut ok, mut limited) = (0, 0);
    for _ in 0..burst {
        let reply = raw.recv();
        match reply["error"]["code"].as_i64() {
            None => ok += 1,
            Some(c) if c == code::RATE_LIMITED => limited += 1,
            Some(other) => panic!("unexpected {other}"),
        }
    }
    assert!((50..=70).contains(&ok), "ok {ok}");
    assert_eq!(ok + limited, burst);
    // Another connection has its own budget.
    assert_eq!(fx.client().version().unwrap()["name"], "forge3d");
}

#[test]
fn a_dead_editor_yields_an_internal_error_not_a_hang() {
    let (server, inbox) = Server::start(ServerConfig {
        port: 0,
        token: Some(TOKEN.into()),
        insecure_local: false,
    })
    .unwrap();
    drop(inbox);
    let mut c = Client::connect(server.port(), Some(TOKEN)).unwrap();
    assert_eq!(error_code(&c.version().unwrap_err()), code::INTERNAL);
}

#[test]
fn constant_time_comparison() {
    assert!(ct_eq(b"abc", b"abc"));
    assert!(ct_eq(b"", b""));
    assert!(!ct_eq(b"abc", b"abd"));
    assert!(!ct_eq(b"abc", b"abcd"));
    assert!(!ct_eq(b"abcd", b"abc"));
    assert!(!ct_eq(b"", b"a"));
}

#[test]
fn settings_resolve_from_environment_then_file() {
    let file = json!({"rpc": {"port": 9000, "token": "from-file"}});
    let none = |_: &str| None::<String>;
    let s = RpcSettings::resolve(&none, &file);
    assert_eq!((s.port, s.token.as_deref()), (9000, Some("from-file")));

    let env = |k: &str| match k {
        "FORGE_RPC_PORT" => Some("9100".to_string()),
        "FORGE_RPC_TOKEN" => Some("from-env".to_string()),
        _ => None,
    };
    let s = RpcSettings::resolve(&env, &file);
    assert_eq!((s.port, s.token.as_deref()), (9100, Some("from-env")));

    let s = RpcSettings::resolve(&none, &Value::Null);
    assert_eq!((s.port, s.token), (7323, None));

    // Empty token, bad or out-of-range ports fall through.
    let env = |k: &str| match k {
        "FORGE_RPC_PORT" => Some("not a port".to_string()),
        "FORGE_RPC_TOKEN" => Some(String::new()),
        _ => None,
    };
    let s = RpcSettings::resolve(&env, &json!({"rpc": {"port": 70000, "token": ""}}));
    assert_eq!((s.port, s.token), (7323, None));
}
