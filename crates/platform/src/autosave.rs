//! Autosave files, schedule, session markers and the background writer
//! (docs/specs/PLATFORM.md §7).
//!
//! Everything here is a function of its arguments (paths, a clock, a settings value): the
//! editor joins it to the document in `forge-ui`. Tests use temp directories and
//! [`ManualClock`].

use std::fs::{self, File, OpenOptions, TryLockError};
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Sender, channel};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use serde_json::{Value, json};

/// Suffix of the data file of an autosave entry.
pub const DATA_SUFFIX: &str = ".forge.autosave";
/// Suffix of the metadata file next to it.
pub const META_SUFFIX: &str = ".forge.autosave.json";
/// Folder of the session markers inside the autosave directory.
pub const SESSIONS_DIR: &str = "sessions";
/// Name of the autosave folder inside the data directory.
pub const DIR_NAME: &str = "autosave";

pub const DEFAULT_INTERVAL_SECONDS: u64 = 300;
pub const DEFAULT_MAX_FILES: usize = 10;

/// `$FORGE_AUTOSAVE_DIR`, else `<data>/autosave`; `None` without either.
pub fn autosave_dir_with(
    env: &dyn Fn(&str) -> Option<std::ffi::OsString>,
    data: Option<&Path>,
) -> Option<PathBuf> {
    if let Some(d) = env("FORGE_AUTOSAVE_DIR").filter(|d| !d.is_empty()) {
        return Some(PathBuf::from(d));
    }
    data.map(|d| d.join(DIR_NAME))
}

/// The autosave directory of this process.
pub fn autosave_dir() -> Option<PathBuf> {
    let dirs = crate::dirs();
    autosave_dir_with(
        &|k| std::env::var_os(k),
        dirs.as_ref().map(|d| d.data.as_path()),
    )
}

// ---------------------------------------------------------------- settings

/// The `autosave` object of the settings file (§7.1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Settings {
    pub enabled: bool,
    pub interval_seconds: u64,
    pub max_files: usize,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            enabled: true,
            interval_seconds: DEFAULT_INTERVAL_SECONDS,
            max_files: DEFAULT_MAX_FILES,
        }
    }
}

impl Settings {
    /// From the whole settings file; anything missing or of the wrong type is the default.
    pub fn from_json(file: &Value) -> Settings {
        let a = &file["autosave"];
        let d = Settings::default();
        Settings {
            enabled: a["enabled"].as_bool().unwrap_or(d.enabled),
            interval_seconds: a["interval_seconds"]
                .as_u64()
                .map_or(d.interval_seconds, |s| s.clamp(1, 86_400)),
            max_files: a["max_files"].as_u64().map_or(d.max_files, |n| {
                usize::try_from(n).unwrap_or(1000).clamp(1, 1000)
            }),
        }
    }

    /// Reads the settings file; a missing or broken file gives the defaults.
    pub fn load(path: Option<&Path>) -> Settings {
        let file = path
            .and_then(|p| fs::read_to_string(p).ok())
            .and_then(|t| serde_json::from_str::<Value>(&t).ok())
            .unwrap_or(Value::Null);
        Settings::from_json(&file)
    }
}

// ---------------------------------------------------------------- clock

/// Time source, injected so the schedule is testable.
pub trait Clock: Send {
    /// Monotonic seconds (any origin).
    fn now(&self) -> f64;
    /// Seconds since the Unix epoch.
    fn unix(&self) -> u64;
}

/// The real clock.
pub struct SystemClock(Instant);

impl SystemClock {
    pub fn new() -> SystemClock {
        SystemClock(Instant::now())
    }
}

impl Default for SystemClock {
    fn default() -> Self {
        SystemClock::new()
    }
}

impl Clock for SystemClock {
    fn now(&self) -> f64 {
        self.0.elapsed().as_secs_f64()
    }

    fn unix(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_secs())
    }
}

/// A clock a test moves by hand; clones share the time.
#[derive(Clone, Default)]
pub struct ManualClock(Arc<Mutex<(f64, u64)>>);

impl ManualClock {
    pub fn new(unix: u64) -> ManualClock {
        ManualClock(Arc::new(Mutex::new((0.0, unix))))
    }

