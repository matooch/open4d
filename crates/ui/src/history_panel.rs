//! The undo history panel: the document's transactions, click to jump (docs/specs/UI.md §18.6).

use egui::{RichText, Ui};
use forge_core::Document;

use crate::theme::Theme;

/// Where a row lies relative to the current state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowKind {
    /// Applied before the current state.
    Past,
    /// The current state.
    Current,
    /// Undone, can be redone.
    Future,
}

/// One state of the document: `position` steps applied.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryRow {
    pub position: usize,
    pub label: String,
    pub kind: RowKind,
}

/// The rows for the undo labels (oldest first) and the redo labels (the last one is redone
/// next, as `History::redo_labels` returns them): "Original state", every undo step, then the
/// redo steps in the order they were done.
pub fn history_rows(undo: &[&str], redo: &[&str]) -> Vec<HistoryRow> {
    let current = undo.len();
    let labels = std::iter::once("Original state")
        .chain(undo.iter().copied())
        .chain(redo.iter().rev().copied());
    labels
        .enumerate()
        .map(|(position, label)| HistoryRow {
            position,
            label: label.to_string(),
            kind: match position.cmp(&current) {
                std::cmp::Ordering::Less => RowKind::Past,
                std::cmp::Ordering::Equal => RowKind::Current,
                std::cmp::Ordering::Greater => RowKind::Future,
            },
        })
        .collect()
}

/// The text of a row: `3. Move`.
pub fn row_text(row: &HistoryRow) -> String {
    format!("{}. {}", row.position, row.label)
}

/// Draws the list; the position of the row clicked, if any.
pub fn history_panel_ui(ui: &mut Ui, doc: &Document) -> Option<usize> {
    let theme = Theme::default();
    let history = doc.history();
    let rows = history_rows(&history.undo_labels(), &history.redo_labels());
    let mut jump = None;
    egui::ScrollArea::vertical()
        .id_salt("history_rows")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            for row in &rows {
                ui.horizontal(|ui| {
                    let text = row_text(row);
                    let (text, current) = match row.kind {
                        RowKind::Past => (RichText::new(text), false),
                        RowKind::Current => (RichText::new(text).strong(), true),
                        RowKind::Future => (RichText::new(text).weak().italics(), false),
                    };
                    if ui.add(egui::Button::selectable(current, text)).clicked()
                        && row.kind != RowKind::Current
                    {
                        jump = Some(row.position);
                    }
                    if current {
                        ui.label(RichText::new("current").small().color(theme.accent));
                    }
                });
            }
        });
    jump
}

#[cfg(test)]
mod tests {
    use super::*;

    fn texts(rows: &[HistoryRow]) -> Vec<String> {
        rows.iter().map(row_text).collect()
    }

    #[test]
    fn an_empty_history_has_only_the_original_state() {
        let rows = history_rows(&[], &[]);
        assert_eq!(texts(&rows), ["0. Original state"]);
        assert_eq!(rows[0].kind, RowKind::Current);
    }

    #[test]
    fn rows_run_from_the_original_state_to_the_newest_redo_step() {
        // Created A, B, C; then undid C and B: redo_labels is [C, B] (B is redone next).
        let rows = history_rows(&["Create A"], &["Create C", "Create B"]);
        assert_eq!(
            texts(&rows),
            [
                "0. Original state",
                "1. Create A",
                "2. Create B",
                "3. Create C"
            ]
        );
        let kinds: Vec<RowKind> = rows.iter().map(|r| r.kind).collect();
        assert_eq!(
            kinds,
            [
                RowKind::Past,
                RowKind::Current,
                RowKind::Future,
                RowKind::Future
            ]
        );
        assert!(rows.iter().enumerate().all(|(i, r)| r.position == i));
    }

    #[test]
    fn the_newest_state_is_current_without_redo_steps() {
        let rows = history_rows(&["a", "b"], &[]);
        assert_eq!(rows.last().unwrap().kind, RowKind::Current);
        assert_eq!(
            rows.iter().filter(|r| r.kind == RowKind::Current).count(),
            1
        );
    }
}
