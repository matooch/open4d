//! `forge-cleanroom [ROOT]` — exit 0 clean, 1 violations, 2 config/IO error.

use std::path::PathBuf;
use std::process::ExitCode;

use forge_cleanroom::{Config, scan};

fn main() -> ExitCode {
    let root = PathBuf::from(std::env::args().nth(1).unwrap_or_else(|| ".".into()));
    let cfg_path = root.join("tools/cleanroom/forbidden.txt");
    let result = Config::load(&cfg_path)
        .map_err(|e| format!("{}: {e}", cfg_path.display()))
        .and_then(|cfg| scan(&root, &cfg).map_err(|e| format!("scan failed: {e}")));
    match result {
        Err(e) => {
            eprintln!("cleanroom: {e}");
            ExitCode::from(2)
        }
        Ok(violations) if violations.is_empty() => {
            println!("cleanroom: ok");
            ExitCode::SUCCESS
        }
        Ok(violations) => {
            for v in &violations {
                println!("{v}");
            }
            eprintln!("cleanroom: {} violation(s)", violations.len());
            ExitCode::from(1)
        }
    }
}
