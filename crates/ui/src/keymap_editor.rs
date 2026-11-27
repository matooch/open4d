//! The key map editor window: view, search, rebind, reset (docs/specs/UI.md §14.5).

use egui::{Event, Key, Modifiers, RichText, Ui};
use forge_command::CommandRegistry;
use serde_json::Value;

use crate::keymap::{Binding, Chord, KeyMapState, SEQUENCE_TIMEOUT, Scope, keys_text};
use crate::state::EditorState;
use crate::theme::Theme;

/// One line of the editor: a binding, or a command without one.
#[derive(Debug, Clone, PartialEq)]
pub struct Row {
    pub command: String,
    pub label: String,
    pub args: Value,
    pub scope: Scope,
    pub chord: Option<Chord>,
    /// The second key of a sequence binding (UI.md §14.7).
    pub then: Option<Chord>,
    /// The registry has the command.
    pub known: bool,
    /// The binding is not one of the defaults.
    pub changed: bool,
}

impl Row {
    fn same_binding(&self, b: &Binding) -> bool {
        self.chord == Some(b.chord)
            && self.then == b.then
            && self.command == b.command
            && self.args == b.args
    }
}

/// The rows for `filter`: every registry command in registry order (one row per binding,
/// or one unbound row), then bindings of commands the registry lacks. A row matches when
/// the trimmed, case-insensitive filter is a substring of the command id, its label, its
/// arguments text or its keys (canonical, or each chord as `shown`, joined by a space).
pub fn rows(
    keys: &KeyMapState,
    registry: &CommandRegistry,
    filter: &str,
    shown: &dyn Fn(Chord) -> String,
) -> Vec<Row> {
    let mut out = Vec::new();
    let row_of = |label: &str, known: bool, b: &Binding, changed: bool| Row {
        command: b.command.clone(),
        label: label.to_string(),
        args: b.args.clone(),
        scope: b.scope,
        chord: Some(b.chord),
        then: b.then,
        known,
        changed,
    };
    let unbound = |label: &str, known: bool, command: &str, args: &Value, scope, changed| Row {
        command: command.to_string(),
        label: label.to_string(),
        args: args.clone(),
        scope,
        chord: None,
        then: None,
        known,
        changed,
    };
    for h in registry.iter() {
        let mut any = false;
        for b in keys.map.for_command(h.id()) {
            out.push(row_of(h.label(), true, b, !keys.is_default(b)));
            any = true;
        }
        // default bindings the user has cleared or moved away
        for d in keys.defaults.for_command(h.id()) {
            if !keys
                .map
                .for_command(h.id())
                .any(|b| b.args == d.args && b.scope == d.scope)
            {
                out.push(unbound(h.label(), true, h.id(), &d.args, d.scope, true));
                any = true;
            }
        }
        if !any {
            let none = serde_json::json!({});
            out.push(unbound(
                h.label(),
                true,
                h.id(),
                &none,
                Scope::Global,
                false,
            ));
        }
    }
    for b in keys.map.bindings() {
        if registry.get(&b.command).is_none() {
            out.push(row_of(&b.command, false, b, !keys.is_default(b)));
        }
    }
    let q = filter.trim().to_lowercase();
    if q.is_empty() {
        return out;
    }
    let args_text = |r: &Row| {
        Binding {
            args: r.args.clone(),
            scope: r.scope,
            ..Binding::new(Chord::new(Key::A), "", serde_json::json!({}))
        }
        .args_summary()
    };
    out.retain(|r| {
        let mut hay = format!("{} {} {}", r.command, r.label, args_text(r));
        if let Some(c) = r.chord {
            let shown_keys = match r.then {
                Some(t) => format!("{} {}", shown(c), shown(t)),
                None => shown(c),
            };
            hay = format!("{hay} {} {shown_keys}", keys_text(c, r.then));
        }
        hay.to_lowercase().contains(&q)
    });
    out
}

/// What the next key press means while capturing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Captured {
    /// Escape: stop capturing.
    Cancel,
    Chord(Chord),
}

/// Takes the first key press of the frame as a capture result (consuming it).
pub fn take_captured(input: &mut egui::InputState) -> Option<Captured> {
    let pos = input
        .events
        .iter()
        .position(|e| matches!(e, Event::Key { pressed: true, .. }))?;
    let Event::Key { key, modifiers, .. } = input.events.remove(pos) else {
        return None;
    };
    Some(if key == Key::Escape && modifiers == Modifiers::NONE {
        Captured::Cancel
    } else {
        Captured::Chord(Chord::from_press(&modifiers, key))
    })
}

/// A rebind waiting for an answer because the keys are taken.
#[derive(Debug, Clone)]
struct Pending {
    chord: Chord,
    then: Option<Chord>,
    row: Row,
    /// Labels of the bindings that would be removed.
    taken_by: Vec<String>,
}

