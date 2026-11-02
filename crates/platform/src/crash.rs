//! Crash reports written by the panic hook (docs/specs/PLATFORM.md §8.4, §8.5).
//!
//! [`build_report`] is a pure function of a [`ReportInput`], so the content is tested without
//! panicking. [`install_hook`] feeds it from a real panic.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::autosave::{format_utc, session_id};

/// Folder name of the reports inside the data directory.
pub const DIR_NAME: &str = "crashes";
/// File in the crash folder listing the report names already shown to the user.
pub const SEEN_FILE: &str = ".seen";

/// `$FORGE_CRASH_DIR`, else `<data>/crashes`; `None` without either.
pub fn crash_dir_with(
    env: &dyn Fn(&str) -> Option<std::ffi::OsString>,
    data: Option<&Path>,
) -> Option<PathBuf> {
    if let Some(d) = env("FORGE_CRASH_DIR").filter(|d| !d.is_empty()) {
        return Some(PathBuf::from(d));
    }
    data.map(|d| d.join(DIR_NAME))
}

/// The crash folder of this process.
pub fn crash_dir() -> Option<PathBuf> {
    let dirs = crate::dirs();
    crash_dir_with(
        &|k| std::env::var_os(k),
        dirs.as_ref().map(|d| d.data.as_path()),
    )
}

/// The session id of this process (`<unix seconds>-<pid>`, PLATFORM.md §7.2), chosen once.
pub fn process_session() -> &'static str {
    static SESSION: OnceLock<String> = OnceLock::new();
    SESSION.get_or_init(|| session_id(unix_now(), std::process::id()))
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

static GPU: Mutex<Option<String>> = Mutex::new(None);

/// Records the GPU adapter description for later reports.
pub fn set_gpu_info(text: impl Into<String>) {
    *GPU.lock().unwrap_or_else(|e| e.into_inner()) = Some(text.into());
}

fn gpu_info() -> Option<String> {
    match GPU.try_lock() {
        Ok(g) => g.clone(),
        Err(std::sync::TryLockError::Poisoned(e)) => e.into_inner().clone(),
        Err(std::sync::TryLockError::WouldBlock) => None,
    }
}

/// What the panic said.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PanicDetails {
    pub message: String,
    /// `file:line:column`.
    pub location: Option<String>,
    pub thread: Option<String>,
}

impl PanicDetails {
    pub fn from_hook(info: &std::panic::PanicHookInfo<'_>) -> PanicDetails {
        let payload = info.payload();
        let message = if let Some(s) = payload.downcast_ref::<&str>() {
            (*s).to_string()
        } else if let Some(s) = payload.downcast_ref::<String>() {
            s.clone()
        } else {
            "(non-string panic payload)".to_string()
        };
        PanicDetails {
            message,
            location: info
                .location()
                .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column())),
            thread: std::thread::current().name().map(String::from),
        }
    }
}

/// Everything a report contains.
#[derive(Debug, Clone)]
pub struct ReportInput {
    pub unix: u64,
    pub app: String,
    pub version: String,
    pub session: String,
    pub os: String,
    pub arch: String,
    pub gpu: Option<String>,
    pub panic: PanicDetails,
    pub backtrace: String,
    /// Oldest first.
    pub commands: Vec<String>,
    /// Oldest first.
    pub log_lines: Vec<String>,
    /// Strings that must not appear in the report (the RPC token).
    pub secrets: Vec<String>,
}

/// The report text (§8.4).
pub fn build_report(r: &ReportInput) -> String {
    let mut out = String::new();
    out.push_str("Forge3D crash report\n");
    out.push_str(&format!(
        "Time: {} UTC (unix {})\n",
        format_utc_seconds(r.unix),
        r.unix
    ));
    out.push_str(&format!("Application: {} {}\n", r.app, r.version));
    out.push_str(&format!("Session: {}\n", r.session));
    out.push_str(&format!("OS: {} {}\n", r.os, r.arch));
    out.push_str(&format!("GPU: {}\n", r.gpu.as_deref().unwrap_or("unknown")));
    out.push_str(&format!(
        "Thread: {}\n",
        r.panic.thread.as_deref().unwrap_or("unnamed")
    ));
    out.push_str(&format!("Panic: {}\n", r.panic.message));
    out.push_str(&format!(
        "Location: {}\n",
        r.panic.location.as_deref().unwrap_or("unknown")
    ));
    out.push_str("\nBacktrace:\n");
    out.push_str(r.backtrace.trim_end());
    out.push_str(&format!(
        "\n\nLast {} commands (oldest first):\n",
        r.commands.len()
    ));
    for c in &r.commands {
        out.push_str(c);
        out.push('\n');
    }
    out.push_str(&format!(
        "\nLast {} log lines (oldest first):\n",
        r.log_lines.len()
    ));
    for l in &r.log_lines {
        out.push_str(l);
        out.push('\n');
    }
    redact(out, &r.secrets)
}

