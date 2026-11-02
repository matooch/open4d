//! Autosave of the open document and the list of crashed sessions' autosaves
//! (docs/specs/PLATFORM.md §7, UI.md §15). The pure rules are in `forge_platform::autosave`.

use std::path::{Path, PathBuf};

use forge_core::{Scene, file};
use forge_platform::autosave::{
    Clock, Entry, Meta, Schedule, SessionMarker, Settings, SystemClock, Writer,
    clear_stale_markers, crashed_sessions, data_path, doc_key, list_entries, recoverable,
    session_id,
};

/// Schedule, background writer, session marker and the recovery list.
pub struct Autosave {
    dir: Option<PathBuf>,
    pub settings: Settings,
    clock: Box<dyn Clock>,
    session: String,
    schedule: Schedule,
    writer: Option<Writer>,
    marker: Option<SessionMarker>,
    crashed: Vec<String>,
    /// Autosaves of crashed sessions that have been neither recovered nor discarded.
    pub pending: Vec<Entry>,
}

impl Default for Autosave {
    /// No directory: nothing is written, nothing is offered (tests, headless use).
    fn default() -> Self {
        Autosave {
            dir: None,
            settings: Settings::default(),
            clock: Box::new(SystemClock::new()),
            session: String::new(),
            schedule: Schedule::new(0.0),
            writer: None,
            marker: None,
            crashed: Vec::new(),
            pending: Vec::new(),
        }
    }
}

impl Autosave {
    /// Starts a session in `dir`: finds the crashed sessions' autosaves, then takes this
    /// session's marker.
    pub fn start(dir: PathBuf, settings: Settings, clock: Box<dyn Clock>) -> Autosave {
        let crashed = crashed_sessions(&dir);
        let pending = recoverable(&dir, &crashed);
        clear_stale_markers(&dir, &crashed, &pending);
        let session = session_id(clock.unix(), std::process::id());
        let marker = SessionMarker::acquire(&dir, &session).ok();
        let writer = Writer::spawn(dir.clone(), settings.max_files);
        Autosave {
            schedule: Schedule::new(clock.now()),
            dir: Some(dir),
            settings,
            clock,
            session,
            writer: Some(writer),
            marker,
            crashed,
            pending,
        }
    }

    /// The real directory, settings file and clock of this process.
    pub fn start_default(settings_file: Option<&Path>) -> Autosave {
        match forge_platform::autosave::autosave_dir() {
            Some(dir) => Autosave::start(
                dir,
                Settings::load(settings_file),
                Box::new(SystemClock::new()),
            ),
            None => Autosave::default(),
        }
    }

    /// True when there is a directory to write to.
    pub fn is_active(&self) -> bool {
        self.dir.is_some()
    }

    pub fn dir(&self) -> Option<&Path> {
        self.dir.as_deref()
    }

    pub fn session(&self) -> &str {
        &self.session
    }

    /// The key of the document with this path (`None`: not saved yet).
    pub fn key_for(&self, path: Option<&str>) -> String {
        doc_key(&self.session, path.map(Path::new))
    }

    /// Seconds until the next possible autosave, for scheduling a repaint; `None` when
    /// periodic autosave is off.
    pub fn next_check(&self) -> Option<f64> {
        (self.is_active() && self.settings.enabled)
            .then(|| self.schedule.remaining(&self.settings, self.clock.now()))
    }

    /// Called every frame: autosaves when the schedule says so (PLATFORM.md §7.3).
    pub fn tick(&mut self, scene: &Scene, path: Option<&str>, saved_revision: u64) {
        if !self.is_active() {
            return;
        }
        let (now, rev) = (self.clock.now(), scene.revision());
        if self.schedule.due(&self.settings, now, rev, saved_revision) {
            // The schedule moves on even when the write fails, so a broken disk is not
            // retried every frame.
            let _ = self.queue_now(scene, path);
        }
    }

    /// Serialises the document here and queues the write on the background thread.
    /// Returns the autosave file's path.
    pub fn queue_now(&mut self, scene: &Scene, path: Option<&str>) -> Result<String, String> {
        let (Some(dir), Some(writer)) = (&self.dir, &self.writer) else {
            return Err("autosave unavailable: no data directory".into());
        };
        let base_dir = path
            .and_then(|p| std::path::absolute(p).ok())
            .and_then(|a| a.parent().map(Path::to_path_buf));
        let bytes = file::save_with(
            scene,
            &file::SaveOptions {
                base_dir,
                ..file::SaveOptions::default()
            },
        );
        let key = doc_key(&self.session, path.map(Path::new));
        let meta = Meta {
            session: self.session.clone(),
            original: path.map(String::from),
            saved_at: self.clock.unix(),
        };
        writer.write(&key, meta, bytes);
        self.schedule.attempted(self.clock.now(), scene.revision());
        Ok(data_path(dir, &key).to_string_lossy().into_owned())
    }

    /// Errors of finished background writes (without waiting).
    pub fn flush_errors(&mut self) -> Vec<String> {
        self.writer
            .as_ref()
            .map_or_else(Vec::new, Writer::take_errors)
    }

    /// Waits for the queued writes and returns the errors the writer met.
    pub fn flush(&mut self) -> Vec<String> {
        match &self.writer {
            Some(w) => {
                w.flush();
                w.take_errors()
            }
            None => Vec::new(),
        }
    }

    fn remove_keys(&mut self, paths: &[Option<&str>]) {
        if let Some(w) = &self.writer {
            for p in paths {
                w.remove(&doc_key(&self.session, p.map(Path::new)));
            }
        }
        self.schedule.reset(self.clock.now());
    }

    /// The document was saved to `new`: its autosaves are no longer needed (§7.6).
    pub fn document_saved(&mut self, old: Option<&str>, new: &str) {
        self.remove_keys(&[None, old, Some(new)]);
    }

    /// The document was replaced (new, open, recover): the old one's autosave goes.
    pub fn document_replaced(&mut self, old: Option<&str>) {
        self.remove_keys(&[None, old]);
    }

    /// Forgets a recovered or discarded entry (and removes its files unless `keep`).
    pub fn resolve_pending(&mut self, index: usize, delete: bool) -> Option<Entry> {
        if index >= self.pending.len() {
            return None;
        }
        let entry = self.pending.remove(index);
        if delete && let Some(w) = &self.writer {
            w.remove(&entry.key);
            w.flush();
        }
        if let Some(dir) = &self.dir {
            clear_stale_markers(dir, &self.crashed, &self.pending);
        }
        Some(entry)
    }

    /// Clean exit: flushes, removes this session's autosaves and marker (§7.6).
    pub fn finish(&mut self) {
        if let Some(w) = self.writer.take() {
            w.flush();
            drop(w);
        }
        if let Some(dir) = &self.dir {
            for e in list_entries(dir) {
                if e.meta.as_ref().is_some_and(|m| m.session == self.session) {
                    forge_platform::autosave::remove_entry(dir, &e.key);
                }
            }
        }
        if let Some(m) = self.marker.take() {
            m.release();
        }
    }

    /// Drops the marker without cleaning up, like a crash (tests).
    pub fn simulate_crash(&mut self) {
        if let Some(w) = self.writer.take() {
            w.flush();
        }
        self.marker = None;
    }
}