    /// Moves both clocks forward.
    pub fn advance(&self, seconds: f64) {
        let mut t = self.0.lock().expect("clock lock");
        t.0 += seconds;
        t.1 += seconds as u64;
    }
}

impl Clock for ManualClock {
    fn now(&self) -> f64 {
        self.0.lock().expect("clock lock").0
    }

    fn unix(&self) -> u64 {
        self.0.lock().expect("clock lock").1
    }
}

/// `YYYY-MM-DD HH:MM` in UTC (civil-from-days, Howard Hinnant's public-domain algorithm).
pub fn format_utc(unix: u64) -> String {
    let days = (unix / 86_400) as i64;
    let secs = unix % 86_400;
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!(
        "{year:04}-{month:02}-{day:02} {:02}:{:02}",
        secs / 3600,
        secs % 3600 / 60
    )
}

// ---------------------------------------------------------------- naming

/// `<unix seconds>-<pid>`: unique per process start.
pub fn session_id(unix: u64, pid: u32) -> String {
    format!("{unix}-{pid}")
}

fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in bytes {
        h ^= u64::from(*b);
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

/// The stable name of a document's autosave (§7.2).
pub fn doc_key(session: &str, path: Option<&Path>) -> String {
    let Some(path) = path else {
        return format!("untitled-{session}");
    };
    let abs = std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf());
    let stem: String = path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default()
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-') {
                c
            } else {
                '_'
            }
        })
        .take(40)
        .collect();
    let stem = if stem.is_empty() {
        "scene".to_string()
    } else {
        stem
    };
    format!("{stem}-{:016x}", fnv1a64(abs.to_string_lossy().as_bytes()))
}

pub fn data_path(dir: &Path, key: &str) -> PathBuf {
    dir.join(format!("{key}{DATA_SUFFIX}"))
}

pub fn meta_path(dir: &Path, key: &str) -> PathBuf {
    dir.join(format!("{key}{META_SUFFIX}"))
}

// ---------------------------------------------------------------- schedule

/// When to autosave (§7.3). Pure: the caller passes the clock and the revisions.
#[derive(Debug, Clone, PartialEq)]
pub struct Schedule {
    last: f64,
    last_autosaved: Option<u64>,
}

impl Schedule {
    pub fn new(now: f64) -> Schedule {
        Schedule {
            last: now,
            last_autosaved: None,
        }
    }

    /// True when an autosave should happen now.
    pub fn due(&self, s: &Settings, now: f64, revision: u64, saved_revision: u64) -> bool {
        s.enabled
            && now - self.last >= s.interval_seconds as f64
            && revision != saved_revision
            && self.last_autosaved != Some(revision)
    }

    /// An autosave of `revision` was queued at `now`.
    pub fn attempted(&mut self, now: f64, revision: u64) {
        self.last = now;
        self.last_autosaved = Some(revision);
    }

    /// The document was saved, replaced or recovered.
    pub fn reset(&mut self, now: f64) {
        self.last = now;
        self.last_autosaved = None;
    }

    /// Seconds until the next possible autosave (0 when the interval has passed).
    pub fn remaining(&self, s: &Settings, now: f64) -> f64 {
        (s.interval_seconds as f64 - (now - self.last)).max(0.0)
    }
}

// ---------------------------------------------------------------- entries

/// The metadata file of an entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Meta {
    pub session: String,
    /// The document's file path; `None` for an untitled document.
    pub original: Option<String>,
    pub saved_at: u64,
}

impl Meta {
    fn to_json(&self) -> Value {
        json!({"session": self.session, "original": self.original, "saved_at": self.saved_at})
    }

    fn from_json(v: &Value) -> Option<Meta> {
        Some(Meta {
            session: v["session"].as_str()?.to_string(),
            original: v["original"].as_str().map(String::from),
            saved_at: v["saved_at"].as_u64().unwrap_or(0),
        })
    }
}

/// One autosave found on disk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub key: String,
    pub meta: Option<Meta>,
    pub data: PathBuf,
    pub size: u64,
}

impl Entry {
    /// Display name: the original file name, or `Untitled`.
    pub fn name(&self) -> String {
        self.meta
            .as_ref()
            .and_then(|m| m.original.as_deref())
            .and_then(|p| Path::new(p).file_name())
            .map_or_else(
                || "Untitled".to_string(),
                |n| n.to_string_lossy().into_owned(),
            )
    }

