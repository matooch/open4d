//! The Console panel: the in-memory log tail with a level filter, search and copy, and a Rhai
//! REPL with the Script Manager's sandbox and undo rule (docs/specs/UI.md §19).

use std::time::Duration;

use egui::{Color32, Key, KeyboardShortcut, Modifiers, RichText, Ui};

use crate::state::EditorState;
use crate::theme::Theme;

/// Transcript lines kept.
pub const MAX_TRANSCRIPT: usize = 1000;
/// REPL entries kept for Up / Down.
pub const MAX_HISTORY: usize = 200;
/// The colour of warnings in the log (own palette, readable on the panel colour).
const WARN: Color32 = Color32::from_rgb(229, 164, 50);

// ------------------------------------------------------------------ log filtering

/// A log level, least to most severe.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl LogLevel {
    pub const ALL: [LogLevel; 5] = [
        LogLevel::Trace,
        LogLevel::Debug,
        LogLevel::Info,
        LogLevel::Warn,
        LogLevel::Error,
    ];

    /// The drop-down text.
    pub fn label(self) -> &'static str {
        match self {
            LogLevel::Trace => "Trace (all)",
            LogLevel::Debug => "Debug",
            LogLevel::Info => "Info",
            LogLevel::Warn => "Warn",
            LogLevel::Error => "Error",
        }
    }

    fn of_token(token: &str) -> Option<LogLevel> {
        Some(match token {
            "TRACE" => LogLevel::Trace,
            "DEBUG" => LogLevel::Debug,
            "INFO" => LogLevel::Info,
            "WARN" => LogLevel::Warn,
            "ERROR" => LogLevel::Error,
            _ => return None,
        })
    }
}

/// The level of a log line (`<time> <LEVEL> <target>: <message>`): its second token when that is
/// exactly a level name; `None` for any other line (UI.md §19.1).
pub fn parse_level(line: &str) -> Option<LogLevel> {
    line.split_whitespace().nth(1).and_then(LogLevel::of_token)
}

/// One line that passed the filter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogLine {
    /// Position in the unfiltered lines.
    pub index: usize,
    pub level: LogLevel,
    pub text: String,
}

/// The lines at least as severe as `min` that contain `search` (case-insensitive; empty
/// matches all). A line without a level takes the level of the line before it (Info first).
pub fn filter_log(lines: &[String], min: LogLevel, search: &str) -> Vec<LogLine> {
    let needle = search.to_lowercase();
    let mut level = LogLevel::Info;
    let mut out = Vec::new();
    for (index, text) in lines.iter().enumerate() {
        if let Some(l) = parse_level(text) {
            level = l;
        }
        if level >= min && (needle.is_empty() || text.to_lowercase().contains(&needle)) {
            out.push(LogLine {
                index,
                level,
                text: text.clone(),
            });
        }
    }
    out
}

/// The lines as copied to the clipboard.
pub fn log_text(lines: &[LogLine]) -> String {
    lines
        .iter()
        .map(|l| l.text.as_str())
        .collect::<Vec<_>>()
        .join("\n")
}

