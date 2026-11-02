//! `forge-rpc-serve`: a headless RPC server around an in-process `App` (docs/specs/RPC.md
//! section 9). Used by the Python client tests; no window, no GPU.

use std::io::Write;
use std::process::ExitCode;
use std::sync::atomic::AtomicBool;

use forge_app::App;
use forge_rpc::{RpcSettings, Server, ServerConfig};

const USAGE: &str = "usage: forge-rpc-serve [--port N] [--insecure-local] [scene-file]\n\
Serves the JSON-RPC API on 127.0.0.1 (default port 0 = any free port). The token comes\n\
from FORGE_RPC_TOKEN or the settings file. Prints `FORGE_RPC_PORT=<port>` on stdout once\n\
listening; runs until killed or until stdin reaches EOF.";

fn main() -> ExitCode {
    let mut port = 0u16;
    let mut insecure_local = false;
    let mut scene = None;
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "-h" | "--help" => {
                println!("{USAGE}");
                return ExitCode::SUCCESS;
            }
            "--insecure-local" => insecure_local = true,
            "--port" => match it.next().and_then(|v| v.parse::<u16>().ok()) {
                Some(p) => port = p,
                None => {
                    eprintln!("forge-rpc-serve: --port needs a number\n{USAGE}");
                    return ExitCode::from(2);
                }
            },
            o if o.starts_with('-') => {
                eprintln!("forge-rpc-serve: unknown option {o}\n{USAGE}");
                return ExitCode::from(2);
            }
            o if scene.is_none() => scene = Some(o.to_string()),
            o => {
                eprintln!("forge-rpc-serve: unexpected argument {o}\n{USAGE}");
                return ExitCode::from(2);
            }
        }
    }
    let token = RpcSettings::from_environment().token;
    forge_app::diagnostics::install(
        "forge-rpc-serve",
        env!("CARGO_PKG_VERSION"),
        token.iter().cloned().collect(),
    );
    let mut app = App::new_default();
    if let Some(p) = &scene
        && let Err(e) = app.open(p)
    {
        eprintln!("forge-rpc-serve: cannot open {p}: {e}");
        return ExitCode::from(2);
    }
    let (server, inbox) = match Server::start(ServerConfig {
        port,
        token,
        insecure_local,
    }) {
        Ok(s) => s,
        Err(e) => {
            tracing::error!("cannot start the RPC server: {e:?}");
            eprintln!("forge-rpc-serve: cannot start: {e:?}");
            return ExitCode::from(1);
        }
    };
    tracing::info!("serving on port {}", server.port());
    println!("FORGE_RPC_PORT={}", server.port());
    let _ = std::io::stdout().flush();
    // Stop when the parent closes our stdin, so a crashed test run leaves no server behind.
    let stop: &'static AtomicBool = Box::leak(Box::new(AtomicBool::new(false)));
    std::thread::spawn(move || {
        let mut sink = String::new();
        while std::io::stdin()
            .read_line(&mut sink)
            .map(|n| n > 0)
            .unwrap_or(false)
        {
            sink.clear();
        }
        stop.store(true, std::sync::atomic::Ordering::SeqCst);
    });
    inbox.run_until(&mut app, stop);
    drop(server);
    ExitCode::SUCCESS
}