    fn saved_at(&self) -> u64 {
        self.meta.as_ref().map_or(0, |m| m.saved_at)
    }
}

/// Writes `bytes` to `path` atomically: a temp file in the same folder, synced, renamed.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let tmp = path.with_file_name(format!(
        "{}.tmp",
        path.file_name().map_or_else(
            || "autosave".to_string(),
            |n| n.to_string_lossy().into_owned()
        )
    ));
    let result = (|| {
        let mut f = File::create(&tmp)?;
        f.write_all(bytes)?;
        f.sync_all()?;
        drop(f);
        fs::rename(&tmp, path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    result
}

/// Writes an entry: metadata first, then the data (each atomically).
pub fn write_entry(dir: &Path, key: &str, meta: &Meta, bytes: &[u8]) -> std::io::Result<PathBuf> {
    fs::create_dir_all(dir)?;
    write_atomic(&meta_path(dir, key), meta.to_json().to_string().as_bytes())?;
    let data = data_path(dir, key);
    write_atomic(&data, bytes)?;
    Ok(data)
}

/// Removes an entry's two files (missing files are fine).
pub fn remove_entry(dir: &Path, key: &str) {
    let _ = fs::remove_file(data_path(dir, key));
    let _ = fs::remove_file(meta_path(dir, key));
}

/// Every entry in `dir` (those with a data file), newest first.
pub fn list_entries(dir: &Path) -> Vec<Entry> {
    let Ok(rd) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut out: Vec<Entry> = rd
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            let key = name.strip_suffix(DATA_SUFFIX)?.to_string();
            let size = e.metadata().ok()?.len();
            let meta = fs::read_to_string(meta_path(dir, &key))
                .ok()
                .and_then(|t| serde_json::from_str::<Value>(&t).ok())
                .and_then(|v| Meta::from_json(&v));
            Some(Entry {
                key,
                meta,
                data: e.path(),
                size,
            })
        })
        .collect();
    out.sort_by(|a, b| {
        b.saved_at()
            .cmp(&a.saved_at())
            .then_with(|| a.key.cmp(&b.key))
    });
    out
}

/// Keys to remove so that at most `keep` entries remain: the newest stay, and `protect`
/// (the entry just written) always stays. `entries` must be newest first.
pub fn prune(entries: &[Entry], keep: usize, protect: &str) -> Vec<String> {
    let mut kept = 0;
    let mut remove = Vec::new();
    // The protected entry takes one of the slots first.
    let protected = entries.iter().any(|e| e.key == protect);
    for e in entries {
        if e.key == protect {
            continue;
        }
        if kept + usize::from(protected) < keep {
            kept += 1;
        } else {
            remove.push(e.key.clone());
        }
    }
    remove
}

/// Applies [`prune`] to `dir`; also removes orphan metadata and stray temp files.
pub fn prune_dir(dir: &Path, keep: usize, protect: &str) {
    for key in prune(&list_entries(dir), keep, protect) {
        remove_entry(dir, &key);
    }
    let Ok(rd) = fs::read_dir(dir) else {
        return;
    };
    for e in rd.flatten() {
        let name = e.file_name().to_string_lossy().into_owned();
        let orphan_meta = name
            .strip_suffix(META_SUFFIX)
            .is_some_and(|k| !data_path(dir, k).exists());
        if orphan_meta || name.ends_with(".tmp") {
            let _ = fs::remove_file(e.path());
        }
    }
}

// ---------------------------------------------------------------- session markers

/// This process's marker file with its exclusive lock (§7.5). Dropping it without
/// [`SessionMarker::release`] leaves the file behind with the lock gone: what a crash does.
pub struct SessionMarker {
    path: PathBuf,
    file: File,
}

fn marker_dir(dir: &Path) -> PathBuf {
    dir.join(SESSIONS_DIR)
}

fn marker_path(dir: &Path, session: &str) -> PathBuf {
    marker_dir(dir).join(format!("{session}.lock"))
}