/// Editor window state; the key map itself lives in `EditorState.keys`.
#[derive(Default)]
pub struct KeyMapEditor {
    pub open: bool,
    pub filter: String,
    capture: Option<Row>,
    /// The first key of the capture, with the time it was pressed (UI.md §14.7): a second
    /// key within [`SEQUENCE_TIMEOUT`] makes a sequence, the timeout keeps the single key.
    first: Option<(Chord, f64)>,
    pending: Option<Pending>,
    /// The last storing problem, if any.
    pub message: String,
}

impl KeyMapEditor {
    pub fn is_capturing(&self) -> bool {
        self.capture.is_some()
    }

    /// True while a rebind waits for Replace or Cancel.
    pub fn has_pending(&self) -> bool {
        self.pending.is_some()
    }

    /// Starts capturing new keys for `row`.
    pub fn start_capture(&mut self, row: Row) {
        self.pending = None;
        self.first = None;
        self.capture = Some(row);
    }

    /// The first key of a capture that waits for a possible second key.
    pub fn first_key(&self) -> Option<Chord> {
        self.first.map(|(c, _)| c)
    }

    /// A key press at time `now` while capturing (UI.md §14.5, §14.7). Escape cancels. The
    /// first press waits for a second one: a second press within [`SEQUENCE_TIMEOUT`] binds
    /// the two-key sequence at once; without one, [`KeyMapEditor::tick`] binds the single
    /// key when the time is up. Keys that are taken wait for Replace or Cancel (§14.4).
    pub fn captured(&mut self, result: Captured, keys: &mut KeyMapState, now: f64) {
        if self.capture.is_none() {
            return;
        }
        let Captured::Chord(chord) = result else {
            self.cancel();
            return;
        };
        if let Some((first, at)) = self.first.take() {
            if now - at <= SEQUENCE_TIMEOUT {
                self.finish(first, Some(chord), keys);
                return;
            }
            // too late for a sequence: the first key stands alone, this press is dropped
            self.finish(first, None, keys);
            return;
        }
        if crate::keymap::is_reserved(chord) {
            self.capture = None;
            self.message = "1, 2 and 3 are navigation keys and cannot be bound".to_string();
            return;
        }
        self.first = Some((chord, now));
    }

    /// Time passing during a capture: a first key that got no second key within the
    /// timeout is bound as a single key. True while a first key is still waiting (the
    /// window should repaint).
    pub fn tick(&mut self, keys: &mut KeyMapState, now: f64) -> bool {
        match self.first {
            Some((first, at)) if now - at > SEQUENCE_TIMEOUT => {
                self.first = None;
                self.finish(first, None, keys);
                false
            }
            Some(_) => true,
            None => false,
        }
    }

    /// Ends the capture with `chord` (and `then`): bound at once when free, else asks.
    fn finish(&mut self, chord: Chord, then: Option<Chord>, keys: &mut KeyMapState) {
        let Some(row) = self.capture.take() else {
            return;
        };
        if row.chord == Some(chord) && row.then == then {
            return;
        }
        if then.is_some_and(crate::keymap::is_reserved) {
            self.message = "1, 2 and 3 are navigation keys and cannot be bound".to_string();
            return;
        }
        self.message.clear();
        let taken_by: Vec<String> = keys
            .map
            .conflicts_with_keys(chord, then, row.scope)
            .iter()
            .map(|b| b.command.clone())
            .collect();
        if taken_by.is_empty() {
            self.apply(chord, then, &row, keys);
        } else {
            self.pending = Some(Pending {
                chord,
                then,
                row,
                taken_by,
            });
        }
    }

    /// Replace: rebind and drop the bindings that held the keys.
    pub fn replace_pending(&mut self, keys: &mut KeyMapState) {
        if let Some(p) = self.pending.take() {
            self.apply(p.chord, p.then, &p.row, keys);
        }
    }

    pub fn cancel(&mut self) {
        self.pending = None;
        self.capture = None;
        self.first = None;
    }

    fn apply(&mut self, chord: Chord, then: Option<Chord>, row: &Row, keys: &mut KeyMapState) {
        if let Some(i) = keys.map.bindings().iter().position(|b| row.same_binding(b)) {
            keys.map.remove(i);
        }
        keys.map.bind(Binding {
            then,
            args: row.args.clone(),
            scope: row.scope,
            ..Binding::new(chord, row.command.clone(), serde_json::json!({}))
        });
        self.store(keys);
    }

    /// Unbinds the row's chord.
    pub fn clear(&mut self, row: &Row, keys: &mut KeyMapState) {
        if let Some(i) = keys.map.bindings().iter().position(|b| row.same_binding(b)) {
            keys.map.remove(i);
            self.store(keys);
        }
    }

    /// Puts the defaults back on the row's chord.
    pub fn reset(&mut self, row: &Row, keys: &mut KeyMapState) {
        keys.reset_binding(&row.command, &row.args);
        self.store(keys);
    }

