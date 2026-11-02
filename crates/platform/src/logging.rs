//! Structured logging to a rotating file (docs/specs/PLATFORM.md §8.1, §8.2).
//!
//! The pure parts ([`LogConfig`], [`RotatingFile`], [`LogTail`]) take their inputs as
//! arguments; [`init`] joins them to the process and installs the `tracing` subscriber.

use std::collections::VecDeque;
use std::ffi::OsString;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

use serde_json::Value;
use tracing_subscriber::Layer;
use tracing_subscriber::filter::Targets;
use tracing_subscriber::fmt::MakeWriter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

/// Folder name of the logs inside the data directory.
pub const DIR_NAME: &str = "logs";
/// The current log file.
pub const FILE_NAME: &str = "forge.log";
pub const DEFAULT_LEVEL: &str = "info";
pub const DEFAULT_MAX_BYTES: u64 = 1024 * 1024;
pub const DEFAULT_MAX_FILES: usize = 5;
/// Lines kept in memory for crash reports.
pub const TAIL_LINES: usize = 200;

/// `$FORGE_LOG_DIR`, else `<data>/logs`; `None` without either.
pub fn log_dir_with(
    env: &dyn Fn(&str) -> Option<OsString>,
    data: Option<&Path>,
) -> Option<PathBuf> {
    if let Some(d) = env("FORGE_LOG_DIR").filter(|d| !d.is_empty()) {
        return Some(PathBuf::from(d));
    }
    data.map(|d| d.join(DIR_NAME))
}

/// The log directory of this process.
pub fn log_dir() -> Option<PathBuf> {
    let dirs = crate::dirs();
    log_dir_with(
        &|k| std::env::var_os(k),
        dirs.as_ref().map(|d| d.data.as_path()),
    )
}

/// Parses a `tracing-subscriber` target filter (`debug`, `warn,forge_ui=debug`).
pub fn parse_filter(text: &str) -> Option<Targets> {
    let t = text.trim();
    if t.is_empty() {
        return None;
    }
    t.parse::<Targets>().ok()
}

/// Logging settings (§8.2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogConfig {
    pub level: String,
    pub max_bytes: u64,
    pub max_files: usize,
    /// A rejected `FORGE_LOG` value, reported once after setup.
    pub rejected_level: Option<String>,
}

impl Default for LogConfig {
    fn default() -> Self {
        LogConfig {
            level: DEFAULT_LEVEL.to_string(),
            max_bytes: DEFAULT_MAX_BYTES,
            max_files: DEFAULT_MAX_FILES,
            rejected_level: None,
        }
    }
}

impl LogConfig {
    /// `env_level` is `FORGE_LOG`; `settings` is the whole settings file.
    pub fn resolve(env_level: Option<&str>, settings: &Value) -> LogConfig {
        let d = LogConfig::default();
        let l = &settings["log"];
        let mut rejected_level = None;
        let mut level = None;
        if let Some(v) = env_level.filter(|v| !v.trim().is_empty()) {
            if parse_filter(v).is_some() {
                level = Some(v.trim().to_string());
            } else {
                rejected_level = Some(v.to_string());
            }
        }
        if level.is_none()
            && let Some(v) = l["level"].as_str()
            && parse_filter(v).is_some()
        {
            level = Some(v.trim().to_string());
        }
        LogConfig {
            level: level.unwrap_or(d.level),
            max_bytes: l["max_bytes"]
                .as_u64()
                .map_or(d.max_bytes, |n| n.clamp(1024, 1 << 30)),
            max_files: l["max_files"].as_u64().map_or(d.max_files, |n| {
                usize::try_from(n).unwrap_or(100).clamp(1, 100)
            }),
            rejected_level,
        }
    }
}

// ---------------------------------------------------------------- rotating file

/// An append-mode log file that rotates by size (§8.2). Lines are never split.
pub struct RotatingFile {
    dir: PathBuf,
    max_bytes: u64,
    max_files: usize,
    file: File,
    size: u64,
}

impl RotatingFile {
    /// Opens (creating the folder) `<dir>/forge.log` for appending.
    pub fn open(dir: &Path, max_bytes: u64, max_files: usize) -> std::io::Result<RotatingFile> {
        fs::create_dir_all(dir)?;
        let path = dir.join(FILE_NAME);
        let file = OpenOptions::new().create(true).append(true).open(&path)?;
        let size = file.metadata()?.len();
        Ok(RotatingFile {
            dir: dir.to_path_buf(),
            max_bytes,
            max_files: max_files.max(1),
            file,
            size,
        })
    }

    /// `forge.log.<i>`.
    pub fn rotated_path(dir: &Path, i: usize) -> PathBuf {
        dir.join(format!("{FILE_NAME}.{i}"))
    }