impl SessionMarker {
    /// Creates and locks `sessions/<id>.lock`.
    pub fn acquire(dir: &Path, session: &str) -> std::io::Result<SessionMarker> {
        fs::create_dir_all(marker_dir(dir))?;
        let path = marker_path(dir, session);
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&path)?;
        file.try_lock()
            .map_err(|e| std::io::Error::other(format!("session marker is locked: {e}")))?;
        file.set_len(0)?;
        let _ = writeln!(file, "pid {}", std::process::id());
        Ok(SessionMarker { path, file })
    }

    /// Clean exit: unlocks and deletes the marker.
    pub fn release(self) {
        let _ = self.file.unlock();
        drop(self.file);
        let _ = fs::remove_file(&self.path);
    }
}

/// The ids of sessions whose marker exists but is not locked (they crashed).
pub fn crashed_sessions(dir: &Path) -> Vec<String> {
    let Ok(rd) = fs::read_dir(marker_dir(dir)) else {
        return Vec::new();
    };
    let mut out: Vec<String> = rd
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            let id = name.strip_suffix(".lock")?.to_string();
            let f = OpenOptions::new()
                .read(true)
                .write(true)
                .open(e.path())
                .ok()?;
            match f.try_lock() {
                Ok(()) => {
                    let _ = f.unlock();
                    Some(id)
                }
                Err(TryLockError::WouldBlock | TryLockError::Error(_)) => None,
            }
        })
        .collect();
    out.sort();
    out
}

/// Deletes the markers of crashed sessions that have no recoverable entry left.
pub fn clear_stale_markers(dir: &Path, crashed: &[String], remaining: &[Entry]) {
    for id in crashed {
        let in_use = remaining
            .iter()
            .any(|e| e.meta.as_ref().is_some_and(|m| &m.session == id));
        if !in_use {
            let _ = fs::remove_file(marker_path(dir, id));
        }
    }
}

/// The entries of crashed sessions (§7.5), newest first.
pub fn recoverable(dir: &Path, crashed: &[String]) -> Vec<Entry> {
    list_entries(dir)
        .into_iter()
        .filter(|e| {
            e.meta
                .as_ref()
                .is_some_and(|m| crashed.iter().any(|c| c == &m.session))
        })
        .collect()
}

// ---------------------------------------------------------------- background writer

enum Job {
    Write {
        key: String,
        meta: Meta,
        bytes: Vec<u8>,
    },
    Remove(String),
    Flush(Sender<()>),
}

/// One thread that does the disk work, in the order jobs are queued (§7.4).
pub struct Writer {
    tx: Option<Sender<Job>>,
    handle: Option<JoinHandle<()>>,
    errors: Arc<Mutex<Vec<String>>>,
}

impl Writer {
    /// Starts the thread for `dir`; every write prunes to `max_files`.
    pub fn spawn(dir: PathBuf, max_files: usize) -> Writer {
        let (tx, rx) = channel::<Job>();
        let errors = Arc::new(Mutex::new(Vec::new()));
        let errs = errors.clone();
        let handle = std::thread::Builder::new()
            .name("forge-autosave".into())
            .spawn(move || {
                for job in rx {
                    match job {
                        Job::Write { key, meta, bytes } => {
                            match write_entry(&dir, &key, &meta, &bytes) {
                                Ok(_) => prune_dir(&dir, max_files, &key),
                                Err(e) => errs
                                    .lock()
                                    .expect("error lock")
                                    .push(format!("autosave of {key} failed: {e}")),
                            }
                        }
                        Job::Remove(key) => remove_entry(&dir, &key),
                        Job::Flush(done) => {
                            let _ = done.send(());
                        }
                    }
                }
            })
            .expect("spawn the autosave thread");
        Writer {
            tx: Some(tx),
            handle: Some(handle),
            errors,
        }
    }

    fn send(&self, job: Job) {
        if let Some(tx) = &self.tx {
            let _ = tx.send(job);
        }
    }

    /// Queues the write of an entry.
    pub fn write(&self, key: &str, meta: Meta, bytes: Vec<u8>) {
        self.send(Job::Write {
            key: key.to_string(),
            meta,
            bytes,
        });
    }

    /// Queues the removal of an entry.
    pub fn remove(&self, key: &str) {
        self.send(Job::Remove(key.to_string()));
    }

    /// Waits until everything queued so far is on disk.
    pub fn flush(&self) {
        let (done_tx, done_rx) = channel();
        self.send(Job::Flush(done_tx));
        let _ = done_rx.recv();
    }