fn redact(mut text: String, secrets: &[String]) -> String {
    for s in secrets.iter().filter(|s| !s.is_empty()) {
        text = text.replace(s.as_str(), "<redacted>");
    }
    text
}

/// `YYYY-MM-DD HH:MM:SS` for a Unix time (the autosave formatter gives minutes only).
fn format_utc_seconds(unix: u64) -> String {
    format!("{}:{:02}", format_utc(unix), unix % 60)
}

/// Writes `crash-<session>-<n>.txt` into `dir` (created) and returns its path. The first free
/// `n` is used, so a process that panics twice keeps both reports.
pub fn write_report(dir: &Path, r: &ReportInput) -> std::io::Result<PathBuf> {
    use std::io::Write;
    fs::create_dir_all(dir)?;
    let text = build_report(r);
    for n in 1..1000 {
        let path = dir.join(format!("crash-{}-{n}.txt", r.session));
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
        {
            Ok(mut f) => {
                f.write_all(text.as_bytes())?;
                f.sync_all()?;
                return Ok(path);
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(e),
        }
    }
    Err(std::io::Error::other(
        "too many crash reports for one session",
    ))
}

// ---------------------------------------------------------------- the hook

/// Supplies the last log lines or commands when a panic happens.
pub type LinesSource = fn() -> Vec<String>;

/// How the hook builds a report.
pub struct HookConfig {
    pub app: String,
    pub version: String,
    /// `None`: no report is written (no data folder), the previous hook still runs.
    pub dir: Option<PathBuf>,
    pub commands: LinesSource,
    pub log_lines: LinesSource,
    pub secrets: Vec<String>,
}

static IN_HOOK: AtomicBool = AtomicBool::new(false);

/// Builds and writes the report of one panic. Public so tests can call it with the details of
/// a synthetic panic.
pub fn report_panic(config: &HookConfig, panic: PanicDetails) -> Option<PathBuf> {
    let dir = config.dir.as_deref()?;
    let input = ReportInput {
        unix: unix_now(),
        app: config.app.clone(),
        version: config.version.clone(),
        session: process_session().to_string(),
        os: std::env::consts::OS.to_string(),
        arch: std::env::consts::ARCH.to_string(),
        gpu: gpu_info(),
        panic,
        backtrace: std::backtrace::Backtrace::force_capture().to_string(),
        commands: (config.commands)(),
        log_lines: (config.log_lines)(),
        secrets: config.secrets.clone(),
    };
    write_report(dir, &input).ok()
}

/// Installs the panic hook: write the report, log it, then call the previous hook.
pub fn install_hook(config: HookConfig) {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        // A panic while reporting a panic must not recurse.
        if !IN_HOOK.swap(true, Ordering::SeqCst) {
            let details = PanicDetails::from_hook(info);
            let message = details.message.clone();
            if let Some(path) = report_panic(&config, details) {
                tracing::error!(path = %path.display(), "panic: {message}; crash report written");
            } else {
                tracing::error!("panic: {message}");
            }
            IN_HOOK.store(false, Ordering::SeqCst);
        }
        previous(info);
    }));
}

// ---------------------------------------------------------------- next start

/// A crash report file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Report {
    pub name: String,
    pub path: PathBuf,
}

fn seen_names(dir: &Path) -> BTreeSet<String> {
    fs::read_to_string(dir.join(SEEN_FILE))
        .map(|t| t.lines().map(str::to_string).collect())
        .unwrap_or_default()
}

/// Reports in `dir` not yet marked seen, newest name first (§8.5).
pub fn unseen(dir: &Path) -> Vec<Report> {
    let seen = seen_names(dir);
    let mut found: Vec<Report> = fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            (name.starts_with("crash-") && name.ends_with(".txt") && !seen.contains(&name)).then(
                || Report {
                    name,
                    path: e.path(),
                },
            )
        })
        .collect();
    // Session ids start with the Unix time, so a reverse name sort is newest first.
    found.sort_by(|a, b| b.name.cmp(&a.name));
    found
}

