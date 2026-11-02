//! End to end: commands run through the app, then a forced panic in a child process writes a
//! report with those commands and the log tail, without the RPC token
//! (docs/specs/PLATFORM.md §8). The test binary runs itself; the child test is a no-op unless
//! `FORGE_TEST_CRASH_CHILD` is set.

use std::path::PathBuf;
use std::process::Command;

use forge_app::App;
use serde_json::json;

#[test]
fn child_runs_commands_then_panics() {
    if std::env::var_os("FORGE_TEST_CRASH_CHILD").is_none() {
        return;
    }
    forge_app::diagnostics::install("app-test", "1.2.3", vec!["tok-abc123".into()]);
    let mut app = App::new_default();
    for i in 0..60 {
        app.run("object.create", json!({"type": "forge.null"}))
            .unwrap();
        app.run(
            "object.rename",
            json!({"node": 1, "name": format!("n{i}"), "rpc_token": "tok-abc123"}),
        )
        .ok();
    }
    app.run("no.such_command", json!({})).unwrap_err();
    tracing::warn!("about to fail with tok-abc123");
    panic!("forced crash");
}

#[test]
fn a_forced_panic_reports_the_last_fifty_commands() {
    let crashes = tempfile::tempdir().unwrap();
    let logs = tempfile::tempdir().unwrap();
    let out = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "child_runs_commands_then_panics",
            "--nocapture",
            "--test-threads=1",
        ])
        .env("FORGE_TEST_CRASH_CHILD", "1")
        .env("FORGE_CRASH_DIR", crashes.path())
        .env("FORGE_LOG_DIR", logs.path())
        .env("FORGE_SETTINGS", logs.path().join("none.json"))
        .env("FORGE_LOG", "debug")
        .env("RUST_BACKTRACE", "0")
        .output()
        .unwrap();
    assert!(!out.status.success());

    let reports = forge_platform::crash::unseen(crashes.path());
    assert_eq!(reports.len(), 1);
    let text = std::fs::read_to_string(&reports[0].path).unwrap();
    assert!(text.contains("Application: app-test 1.2.3"), "{text}");
    assert!(text.contains("Panic: forced crash"));
    assert!(text.contains("Last 50 commands (oldest first):"), "{text}");
    // 121 runs happened: the newest 50 are kept, ending with the failing unknown command.
    let commands: Vec<&str> = text
        .split("commands (oldest first):\n")
        .nth(1)
        .unwrap()
        .split("\n\n")
        .next()
        .unwrap()
        .lines()
        .collect();
    assert_eq!(commands.len(), 50);
    assert!(commands[49].ends_with("no.such_command {}"), "{commands:?}");
    assert!(commands[0].starts_with("72 "), "{:?}", commands[0]);
    assert!(
        text.contains("about to fail with <redacted>"),
        "log tail missing"
    );
    assert!(!text.contains("tok-abc123"), "token leaked");
    assert!(text.contains("\"rpc_token\":\"<redacted>\""));

    // the log file got the same warning
    let log: PathBuf = logs.path().join("forge.log");
    assert!(std::fs::read_to_string(log).unwrap().contains("WARN"));
}