    /// Errors of finished jobs since the last call.
    pub fn take_errors(&self) -> Vec<String> {
        std::mem::take(&mut *self.errors.lock().expect("error lock"))
    }
}

impl Drop for Writer {
    fn drop(&mut self) {
        self.tx = None;
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp() -> tempfile::TempDir {
        tempfile::tempdir().expect("temp dir")
    }

    fn meta(session: &str, original: Option<&str>, at: u64) -> Meta {
        Meta {
            session: session.into(),
            original: original.map(String::from),
            saved_at: at,
        }
    }

    #[test]
    fn settings_defaults_and_clamps() {
        let d = Settings::from_json(&Value::Null);
        assert_eq!(d, Settings::default());
        assert!(d.enabled);
        assert_eq!(d.interval_seconds, 300);
        let s = Settings::from_json(&json!({"autosave": {
        "enabled": false, "interval_seconds": 0, "max_files": 5000}}));
        assert_eq!(
            (s.enabled, s.interval_seconds, s.max_files),
            (false, 1, 1000)
        );
        let s = Settings::from_json(&json!({"autosave": {
        "enabled": "yes", "interval_seconds": "soon", "max_files": -3}}));
        assert_eq!(s, Settings::default());
        let s = Settings::from_json(&json!({"autosave": {"interval_seconds": 99999999}}));
        assert_eq!(s.interval_seconds, 86_400);
    }

    #[test]
    fn settings_load_tolerates_missing_and_broken_files() {
        let t = tmp();
        assert_eq!(Settings::load(None), Settings::default());
        assert_eq!(
            Settings::load(Some(&t.path().join("none.json"))),
            Settings::default()
        );
        let p = t.path().join("s.json");
        fs::write(&p, "{ not json").unwrap();
        assert_eq!(Settings::load(Some(&p)), Settings::default());
        fs::write(
            &p,
            r#"{"rpc":{"port":1},"autosave":{"interval_seconds":60}}"#,
        )
        .unwrap();
        assert_eq!(Settings::load(Some(&p)).interval_seconds, 60);
    }

    #[test]
    fn dir_prefers_the_override() {
        let none = |_: &str| None;
        assert_eq!(autosave_dir_with(&none, None), None);
        assert_eq!(
            autosave_dir_with(&none, Some(Path::new("/d/forge3d"))),
            Some(PathBuf::from("/d/forge3d/autosave"))
        );
        let env = |k: &str| (k == "FORGE_AUTOSAVE_DIR").then(|| "/x/y".into());
        assert_eq!(
            autosave_dir_with(&env, Some(Path::new("/d"))),
            Some(PathBuf::from("/x/y"))
        );
        let empty = |k: &str| (k == "FORGE_AUTOSAVE_DIR").then(|| "".into());
        assert_eq!(
            autosave_dir_with(&empty, Some(Path::new("/d"))),
            Some(PathBuf::from("/d/autosave"))
        );
    }

    #[test]
    fn keys_are_stable_and_distinguish_folders() {
        let s = session_id(1_730_000_000, 4242);
        assert_eq!(s, "1730000000-4242");
        assert_eq!(doc_key(&s, None), "untitled-1730000000-4242");
        let (pa, pb) = (
            Path::new("/work/a/scene.forge"),
            Path::new("/work/b/scene.forge"),
        );
        let a = doc_key(&s, Some(pa));
        let b = doc_key(&s, Some(pb));
        assert_ne!(a, b);
        assert_eq!(a, doc_key("other-session", Some(pa)));
        assert!(a.starts_with("scene-"));
        assert_eq!(a.len(), "scene-".len() + 16);
        // odd characters and empty stems
        let odd = doc_key(&s, Some(Path::new("/w/my scene (2)!.forge")));
        assert!(odd.starts_with("my_scene__2__-"), "{odd}");
        assert!(doc_key(&s, Some(Path::new("/w/.forge"))).starts_with(".forge-"));
        let long = doc_key(
            &s,
            Some(Path::new(&format!("/w/{}.forge", "x".repeat(200)))),
        );
        assert_eq!(long.len(), 40 + 1 + 16);
    }

    #[test]
    fn schedule_waits_for_interval_dirty_and_new_revision() {
        let s = Settings {
            interval_seconds: 300,
            ..Settings::default()
        };
        let mut sch = Schedule::new(0.0);
        // too early
        assert!(!sch.due(&s, 299.0, 5, 1));
        // on time and dirty
        assert!(sch.due(&s, 300.0, 5, 1));
        // clean (saved revision equals the revision)
        assert!(!sch.due(&s, 900.0, 5, 5));
        // disabled
        let off = Settings {
            enabled: false,
            ..s.clone()
        };
        assert!(!sch.due(&off, 900.0, 5, 1));
        // after an autosave: not again for the same revision, even much later
        sch.attempted(300.0, 5);
        assert!(!sch.due(&s, 5000.0, 5, 1));
        // changed, but the interval restarted at the autosave
        assert!(!sch.due(&s, 599.0, 6, 1));
        assert!(sch.due(&s, 600.0, 6, 1));
        // a save (or new document) resets
        sch.reset(1000.0);
        assert!(!sch.due(&s, 1200.0, 5, 1));
        assert!(sch.due(&s, 1300.0, 5, 1));
        assert_eq!(sch.remaining(&s, 1100.0), 200.0);
        assert_eq!(sch.remaining(&s, 5000.0), 0.0);
    }

    #[test]
    fn manual_clock_is_shared_between_clones() {
        let c = ManualClock::new(1000);
        let d = c.clone();
        c.advance(61.0);
        assert_eq!((d.now(), d.unix()), (61.0, 1061));
    }

    #[test]
    fn utc_formatting() {
        assert_eq!(format_utc(0), "1970-01-01 00:00");
        assert_eq!(
            format_utc(951_782_400 + 3600 * 23 + 60 * 59),
            "2000-02-29 23:59"
        );
        assert_eq!(format_utc(1_730_000_000), "2024-10-27 03:33");
    }

    #[test]
    fn atomic_write_replaces_without_leaving_temp_files() {
        let t = tmp();
        let p = t.path().join("x.forge.autosave");
        write_atomic(&p, b"one").unwrap();
        write_atomic(&p, b"two").unwrap();
        assert_eq!(fs::read(&p).unwrap(), b"two");
        let names: Vec<_> = fs::read_dir(t.path())
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        assert_eq!(names, ["x.forge.autosave"]);
        // a failing write (the folder is missing) leaves nothing and an error
        assert!(write_atomic(&t.path().join("nope/x"), b"a").is_err());
    }

    #[test]
    fn entries_round_trip_and_sort_newest_first() {
        let t = tmp();
        write_entry(t.path(), "a", &meta("s1", Some("/w/a.forge"), 10), b"A").unwrap();
        write_entry(t.path(), "b", &meta("s1", None, 30), b"BB").unwrap();
        write_entry(t.path(), "c", &meta("s2", None, 20), b"CCC").unwrap();
        let list = list_entries(t.path());
        let keys: Vec<_> = list.iter().map(|e| e.key.as_str()).collect();
        assert_eq!(keys, ["b", "c", "a"]);
        assert_eq!(list[0].name(), "Untitled");
        assert_eq!(list[2].name(), "a.forge");
        assert_eq!(list[1].size, 3);
        assert_eq!(fs::read(&list[2].data).unwrap(), b"A");
        remove_entry(t.path(), "b");
        assert_eq!(list_entries(t.path()).len(), 2);
        remove_entry(t.path(), "missing");
        // data without metadata is still listed (a crash between the two renames)
        fs::remove_file(meta_path(t.path(), "c")).unwrap();
        let list = list_entries(t.path());
        assert_eq!(list.len(), 2);
        assert!(list.iter().find(|e| e.key == "c").unwrap().meta.is_none());
    }

    fn fake(key: &str, at: u64) -> Entry {
        Entry {
            key: key.into(),
            meta: Some(meta("s", None, at)),
            data: PathBuf::new(),
            size: 0,
        }
    }

    #[test]
    fn prune_keeps_newest_and_the_protected_entry() {
        let e = [fake("d", 40), fake("c", 30), fake("b", 20), fake("a", 10)];
        assert!(prune(&e, 4, "x").is_empty());
        assert_eq!(prune(&e, 2, "x"), ["b", "a"]);
        // the protected one is old but stays; it takes a slot
        assert_eq!(prune(&e, 2, "a"), ["c", "b"]);
        assert_eq!(prune(&e, 1, "a"), ["d", "c", "b"]);
        assert_eq!(prune(&e, 1, "d"), ["c", "b", "a"]);
        // keep 0 never removes the protected entry either
        assert_eq!(prune(&e, 0, "c"), ["d", "b", "a"]);
    }

    #[test]
    fn prune_dir_removes_files_orphans_and_temp_files() {
        let t = tmp();
        for (k, at) in [("a", 1), ("b", 2), ("c", 3)] {
            write_entry(t.path(), k, &meta("s", None, at), b"x").unwrap();
        }
        fs::write(t.path().join("z.forge.autosave.json"), r#"{"session":"s"}"#).unwrap();
        fs::write(t.path().join("q.forge.autosave.tmp"), b"half").unwrap();
        prune_dir(t.path(), 2, "c");
        let keys: Vec<_> = list_entries(t.path()).into_iter().map(|e| e.key).collect();
        assert_eq!(keys, ["c", "b"]);
        assert!(!meta_path(t.path(), "a").exists());
        assert!(!meta_path(t.path(), "z").exists());
        assert!(!t.path().join("q.forge.autosave.tmp").exists());
    }

    #[test]
    fn markers_tell_a_crash_from_a_running_session_and_a_clean_exit() {
        let t = tmp();
        assert!(crashed_sessions(t.path()).is_empty());
        let running = SessionMarker::acquire(t.path(), "running").unwrap();
        let clean = SessionMarker::acquire(t.path(), "clean").unwrap();
        let crashed = SessionMarker::acquire(t.path(), "crashed").unwrap();
        clean.release();
        drop(crashed); // the lock goes, the file stays
        assert_eq!(crashed_sessions(t.path()), ["crashed"]);
        // scanning does not disturb the running session's lock
        assert_eq!(crashed_sessions(t.path()), ["crashed"]);
        running.release();
        assert_eq!(crashed_sessions(t.path()), ["crashed"]);
        // a second process cannot take over a live marker
        let live = SessionMarker::acquire(t.path(), "live").unwrap();
        assert!(SessionMarker::acquire(t.path(), "live").is_err());
        live.release();
    }

    #[test]
    fn recoverable_entries_belong_to_crashed_sessions_only() {
        let t = tmp();
        write_entry(t.path(), "a", &meta("crashed", None, 3), b"a").unwrap();
        write_entry(t.path(), "b", &meta("running", None, 2), b"b").unwrap();
        write_entry(t.path(), "c", &meta("gone", None, 1), b"c").unwrap();
        fs::write(data_path(t.path(), "d"), b"no meta").unwrap();
        let crashed = vec!["crashed".to_string()];
        let r = recoverable(t.path(), &crashed);
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].key, "a");

        // stale markers stay while an entry remains, and go when none does
        let m = SessionMarker::acquire(t.path(), "crashed").unwrap();
        drop(m);
        clear_stale_markers(t.path(), &crashed, &r);
        assert_eq!(crashed_sessions(t.path()), ["crashed"]);
        remove_entry(t.path(), "a");
        clear_stale_markers(t.path(), &crashed, &recoverable(t.path(), &crashed));
        assert!(crashed_sessions(t.path()).is_empty());
    }

    #[test]
    fn writer_runs_jobs_in_order_and_flushes() {
        let t = tmp();
        let w = Writer::spawn(t.path().to_path_buf(), 2);
        w.write("a", meta("s", None, 1), b"1".to_vec());
        w.write("a", meta("s", None, 2), b"2".to_vec());
        w.write("b", meta("s", None, 3), b"3".to_vec());
        w.write("c", meta("s", None, 4), b"4".to_vec());
        w.remove("b");
        w.flush();
        let list = list_entries(t.path());
        let keys: Vec<_> = list.iter().map(|e| e.key.as_str()).collect();
        // a was pruned when c arrived (keep 2: c and b), then b was removed
        assert_eq!(keys, ["c"]);
        assert!(w.take_errors().is_empty());
        // an unwritable target reports instead of panicking
        let bad = Writer::spawn(t.path().join("c.forge.autosave"), 2);
        bad.write("x", meta("s", None, 1), b"1".to_vec());
        bad.flush();
        assert_eq!(bad.take_errors().len(), 1);
    }
}
