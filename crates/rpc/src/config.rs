//! Port, token and the settings file (docs/specs/RPC.md §2).

use std::path::PathBuf;

use serde_json::Value;

pub const DEFAULT_PORT: u16 = 7323;

/// What the server and the client need to find each other.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RpcSettings {
    pub port: u16,
    pub token: Option<String>,
}

impl RpcSettings {
    /// Resolves port and token from `env` (variable lookup) and the parsed settings file:
    /// `FORGE_RPC_PORT` / `rpc.port`, `FORGE_RPC_TOKEN` / `rpc.token`. An empty token is
    /// no token; an unusable port falls back to the default.
    pub fn resolve(env: &dyn Fn(&str) -> Option<String>, file: &Value) -> RpcSettings {
        let port = env("FORGE_RPC_PORT")
            .and_then(|p| p.trim().parse::<u16>().ok())
            .or_else(|| {
                file["rpc"]["port"]
                    .as_u64()
                    .and_then(|p| u16::try_from(p).ok())
            })
            .unwrap_or(DEFAULT_PORT);
        let token = env("FORGE_RPC_TOKEN")
            .filter(|t| !t.is_empty())
            .or_else(|| {
                file["rpc"]["token"]
                    .as_str()
                    .filter(|t| !t.is_empty())
                    .map(String::from)
            });
        RpcSettings { port, token }
    }

    /// The process environment and the settings file on disk.
    pub fn from_environment() -> RpcSettings {
        let file = settings_path()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or(Value::Null);
        RpcSettings::resolve(&|k| std::env::var(k).ok(), &file)
    }
}

/// The settings file (`forge_platform::settings_path`): `$FORGE_SETTINGS`, else the per-OS config
/// directory (docs/specs/PLATFORM.md).
pub fn settings_path() -> Option<PathBuf> {
    forge_platform::settings_path()
}

/// Constant-time equality: the loop always runs over the longer input and has no early exit.
pub fn ct_eq(a: &[u8], b: &[u8]) -> bool {
    let mut diff = a.len() ^ b.len();
    for i in 0..a.len().max(b.len()) {
        let x = a.get(i).copied().unwrap_or(0);
        let y = b.get(i).copied().unwrap_or(0);
        diff |= usize::from(x ^ y);
    }
    std::hint::black_box(diff) == 0
}
