//! The key map editor window: view, search, rebind, reset (docs/specs/UI.md §14.5).

use egui::{Event, Key, Modifiers, RichText, Ui};
use forge_command::CommandRegistry;
use serde_json::Value;

use crate::keymap::{Binding, Chord, KeyMapState, Scope};
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
    /// The registry has the command.
    pub known: bool,
    /// The binding is not one of the defaults.
    pub changed: bool,
}

impl Row {
    fn same_binding(&self, b: &Binding) -> bool {
        self.chord == Some(b.chord) && self.command == b.command && self.args == b.args
    }
}

/// The rows for `filter`: every registry command in registry order (one row per binding,
/// or one unbound row), then bindings of commands the registry lacks. A row matches when
/// the trimmed, case-insensitive filter is a substring of the command id, its label, its
/// arguments text or its chord (canonical or `shown`).
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
        known,
        changed,
    };
    let unbound = |label: &str, known: bool, command: &str, args: &Value, scope, changed| Row {
        command: command.to_string(),
        label: label.to_string(),
        args: args.clone(),
        scope,
        chord: None,
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
            chord: Chord::new(Key::A),
            command: String::new(),
            args: r.args.clone(),
            scope: r.scope,
        }
        .args_summary()
    };
    out.retain(|r| {
        let mut hay = format!("{} {} {}", r.command, r.label, args_text(r));
        if let Some(c) = r.chord {
            hay = format!("{hay} {c} {}", shown(c));
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

/// A rebind waiting for an answer because the chord is taken.
#[derive(Debug, Clone)]
struct Pending {
    chord: Chord,
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

    /// Starts capturing a new chord for `row`.
    pub fn start_capture(&mut self, row: Row) {
        self.pending = None;
        self.capture = Some(row);
    }

    /// A key press while capturing: Escape cancels; a free chord is bound at once; a chord
    /// that is taken waits for Replace or Cancel (UI.md §14.4).
    pub fn captured(&mut self, result: Captured, keys: &mut KeyMapState) {
        let Some(row) = self.capture.take() else {
            return;
        };
        let Captured::Chord(chord) = result else {
            return;
        };
        if row.chord == Some(chord) {
            return;
        }
        let taken_by: Vec<String> = keys
            .map
            .conflicts_with(chord, row.scope)
            .iter()
            .map(|b| b.command.clone())
            .collect();
        if taken_by.is_empty() {
            self.apply(chord, &row, keys);
        } else {
            self.pending = Some(Pending {
                chord,
                row,
                taken_by,
            });
        }
    }

    /// Replace: rebind and drop the bindings that held the chord.
    pub fn replace_pending(&mut self, keys: &mut KeyMapState) {
        if let Some(p) = self.pending.take() {
            self.apply(p.chord, &p.row, keys);
        }
    }

    pub fn cancel(&mut self) {
        self.pending = None;
        self.capture = None;
    }

    fn apply(&mut self, chord: Chord, row: &Row, keys: &mut KeyMapState) {
        if let Some(i) = keys.map.bindings().iter().position(|b| row.same_binding(b)) {
            keys.map.remove(i);
        }
        keys.map.bind(Binding {
            chord,
            command: row.command.clone(),
            args: row.args.clone(),
            scope: row.scope,
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
                ui.label(format!(
                    "{} is bound to {}.",
                    p.chord.display(ui.ctx()),
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
            chord: Chord::new(Key::A),
            command: String::new(),
            args: row.args.clone(),
            scope: row.scope,
        }
        .args_summary();
        ui.weak(if args.is_empty() {
            row.command.clone()
        } else {
            format!("{} ({args})", row.command)
        });
        let capturing = self.capture.as_ref().is_some_and(|c| {
            c.command == row.command && c.args == row.args && c.chord == row.chord
        });
        let text = if capturing {
            "Press keys\u{2026}".to_string()
        } else {
            row.chord
                .map_or_else(|| "(none)".to_string(), |c| c.display(ui.ctx()))
        };
        if ui
            .button(text)
            .on_hover_text("Click, then press the new keys (Esc cancels)")
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