/// Marks the reports as seen (they stay on disk).
pub fn mark_seen(dir: &Path, reports: &[Report]) -> std::io::Result<()> {
    let mut names = seen_names(dir);
    names.extend(reports.iter().map(|r| r.name.clone()));
    let text: String = names.iter().map(|n| format!("{n}\n")).collect();
    fs::write(dir.join(SEEN_FILE), text)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input() -> ReportInput {
        ReportInput {
            unix: 1_730_000_000,
            app: "forge-studio".into(),
            version: "0.1.0".into(),
            session: "1730000000-4242".into(),
            os: "linux".into(),
            arch: "x86_64".into(),
            gpu: Some("Test GPU (Vulkan)".into()),
            panic: PanicDetails {
                message: "index out of bounds: the len is 3".into(),
                location: Some("crates/x/src/lib.rs:10:5".into()),
                thread: Some("main".into()),
            },
            backtrace: "   0: forge::boom\n   1: main\n".into(),
            commands: vec![
                "1 file.new {}".into(),
                r#"2 object.create {"type":"forge.cube"}"#.into(),
            ],
            log_lines: vec!["INFO forge: started".into(), "WARN forge: careful".into()],
            secrets: vec![],
        }
    }

    #[test]
    fn report_has_every_section() {
        let text = build_report(&input());
        for needle in [
            "Forge3D crash report\n",
            "Time: 2024-10-27 03:33:20 UTC (unix 1730000000)\n",
            "Application: forge-studio 0.1.0\n",
            "Session: 1730000000-4242\n",
            "OS: linux x86_64\n",
            "GPU: Test GPU (Vulkan)\n",
            "Thread: main\n",
            "Panic: index out of bounds: the len is 3\n",
            "Location: crates/x/src/lib.rs:10:5\n",
            "\nBacktrace:\n   0: forge::boom\n   1: main\n",
            "Last 2 commands (oldest first):\n1 file.new {}\n2 object.create",
            "Last 2 log lines (oldest first):\nINFO forge: started\nWARN forge: careful\n",
        ] {
            assert!(text.contains(needle), "missing {needle:?} in:\n{text}");
        }
    }

    #[test]
    fn unknown_gpu_thread_and_location_are_marked() {
        let mut i = input();
        i.gpu = None;
        i.panic.thread = None;
        i.panic.location = None;
        let text = build_report(&i);
        assert!(text.contains("GPU: unknown\n"));
        assert!(text.contains("Thread: unnamed\n"));
        assert!(text.contains("Location: unknown\n"));
    }

    #[test]
    fn secrets_never_reach_the_report() {
        let mut i = input();
        i.secrets = vec!["tok-123456".into()];
        i.commands.push(r#"3 x.y {"note":"tok-123456"}"#.into());
        i.log_lines.push("auth tok-123456 ok".into());
        i.panic.message = "bad tok-123456".into();
        let text = build_report(&i);
        assert!(!text.contains("tok-123456"), "{text}");
        assert!(text.contains("<redacted>"));
    }

    #[test]
    fn write_report_never_overwrites() {
        let t = tempfile::tempdir().unwrap();
        let a = write_report(t.path(), &input()).unwrap();
        let b = write_report(t.path(), &input()).unwrap();
        assert_ne!(a, b);
        assert_eq!(
            a.file_name().unwrap().to_str().unwrap(),
            "crash-1730000000-4242-1.txt"
        );
        assert!(
            fs::read_to_string(b)
                .unwrap()
                .starts_with("Forge3D crash report")
        );
    }

    #[test]
    fn report_panic_uses_the_sources() {
        let t = tempfile::tempdir().unwrap();
        let config = HookConfig {
            app: "unit".into(),
            version: "9.9.9".into(),
            dir: Some(t.path().to_path_buf()),
            commands: || vec!["7 a.b {}".to_string()],
            log_lines: || vec!["a log line".to_string()],
            secrets: vec![],
        };
        let path = report_panic(
            &config,
            PanicDetails {
                message: "synthetic".into(),
                location: None,
                thread: None,
            },
        )
        .unwrap();
        let text = fs::read_to_string(path).unwrap();
        assert!(text.contains("Application: unit 9.9.9"));
        assert!(text.contains("Panic: synthetic"));
        assert!(text.contains("7 a.b {}") && text.contains("a log line"));
        assert!(text.contains("Backtrace:"));
        assert!(text.contains(&format!(
            "OS: {} {}",
            std::env::consts::OS,
            std::env::consts::ARCH
        )));
    }

    #[test]
    fn unseen_lists_new_reports_until_marked() {
        let t = tempfile::tempdir().unwrap();
        assert!(unseen(t.path()).is_empty());
        assert!(unseen(&t.path().join("missing")).is_empty());
        fs::write(t.path().join("crash-100-1-1.txt"), "a").unwrap();
        fs::write(t.path().join("crash-200-1-1.txt"), "b").unwrap();
        fs::write(t.path().join("notes.txt"), "c").unwrap();
        let first = unseen(t.path());
        assert_eq!(
            first.iter().map(|r| r.name.as_str()).collect::<Vec<_>>(),
            ["crash-200-1-1.txt", "crash-100-1-1.txt"]
        );
        mark_seen(t.path(), &first[..1]).unwrap();
        assert_eq!(unseen(t.path()).len(), 1);
        fs::write(t.path().join("crash-300-1-1.txt"), "d").unwrap();
        assert_eq!(unseen(t.path()).len(), 2);
        mark_seen(t.path(), &unseen(t.path())).unwrap();
        assert!(unseen(t.path()).is_empty());
        assert!(t.path().join("crash-100-1-1.txt").exists());
    }

    #[test]
    fn crash_dir_follows_the_environment() {
        let env = |k: &str| (k == "FORGE_CRASH_DIR").then(|| "/c".into());
        assert_eq!(crash_dir_with(&env, None), Some(PathBuf::from("/c")));
        assert_eq!(
            crash_dir_with(&|_| None, Some(Path::new("/d"))),
            Some(PathBuf::from("/d/crashes"))
        );
    }
}
