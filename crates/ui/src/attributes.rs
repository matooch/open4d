//! Attribute panel rows derived from parameter descriptors (docs/specs/UI.md §4).

use forge_core::{
    Category, Document, EditError, EnumOption, NodeId, ParamKind, Scene, Unit, Value,
};

use crate::animate::{AnimCtx, write_param};
use crate::tree::type_label;

#[derive(Debug, Clone, PartialEq)]
pub enum Widget {
    Checkbox,
    IntDrag {
        min: i64,
        max: i64,
    },
    FloatDrag {
        min: f64,
        max: f64,
        unit: Unit,
    },
    Vec3Drag {
        min: f64,
        max: f64,
        unit: Unit,
    },
    Color,
    Combo {
        options: Vec<EnumOption>,
    },
    Text,
    Link {
        accepts: Vec<Category>,
    },
    /// Raw value of a node whose type is not registered.
    ReadOnly,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AttrRow {
    pub param: String,
    pub label: String,
    pub widget: Widget,
    pub value: Value,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AttributeView {
    pub name: String,
    pub type_label: String,
    pub editable: bool,
    pub groups: Vec<(String, Vec<AttrRow>)>,
}

const DEFAULT_GROUP: &str = "Parameters";

fn widget(kind: &ParamKind, unit: Unit) -> Widget {
    match kind {
        ParamKind::Bool => Widget::Checkbox,
        ParamKind::Int { min, max } => Widget::IntDrag {
            min: *min,
            max: *max,
        },
        ParamKind::Float { min, max } => Widget::FloatDrag {
            min: *min,
            max: *max,
            unit,
        },
        ParamKind::Vec3 { min, max } => Widget::Vec3Drag {
            min: *min,
            max: *max,
            unit,
        },
        ParamKind::Color => Widget::Color,
        ParamKind::Enum { options } => Widget::Combo {
            options: options.clone(),
        },
        ParamKind::String => Widget::Text,
        ParamKind::Link { accepts } => Widget::Link {
            accepts: accepts.clone(),
        },
        ParamKind::Mesh | ParamKind::IndexList => Widget::ReadOnly,
    }
}

fn push(groups: &mut Vec<(String, Vec<AttrRow>)>, group: &str, row: AttrRow) {
    let group = if group.is_empty() {
        DEFAULT_GROUP
    } else {
        group
    };
    match groups.iter_mut().find(|(g, _)| g == group) {
        Some((_, rows)) => rows.push(row),
        None => groups.push((group.to_string(), vec![row])),
    }
}

/// The attribute view of a node, or `None` if it does not exist.
pub fn attribute_rows(scene: &Scene, id: NodeId) -> Option<AttributeView> {
    let node = scene.node(id)?;
    let (label, missing) = type_label(scene, id);
    let mut groups = Vec::new();
    match scene.node_type(id) {
        Some(ty) => {
            for desc in ty.params.iter().filter(|d| !d.ui.hidden) {
                let value = node
                    .params
                    .get(&desc.id)
                    .cloned()
                    .unwrap_or_else(|| desc.default.clone());
                let row = AttrRow {
                    param: desc.id.clone(),
                    label: desc.label.clone(),
                    widget: widget(&desc.kind, desc.ui.unit),
                    value,
                };
                push(&mut groups, &desc.ui.group, row);
            }
        }
        None => {
            for (param, value) in &node.params {
                let row = AttrRow {
                    param: param.clone(),
                    label: param.clone(),
                    widget: Widget::ReadOnly,
                    value: value.clone(),
                };
                push(&mut groups, "", row);
            }
        }
    }
    Some(AttributeView {
        name: node.name.clone(),
        type_label: label,
        editable: !missing,
        groups,
    })
}

/// Stored value → value shown to the user.
pub fn to_display(unit: Unit, v: f64) -> f64 {
    match unit {
        Unit::Angle => v.to_degrees(),
        Unit::Percent => v * 100.0,
        _ => v,
    }
}

/// Value typed by the user → stored value.
pub fn from_display(unit: Unit, v: f64) -> f64 {
    match unit {
        Unit::Angle => v.to_radians(),
        Unit::Percent => v / 100.0,
        _ => v,
    }
}

/// Suffix shown after a value in this unit.
pub fn unit_suffix(unit: Unit) -> &'static str {
    match unit {
        Unit::Angle => "°",
        Unit::Percent => " %",
        _ => "",
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Gesture {
    /// A single, final change.
    Commit,
    /// Part of the continuous drag with this id; merged into one undo step.
    Drag(u64),
}

/// Applies a parameter edit as an undoable step.
pub fn apply_edit(
    doc: &mut Document,
    id: NodeId,
    param: &str,
    value: Value,
    gesture: Gesture,
) -> Result<(), EditError> {
    apply_edit_anim(doc, id, param, value, gesture, None)
}

/// [`apply_edit`] with animation: autokey and rule 5 write keys (docs/specs/ANIM.md §2.4).
pub fn apply_edit_anim(
    doc: &mut Document,
    id: NodeId,
    param: &str,
    value: Value,
    gesture: Gesture,
    ctx: Option<&AnimCtx>,
) -> Result<(), EditError> {
    let label = doc
        .scene()
        .node_type(id)
        .and_then(|t| t.find_param(param))
        .map_or_else(|| param.to_string(), |d| d.label.clone());
    let label = format!("Set {label}");
    let edit = |tx: &mut forge_core::Tx| write_param(tx, id, param, value, ctx);
    match gesture {
        Gesture::Commit => doc.transact(&label, edit),
        Gesture::Drag(n) => doc.transact_merged(&label, &format!("drag:{id}:{param}:{n}"), edit),
    }
}
