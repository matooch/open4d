//! "Forge3D crashed last time": the notice shown on the start after a crash report was written
//! (docs/specs/UI.md §15.2, PLATFORM.md §8.5).

use std::path::{Path, PathBuf};

use forge_platform::crash::{self, Report};

/// Opens a folder in the system file browser.
pub type FolderOpener = Box<dyn FnMut(&Path) + Send>;

/// The unseen crash reports and what the buttons do.
pub struct CrashNotice {
    dir: Option<PathBuf>,
    pub reports: Vec<Report>,
    opener: FolderOpener,
}

impl Default for CrashNotice {
    /// No folder: nothing to show (tests, headless use).
    fn default() -> Self {
        CrashNotice {
            dir: None,
            reports: Vec::new(),
            opener: Box::new(open_in_file_browser),
        }
    }
}

impl CrashNotice {
    /// Lists the unseen reports in `dir`.
    pub fn start(dir: PathBuf) -> CrashNotice {
        let reports = crash::unseen(&dir);
        CrashNotice {
            dir: Some(dir),
            reports,
            opener: Box::new(open_in_file_browser),
        }
    }

    /// The crash folder of this process.
    pub fn start_default() -> CrashNotice {
        crash::crash_dir().map_or_else(CrashNotice::default, CrashNotice::start)
    }

    /// Replaces the folder opener (tests).
    pub fn set_opener(&mut self, opener: FolderOpener) {
        self.opener = opener;
    }

    /// True while the window is up.
    pub fn is_open(&self) -> bool {
        !self.reports.is_empty()
    }

    /// Opens the crash folder and dismisses the notice.
    pub fn open_folder(&mut self) {
        if let Some(dir) = self.dir.clone() {
            (self.opener)(&dir);
        }
        self.dismiss();
    }

    /// Marks the reports seen and closes the window. The files stay.
    pub fn dismiss(&mut self) {
        if let Some(dir) = &self.dir
            && let Err(e) = crash::mark_seen(dir, &self.reports)
        {
            tracing::warn!("cannot record the crash reports as seen: {e}");
        }
        self.reports.clear();
    }

    /// Draws the window. `recoverable` is the number of autosaves the recovery window offers.
    pub fn show(&mut self, ctx: &egui::Context, recoverable: usize) {
        if !self.is_open() {
            return;
        }
        let (mut open, mut dismiss) = (false, false);
        egui::Window::new("Forge3D crashed last time")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_TOP, [0.0, 60.0])
            .show(ctx, |ui| {
                ui.label(
                    "A crash report was written. Please attach it when you report the problem.",
                );
                for r in &self.reports {
                    ui.monospace(&r.name);
                }
                if recoverable > 0 {
                    ui.label("Autosaved work may be offered in the recovery window.");
                }
                ui.horizontal(|ui| {
                    open = ui.button("Open folder").clicked();
                    dismiss = ui.button("Dismiss").clicked();
                });
            });
        if open {
            self.open_folder();
        } else if dismiss {
            self.dismiss();
        }
    }
}

/// `xdg-open` / `open` / `explorer` on `dir`; failures are logged, never fatal.
fn open_in_file_browser(dir: &Path) {
    let program = if cfg!(target_os = "windows") {
        "explorer"
    } else if cfg!(target_os = "macos") {
        "open"
    } else {
        "xdg-open"
    };
    match std::process::Command::new(program).arg(dir).spawn() {
        Ok(mut child) => {
            // Reap it off the UI thread so no zombie is left.
            std::thread::spawn(move || {
                let _ = child.wait();
            });
        }
        Err(e) => tracing::warn!("cannot open {} with {program}: {e}", dir.display()),
    }
}
