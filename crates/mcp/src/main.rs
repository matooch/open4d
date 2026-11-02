//! `forge-mcp`: MCP over stdio (docs/specs/RPC.md section 8).

use std::io::{BufRead, Write};
use std::process::ExitCode;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use forge_app::App;
use forge_mcp::{Backend, EditorBackend, HeadlessBackend, Server};
use forge_rpc::RpcSettings;

const USAGE: &str = "usage: forge-mcp [--port N] | forge-mcp --headless [scene-file]\n\
Speaks MCP over stdin/stdout. Default: forwards to the running editor over RPC on\n\
127.0.0.1 (port from --port, FORGE_RPC_PORT or the settings file; token from\n\
FORGE_RPC_TOKEN or the settings file). --headless runs an in-process app instead.";

enum Mode {
    Editor { port: Option<u16> },
    Headless { scene: Option<String> },
}

fn parse_args(args: &[String]) -> Result<Option<Mode>, String> {
    let mut headless = false;
    let mut port = None;
    let mut scene = None;
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "-h" | "--help" => return Ok(None),
            "--headless" => headless = true,
            "--port" => {
                let v = it.next().ok_or("--port needs a value")?;
                port = Some(v.parse::<u16>().map_err(|_| format!("bad port {v}"))?);
            }
            other if other.starts_with('-') => return Err(format!("unknown option {other}")),
            other if scene.is_none() => scene = Some(other.to_string()),
            other => return Err(format!("unexpected argument {other}")),
        }
    }
    if headless {
        if port.is_some() {
            return Err("--port does not apply to --headless".into());
        }
        Ok(Some(Mode::Headless { scene }))
    } else if scene.is_some() {
        Err("a scene file needs --headless".into())
    } else {
        Ok(Some(Mode::Editor { port }))
    }
}

/// One line on stdout; the lock keeps replies and notifications from interleaving.
fn write_line(out: &Mutex<std::io::Stdout>, line: &str) -> bool {
    let mut out = out.lock().unwrap_or_else(|e| e.into_inner());
    writeln!(out, "{line}").and_then(|_| out.flush()).is_ok()
}

fn serve(backend: impl Backend, watch: Option<(u16, Option<String>)>) -> ExitCode {
    let mut server = Server::new(backend);
    let stdin = std::io::stdin();
    let stdout = Arc::new(Mutex::new(std::io::stdout()));
    let stop = Arc::new(AtomicBool::new(false));
    if let Some((port, token)) = watch {
        let (stale, out, stop) = (server.stale_flag(), stdout.clone(), stop.clone());
        std::thread::spawn(move || {
            forge_mcp::watch_command_changes(port, token, &stop, || {
                stale.store(true, Ordering::SeqCst);
                let note = r#"{"jsonrpc":"2.0","method":"notifications/tools/list_changed"}"#;
                write_line(&out, note);
            });
        });
    }
    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        if line.trim().is_empty() {
            continue;
        }
        if let Some(reply) = server.handle_line(&line)
            && !write_line(&stdout, &reply)
        {
            break;
        }
    }
    stop.store(true, Ordering::SeqCst);
    ExitCode::SUCCESS
}

/// Log file and crash reports; stdout stays clean for the protocol (PLATFORM.md §8.2).
fn install_diagnostics(token: Option<String>) {
    forge_app::diagnostics::install(
        "forge-mcp",
        env!("CARGO_PKG_VERSION"),
        token.into_iter().collect(),
    );
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match parse_args(&args) {
        Ok(None) => {
            println!("{USAGE}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("forge-mcp: {e}\n{USAGE}");
            ExitCode::from(2)
        }
        Ok(Some(Mode::Headless { scene })) => {
            install_diagnostics(RpcSettings::from_environment().token);
            let mut app = App::new_default();
            if let Some(path) = scene
                && let Err(e) = app.open(&path)
            {
                tracing::error!("cannot open {path}: {e}");
                eprintln!("forge-mcp: cannot open {path}: {e}");
                return ExitCode::from(1);
            }
            serve(HeadlessBackend::new(app), None)
        }
        Ok(Some(Mode::Editor { port })) => {
            let settings = RpcSettings::from_environment();
            install_diagnostics(settings.token.clone());
            let port = port.unwrap_or(settings.port);
            let watch = Some((port, settings.token.clone()));
            serve(EditorBackend::new(port, settings.token), watch)
        }
    }
}