    fn rotate(&mut self) -> std::io::Result<()> {
        self.file.flush()?;
        let _ = fs::remove_file(Self::rotated_path(&self.dir, self.max_files));
        for i in (1..self.max_files).rev() {
            let from = Self::rotated_path(&self.dir, i);
            if from.exists() {
                let _ = fs::rename(&from, Self::rotated_path(&self.dir, i + 1));
            }
        }
        fs::rename(self.dir.join(FILE_NAME), Self::rotated_path(&self.dir, 1))?;
        self.file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.dir.join(FILE_NAME))?;
        self.size = 0;
        Ok(())
    }

    /// Writes one entry (a whole line or more), rotating first when it would not fit.
    pub fn write_entry(&mut self, bytes: &[u8]) -> std::io::Result<()> {
        if self.size > 0 && self.size + bytes.len() as u64 > self.max_bytes {
            self.rotate()?;
        }
        self.file.write_all(bytes)?;
        self.size += bytes.len() as u64;
        Ok(())
    }
}

// ---------------------------------------------------------------- tail

/// The last lines logged, kept in memory for crash reports.
#[derive(Debug, Clone)]
pub struct LogTail {
    capacity: usize,
    lines: Arc<Mutex<VecDeque<String>>>,
}

impl LogTail {
    pub fn new(capacity: usize) -> LogTail {
        LogTail {
            capacity: capacity.max(1),
            lines: Arc::new(Mutex::new(VecDeque::new())),
        }
    }

    /// Adds every line of `text` (a trailing newline does not add an empty line).
    pub fn push_text(&self, text: &str) {
        let mut lines = self.lines.lock().unwrap_or_else(|e| e.into_inner());
        for line in text.lines() {
            if lines.len() == self.capacity {
                lines.pop_front();
            }
            lines.push_back(line.to_string());
        }
    }

    /// Oldest first. Never blocks: an unavailable tail is reported as empty.
    pub fn snapshot(&self) -> Vec<String> {
        match self.lines.try_lock() {
            Ok(l) => l.iter().cloned().collect(),
            Err(std::sync::TryLockError::Poisoned(e)) => e.into_inner().iter().cloned().collect(),
            Err(std::sync::TryLockError::WouldBlock) => Vec::new(),
        }
    }
}

// ---------------------------------------------------------------- the subscriber

struct Sink {
    file: Mutex<Option<RotatingFile>>,
    tail: LogTail,
    stderr: bool,
}

impl Sink {
    fn emit(&self, bytes: &[u8]) {
        if bytes.is_empty() {
            return;
        }
        self.tail.push_text(&String::from_utf8_lossy(bytes));
        if let Ok(mut f) = self.file.lock()
            && let Some(file) = f.as_mut()
        {
            // A full disk must not take the application down.
            let _ = file.write_entry(bytes);
        }
        if self.stderr {
            let _ = std::io::stderr().write_all(bytes);
        }
    }
}

/// Collects the pieces of one event and hands them to the sink as one entry.
struct EventWriter {
    sink: Arc<Sink>,
    buf: Vec<u8>,
}

impl Write for EventWriter {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.buf.extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl Drop for EventWriter {
    fn drop(&mut self) {
        self.sink.emit(&self.buf);
    }
}

#[derive(Clone)]
struct SinkWriter(Arc<Sink>);

impl<'a> MakeWriter<'a> for SinkWriter {
    type Writer = EventWriter;

    fn make_writer(&'a self) -> EventWriter {
        EventWriter {
            sink: self.0.clone(),
            buf: Vec::new(),
        }
    }
}

static TAIL: OnceLock<LogTail> = OnceLock::new();

/// The last log lines of this process (empty before [`init`]).
pub fn recent_lines() -> Vec<String> {
    TAIL.get().map(LogTail::snapshot).unwrap_or_default()
}

/// What [`init`] set up.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogInfo {
    pub dir: Option<PathBuf>,
    pub level: String,
}

/// Installs the global subscriber once (later calls return `None`). `settings_file` is the
/// shared settings file; the level comes from `FORGE_LOG`, then `log.level`, then `info`.
pub fn init(settings_file: Option<&Path>) -> Option<LogInfo> {
    let settings = settings_file
        .and_then(|p| fs::read_to_string(p).ok())
        .and_then(|t| serde_json::from_str::<Value>(&t).ok())
        .unwrap_or(Value::Null);
    let env_level = std::env::var("FORGE_LOG").ok();
    let config = LogConfig::resolve(env_level.as_deref(), &settings);
    let stderr = cfg!(debug_assertions)
        || std::env::var("FORGE_LOG_STDERR").is_ok_and(|v| v == "1" || v == "true");
    init_with(log_dir(), &config, stderr)
}