    pub fn reset_all(&mut self, keys: &mut KeyMapState) {
        self.cancel();
        keys.reset_all();
        self.store(keys);
    }

    fn store(&mut self, keys: &KeyMapState) {
        self.message = keys.save().err().unwrap_or_default();
    }

    /// Draws the window when open.
    pub fn show(&mut self, ctx: &egui::Context, state: &mut EditorState) {
        let mut open = self.open;
        let now = ctx.input(|i| i.time);
        if self.tick(&mut state.keys, now) {
            ctx.request_repaint_after(std::time::Duration::from_millis(200));
        }
        egui::Window::new("Key map")
            .open(&mut open)
            .default_size([560.0, 460.0])
            .show(ctx, |ui| self.ui(ui, state));
        self.open = open;
        if !open {
            self.cancel();
        }
    }

    fn ui(&mut self, ui: &mut Ui, state: &mut EditorState) {
        let theme = Theme::current(ui.ctx());
        let registry = state.app.commands.clone();
        ui.horizontal(|ui| {
            let label = ui.label("Search");
            ui.text_edit_singleline(&mut self.filter)
                .labelled_by(label.id);
            if ui.button("Reset all").clicked() {
                self.reset_all(&mut state.keys);
            }
        });
        let conflicts = state.keys.map.conflicts().len();
        let unknown = state.keys.map.unknown_commands(&registry).len();
        if conflicts > 0 {
            ui.colored_label(theme.error, format!("{conflicts} conflicting chord(s)"));
        }
        if unknown > 0 {
            ui.colored_label(theme.error, format!("{unknown} unknown command(s)"));
        }
        for w in &state.keys.warnings {
            ui.colored_label(theme.error, w);
        }
        if !self.message.is_empty() {
            ui.colored_label(theme.error, &self.message);
        }
        if let Some(p) = self.pending.clone() {
            ui.group(|ui| {
                let keys_shown = match p.then {
                    Some(t) => format!("{} {}", p.chord.display(ui.ctx()), t.display(ui.ctx())),
                    None => p.chord.display(ui.ctx()),
                };
                ui.label(format!(
                    "{keys_shown} is bound to {}.",
                    p.taken_by.join(", ")
                ));
                ui.horizontal(|ui| {
                    if ui.button("Replace").clicked() {
                        self.replace_pending(&mut state.keys);
                    }
                    if ui.button("Cancel").clicked() {
                        self.cancel();
                    }
                });
            });
        }
        ui.separator();
        let ctx = ui.ctx().clone();
        let shown = move |c: Chord| c.display(&ctx);
        let list = rows(&state.keys, &registry, &self.filter, &shown);
        egui::ScrollArea::vertical().show(ui, |ui| {
            egui::Grid::new("keymap_rows")
                .num_columns(5)
                .striped(true)
                .show(ui, |ui| {
                    for row in &list {
                        self.row_ui(ui, row, &mut state.keys);
                        ui.end_row();
                    }
                });
        });
    }

    fn row_ui(&mut self, ui: &mut Ui, row: &Row, keys: &mut KeyMapState) {
        let name = if row.known {
            RichText::new(&row.label)
        } else {
            RichText::new(format!("{} (unknown)", row.label)).color(Theme::current(ui.ctx()).error)
        };
        ui.label(name);
        let args = Binding {
            args: row.args.clone(),
            scope: row.scope,
            ..Binding::new(Chord::new(Key::A), "", serde_json::json!({}))
        }
        .args_summary();
        ui.weak(if args.is_empty() {
            row.command.clone()
        } else {
            format!("{} ({args})", row.command)
        });
        let capturing = self.capture.as_ref().is_some_and(|c| {
            c.command == row.command
                && c.args == row.args
                && c.chord == row.chord
                && c.then == row.then
        });
        let text = if capturing {
            match self.first_key() {
                Some(c) => format!("{} \u{2026}", c.display(ui.ctx())),
                None => "Press keys\u{2026}".to_string(),
            }
        } else {
            match (row.chord, row.then) {
                (Some(c), Some(t)) => format!("{} {}", c.display(ui.ctx()), t.display(ui.ctx())),
                (Some(c), None) => c.display(ui.ctx()),
                (None, _) => "(none)".to_string(),
            }
        };
        if ui
            .button(text)
            .on_hover_text(
                "Click, then press the new key; press a second key within 1.5 s for a \
                 two-key sequence (Esc cancels)",
            )
            .clicked()
        {
            self.start_capture(row.clone());
        }
        if ui
            .add_enabled(row.chord.is_some(), egui::Button::new("Clear"))
            .clicked()
        {
            self.clear(row, keys);
        }
        if ui
            .add_enabled(row.changed, egui::Button::new("Reset"))
            .clicked()
        {
            self.reset(row, keys);
        }
    }
}
