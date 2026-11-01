//! Notifications, screenshot errors and jobs (docs/specs/RPC.md §10.5). Port 0 everywhere.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use forge_app::{App, default_command_registry};
use forge_command::{CommandError, CommandSpec};
use forge_rpc::{Client, ClientError, Host, Notification, Server, ServerConfig};
use serde_json::{Value, json};

const TOKEN: &str = "tok";
const WAIT: Duration = Duration::from_secs(10);

/// An `App` host that can replace its command registry (command `test.swap_commands`).
struct TestHost(App);

impl Host for TestHost {
    fn app(&self) -> &App {
        &self.0
    }

    fn run_command(&mut self, id: &str, args: Value) -> Result<Value, CommandError> {
        if id == "test.swap_commands" {
            let mut commands = default_command_registry();
            commands
                .register_fn(
                    CommandSpec {
                        id: "test.hello",
                        label: "Hello",
                        undoable: false,
                        sample: || json!({}),
                    },
                    |_c, _a: Value| Ok(json!({"hello": true})),
                )
                .unwrap();
            self.0.commands = Arc::new(commands);
            return Ok(json!({}));
        }
        self.0.run(id, args)
    }
}

struct Fixture {
    server: Server,
    stop: Arc<AtomicBool>,
    pump: Option<JoinHandle<()>>,
}

impl Fixture {
    fn start() -> Fixture {
        let (server, inbox) = Server::start(ServerConfig {
            port: 0,
            token: Some(TOKEN.into()),
            insecure_local: false,
        })
        .unwrap();
        let stop = Arc::new(AtomicBool::new(false));
        let flag = stop.clone();
        let pump = std::thread::spawn(move || {
            let mut host = TestHost(App::new_default());
            inbox.run_until(&mut host, &flag);
        });
        Fixture {
            server,
            stop,
            pump: Some(pump),
        }
    }

    fn client(&self) -> Client {
        Client::connect(self.server.port(), Some(TOKEN)).unwrap()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(p) = self.pump.take() {
            let _ = p.join();
        }
    }
}

fn expect(c: &mut Client, method: &str) -> Notification {
    let deadline = Instant::now() + WAIT;
    while Instant::now() < deadline {
        if let Some(n) = c.next_notification(Duration::from_millis(200)).unwrap() {
            if n.method == method {
                return n;
            }
            panic!("expected {method}, got {} {}", n.method, n.params);
        }
    }
    panic!("no {method} within {WAIT:?}");
}

fn none_within(c: &mut Client, ms: u64) {
    if let Some(n) = c.next_notification(Duration::from_millis(ms)).unwrap() {
        panic!("unexpected {} {}", n.method, n.params);
    }
}

fn rpc_code(e: ClientError) -> i64 {
    match e {
        ClientError::Rpc { code, .. } => code,
        other => panic!("not an RPC error: {other}"),
    }
}

#[test]
fn frame_change_is_notified_to_subscribers_only() {
    let f = Fixture::start();
    let mut watcher = f.client();
    let mut other = f.client();
    let mut actor = f.client();
    assert_eq!(watcher.subscribe(&["frame"]).unwrap(), ["frame"]);
    // Let the pump record its baseline before the change.
    actor.version().unwrap();
    actor
        .command_run("timeline.set_frame", json!({"frame": 7}))
        .unwrap();
    let n = expect(&mut watcher, "frame.changed");
    assert_eq!(n.params, json!({"frame": 7}));
    none_within(&mut other, 200);
    none_within(&mut actor, 100);
    // Only the kinds asked for.
    actor
        .command_run("object.create", json!({"type": "forge.cube"}))
        .unwrap();
    none_within(&mut watcher, 300);
}

