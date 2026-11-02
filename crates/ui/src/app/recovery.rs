//! The "Recover unsaved work" window and what its buttons do (docs/specs/UI.md §15).

use std::path::Path;

use forge_command::CommandContext;
use forge_core::file::{self, LoadOptions};
use forge_platform::autosave::format_utc;

use super::Editor;

enum Action {
    Recover(usize),
    Discard(usize),
    DiscardAll,
}

impl Editor {
    /// Autosaves of crashed sessions still waiting for an answer.
    pub fn recoverable_count(&self) -> usize {
        self.state.autosave.pending.len()
    }

    /// Called every frame: autosaves when due and draws the recovery window.
    pub(super) fn autosave_frame(&mut self, ctx: &egui::Context) {
        let path = self.state.file_path.clone();
        let saved = self.state.saved_revision;
        self.state
            .autosave
            .tick(self.state.app.doc.scene(), path.as_deref(), saved);
        if let Some(e) = self.state.autosave.flush_errors().into_iter().next() {
            self.state.status = e;
        }
        if let Some(wait) = self.state.autosave.next_check() {
            ctx.request_repaint_after(std::time::Duration::from_secs_f64(wait.clamp(1.0, 10.0)));
        }
        self.recovery_window(ctx);
        let recoverable = self.recoverable_count();
        self.state.crash_notice.show(ctx, recoverable);
    }

    fn recovery_window(&mut self, ctx: &egui::Context) {
        if self.state.autosave.pending.is_empty() {
            return;
        }
        let mut action = None;
        egui::Window::new("Recover unsaved work")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.label("Forge3D did not exit cleanly. These documents were autosaved:");
                for (i, e) in self.state.autosave.pending.iter().enumerate() {
                    ui.horizontal(|ui| {
                        let when = e
                            .meta
                            .as_ref()
                            .map_or_else(String::new, |m| format_utc(m.saved_at));
                        ui.label(format!(
                            "{}  {when} UTC  {} KB",
                            e.name(),
                            e.size.div_ceil(1024)
                        ));
                        if ui.button("Recover").clicked() {
                            action = Some(Action::Recover(i));
                        }
                        if ui.button("Discard").clicked() {
                            action = Some(Action::Discard(i));
                        }
                    });
                }
                ui.separator();
                if ui.button("Discard all").clicked() {
                    action = Some(Action::DiscardAll);
                }
            });
        match action {
            Some(Action::Recover(i)) => {
                self.recover_autosave(i);
            }
            Some(Action::Discard(i)) => self.discard_autosave(i),
            Some(Action::DiscardAll) => {
                while !self.state.autosave.pending.is_empty() {
                    self.discard_autosave(0);
                }
            }
            None => {}
        }
    }

    /// Removes pending entry `index` without opening it.
    pub fn discard_autosave(&mut self, index: usize) {
        self.state.autosave.resolve_pending(index, true);
    }

    /// Opens pending entry `index` as a dirty document that remembers the original path
    /// (PLATFORM.md §7.7). True if it was opened.
    pub fn recover_autosave(&mut self, index: usize) -> bool {
        let Some(entry) = self.state.autosave.pending.get(index).cloned() else {
            return false;
        };
        let original = entry.meta.as_ref().and_then(|m| m.original.clone());
        let opts = LoadOptions {
            base_dir: original
                .as_deref()
                .and_then(|p| std::path::absolute(p).ok())
                .and_then(|a| a.parent().map(Path::to_path_buf)),
            check_external: false,
        };
        let registry = self.state.doc.scene().registry().clone();
        let loaded = std::fs::read(&entry.data)
            .map_err(|e| e.to_string())
            .and_then(|b| file::load_with(registry, &b, &opts).map_err(|e| e.to_string()));
        let (scene, report) = match loaded {
            Ok(x) => x,
            Err(e) => {
                self.state.status = format!("Recover failed: {e}");
                return false;
            }
        };
        if !self.confirm_discard() {
            return false;
        }
        self.state.reset_document(
            forge_core::Document::from_scene(scene),
            original.clone(),
            false,
        );
        self.state.app.mark_unsaved();
        // Autosave the recovered document at once, then drop the old entry.
        let _ = self
            .state
            .autosave
            .queue_now(self.state.app.doc.scene(), original.as_deref());
        let new_key = self.state.autosave.key_for(original.as_deref());
        let same = new_key == entry.key;
        self.state.autosave.resolve_pending(index, !same);
        self.state.status = report.warnings.join("; ");
        true
    }
}