// ------------------------------------------------------------------ REPL state

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplKind {
    /// The entry as typed.
    Input,
    /// A `print` line.
    Output,
    /// The value of the last expression.
    Result,
    /// A failure message.
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplLine {
    pub text: String,
    pub kind: ReplKind,
}

/// State of the Console panel.
pub struct ConsoleState {
    /// Lowest level shown.
    pub level: LogLevel,
    pub search: String,
    /// Where the log lines come from (the in-memory tail of the process).
    pub log_source: Box<dyn Fn() -> Vec<String> + Send>,
    /// The text of the last Copy.
    pub last_copy: String,
    /// The REPL input field.
    pub input: String,
    pub repl: Vec<ReplLine>,
    history: Vec<String>,
    /// The history entry shown while browsing with Up / Down.
    browsing: Option<usize>,
    /// The text that was being typed when browsing started.
    draft: String,
}

impl Default for ConsoleState {
    fn default() -> Self {
        ConsoleState {
            level: LogLevel::Trace,
            search: String::new(),
            log_source: Box::new(forge_platform::logging::recent_lines),
            last_copy: String::new(),
            input: String::new(),
            repl: Vec::new(),
            history: Vec::new(),
            browsing: None,
            draft: String::new(),
        }
    }
}

impl ConsoleState {
    pub fn push(&mut self, text: impl Into<String>, kind: ReplKind) {
        self.repl.push(ReplLine {
            text: text.into(),
            kind,
        });
        if self.repl.len() > MAX_TRANSCRIPT {
            self.repl.remove(0);
        }
    }

    /// The remembered entries, oldest first.
    pub fn history(&self) -> &[String] {
        &self.history
    }

    /// Remembers a run entry (not a repeat of the last one) and ends browsing.
    pub fn remember(&mut self, entry: &str) {
        self.browsing = None;
        if self.history.last().map(String::as_str) != Some(entry) {
            self.history.push(entry.to_string());
            if self.history.len() > MAX_HISTORY {
                self.history.remove(0);
            }
        }
    }

    /// Up: the entry before the one shown (the newest when not browsing). `current` is the
    /// field's text, kept as the draft when browsing starts. `None` without history.
    pub fn recall_previous(&mut self, current: &str) -> Option<String> {
        if self.history.is_empty() {
            return None;
        }
        let i = match self.browsing {
            None => {
                self.draft = current.to_string();
                self.history.len() - 1
            }
            Some(i) => i.saturating_sub(1),
        };
        self.browsing = Some(i);
        Some(self.history[i].clone())
    }

    /// Down: the entry after the one shown, then the draft. `None` when not browsing.
    pub fn recall_next(&mut self) -> Option<String> {
        let i = self.browsing?;
        if i + 1 < self.history.len() {
            self.browsing = Some(i + 1);
            Some(self.history[i + 1].clone())
        } else {
            self.browsing = None;
            Some(std::mem::take(&mut self.draft))
        }
    }
}

impl EditorState {
    /// Runs one REPL entry on the editor (UI.md §19.2): echo, sandboxed run through the
    /// registry as one undo step, then output and result or error. Blank text does nothing.
    pub fn run_repl(&mut self, source: &str) {
        let source = source.trim_end();
        if source.trim().is_empty() {
            return;
        }
        self.console.remember(source);
        for (i, line) in source.lines().enumerate() {
            let prefix = if i == 0 { "> " } else { ". " };
            self.console
                .push(format!("{prefix}{line}"), ReplKind::Input);
        }
        let commands = self.app.commands.clone();
        let result = forge_script::run_script(
            self,
            &commands,
            source,
            &forge_script::ScriptOptions::default(),
        );
        match result {
            Ok(report) => {
                for line in report.output {
                    self.console.push(line, ReplKind::Output);
                }
                if !report.value.is_null() {
                    self.console
                        .push(format!("=> {}", report.value), ReplKind::Result);
                }
            }
            Err(failure) => {
                for line in &failure.output {
                    self.console.push(line.clone(), ReplKind::Output);
                }
                self.console.push(failure.to_string(), ReplKind::Error);
            }
        }
    }
}

// ------------------------------------------------------------------ drawing

/// Draws the panel.
pub fn console_ui(ui: &mut Ui, state: &mut EditorState) {
    // New log lines appear without any input.
    ui.ctx().request_repaint_after(Duration::from_secs(1));
    ui.columns(2, |cols| {
        log_ui(&mut cols[0], &mut state.console);
        repl_ui(&mut cols[1], state);
    });
}

fn level_color(level: LogLevel, theme: &Theme) -> Option<Color32> {
    match level {
        LogLevel::Error => Some(theme.error),
        LogLevel::Warn => Some(WARN),
        LogLevel::Info => None,
        LogLevel::Debug | LogLevel::Trace => Some(theme.weak_text),
    }
}

fn log_ui(ui: &mut Ui, console: &mut ConsoleState) {
    let theme = Theme::current(ui.ctx());
    let lines = (console.log_source)();
    let shown = filter_log(&lines, console.level, &console.search);
    ui.horizontal(|ui| {
        let l = ui.label("Level");
        egui::ComboBox::from_id_salt("console_level")
            .selected_text(console.level.label())
            .show_ui(ui, |ui| {
                for level in LogLevel::ALL {
                    ui.selectable_value(&mut console.level, level, level.label());
                }
            })
            .response
            .labelled_by(l.id);
        let s = ui.label("Search");
        ui.add(
            egui::TextEdit::singleline(&mut console.search)
                .hint_text("Search log")
                .desired_width(140.0),
        )
        .labelled_by(s.id);
        if ui.button("Copy").clicked() {
            console.last_copy = log_text(&shown);
            ui.ctx().copy_text(console.last_copy.clone());
        }
        ui.weak(format!("{} of {} lines", shown.len(), lines.len()));
    });
    ui.separator();
    egui::ScrollArea::vertical()
        .id_salt("console_log")
        .stick_to_bottom(true)
        .auto_shrink([false, false])
        .show(ui, |ui| {
            for line in &shown {
                let mut text = RichText::new(&line.text).monospace();
                if let Some(c) = level_color(line.level, &theme) {
                    text = text.color(c);
                }
                ui.label(text);
            }
        });
}

fn repl_ui(ui: &mut Ui, state: &mut EditorState) {
    let theme = Theme::current(ui.ctx());
    let id = ui.make_persistent_id("console_repl_input");
    let focused = ui.memory(|m| m.has_focus(id));
    let mut run = false;

    // Take Enter and the history keys before the text field sees them. Shift+Enter is the
    // field's own newline key (`return_key` below), so a plain Enter must not have Shift.
    if focused {
        run = ui.input_mut(|i| !i.modifiers.shift && i.consume_key(Modifiers::NONE, Key::Enter));
        if !state.console.input.contains('\n') {
            let (up, down) = ui.input_mut(|i| {
                (
                    i.consume_key(Modifiers::NONE, Key::ArrowUp),
                    i.consume_key(Modifiers::NONE, Key::ArrowDown),
                )
            });
            let current = state.console.input.clone();
            let recalled = if up {
                state.console.recall_previous(&current)
            } else if down {
                state.console.recall_next()
            } else {
                None
            };
            if let Some(text) = recalled {
                state.console.input = text;
                move_cursor_to_end(ui.ctx(), id, &state.console.input);
            }
        }
    }

    let mut label_id = None;
    ui.horizontal(|ui| {
        label_id = Some(ui.label("REPL input").id);
        if ui.button("Run").clicked() {
            run = true;
        }
        if ui.button("Clear").clicked() {
            state.console.repl.clear();
        }
        ui.weak("Enter runs, Shift+Enter adds a line");
    });

    let transcript_height = (ui.available_height() - 64.0).max(40.0);
    egui::ScrollArea::vertical()
        .id_salt("console_repl")
        .max_height(transcript_height)
        .stick_to_bottom(true)
        .auto_shrink([false, false])
        .show(ui, |ui| {
            for line in &state.console.repl {
                let text = RichText::new(&line.text).monospace();
                match line.kind {
                    ReplKind::Input => ui.label(text.weak()),
                    ReplKind::Output => ui.label(text),
                    ReplKind::Result => ui.label(text.color(theme.accent)),
                    ReplKind::Error => ui.label(text.color(theme.error)),
                };
            }
        });

    let edit = ui.add(
        egui::TextEdit::multiline(&mut state.console.input)
            .id(id)
            .code_editor()
            .desired_rows(2)
            .desired_width(f32::INFINITY)
            .hint_text("Rhai")
            .return_key(KeyboardShortcut::new(Modifiers::SHIFT, Key::Enter)),
    );
    if let Some(l) = label_id {
        edit.labelled_by(l);
    }

    if run {
        let text = std::mem::take(&mut state.console.input);
        state.run_repl(&text);
        ui.memory_mut(|m| m.request_focus(id));
    }
}

fn move_cursor_to_end(ctx: &egui::Context, id: egui::Id, text: &str) {
    let mut st = egui::TextEdit::load_state(ctx, id).unwrap_or_default();
    let end = egui::text::CCursor::new(text.chars().count());
    st.cursor
        .set_char_range(Some(egui::text::CCursorRange::one(end)));
    st.store(ctx, id);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    const SAMPLE: [&str; 6] = [
        "2026-11-03T10:00:00.000000Z  INFO forge_ui: window opened",
        "2026-11-03T10:00:01.000000Z DEBUG forge_command::registry: run command=\"object.create\"",
        "2026-11-03T10:00:02.000000Z  WARN forge_command::registry: command failed command=\"x\" error=boom",
        "2026-11-03T10:00:03.000000Z ERROR forge_rpc::server: handler panic",
        "  continuation of the error",
        "2026-11-03T10:00:04.000000Z TRACE forge_ui: frame",
    ];

    #[test]
    fn levels_are_read_from_the_second_token() {
        let l: Vec<Option<LogLevel>> = SAMPLE.iter().map(|s| parse_level(s)).collect();
        assert_eq!(
            l,
            [
                Some(LogLevel::Info),
                Some(LogLevel::Debug),
                Some(LogLevel::Warn),
                Some(LogLevel::Error),
                None,
                Some(LogLevel::Trace)
            ]
        );
        for junk in [
            "",
            "INFO",
            "ERROR boom",
            "time info x",
            "a b",
            "a  WARNING x",
        ] {
            assert_eq!(parse_level(junk), None, "{junk:?}");
        }
        assert!(LogLevel::Error > LogLevel::Warn && LogLevel::Warn > LogLevel::Info);
        assert!(LogLevel::Info > LogLevel::Debug && LogLevel::Debug > LogLevel::Trace);
    }

    fn indices(v: &[LogLine]) -> Vec<usize> {
        v.iter().map(|l| l.index).collect()
    }

    #[test]
    fn the_level_filter_keeps_that_level_and_above() {
        let all = lines(&SAMPLE);
        assert_eq!(
            indices(&filter_log(&all, LogLevel::Trace, "")),
            [0, 1, 2, 3, 4, 5]
        );
        assert_eq!(
            indices(&filter_log(&all, LogLevel::Debug, "")),
            [0, 1, 2, 3, 4]
        );
        assert_eq!(indices(&filter_log(&all, LogLevel::Info, "")), [0, 2, 3, 4]);
        // the continuation line follows the error before it
        assert_eq!(indices(&filter_log(&all, LogLevel::Warn, "")), [2, 3, 4]);
        assert_eq!(indices(&filter_log(&all, LogLevel::Error, "")), [3, 4]);
        let f = filter_log(&all, LogLevel::Warn, "");
        assert_eq!(f[2].level, LogLevel::Error);
        assert_eq!(f[2].text, SAMPLE[4]);
    }

    #[test]
    fn a_continuation_line_at_the_start_counts_as_info() {
        let l = lines(&["orphan", "2026 WARN x: y"]);
        assert_eq!(indices(&filter_log(&l, LogLevel::Info, "")), [0, 1]);
        assert_eq!(indices(&filter_log(&l, LogLevel::Warn, "")), [1]);
    }

    #[test]
    fn search_is_a_case_insensitive_substring_on_top_of_the_level() {
        let all = lines(&SAMPLE);
        assert_eq!(
            indices(&filter_log(&all, LogLevel::Trace, "REGISTRY")),
            [1, 2]
        );
        assert_eq!(indices(&filter_log(&all, LogLevel::Warn, "registry")), [2]);
        assert_eq!(
            indices(&filter_log(&all, LogLevel::Trace, "continuation")),
            [4]
        );
        assert!(filter_log(&all, LogLevel::Trace, "nothing like this").is_empty());
        // whitespace is part of the needle
        assert_eq!(
            indices(&filter_log(&all, LogLevel::Trace, "window opened")),
            [0]
        );
        assert!(filter_log(&[], LogLevel::Trace, "").is_empty());
    }

    #[test]
    fn copied_text_is_the_shown_lines_joined() {
        let all = lines(&SAMPLE);
        let shown = filter_log(&all, LogLevel::Error, "");
        assert_eq!(log_text(&shown), format!("{}\n{}", SAMPLE[3], SAMPLE[4]));
        assert_eq!(log_text(&[]), "");
    }

    #[test]
    fn history_recalls_with_up_and_down_and_restores_the_draft() {
        let mut c = ConsoleState::default();
        assert_eq!(c.recall_previous("x"), None);
        assert_eq!(c.recall_next(), None);
        for e in ["one", "two", "three"] {
            c.remember(e);
        }
        c.remember("three"); // a repeat of the last is not added
        assert_eq!(c.history(), ["one", "two", "three"]);
        assert_eq!(c.recall_previous("draft").as_deref(), Some("three"));
        assert_eq!(c.recall_previous("three").as_deref(), Some("two"));
        assert_eq!(c.recall_previous("two").as_deref(), Some("one"));
        assert_eq!(c.recall_previous("one").as_deref(), Some("one")); // stays at the oldest
        assert_eq!(c.recall_next().as_deref(), Some("two"));
        assert_eq!(c.recall_next().as_deref(), Some("three"));
        assert_eq!(c.recall_next().as_deref(), Some("draft")); // past the newest: the draft
        assert_eq!(c.recall_next(), None); // no longer browsing
        // running an entry ends browsing
        c.recall_previous("");
        c.remember("four");
        assert_eq!(c.recall_previous("").as_deref(), Some("four"));
    }

    #[test]
    fn history_and_transcript_are_capped() {
        let mut c = ConsoleState::default();
        for i in 0..(MAX_HISTORY + 20) {
            c.remember(&format!("e{i}"));
        }
        assert_eq!(c.history().len(), MAX_HISTORY);
        assert_eq!(c.history()[0], "e20");
        for i in 0..(MAX_TRANSCRIPT + 5) {
            c.push(format!("l{i}"), ReplKind::Output);
        }
        assert_eq!(c.repl.len(), MAX_TRANSCRIPT);
        assert_eq!(c.repl[0].text, "l5");
    }
}
