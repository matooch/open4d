//! Forge3D editor entry point. `forge-studio --headless <forge args>` runs the command-line
//! tool instead (docs/specs/CLI.md §3), so one installed binary does both.

use std::process::ExitCode;
use std::sync::Arc;

fn main() -> ExitCode {
    let args: Vec<std::ffi::OsString> = std::env::args_os().skip(1).collect();
    if args.first().is_some_and(|a| a == "--headless") {
        return forge_cli::run(args.into_iter().skip(1));
    }
    let insecure_local = args.iter().any(|a| a == "--rpc-insecure-local");
    // Logging and the crash-report hook (PLATFORM.md §8); the RPC token never enters a report.
    forge_app::diagnostics::install(
        "forge-studio",
        env!("CARGO_PKG_VERSION"),
        forge_rpc::RpcSettings::from_environment()
            .token
            .into_iter()
            .collect(),
    );
    match run_editor(insecure_local) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            tracing::error!("editor failed: {e}");
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run_editor(insecure_local: bool) -> Result<(), Box<dyn std::error::Error>> {
    let registry = forge_app::default_registry()?;
    // Held until the window closes; dropping it stops the server.
    let (_server, inbox) = start_rpc(insecure_local).unzip();
    forge_ui::run_with_rpc(Arc::new(registry), inbox)?;
    Ok(())
}

/// Starts the JSON-RPC server (docs/specs/RPC.md §2): only with a token, or with
/// `--rpc-insecure-local`. Failing to start is reported, never fatal.
fn start_rpc(insecure_local: bool) -> Option<(forge_rpc::Server, forge_rpc::Inbox)> {
    let settings = forge_rpc::RpcSettings::from_environment();
    let config = forge_rpc::ServerConfig {
        port: settings.port,
        insecure_local,
        token: settings.token,
    };
    let with_token = config.token.is_some();
    match forge_rpc::Server::start(config) {
        Ok((server, inbox)) => {
            let auth = if with_token {
                "token required"
            } else {
                "NO TOKEN (--rpc-insecure-local)"
            };
            tracing::info!("rpc: listening on {} ({auth})", server.local_addr());
            // The startup banner is documented (docs/manual/automation.md), so it stays.
            eprintln!("rpc: listening on {} ({auth})", server.local_addr());
            Some((server, inbox))
        }
        Err(e) => {
            tracing::warn!("rpc: disabled: {e}");
            eprintln!("rpc: disabled: {e}");
            None
        }
    }
}