#[test]
fn scene_and_selection_events_follow_edits_and_undo() {
    let f = Fixture::start();
    let mut c = f.client();
    c.version().unwrap();
    assert_eq!(
        c.subscribe(&["scene", "selection"]).unwrap(),
        ["scene", "selection"]
    );
    let made = c
        .command_run("object.create", json!({"type": "forge.cube"}))
        .unwrap();
    let node = made["node"].as_u64().unwrap();
    let mut seen = Vec::new();
    while seen.len() < 2 {
        let n = c
            .next_notification(WAIT)
            .unwrap()
            .expect("two notifications");
        seen.push(n);
    }
    let scene = seen.iter().find(|n| n.method == "scene.changed").unwrap();
    let rev = scene.params["revision"].as_u64().unwrap();
    let sel = seen
        .iter()
        .find(|n| n.method == "selection.changed")
        .unwrap();
    assert_eq!(sel.params, json!({"nodes": [node]}));

    c.command_run("edit.undo", json!({})).unwrap();
    let mut methods = Vec::new();
    while let Some(n) = c.next_notification(Duration::from_millis(500)).unwrap() {
        if n.method == "scene.changed" {
            assert_ne!(n.params["revision"].as_u64().unwrap(), rev);
        }
        methods.push(n.method);
    }
    assert!(
        methods.contains(&"scene.changed".to_string()),
        "{methods:?}"
    );
}

#[test]
fn unsubscribe_and_bad_kinds() {
    let f = Fixture::start();
    let mut c = f.client();
    assert_eq!(c.subscribe(&[]).unwrap().len(), 0);
    let all = c.call("events.subscribe", json!({})).unwrap();
    assert_eq!(
        all["kinds"],
        json!(["scene", "frame", "selection", "commands"])
    );
    assert_eq!(
        c.unsubscribe(&["scene", "frame"]).unwrap(),
        ["selection", "commands"]
    );
    assert_eq!(
        c.unsubscribe(&["scene"]).unwrap(),
        ["selection", "commands"]
    );
    c.unsubscribe(&["selection", "commands"]).unwrap();
    c.command_run("timeline.set_frame", json!({"frame": 3}))
        .unwrap();
    none_within(&mut c, 300);
    assert_eq!(rpc_code(c.subscribe(&["bogus"]).unwrap_err()), -32602);
    let e = c
        .call("events.subscribe", json!({"kinds": "frame"}))
        .unwrap_err();
    assert_eq!(rpc_code(e), -32602);
}

#[test]
fn commands_changed_when_the_registry_is_replaced() {
    let f = Fixture::start();
    let mut c = f.client();
    c.version().unwrap();
    c.subscribe(&["commands"]).unwrap();
    let before = c.command_list().unwrap()["commands"]
        .as_array()
        .unwrap()
        .len();
    c.command_run("test.swap_commands", json!({})).unwrap();
    let n = expect(&mut c, "commands.changed");
    assert_eq!(n.params["count"], json!(before + 1));
    let ids: Vec<String> = c.command_list().unwrap()["commands"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["id"].as_str().unwrap().to_string())
        .collect();
    assert!(ids.contains(&"test.hello".to_string()));
}

#[test]
fn notifications_do_not_confuse_calls() {
    let f = Fixture::start();
    let mut c = f.client();
    c.version().unwrap();
    c.subscribe(&["frame"]).unwrap();
    // Many changes, each answered while notifications arrive in between.
    for frame in 1..=20 {
        let r = c
            .command_run("timeline.set_frame", json!({ "frame": frame }))
            .unwrap();
        assert_eq!(r["frame"], json!(frame));
    }
    let mut last = 0;
    while let Some(n) = c.next_notification(Duration::from_millis(300)).unwrap() {
        assert_eq!(n.method, "frame.changed");
        let frame = n.params["frame"].as_i64().unwrap();
        assert!(frame > last);
        last = frame;
    }
    assert_eq!(last, 20, "the latest frame is always reported");
}

#[test]
fn screenshot_without_a_viewport_and_bad_sizes() {
    let f = Fixture::start();
    let mut c = f.client();
    match c.screenshot(64, 48).unwrap_err() {
        ClientError::Rpc {
            code,
            data,
            message,
        } => {
            assert_eq!(code, -32000);
            assert_eq!(data.unwrap()["kind"], "viewport");
            assert!(message.contains("viewport"), "{message}");
        }
        other => panic!("{other}"),
    }
    for params in [
        json!({"width": 0, "height": 10}),
        json!({"width": 10, "height": 5000}),
        json!({"width": 10, "extra": 1}),
        json!({"width": "x"}),
    ] {
        let e = c.call("viewport.screenshot", params).unwrap_err();
        assert_eq!(rpc_code(e), -32602);
    }
    c.version().unwrap();
}

