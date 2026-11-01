//! Forge3D editor entry point. `forge-studio --headless <forge args>` runs the command-line
//! tool instead (docs/specs/CLI.md §3), so one installed binary does both.

use std::process::ExitCode;
use std::sync::Arc;

fn main() -> ExitCode {
    let args: Vec<std::ffi::OsString> = std::env::args_os().skip(1).collect();
    if args.first().is_some_and(|a| a == "--headless") {
        return forge_cli::run(args.into_iter().skip(1));
    }
    match run_editor() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run_editor() -> Result<(), Box<dyn std::error::Error>> {
    let registry = forge_app::default_registry()?;
    forge_ui::run(Arc::new(registry))?;
    Ok(())
}
