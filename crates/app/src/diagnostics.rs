//! Process-level diagnostics: logging, the crash-report panic hook and the command history
//! (docs/specs/PLATFORM.md §8). Called first thing by every executable that is not the CLI.

use forge_platform::crash::{HookConfig, crash_dir, install_hook};
use forge_platform::logging;

/// Sets up the rotating log and the panic hook. `app` and `version` name the executable in
/// reports; `secrets` (the RPC token) are scrubbed from every report. Safe to call twice:
/// the second call changes nothing.
pub fn install(app: &str, version: &str, secrets: Vec<String>) {
    use std::sync::Once;
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        logging::init(forge_platform::settings_path().as_deref());
        install_hook(HookConfig {
            app: app.to_string(),
            version: version.to_string(),
            dir: crash_dir(),
            commands: forge_command::history::snapshot,
            log_lines: logging::recent_lines,
            secrets,
        });
    });
}