fn temp_dir(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("forge-rpc-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

fn count_files(dir: &std::path::Path) -> usize {
    std::fs::read_dir(dir).map_or(0, |d| d.count())
}

#[test]
fn export_sequence_job_reports_progress_and_finishes() {
    let f = Fixture::start();
    let mut c = f.client();
    let mut bystander = f.client();
    c.command_run("object.create", json!({"type": "forge.cube"}))
        .unwrap();
    let dir = temp_dir("seq");
    let id = c
        .job_start(
            "export.obj_sequence",
            json!({"dir": dir.to_str().unwrap(), "start": 0, "end": 4, "prefix": "cube_"}),
        )
        .unwrap();
    let mut fractions = Vec::new();
    let finished = loop {
        let n = c.next_notification(WAIT).unwrap().expect("job ends");
        assert_eq!(n.params["id"], json!(id));
        match n.method.as_str() {
            "job.progress" => {
                assert!(n.params["message"].is_string());
                fractions.push(n.params["fraction"].as_f64().unwrap());
            }
            "job.finished" => break n,
            other => panic!("{other}"),
        }
    };
    assert!(fractions.windows(2).all(|w| w[0] <= w[1]), "{fractions:?}");
    assert_eq!(*fractions.last().unwrap(), 1.0);
    assert!(fractions.len() >= 5);
    assert_eq!(finished.params["state"], "done");
    assert_eq!(finished.params["result"]["files"], 5);
    assert_eq!(count_files(&dir), 5);
    assert!(dir.join("cube_0003.obj").is_file());
    none_within(&mut bystander, 200);

    let list = c.call("job.list", json!({})).unwrap();
    assert_eq!(list["jobs"][0]["id"], json!(id));
    assert_eq!(list["jobs"][0]["state"], "done");
    let late = c.job_cancel(id).unwrap();
    assert_eq!(late, json!({"id": id, "cancelled": false, "state": "done"}));
    assert_eq!(rpc_code(c.job_cancel(999).unwrap_err()), -32000);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_running_job_can_be_cancelled() {
    let f = Fixture::start();
    let mut c = f.client();
    c.command_run("object.create", json!({"type": "forge.cube"}))
        .unwrap();
    let dir = temp_dir("cancel");
    let id = c
        .job_start(
            "export.obj_sequence",
            json!({"dir": dir.to_str().unwrap(), "start": 0, "end": 100000}),
        )
        .unwrap();
    assert_eq!(expect(&mut c, "job.progress").params["id"], json!(id));
    assert_eq!(
        c.job_cancel(id).unwrap(),
        json!({"id": id, "cancelled": true})
    );
    let finished = loop {
        let n = c.next_notification(WAIT).unwrap().expect("finished");
        if n.method == "job.finished" {
            break n;
        }
    };
    assert_eq!(finished.params["state"], "cancelled");
    let written = count_files(&dir);
    assert!(written > 0 && written < 100_000, "{written}");
    // Nothing more after finished.
    none_within(&mut c, 200);
    assert_eq!(count_files(&dir), written);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn job_failures_and_bad_arguments() {
    let f = Fixture::start();
    let mut c = f.client();
    let dir = temp_dir("bad");
    let d = dir.to_str().unwrap();
    for (kind, args) in [
        ("nope", json!({})),
        ("export.obj_sequence", json!({})),
        (
            "export.obj_sequence",
            json!({"dir": d, "start": 5, "end": 1}),
        ),
        ("export.obj_sequence", json!({"dir": d, "step": 0})),
        ("export.obj_sequence", json!({"dir": d, "prefix": "a/b"})),
        ("export.obj_sequence", json!({"dir": d, "bogus": 1})),
    ] {
        let e = c.job_start(kind, args).unwrap_err();
        assert_eq!(rpc_code(e), -32602, "{kind}");
    }
    assert_eq!(
        rpc_code(c.call("job.start", json!({})).unwrap_err()),
        -32602
    );
    // An empty scene: the job starts and fails.
    let id = c
        .job_start("export.obj_sequence", json!({"dir": d, "end": 2}))
        .unwrap();
    let finished = loop {
        let n = c.next_notification(WAIT).unwrap().expect("finished");
        if n.method == "job.finished" {
            break n;
        }
    };
    assert_eq!(finished.params["id"], json!(id));
    assert_eq!(finished.params["state"], "failed");
    assert_eq!(finished.params["error"], "nothing to export");
    let _ = std::fs::remove_dir_all(&dir);
}