/// [`init`] with explicit inputs.
pub fn init_with(dir: Option<PathBuf>, config: &LogConfig, stderr: bool) -> Option<LogInfo> {
    let tail = LogTail::new(TAIL_LINES);
    if TAIL.set(tail.clone()).is_err() {
        return None;
    }
    let file = dir
        .as_deref()
        .and_then(|d| RotatingFile::open(d, config.max_bytes, config.max_files).ok());
    let sink = Arc::new(Sink {
        file: Mutex::new(file),
        tail,
        stderr,
    });
    let filter = parse_filter(&config.level)
        .or_else(|| parse_filter(DEFAULT_LEVEL))
        .unwrap_or_default();
    let layer = tracing_subscriber::fmt::layer()
        .with_ansi(false)
        .with_writer(SinkWriter(sink))
        .with_filter(filter);
    if tracing_subscriber::registry()
        .with(layer)
        .try_init()
        .is_err()
    {
        return None;
    }
    if let Some(bad) = &config.rejected_level {
        tracing::warn!("ignoring invalid FORGE_LOG value {bad:?}");
    }
    tracing::info!(level = %config.level, dir = ?dir, "logging started");
    Some(LogInfo {
        dir,
        level: config.level.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn temp() -> tempfile::TempDir {
        tempfile::tempdir().unwrap()
    }

    #[test]
    fn dir_prefers_the_environment() {
        let env = |k: &str| (k == "FORGE_LOG_DIR").then(|| OsString::from("/x/logs"));
        assert_eq!(
            log_dir_with(&env, Some(Path::new("/d"))),
            Some(PathBuf::from("/x/logs"))
        );
        assert_eq!(
            log_dir_with(&|_| None, Some(Path::new("/d"))),
            Some(PathBuf::from("/d/logs"))
        );
        assert_eq!(log_dir_with(&|_| None, None), None);
    }

    #[test]
    fn level_precedence_and_fallbacks() {
        let s = json!({"log": {"level": "warn"}});
        assert_eq!(LogConfig::resolve(Some("debug"), &s).level, "debug");
        assert_eq!(LogConfig::resolve(None, &s).level, "warn");
        assert_eq!(LogConfig::resolve(None, &Value::Null).level, "info");
        let c = LogConfig::resolve(Some("forge_ui=loud"), &s);
        assert_eq!(c.level, "warn");
        assert_eq!(c.rejected_level.as_deref(), Some("forge_ui=loud"));
        assert_eq!(
            LogConfig::resolve(Some("warn,forge_ui=debug"), &Value::Null).level,
            "warn,forge_ui=debug"
        );
    }

    #[test]
    fn size_settings_are_clamped() {
        let s = json!({"log": {"max_bytes": 1, "max_files": 9999}});
        let c = LogConfig::resolve(None, &s);
        assert_eq!((c.max_bytes, c.max_files), (1024, 100));
        let s = json!({"log": {"max_bytes": "big", "max_files": 0}});
        let c = LogConfig::resolve(None, &s);
        assert_eq!((c.max_bytes, c.max_files), (DEFAULT_MAX_BYTES, 1));
    }

    fn read(dir: &Path, name: &str) -> String {
        fs::read_to_string(dir.join(name)).unwrap_or_default()
    }

    #[test]
    fn rotates_by_size_and_keeps_the_newest_files() {
        let t = temp();
        let mut f = RotatingFile::open(t.path(), 100, 2).unwrap();
        // 40-byte lines: two fit in a file of 100 bytes.
        for i in 0..7 {
            let line = format!("line-{i}-{}\n", "x".repeat(40 - 8));
            assert_eq!(line.len(), 40);
            f.write_entry(line.as_bytes()).unwrap();
        }
        // Files: forge.log (line 6), .1 (lines 4,5), .2 (lines 2,3); lines 0,1 are gone.
        assert!(read(t.path(), "forge.log").contains("line-6"));
        let one = read(t.path(), "forge.log.1");
        assert!(one.contains("line-4") && one.contains("line-5"), "{one}");
        let two = read(t.path(), "forge.log.2");
        assert!(two.contains("line-2") && two.contains("line-3"), "{two}");
        assert!(!t.path().join("forge.log.3").exists());
        for name in ["forge.log", "forge.log.1", "forge.log.2"] {
            assert!(read(t.path(), name).len() <= 100);
        }
        let all = format!("{one}{two}");
        assert!(!all.contains("line-0") && !all.contains("line-1"));
    }

    #[test]
    fn reopening_appends_and_a_long_entry_is_never_split() {
        let t = temp();
        {
            let mut f = RotatingFile::open(t.path(), 1024, 3).unwrap();
            f.write_entry(b"first\n").unwrap();
        }
        let mut f = RotatingFile::open(t.path(), 10, 3).unwrap();
        f.write_entry(b"second line is longer than the limit\n")
            .unwrap();
        assert_eq!(read(t.path(), "forge.log.1"), "first\n");
        assert_eq!(
            read(t.path(), "forge.log"),
            "second line is longer than the limit\n"
        );
    }

    #[test]
    fn tail_keeps_the_last_lines() {
        let tail = LogTail::new(3);
        tail.push_text("a\nb\n");
        tail.push_text("c\nd\ne\n");
        assert_eq!(tail.snapshot(), vec!["c", "d", "e"]);
    }
}
