//! Attribute manager widgets, one module per parameter kind (docs/specs/UI.md §12).
//!
//! A widget draws one row's editor from an [`AttrRow`] and returns an [`Edit`] when the user
//! changed something. It never touches the document: the editor turns the edit into a
//! `param.edit` command. Own design, **parity-pending G4**.

use forge_core::{NodeId, Scene};
use serde_json::{Value, json};

use crate::attributes::AttrRow;

pub mod boolean;
pub mod colour;
pub mod enumeration;
pub mod float;
pub mod int;
pub mod link;
pub mod link_list;
pub mod number;
pub mod readonly;
pub mod scrub;
pub mod string;
pub mod vector;

/// How an edit relates to the undo history.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    /// A single, final change: its own undo step.
    Commit,
    /// Part of a continuous gesture (scrub, colour drag, typing); merged into one step.
    Drag { started: bool },
}

/// A requested change, in display units (the arguments of `param.edit`).
#[derive(Debug, Clone, PartialEq)]
pub struct Edit {
    pub value: Value,
    /// Vec3: the one component `value` (a number) sets.
    pub component: Option<u8>,
    pub phase: Phase,
}

impl Edit {
    pub fn commit(value: Value) -> Edit {
        Edit {
            value,
            component: None,
            phase: Phase::Commit,
        }
    }

    /// Arguments of `param.edit` for the objects `nodes`.
    pub fn args(&self, param: &str, nodes: &[NodeId], gesture: Option<u64>) -> Value {
        let mut a = json!({
            "param": param,
            "value": self.value,
            "nodes": nodes.iter().map(|n| n.0).collect::<Vec<_>>(),
        });
        if let Some(c) = self.component {
            a["component"] = json!(c);
        }
        if let Some(g) = gesture {
            a["gesture"] = json!(g);
        }
        a
    }
}

/// What a widget needs besides its row.
pub struct Cx<'a> {
    pub scene: &'a Scene,
    /// The active object.
    pub node: NodeId,
    /// The row label's widget id, so the editor is announced under the label.
    pub label: egui::Id,
}

/// Draws the editor of `row`, whatever its kind.
pub fn show(ui: &mut egui::Ui, cx: &Cx, row: &AttrRow) -> Option<Edit> {
    use crate::attributes::Widget as W;
    match &row.widget {
        W::Checkbox => boolean::show(ui, cx, row),
        W::IntDrag { min, max } => int::show(ui, cx, row, *min, *max),
        W::FloatDrag { unit, .. } => float::show(ui, cx, row, *unit),
        W::Vec3Drag { unit, .. } => vector::show(ui, cx, row, *unit),
        W::Color => colour::show(ui, cx, row),
        W::Combo { options } => enumeration::show(ui, cx, row, options),
        W::Text => string::show(ui, cx, row),
        W::Link { accepts } => link::show(ui, cx, row, accepts),
        W::LinkList { accepts } => link_list::show(ui, cx, row, accepts),
        W::ReadOnly => readonly::show(ui, row),
    }
}
